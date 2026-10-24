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

use va_core::modes::{ExecutionMode, LanguageLens};

use crate::experience::StepFailure;
use crate::judge::{Judge, JudgeConfig, Verdict};
use crate::patcher::{apply_patch, segment_blocks, PatchOp};
use crate::review::{ReviewError, Reviewer};
use crate::sandbox::{SandboxConfig, SandboxExecutor};
use crate::session::{ContextWindowConfig, NodeKind, Session, WorkingSetEntry};

/// The sentinel that terminates a review loop, mirroring the upstream's
/// `DONE` convention.
pub const REVIEW_DONE: &str = "No further findings.";

/// What the model is asked to do next. Implemented by anything that can turn a
/// trajectory into a next action.
pub trait Planner {
    /// Given the trajectory so far, return the next action.
    fn next_action(&mut self, session: &Session) -> Result<Action, EngineError>;

    /// Called when a patch or verification fails. Return `true` to let the
    /// planner attempt a recovery (Debug feedback loop); the default keeps the
    /// original fail-fast behaviour.
    fn on_failure(&mut self, _failure: &StepFailure) -> bool {
        false
    }
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
    /// A write was attempted in a read-only mode (Chat / Architect).
    ReadOnlyMode {
        mode: String,
    },
    /// The same (file, block, failure) repeated — the loop is spinning.
    StagnationDetected {
        signature: String,
        attempts: u32,
        escalate_tournament: bool,
    },
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
            EngineError::ReadOnlyMode { mode } => write!(
                f,
                "mode `{mode}` is read-only — patches are forbidden; switch to code, web or debug"
            ),
            EngineError::StagnationDetected {
                signature,
                attempts,
                escalate_tournament,
            } => write!(
                f,
                "stagnation detected after {attempts} identical failures ({signature}); escalate_tournament={escalate_tournament}"
            ),
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
    /// How many times context auto-compaction fired.
    pub context_compactions: u32,
    /// Estimated prompt tokens after the latest compaction pass.
    pub context_tokens: usize,
    /// Identical-failure streaks detected.
    pub stagnation_events: u32,
}

/// What to do when the same failure repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagnationPolicy {
    /// Stop with `StagnationDetected`.
    Fail,
    /// Stop and ask the caller to run a speculative tournament.
    EscalateTournament,
}

pub struct Engine {
    repo: PathBuf,
    reviewer: Reviewer,
    judge: Judge,
    judge_config: JudgeConfig,
    sandbox: SandboxExecutor,
    verification_commands: Vec<String>,
    stats: EngineStats,
    /// Hard cap so a misbehaving planner cannot loop forever.
    max_steps: u32,
    mode: ExecutionMode,
    language: LanguageLens,
    context: ContextWindowConfig,
    stagnation_policy: StagnationPolicy,
    /// Last failure key + consecutive count.
    last_failure: Option<(String, u32)>,
    /// Fresh block hashes for files modified during this run.
    working_set: Vec<WorkingSetEntry>,
}

impl Engine {
    pub fn new(repo: &Path) -> Self {
        let sandbox_config = SandboxConfig::default_for(repo);
        let judge_config = JudgeConfig::default();
        Self {
            repo: repo.to_path_buf(),
            reviewer: Reviewer::new(repo),
            judge: Judge::new(judge_config.clone()),
            judge_config,
            sandbox: SandboxExecutor::new(sandbox_config),
            verification_commands: Vec::new(),
            stats: EngineStats::default(),
            max_steps: 50,
            mode: ExecutionMode::Code,
            language: LanguageLens::Auto,
            context: ContextWindowConfig::default(),
            stagnation_policy: StagnationPolicy::EscalateTournament,
            last_failure: None,
            working_set: Vec::new(),
        }
    }

    pub fn with_judge_config(mut self, config: JudgeConfig) -> Self {
        self.judge_config = config.clone();
        self.judge = Judge::new(config);
        self
    }

    /// Select the execution mode and language lens. Their guardrails are
    /// appended to the judge configuration.
    pub fn with_profile(mut self, mode: ExecutionMode, language: LanguageLens) -> Self {
        self.mode = mode;
        self.language = language;
        let mut guardrails: Vec<(String, String)> = Vec::new();
        for (token, message) in language.guardrails() {
            guardrails.push((token.to_string(), message.to_string()));
        }
        for (token, message) in mode.mode_guardrails() {
            guardrails.push((token.to_string(), message.to_string()));
        }
        self.judge_config = self.judge_config.clone().with_guardrails(guardrails);
        self.judge = Judge::new(self.judge_config.clone());
        self
    }

    pub fn with_context_window(mut self, config: ContextWindowConfig) -> Self {
        self.context = config;
        self
    }

    pub fn with_stagnation_policy(mut self, policy: StagnationPolicy) -> Self {
        self.stagnation_policy = policy;
        self
    }

    pub fn mode(&self) -> ExecutionMode {
        self.mode
    }

    pub fn language(&self) -> LanguageLens {
        self.language
    }

    /// Fresh hashes for every file modified during this run, for the pinned
    /// working-set tail.
    pub fn working_set(&self) -> &[WorkingSetEntry] {
        &self.working_set
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

            // Automatic context management: compact the *view* at the model's
            // watermark. The immutable DAG is never mutated, so replay and
            // rollback keep working exactly as before.
            let (surface, compacted) = session.context_surface_with_budget(&self.context)?;
            self.stats.context_tokens = surface.estimate_tokens();
            if compacted > 0 {
                self.stats.context_compactions += 1;
            }

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
                    if !self.mode.allows_writes() {
                        return Err(EngineError::ReadOnlyMode {
                            mode: self.mode.as_str().to_string(),
                        });
                    }
                    if path
                        .components()
                        .any(|c| matches!(c, std::path::Component::ParentDir))
                        || path.is_absolute()
                    {
                        return Err(EngineError::Io(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!(
                                "patch path must stay inside the workspace: {}",
                                path.display()
                            ),
                        )));
                    }
                    let full = self.repo.join(&path);
                    let before = std::fs::read_to_string(&full)?;
                    let after = match apply_patch(&before, &op) {
                        Ok(after) => after,
                        Err(e) => {
                            let failure = StepFailure {
                                language: self.language.as_str().to_string(),
                                domain: self.mode.as_str().to_string(),
                                file: Some(path.display().to_string()),
                                block_hash: Some(op.target_hash),
                                command: "patch".to_string(),
                                detail: e.to_string(),
                            };
                            if self.note_failure(&failure, planner, session)? {
                                continue;
                            }
                            return Err(EngineError::Patch(e));
                        }
                    };
                    std::fs::write(&full, &after)?;
                    self.stats.patches_applied += 1;
                    self.refresh_working_set(&path);
                    self.last_failure = None;

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
                        let failure = StepFailure {
                            language: self.language.as_str().to_string(),
                            domain: self.mode.as_str().to_string(),
                            file: None,
                            block_hash: None,
                            command: command.clone(),
                            detail: err_detail.clone(),
                        };
                        if self.note_failure(&failure, planner, session)? {
                            continue;
                        }
                        return Err(EngineError::Verify {
                            command,
                            code: res.exit_code.unwrap_or(-1),
                            stderr: err_detail,
                        });
                    }
                    self.last_failure = None;
                }
            }
        }

        Ok(records)
    }

    /// Record a failure, detect identical-failure stagnation, and let the
    /// planner attempt a recovery.
    fn note_failure<P: Planner>(
        &mut self,
        failure: &StepFailure,
        planner: &mut P,
        session: &mut Session,
    ) -> Result<bool, EngineError> {
        let key = failure.signature();
        let attempts = match &self.last_failure {
            Some((last, count)) if *last == key => count + 1,
            _ => 1,
        };
        self.last_failure = Some((key.clone(), attempts));

        session.push(NodeKind::Observation {
            text: format!(
                "[failure:{attempts}] {}\n{}",
                failure.command, failure.detail
            ),
        });
        session.commit()?;

        if attempts >= 2 {
            self.stats.stagnation_events += 1;
            return Err(EngineError::StagnationDetected {
                signature: key,
                attempts,
                escalate_tournament: self.stagnation_policy == StagnationPolicy::EscalateTournament,
            });
        }
        Ok(planner.on_failure(failure))
    }

    /// Re-hash every block of a just-modified file so the pinned working-set
    /// tail never carries stale hashes.
    fn refresh_working_set(&mut self, path: &Path) {
        let full = self.repo.join(path);
        let Ok(content) = std::fs::read_to_string(&full) else {
            return;
        };
        let blocks: Vec<crate::session::WorkingSetBlock> = segment_blocks(&content)
            .into_iter()
            .take(32)
            .map(|b| crate::session::WorkingSetBlock {
                hash: b.hash,
                start_line: b.start_line,
                end_line: b.end_line,
            })
            .collect();
        let rel = path.to_string_lossy().replace('\\', "/");
        if let Some(existing) = self.working_set.iter_mut().find(|e| e.path == rel) {
            existing.blocks = blocks;
        } else {
            self.working_set.push(WorkingSetEntry { path: rel, blocks });
        }
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

    /// A planner that always asks for the same failing verification and accepts
    /// recovery, which is exactly the spin loop the stagnation guard must stop.
    struct AlwaysRetryPlanner {
        command: String,
    }

    impl Planner for AlwaysRetryPlanner {
        fn next_action(&mut self, _session: &Session) -> Result<Action, EngineError> {
            Ok(Action::Verify {
                command: self.command.clone(),
            })
        }

        fn on_failure(&mut self, _failure: &StepFailure) -> bool {
            true
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
        let head = session.head().unwrap();
        assert_ne!(head, good, "the failure observation must be recorded");
        assert_eq!(
            session.last_verified().unwrap(),
            Some(good),
            "the verified checkpoint must survive the failure"
        );
        assert!(
            session.current_path().unwrap().contains(&good),
            "the failure branch must descend from the last verified node"
        );
        assert!(matches!(
            session.get(head).map(|n| &n.kind),
            Some(NodeKind::Observation { .. })
        ));
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn repeated_identical_failure_triggers_stagnation_and_escalation() {
        let repo = temp_repo("spin");
        let mut engine = Engine::new(&repo)
            .with_max_steps(10)
            .with_stagnation_policy(StagnationPolicy::EscalateTournament);
        let mut session = Session::new();
        let mut planner = AlwaysRetryPlanner {
            command: "exit 1".to_string(),
        };
        match engine.run(&mut session, &mut planner) {
            Err(EngineError::StagnationDetected {
                attempts,
                escalate_tournament,
                ..
            }) => {
                assert_eq!(attempts, 2, "two identical failures must trip the guard");
                assert!(escalate_tournament, "policy must request the tournament");
            }
            other => panic!("expected StagnationDetected, got {other:?}"),
        }
        assert_eq!(engine.stats().stagnation_events, 1);
        let failures = session.to_jsonl().matches("[failure:").count();
        assert_eq!(
            failures, 2,
            "both failed attempts must stay in the DAG even when the branch is rolled back"
        );
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn chat_mode_is_a_hard_read_only_boundary() {
        let repo = temp_repo("readonly");
        let path = repo.join("main.rs");
        std::fs::write(&path, "fn value() -> u32 {\n    1\n}\n").unwrap();
        let before = std::fs::read_to_string(&path).unwrap();
        let block = segment_blocks(&before).first().cloned().unwrap();
        let op = PatchOp::new(block.hash, "fn value() -> u32 {\n    2\n}\n");
        let mut planner = ScriptedPlanner {
            script: vec![
                Action::Patch {
                    path: PathBuf::from("main.rs"),
                    op,
                },
                Action::Done,
            ],
            index: 0,
        };
        let mut engine = Engine::new(&repo)
            .with_profile(ExecutionMode::Chat, LanguageLens::Rust)
            .with_max_steps(4);
        let mut session = Session::new();
        let result = engine.run(&mut session, &mut planner);
        assert!(
            matches!(result, Err(EngineError::ReadOnlyMode { .. })),
            "chat/architect must reject writes, got {result:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            before,
            "read-only mode must not modify a single byte"
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
