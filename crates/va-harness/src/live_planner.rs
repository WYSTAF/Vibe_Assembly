//! Live Router-Backed Planner (`va-harness::live_planner`).
//!
//! Production planner wired to `va-core::router_client`'s real
//! `POST /v1/chat/completions` client, the harness RepoMap, the diagnostic
//! pinpointer, the immutable session DAG, and `.ai/EXPERIENCE.md`.
//!
//! Flow:
//! 1. **Observe** — scan the workspace into a token-budgeted `RepoMap`.
//! 2. **Think (Boss)** — ask the router which files/blocks the ticket touches
//!    and what verification command proves it.
//! 3. **Patch (Code)** — send the exact hashed block bodies and receive
//!    model-authored replacements, applied through `va-harness::patcher`.
//! 4. **Verify (Debug)** — when a verification fails, feed pinned diagnostics
//!    back to the Debug role and apply the recovery patch.
//! 5. **Judge** — deterministic anti-slop gate; the Judge always runs
//!    independently of the model that authored the patch.
//!
//! Offline synthesis exists only behind `offline_mock` (unit tests and the
//! explicit `--offline-mock` flag). A normal live run never fabricates edits.

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};

use serde_json::Value;
use va_core::modes::{ExecutionMode, LanguageLens};
use va_core::parser::parse_active_task;
use va_core::router_client::{
    chat_completion, fetch_assignment_with_context, ChatMessage, RequestContext, RouterAuth,
    RouterStatus,
};

use crate::diagnostics::{DiagnosticPinpointer, PinnedDiagnostic};
use crate::engine::{Action, EngineError, Planner};
use crate::experience::{
    ContrastiveRecovery, ExperienceStore, HarnessLayers, RecoveryCandidate, StepFailure,
};
use crate::patcher::{hash_file, segment_blocks, PatchOp};
use crate::repomap::RepoMap;
use crate::session::{hash_str, ContextWindowConfig, NodeKind, Session, WorkingSetEntry};

/// Configuration for `LiveRouterPlanner`.
#[derive(Debug, Clone)]
pub struct LivePlannerConfig {
    pub repo_root: PathBuf,
    pub router_host: String,
    pub router_port: u16,
    pub router_path: String,
    pub auth: RouterAuth,
    pub timeout_ms: u64,
    pub repomap_token_budget: usize,
    pub verification_command: Option<String>,
    pub mock_actions: Option<Vec<Action>>,
    /// Active execution mode (code / web / chat / debug).
    pub mode: ExecutionMode,
    /// Language lens; `Auto` resolves against the repository.
    pub language: LanguageLens,
    /// Pinned model for every role, or `None` for router auto-combo.
    pub model: Option<String>,
    /// Optional independent judge model (reported to the UI; the deterministic
    /// judge always runs regardless).
    pub judge_model: Option<String>,
    /// Deterministic local synthesis. Never enabled for a real live run.
    pub offline_mock: bool,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub context: ContextWindowConfig,
    /// Explicit task text from `vibe-assembly prompt/chat`.
    pub task_prompt: Option<String>,
}

impl LivePlannerConfig {
    pub fn default_for(repo_root: &Path) -> Self {
        Self {
            repo_root: repo_root.to_path_buf(),
            router_host: "127.0.0.1".into(),
            router_port: 20128,
            router_path: "/v1/chat/completions".into(),
            auth: RouterAuth::discover(Some(repo_root)),
            timeout_ms: 180_000,
            repomap_token_budget: 2048,
            verification_command: None,
            mock_actions: None,
            mode: ExecutionMode::Code,
            language: LanguageLens::Auto,
            model: None,
            judge_model: None,
            offline_mock: false,
            temperature: 0.2,
            max_tokens: None,
            context: ContextWindowConfig::default(),
            task_prompt: None,
        }
    }

    pub fn with_router(mut self, host: impl Into<String>, port: u16) -> Self {
        self.router_host = host.into();
        self.router_port = port;
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// One block the Boss wants Code to rewrite.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Target {
    file: PathBuf,
    block_hash: Option<u64>,
}

pub struct LiveRouterPlanner {
    config: LivePlannerConfig,
    step_index: usize,
    repomap_rendered: Option<String>,
    last_diagnostics: Vec<PinnedDiagnostic>,
    pub last_cost_micros: u64,
    pub last_latency_ms: u64,
    /// Resolved role → model map, cached after the first assignment query.
    models: BTreeMap<String, String>,
    /// Boss targets waiting for a Code call.
    plan_targets: Vec<Target>,
    boss_done: bool,
    verification_emitted: bool,
    /// Patch actions produced by one Code response.
    pending_actions: VecDeque<Action>,
    /// The last block we patched, as `(file, signature)`. The signature lets
    /// Debug re-resolve the *new* hash after the patch changed the block body.
    last_patch: Option<(PathBuf, String)>,
    /// Files already covered by the current Code response. Only the first
    /// patch per file carries the whole-file drift assertion; later patches
    /// run against the file as updated by earlier patches in the same batch.
    response_files: std::collections::HashSet<PathBuf>,
    /// Raw failure text when diagnostics could not be pinned to a span.
    last_failure_detail: Option<String>,
    /// Durable lessons.
    experience: ExperienceStore,
    /// Eight harness surfaces, strictly reset to base at every task start.
    layers: HarnessLayers,
    /// Offline-mock: the no-op patch has been produced.
    offline_patched: bool,
}

impl LiveRouterPlanner {
    pub fn new(repo_root: &Path) -> Self {
        Self::with_config(LivePlannerConfig::default_for(repo_root))
    }

    pub fn with_config(config: LivePlannerConfig) -> Self {
        let experience = ExperienceStore::load(&config.repo_root);
        Self {
            config,
            step_index: 0,
            repomap_rendered: None,
            last_diagnostics: Vec::new(),
            last_cost_micros: 0,
            last_latency_ms: 0,
            models: BTreeMap::new(),
            plan_targets: Vec::new(),
            boss_done: false,
            verification_emitted: false,
            pending_actions: VecDeque::new(),
            last_patch: None,
            response_files: std::collections::HashSet::new(),
            last_failure_detail: None,
            experience,
            layers: HarnessLayers::base(),
            offline_patched: false,
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

    pub fn config(&self) -> &LivePlannerConfig {
        &self.config
    }

    pub fn mode(&self) -> ExecutionMode {
        self.config.mode
    }

    pub fn language(&self) -> LanguageLens {
        self.config.language.resolve(&self.config.repo_root)
    }

    /// Model used by the independent judge role (display / audit).
    pub fn judge_model(&self) -> Option<String> {
        if let Some(m) = &self.config.judge_model {
            return Some(m.clone());
        }
        self.models.get("judge").cloned()
    }

    /// Access the generated repository symbol map text.
    pub fn repomap(&self) -> Option<&str> {
        self.repomap_rendered.as_deref()
    }

    /// Feed pinned diagnostics from compiler failure to the planner.
    pub fn set_diagnostics(&mut self, diags: Vec<PinnedDiagnostic>) {
        self.last_diagnostics = diags;
    }

    /// Start a new task: strict reset-to-base for all eight harness surfaces.
    pub fn begin_task(&mut self) {
        self.layers.reset_to_base();
        self.step_index = 0;
        self.plan_targets.clear();
        self.boss_done = false;
        self.verification_emitted = false;
        self.pending_actions.clear();
        self.last_diagnostics.clear();
        self.last_patch = None;
        self.response_files.clear();
        self.offline_patched = false;
        self.last_failure_detail = None;
    }

    /// Model-authored patches are the only live path; nothing is synthesized.
    fn is_live(&self) -> bool {
        !self.config.offline_mock && self.config.mock_actions.is_none()
    }

    fn ensure_repomap(&mut self) {
        if self.repomap_rendered.is_none() {
            let map = RepoMap::scan(&self.config.repo_root);
            self.repomap_rendered =
                Some(map.render_within_budget(self.config.repomap_token_budget, &[]));
        }
    }

    fn task_text(&self) -> String {
        if let Some(prompt) = &self.config.task_prompt {
            return prompt.clone();
        }
        let path = self.config.repo_root.join(".ai").join("active_task.md");
        std::fs::read_to_string(path).unwrap_or_else(|_| {
            "No .ai/active_task.md found. Inspect the repository and report done.".to_string()
        })
    }

    /// Fresh working set from every patch on the current branch.
    fn working_set(&self, session: &Session) -> Vec<WorkingSetEntry> {
        let path = match session.current_path() {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let mut entries: Vec<WorkingSetEntry> = Vec::new();
        for id in path {
            let Some(node) = session.get(id) else {
                continue;
            };
            let NodeKind::Patch { path: rel, .. } = &node.kind else {
                continue;
            };
            if entries.iter().any(|e| &e.path == rel) {
                continue;
            }
            let full = self.config.repo_root.join(rel);
            let Ok(content) = std::fs::read_to_string(&full) else {
                continue;
            };
            let mut entry = WorkingSetEntry::new(rel.clone());
            for block in segment_blocks(&content).into_iter().take(32) {
                entry = entry.with_block(block.hash, block.start_line, block.end_line);
            }
            entries.push(entry);
        }
        entries
    }

    fn system_prompt(&self) -> String {
        let language = self.language();
        let mut out = String::new();
        out.push_str("# Vibe Assembly execution contract\n\n");
        out.push_str(self.config.mode.contract());
        out.push_str("\n\n");
        out.push_str(&format!(
            "Language lens: {} ({})\nVerification: {}\n\n",
            language.label(),
            language.as_str(),
            language.verification_command(&self.config.repo_root)
        ));
        out.push_str("Language guardrails (a patch that violates one will be rejected):\n");
        for (token, message) in language.guardrails() {
            out.push_str(&format!("- `{token}` — {message}\n"));
        }
        for (token, message) in self.config.mode.mode_guardrails() {
            out.push_str(&format!("- `{token}` — {message}\n"));
        }
        out.push_str("\nJudge note: ");
        out.push_str(language.judge_note());
        out.push('\n');
        if !self.config.mode.allows_writes() {
            out.push_str(
                "\nREAD-ONLY MODE: never emit patches. Answer questions and propose plans only.\n",
            );
        }
        out.push_str("\n<harness_surfaces>\n");
        out.push_str(&self.layers.render());
        out.push_str("</harness_surfaces>\n");
        let experience = self
            .experience
            .render_precondition(language.as_str(), self.config.mode.as_str());
        if !experience.is_empty() {
            out.push('\n');
            out.push_str(&experience);
        }
        out
    }

    /// Resolve the model for a role: pinned override, router assignment, local
    /// combo file, then a conservative default.
    fn resolve_model(&mut self, role: &str) -> String {
        if let Some(model) = &self.config.model {
            return model.clone();
        }
        if let Some(model) = self.models.get(role) {
            return model.clone();
        }
        // Per-mode override: `.ai/models/<mode>.txt` or `.ai/model_<mode>.txt`.
        // This lets an operator pin a different model for Code/Web/Chat/Debug
        // without touching the global combo or the router assignment.
        let mode = self.config.mode.as_str();
        for candidate in [
            self.config
                .repo_root
                .join(".ai")
                .join("models")
                .join(format!("{mode}.txt")),
            self.config
                .repo_root
                .join(".ai")
                .join(format!("model_{mode}.txt")),
        ] {
            if let Ok(raw) = std::fs::read_to_string(candidate) {
                if let Some(model) = first_model_line(&raw) {
                    self.models.insert(role.to_string(), model.clone());
                    return model;
                }
            }
        }
        let ctx = RequestContext {
            role: Some(role.to_string()),
            node_id: None,
            profile: Some(self.config.mode.as_str().to_string()),
            is_retry: false,
        };
        let assignment = fetch_assignment_with_context(
            &self.config.router_host,
            self.config.router_port,
            "/api/assignment",
            &self.config.auth,
            &ctx,
            self.config.timeout_ms.min(2_000),
        );
        if assignment.status == RouterStatus::Live {
            for r in &assignment.roles {
                self.models.insert(r.mode.clone(), r.model.clone());
            }
            if let Some(model) = self.models.get(role) {
                return model.clone();
            }
            if let Some(first) = assignment.roles.first() {
                return first.model.clone();
            }
        }
        let combo = self.config.repo_root.join(".ai").join("model_combo.txt");
        let example = self
            .config
            .repo_root
            .join(".ai")
            .join("model_combo.example.txt");
        for path in [combo, example] {
            if let Ok(raw) = std::fs::read_to_string(path) {
                if let Some(model) = first_model_line(&raw) {
                    return model;
                }
            }
        }
        "gpt-4o-mini".to_string()
    }

    fn complete(
        &mut self,
        role: &str,
        user_prompt: &str,
        session: &Session,
    ) -> Result<String, EngineError> {
        let model = self.resolve_model(role);
        let messages = [
            ChatMessage::system(self.system_prompt()),
            ChatMessage::user(user_prompt),
        ];
        let mut retried = false;
        loop {
            let ctx = RequestContext {
                role: Some(role.to_string()),
                node_id: session.head(),
                profile: Some(self.config.mode.as_str().to_string()),
                is_retry: retried || !self.last_diagnostics.is_empty(),
            };
            match chat_completion(
                &self.config.router_host,
                self.config.router_port,
                &self.config.router_path,
                &self.config.auth,
                &ctx,
                &model,
                &messages,
                self.config.temperature,
                self.config.max_tokens,
                self.config.timeout_ms,
            ) {
                Ok(result) => {
                    if let Some(cost) = result.telemetry.cost_micros {
                        self.last_cost_micros = cost;
                    }
                    if let Some(latency) = result.telemetry.latency_ms {
                        self.last_latency_ms = latency;
                    }
                    return Ok(result.content);
                }
                Err(e) => {
                    let retryable =
                        !e.contains("401") && !e.contains("403") && !e.contains("HTTP 4");
                    if retried || !retryable {
                        return Err(EngineError::Aborted(format!(
                            "Vibe Router chat completion failed: {e}"
                        )));
                    }
                    // One transparent retry for transport / 5xx faults; the
                    // second request carries X-Vibe-Retry: 1.
                    retried = true;
                }
            }
        }
    }

    fn boss_plan(&mut self, session: &Session) -> Result<(), EngineError> {
        let (surface, _) = session
            .context_surface_with_budget(&self.config.context)
            .map_err(|e| EngineError::Aborted(format!("context surface: {e}")))?;
        let working_set = self.working_set(session);
        let prompt = format!(
            "TASK\n{}\n\nREPOSITORY MAP (symbols bound to block hashes)\n{}\n\n\
             SESSION CONTEXT\n{}\n\n\
             Your job: identify the files and exact blocks needed for the next step of the task.\n\
             Reply with JSON only, no prose, using this schema:\n\
             {{\"files\":[\"relative/path\"],\"targets\":[{{\"file\":\"relative/path\",\"block_hash\":\"1A2B\"}}],\
             \"verify\":\"command\",\"done\":false}}\n\
             Rules:\n\
             - `block_hash` is the 4-hex short hash from the repository map.\n\
             - Never invent a hash; copy it from the map.\n\
             - If the task is already complete, reply {{\"done\":true}}.\n\
             - If the ticket requires no code change, reply {{\"done\":true}}.\n",
            self.task_text(),
            self.repomap_rendered.as_deref().unwrap_or(""),
            surface.render_with_working_set(&working_set),
        );

        let response = self.complete("boss", &prompt, session)?;
        let value = extract_json(&response).ok_or_else(|| {
            EngineError::Aborted(format!(
                "boss returned no parseable JSON: {}",
                first_line(&response)
            ))
        })?;

        if value.get("done").and_then(Value::as_bool) == Some(true) {
            self.boss_done = true;
            self.plan_targets.clear();
            return Ok(());
        }

        let mut targets = Vec::new();
        if let Some(list) = value.get("targets").and_then(Value::as_array) {
            for item in list {
                let file = item
                    .get("file")
                    .or_else(|| item.get("path"))
                    .and_then(Value::as_str);
                let Some(file) = file else { continue };
                let hash_text = item
                    .get("block_hash")
                    .or_else(|| item.get("hash"))
                    .and_then(Value::as_str);
                let file_path = PathBuf::from(file);
                let block_hash =
                    hash_text.and_then(|h| resolve_hash(&file_path, h, &self.config.repo_root));
                targets.push(Target {
                    file: file_path,
                    block_hash,
                });
            }
        }
        if targets.is_empty() {
            if let Some(files) = value.get("files").and_then(Value::as_array) {
                for file in files.iter().filter_map(Value::as_str) {
                    targets.push(Target {
                        file: PathBuf::from(file),
                        block_hash: None,
                    });
                }
            }
        }
        // De-duplicate on (file, hash).
        let mut seen = std::collections::HashSet::new();
        targets.retain(|t| seen.insert((t.file.clone(), t.block_hash)));
        if let Some(cmd) = value.get("verify").and_then(Value::as_str) {
            if !cmd.trim().is_empty() {
                self.config.verification_command = Some(cmd.trim().to_string());
            }
        }
        self.plan_targets = targets;
        self.boss_done = true;
        Ok(())
    }

    fn code_prompt(&self, session: &Session) -> Result<String, EngineError> {
        let targets = self.plan_targets.clone();
        let mut block_sections = String::new();
        for target in &targets {
            let full = self.config.repo_root.join(&target.file);
            let content = std::fs::read_to_string(&full).map_err(|e| {
                EngineError::Aborted(format!("cannot read {}: {e}", target.file.display()))
            })?;
            let file_hash = hash_file(&content);
            block_sections.push_str(&format!(
                "\nFILE {}\nfile_hash={file_hash:016X}\n",
                target.file.display()
            ));
            let blocks = segment_blocks(&content);
            let mut emitted = 0usize;
            for block in blocks {
                if let Some(want) = target.block_hash {
                    if block.hash != want {
                        continue;
                    }
                }
                let lines: Vec<&str> = content.lines().collect();
                let body = lines[block.start_line - 1..block.end_line].join("\n");
                block_sections.push_str(&format!(
                    "BLOCK hash={:04X} full={:016X} lines={}-{}\n---\n{}\n---\n",
                    (block.hash >> 48) as u16,
                    block.hash,
                    block.start_line,
                    block.end_line,
                    body
                ));
                emitted += 1;
                if emitted >= 24 {
                    block_sections.push_str("[... more blocks elided ...]\n");
                    break;
                }
            }
            if emitted == 0 {
                block_sections.push_str("[no matching block — re-read the file map]\n");
            }
        }
        let (surface, _) = session
            .context_surface_with_budget(&self.config.context)
            .map_err(|e| EngineError::Aborted(format!("context surface: {e}")))?;
        let working_set = self.working_set(session);

        let mut diagnostics = String::new();
        if !self.last_diagnostics.is_empty() {
            diagnostics.push_str("\nFAILURE EVIDENCE (fix the root cause, not the symptom)\n");
            diagnostics.push_str(&PinnedDiagnostic::format_feedback(&self.last_diagnostics));
            diagnostics.push('\n');
        }
        if let Some(detail) = &self.last_failure_detail {
            diagnostics.push_str("\nRAW FAILURE OUTPUT\n");
            for line in detail.lines().take(40) {
                diagnostics.push_str(line);
                diagnostics.push('\n');
            }
        }

        Ok(format!(
            "TASK\n{}\n\nEXACT BLOCKS TO EDIT\n{}\n\nSESSION CONTEXT\n{}\n{}\
             Reply with JSON only, no prose, using this schema:\n\
             {{\"patches\":[{{\"file\":\"relative/path\",\"target_hash\":\"1A2B\",\
             \"replacement\":\"complete replacement text for the block\"}}],\
             \"done\":false,\"abort\":null}}\n\
             Rules:\n\
             - Address the block by the short hash shown above; never by line number.\n\
             - `replacement` must be the complete new block text, preserving indentation.\n\
             - Author real code. Never emit placeholders such as `todo!()`.\n\
             - If no change is needed, reply {{\"done\":true}}.\n",
            self.task_text(),
            block_sections,
            surface.render_with_working_set(&working_set),
            diagnostics,
        ))
    }

    fn code_action(&mut self, session: &Session) -> Result<Action, EngineError> {
        let prompt = self.code_prompt(session)?;
        let role = if self.last_diagnostics.is_empty() && self.last_failure_detail.is_none() {
            "code"
        } else {
            "debug"
        };
        let response = self.complete(role, &prompt, session)?;
        self.last_diagnostics.clear();
        self.last_failure_detail = None;
        let value = extract_json(&response).ok_or_else(|| {
            EngineError::Aborted(format!(
                "code returned no parseable JSON: {}",
                first_line(&response)
            ))
        })?;
        if value.get("done").and_then(Value::as_bool) == Some(true) {
            self.plan_targets.clear();
            return self.next_terminal_action();
        }
        if let Some(reason) = value.get("abort").and_then(Value::as_str) {
            if !reason.trim().is_empty() {
                return Ok(Action::Abort {
                    reason: reason.to_string(),
                });
            }
        }

        let mut patches = Vec::new();
        self.response_files.clear();
        if let Some(list) = value.get("patches").and_then(Value::as_array) {
            for item in list {
                if let Some(action) = self.parse_patch(item)? {
                    patches.push(action);
                }
            }
        } else if value.get("target_hash").is_some() {
            if let Some(action) = self.parse_patch(&value)? {
                patches.push(action);
            }
        }

        if patches.is_empty() {
            return Ok(Action::Abort {
                reason: "code returned neither patches nor done:true".to_string(),
            });
        }
        self.plan_targets.clear();
        self.verification_emitted = false;
        self.pending_actions.extend(patches);
        self.pending_actions
            .pop_front()
            .ok_or_else(|| EngineError::Aborted("empty patch queue".to_string()))
    }

    fn parse_patch(&mut self, item: &Value) -> Result<Option<Action>, EngineError> {
        let file = item
            .get("file")
            .or_else(|| item.get("path"))
            .or_else(|| item.get("filename"))
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .or_else(|| self.plan_targets.first().map(|t| t.file.clone()));
        let Some(file) = file else {
            return Ok(None);
        };
        let hash_text = item
            .get("target_hash")
            .or_else(|| item.get("block_hash"))
            .or_else(|| item.get("hash"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let replacement = item
            .get("replacement")
            .or_else(|| item.get("content"))
            .or_else(|| item.get("text"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let (Some(hash_text), Some(replacement)) = (hash_text, replacement) else {
            return Ok(None);
        };
        let full = self.config.repo_root.join(&file);
        let content = std::fs::read_to_string(&full)
            .map_err(|e| EngineError::Aborted(format!("cannot read {}: {e}", file.display())))?;
        let Some((hash, block)) = lookup_block(&content, &hash_text) else {
            return Err(EngineError::Aborted(format!(
                "code referenced unknown block hash `{hash_text}` in {}",
                file.display()
            )));
        };
        let first_in_file = self.response_files.insert(file.clone());
        let mut op = PatchOp::new(hash, replacement).with_signature(&block.signature);
        if first_in_file {
            op = op.with_expected_file_hash(hash_file(&content));
        }
        self.last_patch = Some((file.clone(), block.signature.clone()));
        Ok(Some(Action::Patch { path: file, op }))
    }

    fn next_terminal_action(&mut self) -> Result<Action, EngineError> {
        if !self.verification_emitted {
            self.verification_emitted = true;
            let command =
                self.config.verification_command.clone().unwrap_or_else(|| {
                    self.language().verification_command(&self.config.repo_root)
                });
            if !command.trim().is_empty() {
                return Ok(Action::Verify { command });
            }
        }
        Ok(Action::Done)
    }

    /// Deterministic offline synthesis, used only by unit tests and
    /// `--offline-mock`. A no-op patch preserves the file byte-for-byte.
    fn offline_action(&mut self, _session: &Session) -> Result<Action, EngineError> {
        if self.offline_patched {
            return Ok(Action::Done);
        }
        let raw = std::fs::read_to_string(self.config.repo_root.join(".ai").join("active_task.md"))
            .unwrap_or_default();
        let task = parse_active_task(&raw);
        let file = task
            .active
            .as_ref()
            .and_then(|a| a.files_allowed.first())
            .map(PathBuf::from)
            .or_else(|| {
                if self.config.repo_root.join("README.md").exists() {
                    Some(PathBuf::from("README.md"))
                } else {
                    None
                }
            });
        let Some(file) = file else {
            self.offline_patched = true;
            return Ok(Action::Done);
        };
        let full = self.config.repo_root.join(&file);
        let content = std::fs::read_to_string(&full).map_err(|e| {
            EngineError::Aborted(format!("offline-mock cannot read {}: {e}", file.display()))
        })?;
        let blocks = segment_blocks(&content);
        let Some(block) = blocks.first() else {
            self.offline_patched = true;
            return Ok(Action::Done);
        };
        let lines: Vec<&str> = content.lines().collect();
        let body = lines[block.start_line - 1..block.end_line].join("\n");
        let op = PatchOp::new(block.hash, body)
            .with_expected_file_hash(hash_file(&content))
            .with_signature(&block.signature);
        self.offline_patched = true;
        Ok(Action::Patch { path: file, op })
    }

    /// Resolve and return the model that will drive the active role. Used by
    /// the CLI to size the context window to the actual model.
    pub fn active_model(&mut self, role: &str) -> String {
        self.resolve_model(role)
    }

    /// Read-only architect query (`vibe-assembly chat`, `prompt --mode chat`).
    /// Never emits a patch and never writes to the repository.
    pub fn ask(&mut self, question: &str, session: &Session) -> Result<String, EngineError> {
        if self.config.offline_mock {
            return Ok(format!(
                "Offline mock read-only answer (mode=chat, language={}).\n\
                 No router call was made and no file was written.\n\n\
                 Question: {question}",
                self.language().as_str()
            ));
        }
        self.layers.reset_to_base();
        self.ensure_repomap();
        let (surface, _) = session
            .context_surface_with_budget(&self.config.context)
            .map_err(|e| EngineError::Aborted(format!("context surface: {e}")))?;
        let working_set = self.working_set(session);
        let prompt = format!(
            "QUESTION\n{question}\n\nREPOSITORY MAP (symbols bound to block hashes)\n{}\n\n\
             SESSION CONTEXT\n{}\n\n\
             Answer in prose with concrete file/block references. This is a read-only session: \
             do not emit patches, do not propose running mutating commands.",
            self.repomap_rendered.as_deref().unwrap_or(""),
            surface.render_with_working_set(&working_set),
        );
        self.complete("chat", &prompt, session)
    }

    /// Record an evidence-backed lesson for a failure (once per signature).
    fn reflect(&mut self, failure: &StepFailure) {
        if self
            .experience
            .entries()
            .iter()
            .any(|e| e.signature == failure.signature())
        {
            return;
        }
        let mut invariant_evidence = vec![format!("command `{}` failed", failure.command)];
        for line in failure.detail.lines().take(3) {
            if !line.trim().is_empty() {
                invariant_evidence.push(line.trim().to_string());
            }
        }
        let domain_evidence = failure
            .file
            .as_ref()
            .map(|f| vec![format!("file `{f}` is in scope")])
            .unwrap_or_default();
        let invariant = RecoveryCandidate::invariant(
            format!(
                "restore the last verified state, then satisfy `{}` before changing anything else",
                failure.command
            ),
            invariant_evidence,
        );
        let domain = RecoveryCandidate::domain(
            format!(
                "follow the existing {} pattern in this project before introducing a new one",
                failure.language
            ),
            domain_evidence,
        );
        let decision = ContrastiveRecovery::choose(&invariant, &domain);
        let entry = ContrastiveRecovery::reflect(failure, &decision);
        let _ = self.experience.record(entry);
    }
}

impl Planner for LiveRouterPlanner {
    fn next_action(&mut self, session: &Session) -> Result<Action, EngineError> {
        // Deterministic scripted actions (unit tests only).
        if let Some(actions) = &mut self.config.mock_actions {
            let action = actions
                .get(self.step_index)
                .cloned()
                .unwrap_or(Action::Done);
            self.step_index += 1;
            return Ok(action);
        }
        if let Some(action) = self.pending_actions.pop_front() {
            return Ok(action);
        }

        // Every new task starts from pristine harness surfaces.
        if self.step_index == 0 && !self.boss_done {
            self.layers.reset_to_base();
        }
        self.ensure_repomap();
        self.step_index += 1;

        if self.config.offline_mock {
            return self.offline_action(session);
        }

        if self.config.mode == ExecutionMode::Chat {
            // Read-only: produce a final answer, never a patch.
            let prompt = format!(
                "TASK\n{}\n\nREPOSITORY MAP\n{}\n\nSESSION CONTEXT\n{}\n\n\
                 Answer in prose. Do not emit patches; this mode cannot write files.",
                self.task_text(),
                self.repomap_rendered.as_deref().unwrap_or(""),
                session
                    .context_surface_with_budget(&self.config.context)
                    .map(|(s, _)| s.to_prompt_context())
                    .unwrap_or_default(),
            );
            let answer = self.complete("chat", &prompt, session)?;
            self.config.task_prompt = Some(format!(
                "{}\n\nArchitect answer:\n{}",
                self.config.task_prompt.clone().unwrap_or_default(),
                answer
            ));
            return Ok(Action::Done);
        }

        if !self.boss_done {
            self.boss_plan(session)?;
            if self.plan_targets.is_empty() {
                return self.next_terminal_action();
            }
        }

        if self.plan_targets.is_empty() {
            return self.next_terminal_action();
        }

        if !self.is_live() {
            return self.offline_action(session);
        }
        self.code_action(session)
    }

    fn on_failure(&mut self, failure: &StepFailure) -> bool {
        // Mock/offline runs keep deterministic fail-fast semantics.
        if !self.is_live() {
            return false;
        }
        self.last_diagnostics =
            DiagnosticPinpointer::pin_from_output(&self.config.repo_root, &failure.detail);
        if self.last_diagnostics.is_empty() {
            // Keep the raw failure text so Debug still receives evidence when
            // the compiler emitted no parseable span.
            self.last_failure_detail = Some(failure.detail.clone());
        } else {
            self.last_failure_detail = None;
        }
        // Re-target the last edited block so Debug edits the code it broke.
        // The patch replaced the block body, so the old hash is stale: resolve
        // the *new* hash by the stable signature captured before patching.
        if let Some((file, signature)) = self.last_patch.clone() {
            let full = self.config.repo_root.join(&file);
            let retargeted = std::fs::read_to_string(&full).ok().and_then(|content| {
                segment_blocks(&content)
                    .into_iter()
                    .find(|b| b.signature == signature)
                    .map(|b| b.hash)
            });
            self.plan_targets = vec![Target {
                file,
                block_hash: retargeted,
            }];
            self.boss_done = true;
        } else {
            self.boss_done = false;
        }
        let evidence = failure.detail.lines().take(5).collect::<Vec<_>>().join(" ");
        if let Some(base) = self.layers.get("c3_memory").map(str::to_string) {
            let _ = self.layers.set(
                "c3_memory",
                format!(
                    "{base}\nFailure evidence for `{}` ({}): {}",
                    failure.command,
                    failure.language,
                    evidence.trim()
                ),
            );
        }
        self.reflect(failure);
        self.verification_emitted = false;
        true
    }
}

/// Extract the first JSON object from model prose or fenced code.
fn extract_json(text: &str) -> Option<Value> {
    let cleaned = va_core::router_client::strip_think_blocks(text);
    let trimmed = cleaned.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Some(value);
    }
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if end > start {
                if let Ok(value) = serde_json::from_str::<Value>(&trimmed[start..=end]) {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or("").trim().to_string()
}

/// First non-comment, non-`role=model` model id in a combo file.
fn first_model_line(raw: &str) -> Option<String> {
    raw.lines()
        .map(|line| line.split('#').next().unwrap_or("").trim())
        .find(|line| !line.is_empty() && !line.contains('='))
        .map(str::to_string)
}

/// Resolve a short (4-hex) or full 64-bit hash against a file on disk.
fn resolve_hash(file: &Path, hash_text: &str, root: &Path) -> Option<u64> {
    let content = std::fs::read_to_string(root.join(file)).ok()?;
    lookup_block(&content, hash_text).map(|(hash, _)| hash)
}

fn lookup_block(content: &str, hash_text: &str) -> Option<(u64, crate::patcher::Block)> {
    let cleaned = hash_text
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X")
        .to_ascii_uppercase();
    let blocks = segment_blocks(content);
    if cleaned.len() <= 4 {
        let short = u16::from_str_radix(&cleaned, 16).ok()?;
        blocks
            .into_iter()
            .find(|b| ((b.hash >> 48) as u16) == short)
            .map(|b| (b.hash, b))
    } else {
        let full = u64::from_str_radix(&cleaned, 16).ok()?;
        blocks
            .into_iter()
            .find(|b| b.hash == full)
            .map(|b| (b.hash, b))
    }
}

/// Build a deterministic session observation for tests and offline runs.
pub fn observation_id(seed: &str) -> u64 {
    hash_str(seed)
}
