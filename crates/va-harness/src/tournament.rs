//! Multi-Branch Speculative Tournament Mode (`va-harness::tournament`).
//!
//! Evaluates multiple competing patch candidates in parallel across forked DAG branches:
//! * Forks $N$ candidate branches from the last verified node in `va-harness::session`.
//! * Tests candidates in isolated memory buffers/sandboxes.
//! * Disqualifies candidates that fail verification or trigger `judge` slop violations.
//! * Scores surviving candidates by (1) zero warnings, (2) minimal diff churn, (3) lowest cost/latency.
//! * Commits the winner and records a multi-parent merge node in the trajectory DAG.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::judge::{DiffLine, Judge, Verdict};
use crate::patcher::{apply_patch, PatchOp};
use crate::sandbox::{SandboxConfig, SandboxExecutor};
use crate::session::{NodeId, NodeKind, Session, SessionError};

/// A candidate solution proposed for a task step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidatePatch {
    pub id: String,
    pub description: String,
    pub patches: Vec<(PathBuf, PatchOp)>,
    #[serde(default)]
    pub cost_micros: u64,
    #[serde(default)]
    pub latency_ms: u64,
}

/// The evaluation outcome for one candidate branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateEvaluation {
    pub candidate_id: String,
    pub branch_node_id: NodeId,
    pub verified: bool,
    pub judge_passed: bool,
    pub findings: Vec<String>,
    pub warnings_count: usize,
    pub lines_churn: usize,
    pub cost_micros: u64,
    pub latency_ms: u64,
    pub disqualified: bool,
    pub disqualification_reason: Option<String>,
}

/// The final outcome of a speculative tournament.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentOutcome {
    pub winner_id: Option<String>,
    pub winner_branch_id: Option<NodeId>,
    pub merge_node_id: Option<NodeId>,
    pub evaluations: Vec<CandidateEvaluation>,
}

#[derive(Debug)]
pub enum TournamentError {
    Session(SessionError),
    Io(std::io::Error),
    NoCandidates,
    PatchFailed(String),
}

impl std::fmt::Display for TournamentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TournamentError::Session(e) => write!(f, "session error: {e}"),
            TournamentError::Io(e) => write!(f, "io error: {e}"),
            TournamentError::NoCandidates => {
                write!(f, "no candidate patches provided for tournament")
            }
            TournamentError::PatchFailed(s) => write!(f, "patch failed: {s}"),
        }
    }
}

impl std::error::Error for TournamentError {}

impl From<SessionError> for TournamentError {
    fn from(e: SessionError) -> Self {
        TournamentError::Session(e)
    }
}

impl From<std::io::Error> for TournamentError {
    fn from(e: std::io::Error) -> Self {
        TournamentError::Io(e)
    }
}

pub struct SpeculativeTournament {
    repo_root: PathBuf,
    sandbox: SandboxExecutor,
    judge: Judge,
    verification_commands: Vec<String>,
}

impl SpeculativeTournament {
    pub fn new(repo_root: &Path) -> Self {
        let sandbox_config = SandboxConfig::default_for(repo_root);
        Self {
            repo_root: repo_root.to_path_buf(),
            sandbox: SandboxExecutor::new(sandbox_config),
            judge: Judge::with_defaults(),
            verification_commands: Vec::new(),
        }
    }

    pub fn with_verification_command(mut self, cmd: impl Into<String>) -> Self {
        self.verification_commands.push(cmd.into());
        self
    }

    pub fn with_judge(mut self, judge: Judge) -> Self {
        self.judge = judge;
        self
    }

    /// Run the speculative tournament across candidate patches.
    pub fn run(
        &mut self,
        session: &mut Session,
        candidates: &[CandidatePatch],
    ) -> Result<TournamentOutcome, TournamentError> {
        if candidates.is_empty() {
            return Err(TournamentError::NoCandidates);
        }

        let base_node = session
            .last_verified()?
            .or_else(|| session.head())
            .unwrap_or(0);

        let mut evaluations = Vec::with_capacity(candidates.len());
        let mut original_files = std::collections::HashMap::new();

        // 1. Evaluate each candidate on its own forked DAG branch
        for candidate in candidates {
            // Fork candidate branch in session DAG
            let branch_id = if base_node > 0 {
                session.fork(
                    base_node,
                    &format!(
                        "tournament candidate {}: {}",
                        candidate.id, candidate.description
                    ),
                )?
            } else {
                let id = session.push(NodeKind::Fork {
                    reason: format!("tournament candidate {}", candidate.id),
                });
                session.commit()?;
                id
            };

            let mut eval = CandidateEvaluation {
                candidate_id: candidate.id.clone(),
                branch_node_id: branch_id,
                verified: true,
                judge_passed: true,
                findings: Vec::new(),
                warnings_count: 0,
                lines_churn: 0,
                cost_micros: candidate.cost_micros,
                latency_ms: candidate.latency_ms,
                disqualified: false,
                disqualification_reason: None,
            };

            // Backup original files before applying candidate patches
            for (rel_path, _) in &candidate.patches {
                let full = self.repo_root.join(rel_path);
                if !original_files.contains_key(&full) {
                    if let Ok(c) = std::fs::read_to_string(&full) {
                        original_files.insert(full.clone(), c);
                    }
                }
            }

            // Apply candidate patches
            let mut diff_lines = Vec::new();
            for (rel_path, op) in &candidate.patches {
                let full = self.repo_root.join(rel_path);
                let original = match original_files.get(&full) {
                    Some(s) => s.as_str(),
                    None => "",
                };

                match apply_patch(original, op) {
                    Ok(patched) => {
                        let _ = std::fs::write(&full, &patched);
                        let file_diff = compute_simple_diff(rel_path, original, &patched);
                        diff_lines.extend(file_diff);
                    }
                    Err(e) => {
                        eval.disqualified = true;
                        eval.disqualification_reason = Some(format!("patch failed: {e}"));
                        break;
                    }
                }
            }

            if !eval.disqualified {
                eval.lines_churn = diff_lines
                    .iter()
                    .filter(|l| l.marker == '+' || l.marker == '-')
                    .count();

                // Run deterministic Judge
                let line_numbers: Vec<usize> = (1..=diff_lines.len()).collect();
                let verdict = self.judge.review(&diff_lines, &line_numbers);
                match verdict {
                    Verdict::Pass => {
                        eval.judge_passed = true;
                    }
                    Verdict::Reject(findings) => {
                        eval.judge_passed = false;
                        eval.disqualified = true;
                        eval.findings = findings.iter().map(|f| format!("{f}")).collect();
                        eval.disqualification_reason =
                            Some(format!("judge rejection: {}", Judge::feedback(&findings)));
                    }
                }

                // Run verification commands if judge passed
                if !eval.disqualified {
                    for cmd in &self.verification_commands {
                        let res = self.sandbox.execute(cmd, None).map_err(|e| {
                            TournamentError::Io(std::io::Error::other(e.to_string()))
                        })?;
                        if !res.success {
                            eval.verified = false;
                            eval.disqualified = true;
                            eval.disqualification_reason = Some(format!(
                                "verification `{cmd}` failed (code {:?}): {}",
                                res.exit_code, res.stderr
                            ));
                            break;
                        }

                        // Count warning mentions in output
                        let warn_count = res
                            .stderr
                            .lines()
                            .chain(res.stdout.lines())
                            .filter(|l| l.contains("warning:") || l.contains("warning["))
                            .count();
                        eval.warnings_count += warn_count;
                    }
                }
            }

            // Restore files to clean baseline before evaluating next candidate
            for (path, content) in &original_files {
                let _ = std::fs::write(path, content);
            }

            evaluations.push(eval);
        }

        // 2. TournamentScorer: rank candidates and pick the winner
        let winner_opt = TournamentScorer::select_winner(&evaluations);

        if let Some(winner) = winner_opt {
            let winning_candidate = candidates
                .iter()
                .find(|c| c.id == winner.candidate_id)
                .expect("winner candidate must exist in input");

            // Apply winning candidate's patches to real workspace
            for (rel_path, op) in &winning_candidate.patches {
                let full = self.repo_root.join(rel_path);
                if let Ok(current) = std::fs::read_to_string(&full) {
                    if let Ok(updated) = apply_patch(&current, op) {
                        let _ = std::fs::write(&full, &updated);
                    }
                }
            }

            // Record multi-parent merge in session trajectory DAG
            let candidate_branch_ids: Vec<NodeId> =
                evaluations.iter().map(|e| e.branch_node_id).collect();

            let merge_node_id = session.push_merging(
                candidate_branch_ids,
                NodeKind::Thought {
                    text: format!(
                        "Tournament winner: candidate `{}` ({}) — churn: {} lines, warnings: {}, cost: {} micros",
                        winner.candidate_id,
                        winning_candidate.description,
                        winner.lines_churn,
                        winner.warnings_count,
                        winner.cost_micros
                    ),
                },
            );
            session.commit()?;
            let _ = session.record_telemetry(
                merge_node_id,
                Some(winner.cost_micros),
                Some(winner.latency_ms),
            );

            Ok(TournamentOutcome {
                winner_id: Some(winner.candidate_id.clone()),
                winner_branch_id: Some(winner.branch_node_id),
                merge_node_id: Some(merge_node_id),
                evaluations,
            })
        } else {
            Ok(TournamentOutcome {
                winner_id: None,
                winner_branch_id: None,
                merge_node_id: None,
                evaluations,
            })
        }
    }
}

pub struct TournamentScorer;

impl TournamentScorer {
    /// Select the highest-ranking candidate among survivors.
    ///
    /// Ranking criteria:
    /// 1. Qualified (verified && judge_passed)
    /// 2. Lowest `warnings_count` (zero warnings preferred)
    /// 3. Minimal diff churn (`lines_churn` - fewest lines touched)
    /// 4. Lowest `cost_micros` / latency
    pub fn select_winner(evaluations: &[CandidateEvaluation]) -> Option<&CandidateEvaluation> {
        let mut survivors: Vec<&CandidateEvaluation> =
            evaluations.iter().filter(|e| !e.disqualified).collect();

        if survivors.is_empty() {
            return None;
        }

        survivors.sort_by(|a, b| {
            a.warnings_count
                .cmp(&b.warnings_count)
                .then_with(|| a.lines_churn.cmp(&b.lines_churn))
                .then_with(|| a.cost_micros.cmp(&b.cost_micros))
                .then_with(|| a.latency_ms.cmp(&b.latency_ms))
        });

        survivors.first().copied()
    }
}

fn compute_simple_diff(rel_path: &Path, before: &str, after: &str) -> Vec<DiffLine> {
    let mut lines = Vec::new();
    let before_lines: Vec<&str> = before.lines().collect();
    let after_lines: Vec<&str> = after.lines().collect();
    let path_str = rel_path.to_string_lossy().replace('\\', "/");

    for b in &before_lines {
        if !after_lines.contains(b) {
            lines.push(DiffLine {
                marker: '-',
                path: path_str.clone(),
                text: b.to_string(),
            });
        }
    }

    for a in &after_lines {
        if !before_lines.contains(a) {
            lines.push(DiffLine {
                marker: '+',
                path: path_str.clone(),
                text: a.to_string(),
            });
        }
    }

    lines
}
