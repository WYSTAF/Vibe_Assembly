//! Unified execution loop: Observe → Think → Tool Call → Patch → Verify →
//! Judge → Commit.
//!
//! The loop is deliberately **model-agnostic**: the model is supplied as a
//! `Planner` trait so the harness can be tested deterministically without a
//! network, and so Vibe Router allocation can choose the implementation at
//! runtime without the harness knowing anything about routing.
//!
//! The `DONE`-sentinel reviewer pattern comes from the upstream (`pi` durable
//! harness): a reviewer re-prompted until it emits a sentinel. Here the
//! deterministic `judge` plays that role, and the single permitted retry is
//! enforced by `Judge::record_failure`.

use std::path::{Path, PathBuf};

use crate::judge::{Judge, JudgeConfig, Verdict};
use crate::patcher::{apply_patch, PatchOp};
use crate::review::{ReviewError, Reviewer};
use crate::sandbox::{SandboxConfig, SandboxExecutor};
use crate::session::{NodeKind, Session};

/// The sentinel that terminates a review loop, mirroring the upstream's
/// `DONE` convention.
pub const REVIEW_DONE: &str = "No further findings.";

/// What the model is asked to do next. Implemented by anything that can turn a
/// trajectory into a next action.
pub trait Planner {
    /// Given the trajectory so far, return the next action.
    fn next_action(&mut self, session: &Session) -> Result<Action, EngineError>;
}

/// One unit of work the loop can perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Apply a hash-anchored patch to a file.
    Patch { path: PathBuf, op: PatchOp },
    /// Run a verification command.
    Verify { command: String },
    /// Finish successfully.
    Done,
    /// Stop with a reason.
    Abort { reason: String },
}

#[derive(Debug)]
pub enum EngineError {
    Io(std::io::Error),
    Patch(crate::patcher::PatchError),
    Review(ReviewError),
    Verify {
        command: String,
        code: i32,
        stderr: String,
    },
    Rejected(String),
    Aborted(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Io(e) => write!(f, "io error: {e}"),
            EngineError::Patch(e) => write!(f, "patch failed: {e}"),
            EngineError::Review(e) => write!(f, "review failed: {e}"),
            EngineError::Verify {
                command,
                code,
                stderr,
            } => write!(f, "verification `{command}` exited {code}: {stderr}"),
            EngineError::Rejected(feedback) => {
                write!(f, "judge rejected the change:\n{feedback}")
            }
            EngineError::Aborted(r) => write!(f, "aborted: {r}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<std::io::Error> for EngineError {
    fn from(e: std::io::Error) -> Self {
        EngineError::Io(e)
    }
}

impl From<crate::patcher::PatchError> for EngineError {
    fn from(e: crate::patcher::PatchError) -> Self {
        EngineError::Patch(e)
    }
}

impl From<ReviewError> for EngineError {
    fn from(e: ReviewError) -> Self {
        EngineError::Review(e)
    }
}

impl From<crate::session::SessionError> for EngineError {
    fn from(e: crate::session::SessionError) -> Self {
        EngineError::Rejected(format!("session trajectory error: {e}"))
    }
}

/// One completed iteration of the loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepRecord {
    pub action: String,
    pub verified: bool,
    pub judge: Option<String>,
}

/// Counters for the run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EngineStats {
    pub steps: u32,
    pub patches_applied: u32,
    pub verifications_run: u32,
    pub judge_rejections: u32,
    pub retries_used: u32,
}

pub struct Engine {
    repo: PathBuf,
    reviewer: Reviewer,
    judge: Judge,
    sandbox: SandboxExecutor,
    verification_commands: Vec<String>,
    stats: EngineStats,
    /// Hard cap so a misbehaving planner cannot loop forever.
    max_steps: u32,
}

impl Engine {
    pub fn new(repo: &Path) -> Self {
        let sandbox_config = SandboxConfig::default_for(repo);
        Self {
            repo: repo.to_path_buf(),
            reviewer: Reviewer::new(repo),
            judge: Judge::with_defaults(),
            sandbox: SandboxExecutor::new(sandbox_config),
            verification_commands: Vec::new(),
            stats: EngineStats::default(),
            max_steps: 50,
        }
    }

    pub fn with_judge_config(mut self, config: JudgeConfig) -> Self {
        self.judge = Judge::new(config);
        self
    }

    pub fn with_sandbox_config(mut self, config: SandboxConfig) -> Self {
        self.sandbox = SandboxExecutor::new(config);
        self
    }

    pub fn with_verification_command(mut self, cmd: impl Into<String>) -> Self {
        self.verification_commands.push(cmd.into());
        self
    }

    pub fn with_max_steps(mut self, n: u32) -> Self {
        self.max_steps = n;
        self
    }

    pub fn stats(&self) -> &EngineStats {
        &self.stats
    }

    /// Borrow the underlying reviewer, e.g. to take a checkpoint or read the
    /// current delta.
    pub fn reviewer(&self) -> &Reviewer {
        &self.reviewer
    }

    /// Run the loop until the planner says `Done`, a verification fails, or
    /// the judge exhausts its single retry.
    pub fn run<P: Planner>(
        &mut self,
        session: &mut Session,
        planner: &mut P,
    ) -> Result<Vec<StepRecord>, EngineError> {
        let mut records = Vec::new();

        while self.stats.steps < self.max_steps {
            self.stats.steps += 1;

            // Checkpoint so this step's delta is precisely attributable.
            if self.reviewer.is_repository() {
                let _ = self.reviewer.checkpoint();
            }

            let action = planner.next_action(session)?;
            let label = describe(&action);

            match action {
                Action::Done => {
                    // Durable onYield verification gate (ported from pi/packages/durable)
                    if self.reviewer.is_repository() {
                        let verdict = self.reviewer.review_since_checkpoint(&self.judge)?;
                        if let Verdict::Reject(findings) = verdict {
                            let feedback = Judge::feedback(&findings);
                            self.stats.judge_rejections += 1;
                            records.push(StepRecord {
                                action: label.clone(),
                                verified: false,
                                judge: Some(feedback.clone()),
                            });
                            if self.judge.record_failure() {
                                self.stats.retries_used += 1;
                                session.push(NodeKind::Thought {
                                    text: format!("onYield judge feedback: {feedback}"),
                                });
                                session.commit()?;
                                continue;
                            } else {
                                session.push(NodeKind::Verification {
                                    passed: false,
                                    detail: "onYield judge rejected diff".to_string(),
                                });
                                session.commit()?;
                                session.rollback_to_last_verified()?;
                                return Err(EngineError::Rejected(feedback));
                            }
                        }
                    }

                    // Run configured verification commands through sandboxed executor
                    for cmd in &self.verification_commands {
                        self.stats.verifications_run += 1;
                        let res = self
                            .sandbox
                            .execute(cmd, None)
                            .map_err(|e| EngineError::Io(std::io::Error::other(e.to_string())))?;
                        if !res.success {
                            let pinned = crate::diagnostics::DiagnosticPinpointer::pin_from_output(
                                &self.repo,
                                &format!("{}\n{}", res.stdout, res.stderr),
                            );
                            let detail = if !pinned.is_empty() {
                                let feedback =
                                    crate::diagnostics::PinnedDiagnostic::format_feedback(&pinned);
                                format!("onYield verify `{cmd}` failed:\n{feedback}")
                            } else {
                                format!("onYield verify `{cmd}` failed: {}", res.stderr)
                            };

                            session.push(NodeKind::Verification {
                                passed: false,
                                detail: detail.clone(),
                            });
                            session.commit()?;
                            session.rollback_to_last_verified()?;
                            return Err(EngineError::Verify {
                                command: cmd.clone(),
                                code: res.exit_code.unwrap_or(-1),
                                stderr: detail,
                            });
                        }
                    }

                    // All gates passed: record deterministic judge_verified_hash
                    let verified_hash = crate::session::fnv1a64(
                        format!(
                            "verified:{}:{}",
                            session.head().unwrap_or(0),
                            self.stats.steps
                        )
                        .as_bytes(),
                    );

                    let v_id = session.push(NodeKind::Verification {
                        passed: true,
                        detail: "planner reported Done; all verification gates passed".to_string(),
                    });
                    session.commit()?;
                    let _ = session.record_judge_verified(v_id, verified_hash);

                    records.push(StepRecord {
                        action: label,
                        verified: true,
                        judge: None,
                    });
                    return Ok(records);
                }
                Action::Abort { reason } => {
                    session.push(NodeKind::Thought {
                        text: format!("abort: {reason}"),
                    });
                    session.commit()?;
                    return Err(EngineError::Aborted(reason));
                }
                Action::Patch { path, op } => {
                    let full = self.repo.join(&path);
                    let before = std::fs::read_to_string(&full)?;
                    let after = apply_patch(&before, &op).map_err(EngineError::Patch)?;
                    std::fs::write(&full, &after)?;
                    self.stats.patches_applied += 1;

                    session.push(NodeKind::Patch {
                        path: path.display().to_string(),
                        summary: format!("{} bytes → {}", before.len(), after.len()),
                    });
                    session.commit()?;

                    // Judge the delta this patch produced.
                    let verdict = if self.reviewer.is_repository() {
                        self.reviewer.review_since_checkpoint(&self.judge)?
                    } else {
                        Verdict::Pass
                    };
                    match verdict {
                        Verdict::Pass => records.push(StepRecord {
                            action: label,
                            verified: true,
                            judge: None,
                        }),
                        Verdict::Reject(findings) => {
                            let feedback = Judge::feedback(&findings);
                            self.stats.judge_rejections += 1;
                            records.push(StepRecord {
                                action: label,
                                verified: false,
                                judge: Some(feedback.clone()),
                            });
                            if self.judge.record_failure() {
                                // One targeted retry is permitted; the planner
                                // is expected to address the feedback.
                                self.stats.retries_used += 1;
                                session.push(NodeKind::Thought {
                                    text: format!("judge feedback: {feedback}"),
                                });
                                session.commit()?;
                            } else {
                                session.push(NodeKind::Verification {
                                    passed: false,
                                    detail: "judge rejected twice".to_string(),
                                });
                                session.commit()?;
                                return Err(EngineError::Rejected(feedback));
                            }
                        }
                    }
                }
                Action::Verify { command } => {
                    self.stats.verifications_run += 1;
                    let res = self
                        .sandbox
                        .execute(&command, None)
                        .map_err(|e| EngineError::Io(std::io::Error::other(e.to_string())))?;
                    let passed = res.success;

                    let (detail, err_detail) = if !passed {
                        let pinned = crate::diagnostics::DiagnosticPinpointer::pin_from_output(
                            &self.repo,
                            &format!("{}\n{}", res.stdout, res.stderr),
                        );
                        if !pinned.is_empty() {
                            let feedback =
                                crate::diagnostics::PinnedDiagnostic::format_feedback(&pinned);
                            (
                                format!("{command} failed:\n{feedback}"),
                                format!("{feedback}\n{}", res.stderr),
                            )
                        } else {
                            (format!("{command} → exit {:?}", res.exit_code), res.stderr)
                        }
                    } else {
                        (
                            format!("{command} → exit {:?}", res.exit_code),
                            String::new(),
                        )
                    };

                    session.push(NodeKind::Verification { passed, detail });
                    session.commit()?;

                    records.push(StepRecord {
                        action: label,
                        verified: passed,
                        judge: None,
                    });

                    if !passed {
                        // Roll back to the last verified node so the next
                        // attempt branches from a known-good state.
                        session.rollback_to_last_verified()?;
                        return Err(EngineError::Verify {
                            command,
                            code: res.exit_code.unwrap_or(-1),
                            stderr: err_detail,
                        });
                    }
                }
            }
        }

        Ok(records)
    }
}

fn describe(action: &Action) -> String {
    match action {
        Action::Patch { path, .. } => format!("patch {}", path.display()),
        Action::Verify { command } => format!("verify `{command}`"),
        Action::Done => "done".to_string(),
        Action::Abort { reason } => format!("abort: {reason}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A planner that replays a fixed script — no network, fully deterministic.
    struct ScriptedPlanner {
        script: Vec<Action>,
        index: usize,
    }

    impl Planner for ScriptedPlanner {
        fn next_action(&mut self, _session: &Session) -> Result<Action, EngineError> {
            let action = self.script.get(self.index).cloned().unwrap_or(Action::Done);
            self.index += 1;
            Ok(action)
        }
    }

    /// A unique temp repo per call — cargo runs tests in parallel threads, so
    /// a shared fixed name would let one test delete another's directory.
    fn temp_repo(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("va-engine-test-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("lib.rs"), "fn a() {\n    let x = 1;\n    x\n}\n").unwrap();
        dir
    }

    #[test]
    fn reaching_done_is_a_clean_pass() {
        let repo = temp_repo("done");
        let mut engine = Engine::new(&repo);
        let mut session = Session::new();
        let mut planner = ScriptedPlanner {
            script: vec![Action::Done],
            index: 0,
        };
        let records = engine.run(&mut session, &mut planner).unwrap();
        assert_eq!(records.len(), 1);
        assert!(records[0].verified);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn abort_surfaces_the_reason() {
        let repo = temp_repo("abort");
        let mut engine = Engine::new(&repo);
        let mut session = Session::new();
        let mut planner = ScriptedPlanner {
            script: vec![Action::Abort {
                reason: "planner gave up".to_string(),
            }],
            index: 0,
        };
        assert!(matches!(
            engine.run(&mut session, &mut planner),
            Err(EngineError::Aborted(_))
        ));
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn failing_verification_rolls_back_to_last_verified() {
        let repo = temp_repo("rollback");
        let mut engine = Engine::new(&repo);
        let mut session = Session::new();
        // Mark an earlier node verified so there is somewhere to roll back to.
        session.push(NodeKind::Verification {
            passed: true,
            detail: "baseline".to_string(),
        });
        session.commit().unwrap();
        let good = session.head().unwrap();

        // A command that cannot succeed on any platform.
        let mut planner = ScriptedPlanner {
            script: vec![Action::Verify {
                command: "this-command-does-not-exist-xyz".to_string(),
            }],
            index: 0,
        };
        assert!(matches!(
            engine.run(&mut session, &mut planner),
            Err(EngineError::Verify { .. })
        ));
        assert_eq!(
            session.head(),
            Some(good),
            "head must return to the last verified node"
        );
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn step_cap_prevents_an_endless_loop() {
        let repo = temp_repo("cap");
        let mut engine = Engine::new(&repo).with_max_steps(3);
        let mut session = Session::new();
        // A planner that never says Done — verify commands that pass, forever.
        let mut planner = ScriptedPlanner {
            script: (0..100)
                .map(|_| Action::Verify {
                    command: "exit 0".to_string(),
                })
                .collect(),
            index: 0,
        };
        let records = engine.run(&mut session, &mut planner).unwrap();
        assert_eq!(records.len(), 3, "must stop at the cap");
        assert_eq!(engine.stats().verifications_run, 3);
        let _ = std::fs::remove_dir_all(&repo);
    }
}
