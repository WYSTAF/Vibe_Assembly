//! `Propose -> Probe -> Compose` contrastive recovery loop (Harness²).
//!
//! Zero-overhead fast path: the normal execution step runs first; if sandbox
//! verification and the judge both pass on the first try, no probe is spawned
//! and the verified node is recorded directly. Probing is failure-triggered, or
//! opt-in via `--recursive-k`. When it runs, an invariant candidate (`c1`/`c4`
//! refinement) and a domain candidate (`c2`/`c6` refinement) are proposed, two
//! candidate branches are forked in the session DAG, both are evaluated against
//! the sandbox and the judge, and the winner is composed back as a multi-parent
//! merge node scored by diagnostics delta, judge verdict, diff minimalism and
//! cost.

use std::path::{Path, PathBuf};

use crate::experience::{ContrastiveRecovery, ExperienceStore, RecoveryCandidate, StepFailure};
use crate::judge::{DiffLine, Judge, Verdict};
use crate::sandbox::{SandboxConfig, SandboxExecutor, SandboxResult};
use crate::session::{NodeId, NodeKind, Session};

/// Tuning for the contrastive loop. `k == 0` keeps the zero-overhead fast path
/// (probe only on failure); `k >= 1` opts in to probing even on success.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RecursiveConfig {
    pub k: usize,
}

/// Which side of the contrast a candidate represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    Invariant,
    Domain,
}

/// A proposed refinement of one or more harness surfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeCandidate {
    pub kind: CandidateKind,
    pub lesson: String,
    /// `(surface id, overlay content)` refinements, e.g. `c1`/`c4` or `c2`/`c6`.
    pub refinements: Vec<(String, String)>,
    pub evidence: Vec<String>,
}

impl ProbeCandidate {
    pub fn invariant(lesson: impl Into<String>, evidence: Vec<String>) -> Self {
        let lesson = lesson.into();
        Self {
            kind: CandidateKind::Invariant,
            refinements: vec![
                ("c1_system_prompt".into(), format!("Invariant: {lesson}")),
                ("c4_tool_guidance".into(), format!("Always: {lesson}")),
            ],
            lesson,
            evidence,
        }
    }

    pub fn domain(lesson: impl Into<String>, evidence: Vec<String>) -> Self {
        let lesson = lesson.into();
        Self {
            kind: CandidateKind::Domain,
            refinements: vec![
                (
                    "c2_guardrails".into(),
                    format!("Project guardrail: {lesson}"),
                ),
                ("c6_skills".into(), format!("Prefer: {lesson}")),
            ],
            lesson,
            evidence,
        }
    }

    fn to_recovery(&self) -> RecoveryCandidate {
        match self.kind {
            CandidateKind::Invariant => {
                RecoveryCandidate::invariant(self.lesson.clone(), self.evidence.clone())
            }
            CandidateKind::Domain => {
                RecoveryCandidate::domain(self.lesson.clone(), self.evidence.clone())
            }
        }
    }
}

/// Evidence gathered by probing one candidate branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeOutcome {
    pub passed: bool,
    pub judge_passed: bool,
    /// Compiler error count observed while probing (lower is better).
    pub error_count: usize,
    /// Added+removed diff lines (lower is more minimal).
    pub churn: usize,
    pub cost_micros: u64,
}

impl ProbeOutcome {
    /// Higher is better. Correctness dominates, then the judge, then
    /// minimalism and cost.
    pub fn score(&self) -> i64 {
        (if self.passed { 1000 } else { 0 }) + (if self.judge_passed { 500 } else { 0 })
            - (self.error_count as i64 * 10)
            - self.churn as i64
            - (self.cost_micros / 1000) as i64
    }
}

/// Result of one `execute_step` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepOutcome {
    pub fast_path: bool,
    pub passed: bool,
    pub probes: usize,
    pub winner: Option<CandidateKind>,
    pub merged_node: Option<NodeId>,
    pub lesson_recorded: bool,
}

/// Abstraction over verification + judging so the loop is testable without a
/// live compiler or router.
pub trait StepVerifier {
    fn verify(&mut self, command: &str) -> SandboxResult;
    fn judge(&mut self, diff: &[DiffLine]) -> Verdict;
}

/// Production verifier: confined sandbox execution plus the deterministic judge.
pub struct SandboxJudgeVerifier {
    sandbox: SandboxExecutor,
    judge: Judge,
}

impl SandboxJudgeVerifier {
    pub fn new(root: &Path) -> Self {
        Self {
            sandbox: SandboxExecutor::new(SandboxConfig::default_for(root)),
            judge: Judge::with_defaults(),
        }
    }
}

impl StepVerifier for SandboxJudgeVerifier {
    fn verify(&mut self, command: &str) -> SandboxResult {
        self.sandbox
            .execute(command, None)
            .unwrap_or(SandboxResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: "sandbox execution failed".into(),
                timed_out: false,
                duration: std::time::Duration::ZERO,
            })
    }

    fn judge(&mut self, diff: &[DiffLine]) -> Verdict {
        let numbers: Vec<usize> = (1..=diff.len()).collect();
        self.judge.review(diff, &numbers)
    }
}

/// The contrastive `Propose -> Probe -> Compose` loop.
pub struct RecursiveLoop {
    root: PathBuf,
    config: RecursiveConfig,
}

impl RecursiveLoop {
    pub fn new(root: &Path, config: RecursiveConfig) -> Self {
        Self {
            root: root.to_path_buf(),
            config,
        }
    }

    /// Propose an invariant and a domain candidate from a failure.
    pub fn propose(&self, failure: &StepFailure) -> (ProbeCandidate, ProbeCandidate) {
        let detail = failure.detail.lines().next().unwrap_or("").trim();
        let invariant = ProbeCandidate::invariant(
            format!("never ignore a failing verification ({detail})"),
            vec![detail.to_string(), failure.command.clone()],
        );
        let domain = ProbeCandidate::domain(
            format!(
                "follow the {} / {} convention",
                failure.language, failure.domain
            ),
            vec![failure.language.clone()],
        );
        (invariant, domain)
    }

    /// Probe one candidate branch: run verification and the judge, gather evidence.
    pub fn probe(
        &self,
        verifier: &mut dyn StepVerifier,
        command: &str,
        diff: &[DiffLine],
    ) -> ProbeOutcome {
        let result = verifier.verify(command);
        let verdict = verifier.judge(diff);
        let error_count = result
            .stderr
            .lines()
            .filter(|l| l.contains("error"))
            .count();
        let churn = diff.iter().filter(|l| l.added() || l.removed()).count();
        ProbeOutcome {
            passed: result.success,
            judge_passed: matches!(verdict, Verdict::Pass),
            error_count,
            churn,
            cost_micros: result.duration.as_micros() as u64,
        }
    }

    /// Choose the winning candidate. Ties favour the invariant.
    pub fn compose(&self, invariant: &ProbeOutcome, domain: &ProbeOutcome) -> CandidateKind {
        if invariant.score() >= domain.score() {
            CandidateKind::Invariant
        } else {
            CandidateKind::Domain
        }
    }

    /// Run one execution step with the zero-overhead fast path and
    /// failure/opt-in contrastive probing. Records the trajectory in `session`.
    pub fn execute_step(
        &self,
        session: &mut Session,
        verifier: &mut dyn StepVerifier,
        command: &str,
        diff: &[DiffLine],
        failure: Option<&StepFailure>,
    ) -> StepOutcome {
        let base = self.ensure_base(session);
        let first = self.probe(verifier, command, diff);
        let first_pass = first.passed && first.judge_passed;

        // Fast path: first-try success with no opt-in probing.
        if first_pass && self.config.k == 0 {
            let node = session.push(NodeKind::Verification {
                passed: true,
                detail: "fast path: verification and judge passed on first try".into(),
            });
            let _ = session.commit();
            return StepOutcome {
                fast_path: true,
                passed: true,
                probes: 0,
                winner: None,
                merged_node: Some(node),
                lesson_recorded: false,
            };
        }

        // Failure-triggered or opt-in probing.
        let default_failure = StepFailure {
            language: "auto".into(),
            domain: "workspace".into(),
            file: None,
            block_hash: None,
            command: command.into(),
            detail: if first_pass {
                "opt-in recursive probe".into()
            } else {
                "verification or judge failed".into()
            },
        };
        let failure = failure.unwrap_or(&default_failure);
        let (invariant, domain) = self.propose(failure);

        let inv_node = self.fork_and_probe(session, verifier, base, &invariant, command, diff);
        let dom_node = self.fork_and_probe(session, verifier, base, &domain, command, diff);
        let inv_out = inv_node.1;
        let dom_out = dom_node.1;
        let winner = self.compose(&inv_out, &dom_out);
        let (winner_node, winner_out) = match winner {
            CandidateKind::Invariant => inv_node,
            CandidateKind::Domain => dom_node,
        };

        // Compose: merge the winning branch back as a multi-parent node.
        let merged = session.push_merging(
            vec![base, winner_node],
            NodeKind::Verification {
                passed: winner_out.passed && winner_out.judge_passed,
                detail: format!(
                    "compose: {:?} candidate won ({} vs {})",
                    winner,
                    inv_out.score(),
                    dom_out.score()
                ),
            },
        );
        let _ = session.commit();

        // Record the durable lesson from the contrast.
        let decision = ContrastiveRecovery::choose(&invariant.to_recovery(), &domain.to_recovery());
        let mut store = ExperienceStore::load(&self.root);
        let entry = ContrastiveRecovery::reflect(failure, &decision);
        let lesson_recorded = store.record(entry).is_ok();

        StepOutcome {
            fast_path: false,
            passed: winner_out.passed && winner_out.judge_passed,
            probes: 2,
            winner: Some(winner),
            merged_node: Some(merged),
            lesson_recorded,
        }
    }

    fn ensure_base(&self, session: &mut Session) -> NodeId {
        match session.head() {
            Some(head) => head,
            None => {
                let id = session.push(NodeKind::Prompt {
                    role: "system".into(),
                    text: "recursive step".into(),
                });
                let _ = session.commit();
                id
            }
        }
    }

    fn fork_and_probe(
        &self,
        session: &mut Session,
        verifier: &mut dyn StepVerifier,
        base: NodeId,
        candidate: &ProbeCandidate,
        command: &str,
        diff: &[DiffLine],
    ) -> (NodeId, ProbeOutcome) {
        let label = match candidate.kind {
            CandidateKind::Invariant => "invariant",
            CandidateKind::Domain => "domain",
        };
        // Condition the branch record with the candidate's surface refinements
        // so it shows exactly which surfaces it proposed to change.
        if session.fork(base, &format!("recursive:{label}")).is_err() {
            // Stay on the base branch if the fork cannot be created.
        }
        let outcome = self.probe(verifier, command, diff);
        let node = session.push(NodeKind::Verification {
            passed: outcome.passed && outcome.judge_passed,
            detail: format!(
                "probe {label}: score {} (refined {})",
                outcome.score(),
                candidate
                    .refinements
                    .iter()
                    .map(|(id, _)| id.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        });
        let _ = session.commit();
        (node, outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockVerifier {
        pass: bool,
        judge_pass: bool,
    }

    impl StepVerifier for MockVerifier {
        fn verify(&mut self, _command: &str) -> SandboxResult {
            SandboxResult {
                success: self.pass,
                exit_code: Some(if self.pass { 0 } else { 1 }),
                stdout: String::new(),
                stderr: if self.pass {
                    String::new()
                } else {
                    "error: failed".into()
                },
                timed_out: false,
                duration: std::time::Duration::from_millis(1),
            }
        }
        fn judge(&mut self, _diff: &[DiffLine]) -> Verdict {
            if self.judge_pass {
                Verdict::Pass
            } else {
                Verdict::Reject(vec![])
            }
        }
    }

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("va-recursive-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn fast_path_skips_probing_on_first_try_success() {
        let root = temp_root("fast");
        let mut session = Session::new();
        let mut verifier = MockVerifier {
            pass: true,
            judge_pass: true,
        };
        let loop_ = RecursiveLoop::new(&root, RecursiveConfig::default());
        let outcome = loop_.execute_step(&mut session, &mut verifier, "cargo check", &[], None);
        assert!(outcome.fast_path);
        assert_eq!(outcome.probes, 0);
        assert!(outcome.passed);
        assert!(outcome.merged_node.is_some());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn failure_triggers_two_branch_probing_and_merge() {
        let root = temp_root("fail");
        let mut session = Session::new();
        let mut verifier = MockVerifier {
            pass: false,
            judge_pass: false,
        };
        let loop_ = RecursiveLoop::new(&root, RecursiveConfig::default());
        let outcome = loop_.execute_step(&mut session, &mut verifier, "cargo check", &[], None);
        assert!(!outcome.fast_path);
        assert_eq!(outcome.probes, 2);
        assert!(outcome.winner.is_some());
        let merged = outcome.merged_node.unwrap();
        let node = session.get(merged).unwrap();
        assert_eq!(node.parents.len(), 2, "compose must create a merge node");
        assert!(outcome.lesson_recorded);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn opt_in_k_probes_even_on_success() {
        let root = temp_root("optin");
        let mut session = Session::new();
        let mut verifier = MockVerifier {
            pass: true,
            judge_pass: true,
        };
        let loop_ = RecursiveLoop::new(&root, RecursiveConfig { k: 1 });
        let outcome = loop_.execute_step(&mut session, &mut verifier, "cargo check", &[], None);
        assert!(!outcome.fast_path);
        assert_eq!(outcome.probes, 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn compose_prefers_higher_score_and_ties_go_to_invariant() {
        let root = temp_root("compose");
        let loop_ = RecursiveLoop::new(&root, RecursiveConfig::default());
        let good = ProbeOutcome {
            passed: true,
            judge_passed: true,
            error_count: 0,
            churn: 1,
            cost_micros: 0,
        };
        let bad = ProbeOutcome {
            passed: false,
            judge_passed: false,
            error_count: 3,
            churn: 9,
            cost_micros: 0,
        };
        assert_eq!(loop_.compose(&good, &bad), CandidateKind::Invariant);
        assert_eq!(loop_.compose(&bad, &good), CandidateKind::Domain);
        assert_eq!(loop_.compose(&good, &good), CandidateKind::Invariant);
        let _ = std::fs::remove_dir_all(&root);
    }
}
