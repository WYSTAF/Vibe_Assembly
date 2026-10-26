//! Workbench state and the six-zone root composition.
//!
//! The pure state transitions live in `WorkbenchState` so they can be tested
//! without a window. `Workbench` only owns GPUI focus handles and the render
//! tree. This split is the main reason the old single-file dashboard could be
//! replaced without losing keyboard behaviour.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use gpui::prelude::*;
use gpui::{div, px, Context, FocusHandle, KeyDownEvent, Render, Window, WindowControlArea};
use va_core::knowledge::context_window_tokens;
use va_core::lock::{check_task_lock, LockStatus};
use va_core::modes::{ExecutionMode, LanguageLens};
use va_core::router_client::router_endpoint;
use va_core::state::Workspace;
use va_harness::diagnostics::{DiagnosticPinpointer, PinnedDiagnostic};
use va_harness::experience::{ExperienceStore, HarnessLayers};
use va_harness::judge::{DiffLine as JudgeDiffLine, Judge, Verdict};
use va_harness::patcher::{segment_blocks, Block};
use va_harness::repomap::RepoMap;
use va_harness::session::{ContextWindowConfig, NodeId, NodeKind, Session};

use crate::command_palette::PaletteAction;
use crate::widgets::{self, hash8};
use crate::{theme, ui_kit};

// --------------------------------------------------------------------- views

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeftView {
    Files,
    Outline,
    Dag,
}

impl LeftView {
    pub const ALL: [LeftView; 3] = [LeftView::Files, LeftView::Outline, LeftView::Dag];

    pub fn label(self) -> &'static str {
        match self {
            LeftView::Files => "Files",
            LeftView::Outline => "Symbols",
            LeftView::Dag => "DAG",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            LeftView::Files => "▤",
            LeftView::Outline => "◇",
            LeftView::Dag => "⑂",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomTab {
    Problems,
    Verify,
    Harness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorView {
    Code,
    Diff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepPhase {
    Observe,
    Think,
    Patch,
    Verify,
    Judge,
}

impl StepPhase {
    pub fn label(self) -> &'static str {
        match self {
            StepPhase::Observe => "Observe",
            StepPhase::Think => "Think",
            StepPhase::Patch => "Patch",
            StepPhase::Verify => "Verify",
            StepPhase::Judge => "Judge",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            StepPhase::Observe => "observe",
            StepPhase::Think => "think",
            StepPhase::Patch => "patch",
            StepPhase::Verify => "verify",
            StepPhase::Judge => "judge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchSource {
    WorkingTree,
    RecentCommit,
    Session,
}

impl PatchSource {
    pub fn label(self) -> &'static str {
        match self {
            PatchSource::WorkingTree => "Working tree",
            PatchSource::RecentCommit => "HEAD commit",
            PatchSource::Session => "Session DAG",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitOutcome {
    Empty,
    ReadOnly,
    Queued,
}

// -------------------------------------------------------------------- records

#[derive(Debug, Clone)]
pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub children: Vec<FileNode>,
}

#[derive(Debug, Clone)]
pub struct FileRow {
    pub path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub expanded: bool,
}

#[derive(Debug, Clone)]
pub struct OutlineRow {
    pub file_path: PathBuf,
    pub name: String,
    pub kind: String,
    pub line: usize,
    pub block_hash: u64,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct DagNodeView {
    pub id: NodeId,
    pub label: String,
    pub detail: String,
    pub cost_micros: Option<u64>,
    pub latency_ms: Option<u64>,
    pub verified: bool,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct OpenBuffer {
    pub path: PathBuf,
    pub rel_path: String,
    pub content: String,
    pub lines: Vec<String>,
    pub blocks: Vec<Block>,
}

impl OpenBuffer {
    fn new(path: PathBuf, rel_path: String, content: String) -> Self {
        let lines = content.lines().map(ToString::to_string).collect();
        let blocks = segment_blocks(&content);
        Self {
            path,
            rel_path,
            content,
            lines,
            blocks,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionStep {
    pub phase: StepPhase,
    pub title: String,
    pub detail: String,
    pub tokens: usize,
    pub cost_micros: u64,
    pub latency_ms: u64,
    pub think_evicted: bool,
    pub expanded: bool,
    pub status: String,
}

#[derive(Debug, Clone)]
pub enum ThreadItem {
    User { text: String },
    Assistant { text: String, read_only: bool },
    Step(ExecutionStep),
    System { text: String },
}

#[derive(Debug, Clone)]
pub struct DiffLineView {
    pub marker: char,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct PatchPreview {
    pub rel_path: String,
    pub source: PatchSource,
    pub summary: String,
    pub lines: Vec<DiffLineView>,
    pub block_hash: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteCommand {
    Mode(ExecutionMode),
    Language(LanguageLens),
    Compact,
    Tournament,
    Rollback,
    ToggleLeft,
    ToggleBottom,
    ToggleAssistant,
    RefreshRepoMap,
    CloseTab,
}

impl PaletteCommand {
    pub fn group(self) -> &'static str {
        match self {
            PaletteCommand::Mode(_) => "Mode",
            PaletteCommand::Language(_) => "Language",
            PaletteCommand::Compact | PaletteCommand::Tournament | PaletteCommand::Rollback => {
                "Harness"
            }
            PaletteCommand::ToggleLeft
            | PaletteCommand::ToggleBottom
            | PaletteCommand::ToggleAssistant
            | PaletteCommand::RefreshRepoMap
            | PaletteCommand::CloseTab => "View",
        }
    }

    pub fn label(self) -> String {
        match self {
            PaletteCommand::Mode(mode) => format!("Mode: {}", mode.short_label()),
            PaletteCommand::Language(language) => format!("Language: {}", language.label()),
            PaletteCommand::Compact => "Auto-Compact Context Surface".to_string(),
            PaletteCommand::Tournament => "Run Speculative Tournament".to_string(),
            PaletteCommand::Rollback => "Rollback DAG to Last Verified".to_string(),
            PaletteCommand::ToggleLeft => "Toggle Left Dock".to_string(),
            PaletteCommand::ToggleBottom => "Toggle Bottom Drawer".to_string(),
            PaletteCommand::ToggleAssistant => "Toggle Assistant Panel".to_string(),
            PaletteCommand::RefreshRepoMap => "Refresh RepoMap".to_string(),
            PaletteCommand::CloseTab => "Close Active Tab".to_string(),
        }
    }

    pub fn shortcut(self) -> &'static str {
        match self {
            PaletteCommand::ToggleLeft => "Ctrl+B",
            PaletteCommand::ToggleBottom => "Ctrl+J",
            PaletteCommand::ToggleAssistant => "Ctrl+Shift+A",
            PaletteCommand::CloseTab => "Ctrl+W",
            PaletteCommand::Mode(ExecutionMode::Code) => "Ctrl+1",
            PaletteCommand::Mode(ExecutionMode::Web) => "Ctrl+2",
            PaletteCommand::Mode(ExecutionMode::Chat) => "Ctrl+3",
            PaletteCommand::Mode(ExecutionMode::Debug) => "Ctrl+4",
            PaletteCommand::Compact => "Ctrl+K",
            PaletteCommand::Language(_)
            | PaletteCommand::Tournament
            | PaletteCommand::Rollback
            | PaletteCommand::RefreshRepoMap => "",
        }
    }

    pub fn is_active(self, state: &WorkbenchState) -> bool {
        match self {
            PaletteCommand::Mode(mode) => state.mode == mode,
            PaletteCommand::Language(language) => state.language == language,
            PaletteCommand::ToggleLeft => state.show_left,
            PaletteCommand::ToggleBottom => state.show_bottom,
            PaletteCommand::ToggleAssistant => state.show_assistant,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteScope {
    All,
    Mode,
    Language,
    Model,
}

// ---------------------------------------------------------------------- state

pub struct WorkbenchState {
    pub root: PathBuf,
    pub workspace: Workspace,
    pub error: Option<String>,
    pub tree: Vec<FileNode>,
    pub expanded_dirs: HashSet<PathBuf>,
    pub repo_map: RepoMap,
    pub open_buffers: Vec<OpenBuffer>,
    pub active_tab: Option<usize>,
    pub active_block: Option<usize>,
    pub cursor_line: usize,
    pub left_view: LeftView,
    pub show_left: bool,
    pub show_bottom: bool,
    pub show_assistant: bool,
    pub show_palette: bool,
    pub palette_query: String,
    pub palette_index: usize,
    pub palette_scope: PaletteScope,
    pub context_file: bool,
    pub pinned_block: Option<(PathBuf, u64)>,
    pub expanded_dag: HashSet<NodeId>,
    pub expanded_messages: HashSet<usize>,
    pub expanded_rules: HashSet<String>,
    pub show_inspector: bool,
    pub bottom_tab: BottomTab,
    pub editor_view: EditorView,
    pub mode: ExecutionMode,
    pub language: LanguageLens,
    pub model: String,
    pub models: Vec<String>,
    pub judge_on: bool,
    pub prompt_input: String,
    pub thread: Vec<ThreadItem>,
    pub diagnostics: Vec<PinnedDiagnostic>,
    pub session: Session,
    pub dag: Vec<DagNodeView>,
    pub experience: ExperienceStore,
    pub harness: HarnessLayers,
    pub verify_output: String,
    pub context_tokens: usize,
    pub context_window: usize,
    pub context_pct: usize,
    pub spend_micros: u64,
    pub latency_ms: u64,
    pub lock: LockStatus,
    pub git_branch: String,
    pub router_endpoint: String,
    pub router_live: bool,
    pub status_message: String,
    pub patch_preview: Option<PatchPreview>,
    pub tournament_runs: usize,
    pub last_verified: Option<NodeId>,
    pub head: Option<NodeId>,
}

impl WorkbenchState {
    pub fn new(
        workspace: Workspace,
        error: Option<String>,
        mode: ExecutionMode,
        language: LanguageLens,
        model: String,
    ) -> Self {
        let root = workspace.root.clone();
        let tree = scan_project_tree(&root);
        let mut expanded_dirs = HashSet::new();
        for dir in ["crates", "src"] {
            let path = root.join(dir);
            if path.is_dir() {
                expanded_dirs.insert(path);
            }
        }

        let repo_map = RepoMap::scan(&root);
        let session = load_session(&root);
        let experience = ExperienceStore::load(&root);
        let harness = HarnessLayers::base();
        let models = model_choices(&root, &model);
        let model = if model.trim().is_empty() {
            "auto-combo".to_string()
        } else {
            model
        };
        let (host, port, _) = router_endpoint();
        let router_endpoint = format!("{host}:{port}");
        let router_live = workspace.router_assignment.is_live;
        let lock = check_task_lock(&root);
        let git_branch = read_git_branch(&root);
        let context_window = context_window_tokens(&model).max(1);
        let verify_output = read_verification_file(&root);

        let mut state = Self {
            root,
            workspace,
            error,
            tree,
            expanded_dirs,
            repo_map,
            open_buffers: Vec::new(),
            active_tab: None,
            active_block: None,
            cursor_line: 1,
            left_view: LeftView::Files,
            show_left: true,
            show_bottom: true,
            show_assistant: true,
            show_palette: false,
            palette_query: String::new(),
            palette_index: 0,
            palette_scope: PaletteScope::All,
            context_file: true,
            pinned_block: None,
            expanded_dag: HashSet::new(),
            expanded_messages: HashSet::new(),
            expanded_rules: HashSet::new(),
            show_inspector: false,
            bottom_tab: BottomTab::Problems,
            editor_view: EditorView::Code,
            mode,
            language,
            model,
            models,
            judge_on: true,
            prompt_input: String::new(),
            thread: Vec::new(),
            diagnostics: Vec::new(),
            session,
            dag: Vec::new(),
            experience,
            harness,
            verify_output,
            context_tokens: 0,
            context_window,
            context_pct: 0,
            spend_micros: 0,
            latency_ms: 0,
            lock,
            git_branch,
            router_endpoint,
            router_live,
            status_message: "Workbench ready · Ctrl+P for commands".to_string(),
            patch_preview: None,
            tournament_runs: 0,
            last_verified: None,
            head: None,
        };

        state.refresh_session_views();
        state.open_initial_buffer();
        state.refresh_context_usage();
        let greeting = state.greeting();
        state.thread.push(ThreadItem::Assistant {
            text: greeting,
            read_only: false,
        });
        state.status_message = "Workbench ready · Ctrl+P for commands".to_string();
        state
    }

    // ------------------------------------------------------------- workspace

    pub fn rel_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }

    pub fn active_buffer(&self) -> Option<&OpenBuffer> {
        self.active_tab.and_then(|idx| self.open_buffers.get(idx))
    }

    pub fn active_rel_path(&self) -> Option<String> {
        self.active_buffer().map(|buffer| buffer.rel_path.clone())
    }

    pub fn active_file_path(&self) -> Option<PathBuf> {
        self.active_buffer().map(|buffer| buffer.path.clone())
    }

    pub fn open_file(&mut self, path: &Path) -> Result<(), String> {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };

        if absolute.is_dir() {
            return Err(format!("{} is a directory", self.rel_path(&absolute)));
        }

        if let Some(idx) = self.open_buffers.iter().position(|b| b.path == absolute) {
            self.active_tab = Some(idx);
            self.active_block = None;
            self.cursor_line = 1;
            self.status_message = format!("Switched to {}", self.rel_path(&absolute));
            if self.editor_view == EditorView::Diff {
                self.refresh_patch_preview();
            }
            return Ok(());
        }

        let content = std::fs::read_to_string(&absolute)
            .map_err(|err| format!("Cannot open {}: {err}", self.rel_path(&absolute)))?;
        if content.len() > 1_048_576 {
            return Err(format!(
                "{} is larger than the 1 MiB editor limit",
                self.rel_path(&absolute)
            ));
        }
        if content.contains('\0') {
            return Err(format!(
                "{} looks binary; the workbench only opens UTF-8 text",
                self.rel_path(&absolute)
            ));
        }

        let rel_path = self.rel_path(&absolute);
        let buffer = OpenBuffer::new(absolute.clone(), rel_path.clone(), content);
        self.open_buffers.push(buffer);
        self.active_tab = Some(self.open_buffers.len() - 1);
        self.active_block = None;
        self.cursor_line = 1;
        self.status_message = format!(
            "Opened {} · {} blocks",
            rel_path,
            self.active_buffer().map_or(0, |b| b.blocks.len())
        );
        if self.editor_view == EditorView::Diff {
            self.refresh_patch_preview();
        }
        Ok(())
    }

    pub fn open_initial_buffer(&mut self) {
        let active_file = self
            .workspace
            .active_task
            .active
            .as_ref()
            .and_then(|ticket| {
                ticket.files_allowed.iter().find_map(|candidate| {
                    let path = self.root.join(candidate);
                    path.is_file().then_some(path)
                })
            });
        let readme = self.root.join("README.md");
        let path = active_file
            .or_else(|| readme.is_file().then_some(readme))
            .or_else(|| first_file(&self.tree));

        if let Some(path) = path {
            if let Err(err) = self.open_file(&path) {
                self.status_message = err;
            }
        }
    }

    pub fn close_tab(&mut self, index: usize) {
        if index >= self.open_buffers.len() {
            return;
        }
        let previous_active = self.active_tab;
        self.open_buffers.remove(index);
        let len = self.open_buffers.len();
        self.active_tab = match previous_active {
            Some(active) if active == index => {
                if len == 0 {
                    None
                } else {
                    Some(index.min(len - 1))
                }
            }
            Some(active) if active > index => Some(active - 1),
            other => other,
        };
        self.active_block = None;
        self.cursor_line = 1;
        self.patch_preview = None;
        self.status_message = if self.open_buffers.is_empty() {
            "All tabs closed".to_string()
        } else {
            "Closed tab".to_string()
        };
        if self.editor_view == EditorView::Diff && self.active_buffer().is_some() {
            self.refresh_patch_preview();
        }
    }

    pub fn close_active_tab(&mut self) {
        match self.active_tab {
            Some(index) => self.close_tab(index),
            None => self.status_message = "No active tab to close".to_string(),
        }
    }

    pub fn visible_rows(&self) -> Vec<FileRow> {
        let mut rows = Vec::new();
        collect_visible_rows(&self.tree, 0, &self.expanded_dirs, &mut rows);
        rows
    }

    pub fn toggle_dir(&mut self, path: &Path) {
        if self.expanded_dirs.contains(path) {
            self.expanded_dirs.remove(path);
        } else {
            self.expanded_dirs.insert(path.to_path_buf());
        }
    }

    // --------------------------------------------------------------- selection

    pub fn select_block(&mut self, index: usize) {
        let Some(buffer) = self.active_buffer() else {
            return;
        };
        let Some(block) = buffer.blocks.get(index) else {
            return;
        };
        let start = block.start_line;
        let end = block.end_line;
        let hash = block.hash;
        self.pinned_block = self.active_file_path().map(|path| (path, hash));
        self.active_block = Some(index);
        self.cursor_line = start;
        self.status_message = format!("Pinned block {} · lines {start}–{end}", hash8(hash));
    }

    pub fn block_index_at_line(&self, line: usize) -> Option<usize> {
        let buffer = self.active_buffer()?;
        buffer
            .blocks
            .iter()
            .position(|block| block.start_line <= line && line <= block.end_line)
    }

    pub fn current_block_index(&self) -> Option<usize> {
        self.active_block
            .or_else(|| self.block_index_at_line(self.cursor_line))
    }

    pub fn current_block_hash(&self) -> Option<u64> {
        let index = self.current_block_index()?;
        self.active_buffer()?
            .blocks
            .get(index)
            .map(|block| block.hash)
    }

    pub fn active_symbol_name(&self) -> Option<String> {
        let rel = self.active_rel_path()?;
        let entry = self.repo_map.files.get(Path::new(&rel))?;
        entry
            .symbols
            .iter()
            .filter(|symbol| symbol.line <= self.cursor_line)
            .max_by_key(|symbol| symbol.line)
            .map(|symbol| symbol.name.clone())
    }

    pub fn breadcrumb(&self) -> (String, Option<String>, Option<String>) {
        let path = self
            .active_rel_path()
            .unwrap_or_else(|| "No file open".to_string());
        let symbol = self.active_symbol_name();
        let hash = self.current_block_hash().map(hash8);
        (path, symbol, hash)
    }

    pub fn open_at_block(&mut self, path: &Path, hash: u64) -> Result<(), String> {
        self.open_file(path)?;
        let index = self
            .active_buffer()
            .and_then(|buffer| buffer.blocks.iter().position(|block| block.hash == hash));
        if let Some(index) = index {
            self.select_block(index);
            self.editor_view = EditorView::Code;
            self.status_message = format!("Jumped to block {}", hash8(hash));
            Ok(())
        } else {
            Err(format!(
                "Block {} is not present in {}",
                hash8(hash),
                self.rel_path(path)
            ))
        }
    }

    pub fn outline_rows(&self) -> Vec<OutlineRow> {
        let active_rel = self.active_rel_path();
        let mut rows = Vec::new();

        if let Some(rel) = active_rel.as_deref() {
            if let Some(entry) = self.repo_map.files.get(Path::new(rel)) {
                if entry.symbols.is_empty() {
                    for block in &entry.blocks {
                        let name = if block.signature.is_empty() {
                            format!("block {}–{}", block.start_line, block.end_line)
                        } else {
                            widgets::truncate(&block.signature, 56)
                        };
                        rows.push(OutlineRow {
                            file_path: entry.file_path.clone(),
                            name,
                            kind: "block".to_string(),
                            line: block.start_line,
                            block_hash: block.hash,
                            is_active: self.current_block_hash() == Some(block.hash),
                        });
                    }
                } else {
                    for symbol in &entry.symbols {
                        rows.push(OutlineRow {
                            file_path: entry.file_path.clone(),
                            name: symbol.name.clone(),
                            kind: symbol.kind.as_str().to_string(),
                            line: symbol.line,
                            block_hash: symbol.block_hash,
                            is_active: self.cursor_line >= symbol.block_range.0
                                && self.cursor_line <= symbol.block_range.1,
                        });
                    }
                }
            }
        }

        if rows.is_empty() {
            for (path, entry) in &self.repo_map.files {
                for symbol in entry.symbols.iter().take(80) {
                    rows.push(OutlineRow {
                        file_path: path.clone(),
                        name: symbol.name.clone(),
                        kind: symbol.kind.as_str().to_string(),
                        line: symbol.line,
                        block_hash: symbol.block_hash,
                        is_active: false,
                    });
                    if rows.len() >= 80 {
                        break;
                    }
                }
                if rows.len() >= 80 {
                    break;
                }
            }
        }

        rows
    }

    // ---------------------------------------------------------------- session

    fn push_session(&mut self, kind: NodeKind) {
        self.session.push(kind);
        let _ = self.session.commit();
    }

    pub fn refresh_session_views(&mut self) {
        let path: HashSet<NodeId> = self
            .session
            .current_path()
            .unwrap_or_default()
            .into_iter()
            .collect();
        let head = self.session.head();
        let mut latest_verification = None;

        self.dag = self
            .session
            .nodes()
            .map(|node| {
                if let NodeKind::Verification { detail, .. } = &node.kind {
                    latest_verification = Some(detail.clone());
                }
                let detail = match &node.kind {
                    NodeKind::Prompt { role, text } => {
                        format!("{role}: {}", widgets::first_line(text))
                    }
                    NodeKind::Thought { text } => widgets::first_line(text),
                    NodeKind::ToolCall { name, arguments } => {
                        format!("{name} · {}", widgets::truncate(arguments, 72))
                    }
                    NodeKind::Observation { text } => widgets::first_line(text),
                    NodeKind::Patch { path, summary } => {
                        format!("{path} · {summary}")
                    }
                    NodeKind::Verification { passed, detail } => {
                        let verdict = if *passed { "passed" } else { "failed" };
                        format!("{verdict} · {}", widgets::first_line(detail))
                    }
                    NodeKind::Fork { reason } => widgets::first_line(reason),
                    NodeKind::Compacted {
                        start,
                        end,
                        summary,
                    } => {
                        format!("{start}–{end} · {}", widgets::first_line(summary))
                    }
                };
                DagNodeView {
                    id: node.id,
                    label: node.kind.label().to_string(),
                    detail,
                    cost_micros: node.cost_micros,
                    latency_ms: node.latency_ms,
                    verified: node.judge_verified_hash.is_some()
                        || matches!(&node.kind, NodeKind::Verification { passed: true, .. }),
                    active: path.contains(&node.id),
                }
            })
            .collect();

        self.head = head;
        self.last_verified = self.session.last_verified().ok().flatten();
        self.spend_micros = self.session.total_cost_micros();
        self.latency_ms = self.session.total_latency_ms();

        if self.verify_output.trim().is_empty() {
            self.verify_output = latest_verification
                .or_else(|| {
                    let path = self.root.join(".ai").join("verification.md");
                    std::fs::read_to_string(path).ok()
                })
                .unwrap_or_default();
        }
    }

    pub fn refresh_context_usage(&mut self) {
        let window = context_window_tokens(&self.model).max(1);
        let config = ContextWindowConfig::new(window);
        let tokens = self
            .session
            .context_surface_with_budget(&config)
            .map(|(surface, _)| surface.estimate_tokens())
            .unwrap_or(0);
        self.context_window = window;
        self.context_tokens = tokens;
        self.context_pct = tokens
            .saturating_mul(100)
            .checked_div(window)
            .unwrap_or(0)
            .min(100);
    }

    pub fn compact_context(&mut self) {
        let config = ContextWindowConfig::new(self.context_window.max(1));
        match self.session.context_surface_with_budget(&config) {
            Ok((surface, compacted)) => {
                self.context_tokens = surface.estimate_tokens();
                self.context_pct = self
                    .context_tokens
                    .saturating_mul(100)
                    .checked_div(self.context_window.max(1))
                    .unwrap_or(0)
                    .min(100);
                let message = if compacted > 0 {
                    format!(
                        "Auto-compacted {compacted} context item(s) to the 75% watermark · {} tokens",
                        self.context_tokens
                    )
                } else {
                    format!(
                        "Context surface is already below the 75% watermark · {} tokens",
                        self.context_tokens
                    )
                };
                self.thread.push(ThreadItem::System {
                    text: message.clone(),
                });
                self.status_message = message;
            }
            Err(err) => {
                self.status_message = format!("Context compaction unavailable: {err}");
            }
        }
    }

    // ------------------------------------------------------------------- input

    #[cfg(test)]
    pub fn push_char(&mut self, ch: char) {
        if self.prompt_input.chars().count() < 2_000 {
            self.prompt_input.push(ch);
        }
    }

    #[cfg(test)]
    pub fn push_str(&mut self, text: &str) {
        if self.prompt_input.len() + text.len() <= 4_000 {
            self.prompt_input.push_str(text);
        }
    }

    #[cfg(test)]
    pub fn backspace(&mut self) {
        self.prompt_input.pop();
    }

    pub fn submit_prompt(&mut self) -> SubmitOutcome {
        let text = self.prompt_input.trim().to_string();
        if text.is_empty() {
            self.status_message = "Prompt is empty".to_string();
            return SubmitOutcome::Empty;
        }

        self.prompt_input.clear();
        self.thread.push(ThreadItem::User { text: text.clone() });
        self.push_session(NodeKind::Prompt {
            role: "user".to_string(),
            text: text.clone(),
        });

        let read_only = !self.mode.allows_writes();
        let reference = self.context_reference();
        let model = self.model_label();

        if read_only {
            let response = format!(
                "Read-only {} session.\n\nI will use {reference} as the context anchor. \
                 Chat / Architect cannot emit patches, write files, or run mutating commands. \
                 Switch to Code, Web, or Debug before asking for an edit.",
                self.mode.short_label()
            );
            self.push_session(NodeKind::Thought {
                text: response.clone(),
            });
            self.thread.push(ThreadItem::Assistant {
                text: response,
                read_only: true,
            });
            self.status_message = "Read-only response added; no patch staged".to_string();
            self.refresh_session_views();
            self.refresh_context_usage();
            return SubmitOutcome::ReadOnly;
        }

        let file = self
            .active_rel_path()
            .unwrap_or_else(|| "workspace".to_string());
        let symbol = self
            .active_symbol_name()
            .unwrap_or_else(|| "selection".to_string());
        let block = self
            .current_block_hash()
            .map(hash8)
            .unwrap_or_else(|| "no block pinned".to_string());
        let tokens = estimate_tokens(&text);

        self.thread.push(ThreadItem::Step(ExecutionStep {
            phase: StepPhase::Observe,
            title: "Observe workspace".to_string(),
            detail: format!(
                "{file} · {symbol} · block {block} · {}",
                self.resolved_language().short_label()
            ),
            tokens: self.context_tokens.max(tokens),
            cost_micros: 0,
            latency_ms: 0,
            think_evicted: false,
            expanded: false,
            status: "ready".to_string(),
        }));
        self.thread.push(ThreadItem::Step(ExecutionStep {
            phase: StepPhase::Think,
            title: "Think before patch".to_string(),
            detail: format!(
                "{model} plans against the {} lens. The raw thought stays in the DAG until the patch step.",
                self.resolved_language().short_label()
            ),
            tokens,
            cost_micros: 0,
            latency_ms: 0,
            think_evicted: false,
            expanded: false,
            status: "planning".to_string(),
        }));
        self.thread.push(ThreadItem::Step(ExecutionStep {
            phase: StepPhase::Patch,
            title: "Patch preview".to_string(),
            detail: "No filesystem write has been staged. The live CLI applies block patches through va-harness::patcher after verification and the judge pass.".to_string(),
            tokens: 0,
            cost_micros: 0,
            latency_ms: 0,
            think_evicted: true,
            expanded: false,
            status: "preview".to_string(),
        }));
        self.thread.push(ThreadItem::Step(ExecutionStep {
            phase: StepPhase::Verify,
            title: "Verification gate".to_string(),
            detail: self.resolved_language().verification_command(&self.root),
            tokens: 0,
            cost_micros: 0,
            latency_ms: 0,
            think_evicted: true,
            expanded: false,
            status: "gated".to_string(),
        }));
        self.thread.push(ThreadItem::Step(ExecutionStep {
            phase: StepPhase::Judge,
            title: "Independent judge".to_string(),
            detail: if self.judge_on {
                "Judge ON. Open Diff and run Judge to review the current patch before applying it."
                    .to_string()
            } else {
                "Judge is OFF. Enable the independent judge before applying a patch.".to_string()
            },
            tokens: 0,
            cost_micros: 0,
            latency_ms: 0,
            think_evicted: false,
            expanded: false,
            status: if self.judge_on { "armed" } else { "off" }.to_string(),
        }));

        let plan = format!(
            "Plan staged for {reference} in {} mode.\n\n\
             The workbench has recorded the prompt in the session DAG, but has not written files. \
             Select a block, open Diff, and run Judge before handing the plan to the live CLI.",
            self.mode.short_label()
        );
        self.push_session(NodeKind::Thought { text: plan.clone() });
        self.thread.push(ThreadItem::Assistant {
            text: plan,
            read_only: false,
        });
        self.status_message = format!(
            "Prompt queued in {} mode; no file writes",
            self.mode.short_label()
        );
        self.refresh_session_views();
        self.refresh_context_usage();
        SubmitOutcome::Queued
    }

    pub fn clear_thread(&mut self) {
        self.prompt_input.clear();
        self.thread.clear();
        self.expanded_messages.clear();
        self.thread.push(ThreadItem::Assistant {
            text: "New thread. The session DAG and file buffers are unchanged.".to_string(),
            read_only: false,
        });
        self.status_message = "New assistant thread".to_string();
    }

    pub fn context_reference(&self) -> String {
        let mut references = Vec::new();
        if self.context_file {
            if let Some(path) = self.active_rel_path() {
                references.push(path);
            }
        }
        if let Some((path, hash)) = &self.pinned_block {
            references.push(format!("{} [#{}]", self.rel_path(path), hash8(*hash)));
        }
        if references.is_empty() {
            "workspace".into()
        } else {
            references.join(" · ")
        }
    }

    pub fn context_label(&self) -> String {
        format!(
            "Context: {}% · {}/{} · Auto-Compact @ 75%",
            self.context_pct,
            compact_number(self.context_tokens),
            compact_number(self.context_window)
        )
    }

    pub fn model_label(&self) -> String {
        if self.model == "auto-combo" {
            "Auto-Combo".to_string()
        } else {
            self.model.clone()
        }
    }

    pub fn resolved_language(&self) -> LanguageLens {
        self.language.resolve(&self.root)
    }

    pub fn language_label(&self) -> String {
        if self.language == LanguageLens::Auto {
            format!("{} (Auto)", self.resolved_language().short_label())
        } else {
            self.language.short_label().to_string()
        }
    }

    pub fn ticket_label(&self) -> String {
        match (
            self.workspace.active_task.active_id.as_deref(),
            self.workspace.active_task.active.as_ref(),
        ) {
            (Some(id), Some(ticket)) => {
                format!("{id} · {}", widgets::truncate(&ticket.title, 34))
            }
            (Some(id), None) if id != "none" => id.to_string(),
            _ => "No active ticket".to_string(),
        }
    }

    pub fn lock_label(&self) -> &'static str {
        match self.lock {
            LockStatus::Unlocked => "FREE",
            LockStatus::Held(_) => "HELD",
            LockStatus::Stale(_) => "STALE",
        }
    }

    pub fn symbol_count(&self) -> usize {
        self.repo_map
            .files
            .values()
            .map(|entry| entry.symbols.len())
            .sum()
    }

    pub fn block_count(&self) -> usize {
        self.repo_map
            .files
            .values()
            .map(|entry| entry.blocks.len())
            .sum()
    }

    pub fn file_count(&self) -> usize {
        self.repo_map.files.len()
    }

    // -------------------------------------------------------------- transitions

    #[cfg(test)]
    pub fn cycle_mode(&mut self) {
        let index = ExecutionMode::ALL
            .iter()
            .position(|mode| *mode == self.mode)
            .unwrap_or(0);
        self.set_mode(ExecutionMode::ALL[(index + 1) % ExecutionMode::ALL.len()]);
    }

    pub fn set_mode(&mut self, mode: ExecutionMode) {
        self.mode = mode;
        self.status_message = if mode.allows_writes() {
            format!("Mode: {} · writes enabled", mode.label())
        } else {
            format!("Mode: {} · read-only contract enforced", mode.label())
        };
    }

    #[cfg(test)]
    pub fn cycle_language(&mut self) {
        let index = LanguageLens::ALL
            .iter()
            .position(|language| *language == self.language)
            .unwrap_or(0);
        self.set_language(LanguageLens::ALL[(index + 1) % LanguageLens::ALL.len()]);
    }

    pub fn set_language(&mut self, language: LanguageLens) {
        self.language = language;
        self.status_message = format!("Language lens: {}", self.language_label());
    }

    #[cfg(test)]
    pub fn cycle_model(&mut self) {
        if self.models.is_empty() {
            self.models.push("auto-combo".to_string());
        }
        let index = self
            .models
            .iter()
            .position(|model| *model == self.model)
            .unwrap_or(0);
        let next = self.models[(index + 1) % self.models.len()].clone();
        self.set_model(next);
    }

    pub fn set_model(&mut self, model: String) {
        if !self.models.contains(&model) {
            self.models.push(model.clone());
        }
        self.model = model;
        self.refresh_context_usage();
        self.status_message = format!("Model: {} · {}", self.model_label(), self.context_label());
    }

    pub fn toggle_judge(&mut self) {
        self.judge_on = !self.judge_on;
        self.status_message = if self.judge_on {
            "Independent judge enabled".to_string()
        } else {
            "Independent judge disabled".to_string()
        };
    }

    pub fn toggle_left(&mut self) {
        self.show_left = !self.show_left;
    }

    pub fn toggle_bottom(&mut self) {
        self.show_bottom = !self.show_bottom;
    }

    pub fn toggle_assistant(&mut self) {
        self.show_assistant = !self.show_assistant;
    }

    pub fn toggle_palette(&mut self) {
        self.show_palette = !self.show_palette;
        self.palette_query.clear();
        self.palette_index = 0;
        self.palette_scope = PaletteScope::All;
    }

    pub fn set_left_view(&mut self, view: LeftView) {
        self.left_view = view;
        self.show_left = true;
    }

    pub fn set_bottom_tab(&mut self, tab: BottomTab) {
        self.bottom_tab = tab;
        self.show_bottom = true;
    }

    pub fn set_editor_view(&mut self, view: EditorView) {
        self.editor_view = view;
        if view == EditorView::Diff {
            self.refresh_patch_preview();
        }
    }

    pub fn execute_palette(&mut self, command: PaletteCommand) {
        match command {
            PaletteCommand::Mode(mode) => self.set_mode(mode),
            PaletteCommand::Language(language) => self.set_language(language),
            PaletteCommand::Compact => self.compact_context(),
            PaletteCommand::Tournament => self.run_tournament(),
            PaletteCommand::Rollback => self.rollback(),
            PaletteCommand::ToggleLeft => self.toggle_left(),
            PaletteCommand::ToggleBottom => self.toggle_bottom(),
            PaletteCommand::ToggleAssistant => self.toggle_assistant(),
            PaletteCommand::RefreshRepoMap => self.refresh(),
            PaletteCommand::CloseTab => self.close_active_tab(),
        }
        self.show_palette = false;
    }

    pub fn apply_palette_action(&mut self, action: PaletteAction) {
        match action {
            PaletteAction::Command(command) => self.execute_palette(command),
            PaletteAction::Model(model) => self.set_model(model),
            PaletteAction::File(path) => {
                if let Err(error) = self.open_file(&path) {
                    self.status_message = error;
                }
            }
            PaletteAction::Symbol(path, hash) => {
                if let Err(error) = self.open_at_block(&path, hash) {
                    self.status_message = error;
                }
            }
        }
        self.show_palette = false;
    }

    pub fn refresh(&mut self) {
        self.tree = scan_project_tree(&self.root);
        self.repo_map = RepoMap::scan(&self.root);
        self.lock = check_task_lock(&self.root);
        self.git_branch = read_git_branch(&self.root);
        self.status_message = format!(
            "Refreshed · {} files · {} symbols · {} blocks",
            self.file_count(),
            self.symbol_count(),
            self.block_count()
        );
    }

    pub fn run_tournament(&mut self) {
        self.tournament_runs += 1;
        self.thread.push(ThreadItem::Step(ExecutionStep {
            phase: StepPhase::Think,
            title: format!("Speculative tournament #{}", self.tournament_runs),
            detail: "Three candidate branches · judge gate remains independent · no branch is applied by the desktop preview.".to_string(),
            tokens: self.context_tokens,
            cost_micros: 0,
            latency_ms: 0,
            think_evicted: false,
            expanded: false,
            status: "preview".to_string(),
        }));
        self.status_message = format!("Speculative tournament #{} staged", self.tournament_runs);
    }

    pub fn rollback(&mut self) {
        match self.session.rollback_to_last_verified() {
            Ok(Some(id)) => {
                self.refresh_session_views();
                self.refresh_context_usage();
                let message = format!(
                    "Rolled DAG head back to verified node {id}; the working tree was not touched."
                );
                self.thread.push(ThreadItem::System {
                    text: message.clone(),
                });
                self.status_message = message;
            }
            Ok(None) => {
                self.status_message = "No verified checkpoint in the session DAG".to_string();
            }
            Err(err) => {
                self.status_message = format!("Rollback failed: {err}");
            }
        }
    }

    pub fn reset_harness(&mut self) {
        self.harness.reset_to_base();
        let message = "Harness² c1..c8 reset to the pristine base overlays".to_string();
        self.thread.push(ThreadItem::System {
            text: message.clone(),
        });
        self.status_message = message;
    }

    pub fn run_diagnostics(&mut self) {
        self.show_bottom = true;
        self.bottom_tab = BottomTab::Problems;
        self.status_message = "Running cargo check --workspace…".to_string();
        let output = Command::new("cargo")
            .args(["check", "--workspace", "--message-format=json"])
            .current_dir(&self.root)
            .output();

        match output {
            Ok(output) => {
                let mut combined = String::new();
                combined.push_str(&String::from_utf8_lossy(&output.stdout));
                if !output.stderr.is_empty() {
                    combined.push('\n');
                    combined.push_str(&String::from_utf8_lossy(&output.stderr));
                }
                self.verify_output = if combined.trim().is_empty() {
                    "cargo check produced no output.".to_string()
                } else {
                    combined
                };
                self.diagnostics =
                    DiagnosticPinpointer::pin_from_output(&self.root, &self.verify_output);
                let message = if self.diagnostics.is_empty() {
                    "cargo check finished with no pinned diagnostics".to_string()
                } else {
                    format!(
                        "cargo check pinned {} diagnostic(s) to block hashes",
                        self.diagnostics.len()
                    )
                };
                self.thread.push(ThreadItem::System {
                    text: message.clone(),
                });
                self.status_message = message;
            }
            Err(err) => {
                self.status_message = format!("cargo check could not start: {err}");
            }
        }
    }

    pub fn refresh_patch_preview(&mut self) {
        let Some(path) = self.active_file_path() else {
            self.patch_preview = None;
            return;
        };
        let rel_path = self.rel_path(&path);
        let block_hash = self.current_block_hash();

        let mut source = PatchSource::WorkingTree;
        let mut lines = git_diff(&self.root, &rel_path, false);
        if lines.is_empty() {
            source = PatchSource::RecentCommit;
            lines = git_diff(&self.root, &rel_path, true);
        }

        if lines.is_empty() {
            if let Some((session_path, summary)) = self.latest_session_patch() {
                if session_path == rel_path {
                    self.patch_preview = Some(PatchPreview {
                        rel_path,
                        source: PatchSource::Session,
                        summary,
                        lines: Vec::new(),
                        block_hash,
                    });
                    return;
                }
            }
            self.patch_preview = None;
            return;
        }

        let summary = match source {
            PatchSource::WorkingTree => "Uncommitted working-tree patch".to_string(),
            PatchSource::RecentCommit => "Most recent commit touching this file".to_string(),
            PatchSource::Session => "Session DAG patch summary".to_string(),
        };
        self.patch_preview = Some(PatchPreview {
            rel_path,
            source,
            summary,
            lines,
            block_hash,
        });
    }

    fn latest_session_patch(&self) -> Option<(String, String)> {
        self.session
            .nodes()
            .filter_map(|node| match &node.kind {
                NodeKind::Patch { path, summary } => {
                    Some((path.replace('\\', "/"), summary.clone()))
                }
                _ => None,
            })
            .last()
    }

    pub fn accept_patch(&mut self) {
        let Some(patch) = self.patch_preview.as_ref() else {
            self.status_message = "No patch preview to accept".to_string();
            return;
        };
        let message = match patch.source {
            PatchSource::WorkingTree => {
                "Working-tree patch accepted; the files already contain these changes".to_string()
            }
            PatchSource::RecentCommit => {
                "This patch is already committed at HEAD; nothing to apply".to_string()
            }
            PatchSource::Session => {
                "The session DAG patch summary has no line-level replacement to apply".to_string()
            }
        };
        self.thread.push(ThreadItem::System {
            text: message.clone(),
        });
        self.status_message = message;
    }

    pub fn run_judge_preview(&mut self) {
        let Some(patch) = self.patch_preview.as_ref() else {
            self.status_message = "No patch preview to judge".to_string();
            return;
        };
        let diff: Vec<JudgeDiffLine> = patch
            .lines
            .iter()
            .filter(|line| line.marker == '+' || line.marker == '-')
            .map(|line| JudgeDiffLine {
                marker: line.marker,
                path: patch.rel_path.clone(),
                text: line.text.clone(),
            })
            .collect();
        if diff.is_empty() {
            let message =
                "Judge needs added or removed lines; the current preview is a DAG summary"
                    .to_string();
            self.thread.push(ThreadItem::System {
                text: message.clone(),
            });
            self.status_message = message;
            return;
        }

        let line_numbers = vec![0usize; diff.len()];
        let verdict = Judge::with_defaults().review(&diff, &line_numbers);
        let message = match verdict {
            Verdict::Pass => format!(
                "Judge PASS · {} changed line(s) · {}",
                diff.len(),
                patch.rel_path
            ),
            Verdict::Reject(findings) => {
                let detail = findings
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("Judge REJECT · {} finding(s)\n{detail}", findings.len())
            }
        };
        self.thread.push(ThreadItem::System {
            text: message.clone(),
        });
        self.status_message = message
            .lines()
            .next()
            .unwrap_or("Judge finished")
            .to_string();
    }

    // --------------------------------------------------------------- greeting

    fn greeting(&self) -> String {
        "What would you like to build?\n\nPin a block from the editor or add a file for context. I’ll help you plan the next change.\n\nThis desktop stages previews; the CLI runs edits and verification.".to_string()
    }
}

// ---------------------------------------------------------------- GPUI entity

pub struct Workbench {
    pub state: WorkbenchState,
    pub composer_focus: FocusHandle,
    pub root_focus: FocusHandle,
    pub palette_focus: FocusHandle,
    pub prompt_selection: ui_kit::InputSelection,
    pub palette_selection: ui_kit::InputSelection,
    pub palette_scroll: gpui::ScrollHandle,
    pub thread_scroll: gpui::ScrollHandle,
}

impl Workbench {
    pub fn new(state: WorkbenchState, cx: &mut Context<Self>) -> Self {
        Self {
            state,
            composer_focus: cx.focus_handle(),
            root_focus: cx.focus_handle(),
            palette_focus: cx.focus_handle(),
            prompt_selection: Default::default(),
            palette_selection: Default::default(),
            palette_scroll: Default::default(),
            thread_scroll: Default::default(),
        }
    }

    fn handle_shortcut(
        &mut self,
        keystroke: &gpui::Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !keystroke.modifiers.control {
            return false;
        }
        match keystroke.key.as_str() {
            "p" => {
                if self.state.show_palette {
                    self.state.toggle_palette();
                    self.restore_focus(window);
                } else {
                    self.open_palette(PaletteScope::All, window, cx);
                }
                true
            }
            "1" | "2" | "3" | "4" => {
                let index = keystroke.key.parse::<usize>().unwrap_or(1) - 1;
                self.state.set_mode(ExecutionMode::ALL[index]);
                true
            }
            "k" => {
                self.state.compact_context();
                true
            }
            "b" => {
                self.state.toggle_left();
                true
            }
            "j" => {
                self.state.toggle_bottom();
                true
            }
            "w" => {
                self.state.close_active_tab();
                true
            }
            "a" if keystroke.modifiers.shift => {
                self.state.toggle_assistant();
                self.restore_focus(window);
                true
            }
            "q" => {
                cx.quit();
                true
            }
            _ => false,
        }
    }

    pub fn on_composer_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.handle_shortcut(&event.keystroke, window, cx) {
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if self.text_shortcut(&event.keystroke, cx) {
            cx.stop_propagation();
            cx.notify();
            return;
        }
        let key = &event.keystroke;
        if key.modifiers.control || key.modifiers.alt || key.modifiers.platform {
            return;
        }
        if key.key == "enter" {
            if key.modifiers.shift {
                self.edit_input(|text, selection| {
                    selection.replace(text, None, "\n");
                });
            } else {
                self.state.submit_prompt();
                self.prompt_selection = Default::default();
                self.thread_scroll.scroll_to_bottom();
            }
            cx.stop_propagation();
            cx.notify();
        }
    }

    pub fn on_root_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.handle_shortcut(&event.keystroke, window, cx) {
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if event.keystroke.key == "escape" && self.state.show_palette {
            self.state.show_palette = false;
            cx.stop_propagation();
            cx.notify();
        }
    }

    pub fn restore_focus(&self, window: &mut Window) {
        window.focus(if self.state.show_assistant {
            &self.composer_focus
        } else {
            &self.root_focus
        });
    }

    pub fn open_palette(
        &mut self,
        scope: PaletteScope,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.show_palette = true;
        self.state.palette_scope = scope;
        self.state.palette_query.clear();
        self.palette_selection = Default::default();
        self.palette_scroll
            .set_offset(gpui::point(px(0.0), px(0.0)));
        self.state.palette_index = 0;
        window.focus(&self.palette_focus);
        cx.notify();
    }

    pub fn on_palette_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.handle_shortcut(&event.keystroke, window, cx) {
            cx.stop_propagation();
            cx.notify();
            return;
        }
        let key = &event.keystroke;
        if self.text_shortcut(key, cx) {
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if key.modifiers.control || key.modifiers.alt || key.modifiers.platform {
            return;
        }
        match key.key.as_str() {
            "escape" => {
                self.state.show_palette = false;
                self.restore_focus(window);
            }
            "up" => {
                self.state.palette_index = self.state.palette_index.saturating_sub(1);
            }
            "down" => {
                let count = crate::command_palette::items(&self.state).len();
                self.state.palette_index =
                    (self.state.palette_index + 1).min(count.saturating_sub(1));
            }
            "enter" => {
                let items = crate::command_palette::items(&self.state);
                if let Some(item) = items.get(self.state.palette_index) {
                    self.state.apply_palette_action(item.action.clone());
                    self.restore_focus(window);
                }
            }
            _ => return,
        }
        let items = crate::command_palette::items(&self.state);
        let mut groups = 0;
        let mut previous = "";
        for item in items.iter().take(self.state.palette_index + 1) {
            if previous != item.group {
                groups += 1;
                previous = item.group;
            }
        }
        self.palette_scroll
            .scroll_to_item(self.state.palette_index + groups);
        cx.stop_propagation();
        cx.notify();
    }

    fn render_titlebar(
        &self,
        accent: gpui::Rgba,
        _accent_strong: gpui::Rgba,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let (path, symbol, hash) = self.state.breadcrumb();
        let workspace = self
            .state
            .root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("workspace")
            .to_string();
        let router_color = if self.state.router_live {
            theme::success()
        } else {
            theme::text_muted()
        };
        let left = div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(
                div()
                    .window_control_area(WindowControlArea::Drag)
                    .child(crate::branding::logo(22.0)),
            )
            .child(
                ui_kit::button("workspace-pill", workspace)
                    .bg(theme::elevated())
                    .text_size(px(11.5))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.set_left_view(LeftView::Files);
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(px(theme::MICRO))
                    .text_color(theme::text_muted())
                    .child(path.rsplit('/').next().unwrap_or(&path).to_string()),
            )
            .when_some(symbol, |this, symbol| {
                this.child(
                    div()
                        .max_w(px(110.0))
                        .truncate()
                        .text_size(px(theme::MICRO))
                        .text_color(theme::text_secondary())
                        .child(format!("› {symbol}")),
                )
            })
            .when_some(hash, |this, hash| {
                this.child(
                    ui_kit::button("breadcrumb-hash", format!("#{hash}"))
                        .font_family(theme::code_font())
                        .text_size(px(theme::MICRO))
                        .px(px(6.0))
                        .text_color(accent)
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(index) = this.state.current_block_index() {
                                this.state.select_block(index);
                            }
                            if let Some(hash) = this.state.current_block_hash() {
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(hash8(hash)));
                            }
                            cx.notify();
                        })),
                )
            });
        let search = ui_kit::button("command-search", "")
            .w(px(360.0))
            .h(px(26.0))
            .justify_start()
            .bg(theme::elevated())
            .border_1()
            .border_color(theme::border())
            .text_size(px(11.5))
            .text_color(theme::text_muted())
            .child("⌕")
            .child(div().flex_1().child("Search files, symbols, or commands…"))
            .child(widgets::keycap("Ctrl+P"))
            .on_click(
                cx.listener(|this, _, window, cx| this.open_palette(PaletteScope::All, window, cx)),
            );
        let right = div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .justify_end()
            .gap(px(4.0))
            .child(
                ui_kit::icon_button("toggle-left", "◧", self.state.show_left).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.state.toggle_left();
                        cx.notify();
                    }),
                ),
            )
            .child(
                ui_kit::icon_button("toggle-bottom", "⊟", self.state.show_bottom).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.state.toggle_bottom();
                        cx.notify();
                    }),
                ),
            )
            .child(
                ui_kit::icon_button("toggle-assistant", "◨", self.state.show_assistant).on_click(
                    cx.listener(|this, _, window, cx| {
                        this.state.toggle_assistant();
                        this.restore_focus(window);
                        cx.notify();
                    }),
                ),
            )
            .child(
                ui_kit::button(
                    "router-status",
                    if self.state.router_live {
                        "Live"
                    } else {
                        "Offline"
                    },
                )
                .text_size(px(theme::MICRO))
                .text_color(router_color)
                .child(widgets::dot(router_color))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.state.show_inspector = !this.state.show_inspector;
                    cx.notify();
                })),
            )
            .when(self.state.error.is_some(), |this| {
                this.child(div().text_color(theme::warning()).child("!"))
            })
            .child(
                div()
                    .id("window-min")
                    .window_control_area(WindowControlArea::Min)
                    .w(px(30.0))
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme::text_muted())
                    .hover(|style| style.bg(theme::elevated_alt()))
                    .child("−"),
            )
            .child(
                div()
                    .id("window-max")
                    .window_control_area(WindowControlArea::Max)
                    .w(px(30.0))
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme::text_muted())
                    .hover(|style| style.bg(theme::elevated_alt()))
                    .child("□"),
            )
            .child(
                div()
                    .id("window-close")
                    .window_control_area(WindowControlArea::Close)
                    .w(px(30.0))
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme::text_muted())
                    .hover(|style| {
                        style
                            .bg(theme::wash(theme::danger(), 0.18))
                            .text_color(theme::danger())
                    })
                    .child("×"),
            );
        div()
            .h(px(36.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(12.0))
            .bg(theme::sunken())
            .border_b_1()
            .border_color(theme::border())
            .child(left)
            .child(search)
            .child(right)
    }

    fn render_status_bar(&self, accent: gpui::Rgba, cx: &mut Context<Self>) -> gpui::Div {
        div()
            .h(px(26.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(14.0))
            .px(px(12.0))
            .bg(theme::sunken())
            .border_t_1()
            .border_color(theme::border())
            .text_size(px(theme::MICRO))
            .text_color(theme::text_muted())
            .child(ui_kit::TintedBadge::render(
                self.state.mode.short_label(),
                accent,
            ))
            .child(format!("⑂ {}", self.state.git_branch))
            .child(format!("{} problems", self.state.diagnostics.len()))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(self.state.status_message.clone()),
            )
            .child(format!("Ln {}", self.state.cursor_line))
            .child(self.state.language_label())
            .child(
                ui_kit::button(
                    "status-inspector",
                    format!("{}% context  ⓘ", self.state.context_pct),
                )
                .text_size(px(theme::MICRO))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.state.show_inspector = !this.state.show_inspector;
                    cx.notify();
                })),
            )
    }

    fn render_inspector(&self, cx: &mut Context<Self>) -> gpui::Div {
        let state = &self.state;
        let mut panel = div()
            .absolute()
            .right(px(12.0))
            .bottom(px(36.0))
            .w(px(380.0))
            .p(px(16.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .rounded(px(10.0))
            .bg(theme::elevated())
            .border_1()
            .border_color(theme::border_strong())
            .text_size(px(theme::BODY))
            .text_color(theme::text_secondary())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(widgets::section_label("Workspace details"))
                    .child(ui_kit::icon_button("inspector-close", "×", false).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.state.show_inspector = false;
                            cx.notify();
                        }),
                    )),
            );
        for (label, value) in [
            (
                "Router",
                format!(
                    "{} · {}",
                    state.router_endpoint,
                    if state.router_live { "live" } else { "offline" }
                ),
            ),
            ("Workspace lock", state.lock_label().to_string()),
            ("Ticket", state.ticket_label()),
            (
                "Index",
                format!(
                    "{} files · {} symbols · {} blocks",
                    state.file_count(),
                    state.symbol_count(),
                    state.block_count()
                ),
            ),
            (
                "Session",
                format!(
                    "{} nodes · {}ms · ${:.4}",
                    state.session.len(),
                    state.latency_ms,
                    state.spend_micros as f64 / 1_000_000.0
                ),
            ),
            ("Context", state.context_label()),
            (
                "Active file",
                state
                    .active_buffer()
                    .map(|buffer| format!("{} · {} bytes", buffer.rel_path, buffer.content.len()))
                    .unwrap_or_else(|| "No file selected".into()),
            ),
            (
                "Experience",
                format!("{} durable lessons", state.experience.entries().len()),
            ),
        ] {
            panel = panel.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(3.0))
                    .child(widgets::section_label(label))
                    .child(value),
            );
        }
        if let Some(error) = &state.error {
            panel = panel.child(div().text_color(theme::warning()).child(error.clone()));
        }
        panel
    }
}

impl Render for Workbench {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let accent = theme::accent_for(self.state.mode);
        let accent_strong = theme::accent_strong_for(self.state.mode);
        let composer_focused = self.composer_focus.is_focused(window);

        let titlebar = self.render_titlebar(accent, accent_strong, cx);

        let left = self
            .state
            .show_left
            .then(|| crate::left_dock::render(self, accent, cx));
        let center = div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .child(crate::editor_pane::render(self, accent, accent_strong, cx))
            .when(self.state.show_bottom, |this| {
                this.child(crate::bottom_drawer::render(self, accent, cx))
            });
        let assistant = self.state.show_assistant.then(|| {
            crate::assistant_panel::render(self, accent, accent_strong, composer_focused, cx)
        });
        let body = div()
            .flex_1()
            .min_h_0()
            .flex()
            .when_some(left, |this, left| this.child(left))
            .child(center)
            .when_some(assistant, |this, assistant| this.child(assistant));
        let status = self.render_status_bar(accent, cx);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::sunken())
            .text_color(theme::text_primary())
            .track_focus(&self.root_focus)
            .font_family(theme::ui_font())
            .text_size(px(theme::BODY))
            .relative()
            .key_context("Workbench")
            .on_key_down(cx.listener(Self::on_root_key))
            .child(titlebar)
            .child(body)
            .child(status)
            .when(self.state.show_inspector, |this| {
                this.child(self.render_inspector(cx))
            })
            .when(self.state.show_palette, |this| {
                this.child(crate::command_palette::render(
                    self,
                    accent,
                    accent_strong,
                    cx,
                ))
            })
    }
}

// --------------------------------------------------------------------- helpers

fn estimate_tokens(text: &str) -> usize {
    (text.chars().count() / 4).max(1)
}

fn compact_number(value: usize) -> String {
    if value >= 1_048_576 {
        format!("{:.1}m", value as f64 / 1_048_576.0)
    } else if value >= 1_000 {
        format!("{:.1}k", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

fn scan_project_tree(root: &Path) -> Vec<FileNode> {
    let mut budget = 3_000usize;
    scan_dir(root, 0, &mut budget)
}

fn scan_dir(dir: &Path, depth: usize, budget: &mut usize) -> Vec<FileNode> {
    if depth > 12 || *budget == 0 {
        return Vec::new();
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut nodes = Vec::new();
    for entry in entries.flatten() {
        if *budget == 0 {
            break;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if file_type.is_dir() {
            if matches!(
                name.as_str(),
                ".git"
                    | "target"
                    | "node_modules"
                    | "dist"
                    | "build"
                    | ".venv"
                    | "venv"
                    | "__pycache__"
                    | ".cargo"
                    | "portable_data"
                    | "references"
            ) {
                continue;
            }
            *budget = budget.saturating_sub(1);
            nodes.push(FileNode {
                path: path.clone(),
                name,
                is_dir: true,
                children: scan_dir(&path, depth + 1, budget),
            });
        } else if file_type.is_file() {
            *budget = budget.saturating_sub(1);
            nodes.push(FileNode {
                path,
                name,
                is_dir: false,
                children: Vec::new(),
            });
        }
    }
    nodes.sort_by(|left, right| {
        right
            .is_dir
            .cmp(&left.is_dir)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    nodes
}

fn collect_visible_rows(
    nodes: &[FileNode],
    depth: usize,
    expanded: &HashSet<PathBuf>,
    rows: &mut Vec<FileRow>,
) {
    for node in nodes {
        let is_expanded = node.is_dir && expanded.contains(&node.path);
        rows.push(FileRow {
            path: node.path.clone(),
            name: node.name.clone(),
            depth,
            is_dir: node.is_dir,
            expanded: is_expanded,
        });
        if is_expanded {
            collect_visible_rows(&node.children, depth + 1, expanded, rows);
        }
    }
}

fn first_file(nodes: &[FileNode]) -> Option<PathBuf> {
    for node in nodes {
        if node.is_dir {
            if let Some(path) = first_file(&node.children) {
                return Some(path);
            }
        } else {
            return Some(node.path.clone());
        }
    }
    None
}

fn load_session(root: &Path) -> Session {
    let path = root.join(".ai").join("session.jsonl");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| Session::from_jsonl(&raw).ok())
        .unwrap_or_default()
}

fn read_git_branch(root: &Path) -> String {
    std::fs::read_to_string(root.join(".git").join("HEAD"))
        .ok()
        .and_then(|head| {
            let head = head.trim();
            head.strip_prefix("ref: refs/heads/")
                .map(str::to_string)
                .or_else(|| Some(head.to_string()))
        })
        .unwrap_or_else(|| "main".to_string())
}

fn read_verification_file(root: &Path) -> String {
    std::fs::read_to_string(root.join(".ai").join("verification.md")).unwrap_or_default()
}

fn model_choices(root: &Path, current: &str) -> Vec<String> {
    let mut models = vec![
        "auto-combo".to_string(),
        "deepseek-v4.1-flash".to_string(),
        "claude-sonnet".to_string(),
        "gemini-flash".to_string(),
        "qwen-coder".to_string(),
    ];
    for file in [".ai/model_combo.txt", ".ai/model_combo.example.txt"] {
        if let Ok(raw) = std::fs::read_to_string(root.join(file)) {
            for line in raw.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') || line.contains('=') {
                    continue;
                }
                if line.len() <= 80 && !models.iter().any(|model| model == line) {
                    models.push(line.to_string());
                }
            }
        }
    }
    if !current.trim().is_empty() && !models.iter().any(|model| model == current) {
        models.push(current.to_string());
    }
    models
}

fn git_diff(root: &Path, rel_path: &str, commit: bool) -> Vec<DiffLineView> {
    let mut command = Command::new("git");
    command.current_dir(root);
    if commit {
        command.args(["show", "--format=", "--unified=3", "HEAD", "--", rel_path]);
    } else {
        command.args(["diff", "--unified=3", "--", rel_path]);
    }
    let Ok(output) = command.output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    parse_diff(&String::from_utf8_lossy(&output.stdout))
}

fn parse_diff(text: &str) -> Vec<DiffLineView> {
    let mut lines = Vec::new();
    for line in text.lines() {
        if line.starts_with("diff --git")
            || line.starts_with("index ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("@@")
            || line.starts_with("\\ No newline")
            || line.starts_with("commit ")
            || line.starts_with("Author:")
            || line.starts_with("Date:")
        {
            continue;
        }
        let parsed = if let Some(rest) = line.strip_prefix('+') {
            Some(DiffLineView {
                marker: '+',
                text: rest.to_string(),
            })
        } else if let Some(rest) = line.strip_prefix('-') {
            Some(DiffLineView {
                marker: '-',
                text: rest.to_string(),
            })
        } else {
            line.strip_prefix(' ').map(|rest| DiffLineView {
                marker: ' ',
                text: rest.to_string(),
            })
        };
        if let Some(parsed) = parsed {
            lines.push(parsed);
            if lines.len() >= 260 {
                break;
            }
        }
    }
    lines
}

// ----------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::theme;

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_workspace(name: &str) -> PathBuf {
        let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "va-desktop-test-{}-{name}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(dir.join(".ai")).expect("create .ai");
        fs::create_dir_all(dir.join("src")).expect("create src");
        fs::write(dir.join("README.md"), "# Demo\n\nRead me.\n").expect("write README");
        fs::write(
            dir.join("src/lib.rs"),
            "pub fn alpha() {\n    let x = 1;\n}\n\npub fn beta() {\n    let y = 2;\n}\n",
        )
        .expect("write source");
        dir
    }

    fn state_for(dir: &Path) -> WorkbenchState {
        WorkbenchState::new(
            Workspace::load(dir),
            None,
            ExecutionMode::Code,
            LanguageLens::Auto,
            "auto-combo".to_string(),
        )
    }

    #[test]
    fn opens_files_and_computes_live_block_hashes() {
        let dir = temp_workspace("hashes");
        let mut state = state_for(&dir);

        state
            .open_file(Path::new("src/lib.rs"))
            .expect("open source file");
        assert_eq!(state.active_rel_path().as_deref(), Some("src/lib.rs"));

        let blocks = state.active_buffer().expect("buffer").blocks.clone();
        assert!(!blocks.is_empty(), "segment_blocks must produce anchors");
        let hash = blocks[0].hash;
        assert_eq!(widgets::hash8(hash).len(), 8);
        assert_ne!(hash, 0);

        state.select_block(0);
        assert_eq!(state.current_block_hash(), Some(hash));
        assert!(state.active_symbol_name().is_some());
        assert_eq!(
            state.breadcrumb().2,
            Some(widgets::hash8(hash)),
            "breadcrumb carries the selected block hash"
        );

        drop(state);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cycles_modes_and_enforces_chat_read_only() {
        let dir = temp_workspace("modes");
        let mut state = state_for(&dir);
        assert_eq!(state.mode, ExecutionMode::Code);

        state.cycle_mode();
        assert_eq!(state.mode, ExecutionMode::Web);
        assert_eq!(theme::accent_hex(state.mode), "#06B6D4");

        state.cycle_mode();
        assert_eq!(state.mode, ExecutionMode::Chat);
        assert_eq!(theme::accent_hex(state.mode), "#6366F1");
        assert!(!state.mode.allows_writes());

        state.prompt_input = "Review the block".to_string();
        assert_eq!(state.submit_prompt(), SubmitOutcome::ReadOnly);
        assert!(matches!(
            state.thread.last(),
            Some(ThreadItem::Assistant {
                read_only: true,
                ..
            })
        ));

        state.cycle_mode();
        assert_eq!(state.mode, ExecutionMode::Debug);
        assert_eq!(theme::accent_hex(state.mode), "#F43F5E");
        state.cycle_mode();
        assert_eq!(state.mode, ExecutionMode::Code);
        assert_eq!(theme::accent_hex(state.mode), "#F59E0B");

        drop(state);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cycles_language_and_model_lenses() {
        let dir = temp_workspace("lenses");
        let mut state = state_for(&dir);

        state.cycle_language();
        assert_eq!(state.language, LanguageLens::Rust);
        assert_eq!(state.language_label(), "Rust");

        state.cycle_language();
        assert_eq!(state.language, LanguageLens::C);
        state.set_language(LanguageLens::Auto);
        assert_eq!(state.language, LanguageLens::Auto);
        assert_eq!(state.language_label(), "Rust (Auto)");

        let first_model = state.model.clone();
        state.cycle_model();
        assert_ne!(state.model, first_model);
        assert!(state.context_window > 0);

        state.set_model("claude-sonnet".to_string());
        assert_eq!(state.model_label(), "claude-sonnet");
        assert!(state.models.iter().any(|model| model == "claude-sonnet"));

        drop(state);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn toggles_docks_and_executes_palette_commands() {
        let dir = temp_workspace("docks");
        let mut state = state_for(&dir);

        assert!(state.show_left);
        state.toggle_left();
        assert!(!state.show_left);
        state.toggle_bottom();
        assert!(!state.show_bottom);
        state.toggle_assistant();
        assert!(!state.show_assistant);

        state.show_palette = true;
        state.execute_palette(PaletteCommand::ToggleLeft);
        assert!(state.show_left);
        assert!(!state.show_palette);

        state.execute_palette(PaletteCommand::Mode(ExecutionMode::Debug));
        assert_eq!(state.mode, ExecutionMode::Debug);
        state.execute_palette(PaletteCommand::Language(LanguageLens::Python));
        assert_eq!(state.language, LanguageLens::Python);

        state.execute_palette(PaletteCommand::ToggleBottom);
        assert!(state.show_bottom);
        state.set_bottom_tab(BottomTab::Harness);
        assert_eq!(state.bottom_tab, BottomTab::Harness);

        drop(state);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn composer_buffer_accepts_text_backspace_and_submit() {
        let dir = temp_workspace("composer");
        let mut state = state_for(&dir);

        state.push_str("hello");
        state.backspace();
        state.push_char('!');
        assert_eq!(state.prompt_input, "hell!");

        assert_eq!(state.submit_prompt(), SubmitOutcome::Queued);
        assert!(state.prompt_input.is_empty());
        assert!(matches!(
            state.thread.last(),
            Some(ThreadItem::Assistant {
                read_only: false,
                ..
            })
        ));
        assert!(state
            .thread
            .iter()
            .any(|item| matches!(item, ThreadItem::User { text } if text == "hell!")));
        assert_eq!(state.submit_prompt(), SubmitOutcome::Empty);

        drop(state);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn context_compaction_and_harness_reset_are_safe() {
        let dir = temp_workspace("context");
        let mut state = state_for(&dir);

        state.compact_context();
        assert!(state.context_pct <= 100);
        state.reset_harness();
        assert!(state.harness.is_pristine());
        state.run_tournament();
        assert_eq!(state.tournament_runs, 1);

        drop(state);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn context_chips_remove_independently_and_pin_survives_tab_switch() {
        let dir = temp_workspace("context-chips");
        let mut state = state_for(&dir);
        state.open_file(Path::new("src/lib.rs")).unwrap();
        state.select_block(0);
        let pinned = state.pinned_block.clone().unwrap();
        state.open_file(Path::new("README.md")).unwrap();
        assert_eq!(state.pinned_block.as_ref(), Some(&pinned));
        assert!(state.context_reference().contains("src/lib.rs"));
        state.context_file = false;
        assert!(!state.context_reference().contains("README.md"));
        state.pinned_block = None;
        assert_eq!(state.context_reference(), "workspace");
        state.context_file = true;
        assert_eq!(state.context_reference(), "README.md");
        drop(state);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn palette_finds_closed_files_symbols_and_scoped_models() {
        let dir = temp_workspace("palette-search");
        let mut state = state_for(&dir);
        state.palette_query = "src lib.rs".into();
        let results = crate::command_palette::items(&state);
        let file = results
            .iter()
            .find(|item| matches!(item.action, PaletteAction::File(_)))
            .unwrap();
        state.apply_palette_action(file.action.clone());
        assert_eq!(state.active_rel_path().as_deref(), Some("src/lib.rs"));
        state.palette_query = "beta".into();
        let results = crate::command_palette::items(&state);
        let symbol = results
            .iter()
            .find(|item| matches!(item.action, PaletteAction::Symbol(_, _)))
            .unwrap();
        state.apply_palette_action(symbol.action.clone());
        assert!(state.current_block_hash().is_some());
        state.palette_scope = PaletteScope::Model;
        state.palette_query = "claude".into();
        let results = crate::command_palette::items(&state);
        assert!(!results.is_empty());
        assert!(results
            .iter()
            .all(|item| matches!(item.action, PaletteAction::Model(_))));
        state.apply_palette_action(results[0].action.clone());
        assert!(state.model.contains("claude"));
        assert!(!state.show_palette);
        state.palette_query = "no-such-match-zzzz".into();
        assert!(crate::command_palette::items(&state).is_empty());
        drop(state);
        let _ = fs::remove_dir_all(dir);
    }
}
