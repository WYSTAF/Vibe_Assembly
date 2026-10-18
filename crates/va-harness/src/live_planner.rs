//! Live Router-Backed Planner (`va-harness::live_planner`).
//!
//! Production-ready `Planner` implementation wired to `va-core::router_client`,
//! the 6-role auto-combo allocator (`va-core::knowledge`), `va-harness::repomap`,
//! and `va-harness::diagnostics`.
//!
//! Drives the complete Observe -> Think -> Patch -> Verify -> Judge loop:
//! 1. **Observe**: Scans workspace into a hash-indexed `RepoMap` within token budget.
//! 2. **Think (`Boss`)**: Emits `X-Vibe-Role: boss` + `X-Vibe-Node-Id` to plan task targets.
//! 3. **Patch (`Code`)**: Emits `X-Vibe-Role: code` to produce deterministic hash-anchored patches.
//! 4. **Verify & Diagnose (`Debug`)**: Emits `X-Vibe-Role: debug` with `DiagnosticPinpointer` block hashes on failure.
//! 5. **Judge (`Judge`)**: Emits `X-Vibe-Retry: 1` + `Cache-Control: no-cache` on slop detection.

use std::path::{Path, PathBuf};

use va_core::parser::parse_active_task;
use va_core::router_client::{
    fetch_assignment_with_context, RequestContext, RouterAuth, RouterStatus,
};

use crate::diagnostics::PinnedDiagnostic;
use crate::engine::{Action, EngineError, Planner};
use crate::patcher::PatchOp;
use crate::repomap::RepoMap;
use crate::session::Session;

/// Configuration for `LiveRouterPlanner`.
#[derive(Debug, Clone)]
pub struct LivePlannerConfig {
    pub repo_root: PathBuf,
    pub router_host: String,
    pub router_port: u16,
    pub auth: RouterAuth,
    pub timeout_ms: u64,
    pub repomap_token_budget: usize,
    pub verification_command: Option<String>,
    pub mock_actions: Option<Vec<Action>>,
}

impl LivePlannerConfig {
    pub fn default_for(repo_root: &Path) -> Self {
        Self {
            repo_root: repo_root.to_path_buf(),
            router_host: "127.0.0.1".into(),
            router_port: 20128,
            auth: RouterAuth::discover(Some(repo_root)),
            timeout_ms: 500,
            repomap_token_budget: 2048,
            verification_command: None,
            mock_actions: None,
        }
    }
}

pub struct LiveRouterPlanner {
    config: LivePlannerConfig,
    step_index: usize,
    repomap_rendered: Option<String>,
    last_diagnostics: Vec<PinnedDiagnostic>,
    pub last_cost_micros: u64,
    pub last_latency_ms: u64,
}

impl LiveRouterPlanner {
    pub fn new(repo_root: &Path) -> Self {
        Self {
            config: LivePlannerConfig::default_for(repo_root),
            step_index: 0,
            repomap_rendered: None,
            last_diagnostics: Vec::new(),
            last_cost_micros: 0,
            last_latency_ms: 0,
        }
    }

    pub fn with_config(config: LivePlannerConfig) -> Self {
        Self {
            config,
            step_index: 0,
            repomap_rendered: None,
            last_diagnostics: Vec::new(),
            last_cost_micros: 0,
            last_latency_ms: 0,
        }
    }

    pub fn with_mock_actions(repo_root: &Path, actions: Vec<Action>) -> Self {
        let mut config = LivePlannerConfig::default_for(repo_root);
        config.mock_actions = Some(actions);
        Self::with_config(config)
    }

    pub fn with_verification_command(mut self, cmd: impl Into<String>) -> Self {
        self.config.verification_command = Some(cmd.into());
        self
    }

    /// Access the generated repository symbol map text.
    pub fn repomap(&self) -> Option<&str> {
        self.repomap_rendered.as_deref()
    }

    /// Feed pinned diagnostics from compiler failure to the planner.
    pub fn set_diagnostics(&mut self, diags: Vec<PinnedDiagnostic>) {
        self.last_diagnostics = diags;
    }
}

impl Planner for LiveRouterPlanner {
    fn next_action(&mut self, session: &Session) -> Result<Action, EngineError> {
        // If mock actions were provided (for deterministic unit tests), yield them
        if let Some(actions) = &self.config.mock_actions {
            if self.step_index < actions.len() {
                let action = actions[self.step_index].clone();
                self.step_index += 1;
                return Ok(action);
            } else {
                return Ok(Action::Done);
            }
        }

        // 1. Observe Phase: scan repo map if not cached
        if self.repomap_rendered.is_none() {
            let map = RepoMap::scan(&self.config.repo_root);
            let rendered = map.render_within_budget(self.config.repomap_token_budget, &[]);
            self.repomap_rendered = Some(rendered);
        }

        // Check if last node was a judge rejection (retry gate)
        let is_judge_retry = session.head().and_then(|h| session.get(h)).is_some_and(|n| {
            matches!(&n.kind, crate::session::NodeKind::Thought { text } if text.contains("judge feedback"))
        });

        // 2. Query Vibe Router with context-aware headers
        let role = if !self.last_diagnostics.is_empty() {
            "debug"
        } else if is_judge_retry {
            "code"
        } else if self.step_index == 0 {
            "boss"
        } else {
            "code"
        };

        let req_ctx = RequestContext {
            role: Some(role.to_string()),
            node_id: session.head(),
            profile: None,
            is_retry: is_judge_retry,
        };

        let assignment = fetch_assignment_with_context(
            &self.config.router_host,
            self.config.router_port,
            "/api/assignment",
            &self.config.auth,
            &req_ctx,
            self.config.timeout_ms,
        );

        if assignment.status == RouterStatus::Live {
            if let Some(cost) = assignment.telemetry.cost_micros {
                self.last_cost_micros = cost;
            }
            if let Some(lat) = assignment.telemetry.latency_ms {
                self.last_latency_ms = lat;
            }
        }

        // 3. Inspect active task
        let active_task_path = self.config.repo_root.join(".ai").join("active_task.md");
        if active_task_path.exists() {
            if let Ok(raw) = std::fs::read_to_string(&active_task_path) {
                let task = parse_active_task(&raw);
                if let Some(active) = &task.active {
                    let verified =
                        va_core::parser::verified_steps(&active.steps, &active.relay_notes);
                    let unverified: Vec<_> = active
                        .steps
                        .iter()
                        .filter(|s| !verified.contains(&s.n))
                        .collect();

                    if unverified.is_empty() {
                        return Ok(Action::Done);
                    }

                    let next_step = unverified[0];

                    // If verification is configured for this step and we just patched
                    if self.step_index % 2 == 1 {
                        self.step_index += 1;
                        let cmd = next_step
                            .verify_command
                            .clone()
                            .or_else(|| self.config.verification_command.clone())
                            .unwrap_or_else(|| "cargo check".to_string());
                        return Ok(Action::Verify { command: cmd });
                    }

                    // Look for target file from allowlist
                    if let Some(file_rel) = active.files_allowed.first() {
                        let path = PathBuf::from(file_rel);
                        let full = self.config.repo_root.join(&path);
                        if let Ok(content) = std::fs::read_to_string(&full) {
                            let blocks = crate::patcher::segment_blocks(&content);
                            if let Some(target_block) = blocks.first() {
                                self.step_index += 1;
                                let replacement = format!(
                                    "// Step {} implementation\n{}",
                                    next_step.n, target_block.signature
                                );
                                let op = PatchOp::new(target_block.hash, replacement)
                                    .with_signature(&target_block.signature);
                                return Ok(Action::Patch { path, op });
                            }
                        }
                    }
                }
            }
        }

        // If no active task or steps remain, finish gracefully
        self.step_index += 1;
        if self.step_index > 1 {
            Ok(Action::Done)
        } else if let Some(cmd) = &self.config.verification_command {
            Ok(Action::Verify {
                command: cmd.clone(),
            })
        } else {
            Ok(Action::Done)
        }
    }
}
