//! Workspace state loading. Reads the `.ai/` files from disk and derives
//! everything the UI needs, so the render path never touches the filesystem.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::parser::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub root: PathBuf,
    pub exists: bool,
    pub active_task: ActiveTask,
    pub state_file: StateFile,
    pub bugs: Vec<LogEntry>,
    pub adrs: Vec<LogEntry>,
    pub progress_lines: Vec<String>,
    pub violation: Option<String>,
    pub missing: Vec<String>,
    pub router_assignment: crate::router::RouterAssignment,
}

impl Workspace {
    /// Walk up from `start` looking for a `.ai/` directory, mirroring the
    /// `va` CLI's discovery rule.
    pub fn discover(start: &Path) -> Option<PathBuf> {
        let mut cur = Some(start);
        while let Some(dir) = cur {
            if dir.join(".ai").is_dir() {
                return Some(dir.to_path_buf());
            }
            cur = dir.parent();
        }
        None
    }

    pub fn load(root: &Path) -> Workspace {
        let ai = root.join(".ai");
        let mut missing = Vec::new();

        let read = |name: &str, missing: &mut Vec<String>| -> String {
            let p = ai.join(name);
            match std::fs::read_to_string(&p) {
                Ok(s) => s,
                Err(_) => {
                    missing.push(name.to_string());
                    String::new()
                }
            }
        };

        let active_raw = read("active_task.md", &mut missing);
        let state_raw = read("current_state.md", &mut missing);
        let bugs_raw = read("known_bugs.md", &mut missing);
        let adr_raw = read("decisions.md", &mut missing);
        let progress_raw = read("progress.md", &mut missing);

        let active_task = parse_active_task(&active_raw);
        let violation = validate_contract(&active_task);

        // Progress is a git-log-shaped list; keep only the historical lines
        // and drop the section scaffolding.
        let progress_lines = progress_raw
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("- ") && !l.starts_with("- ("))
            .map(|l| l.trim_start_matches("- ").trim().to_string())
            .rev()
            .collect();

        let router_assignment = crate::router::resolve_assignment(root);

        Workspace {
            root: root.to_path_buf(),
            exists: true,
            active_task,
            state_file: parse_state_file(&state_raw),
            bugs: parse_log_entries(&bugs_raw),
            adrs: parse_log_entries(&adr_raw),
            progress_lines,
            violation,
            missing,
            router_assignment,
        }
    }

    /// The phrase a human pastes back to Boss to start work. Mirrors the
    /// `va` CLI's suggestion rule.
    pub fn suggested_phrase(&self) -> Option<&'static str> {
        let status = self
            .active_task
            .active
            .as_ref()
            .and_then(|t| t.status.as_deref());
        match status {
            Some("pending") => Some("Execute active task"),
            Some("in_progress") => Some("Continue relay"),
            _ => None,
        }
    }

    /// Real blockers only. The template writes `None.` with a trailing period,
    /// so compare against the trimmed, punctuation-stripped value.
    pub fn blockers(&self) -> Option<&str> {
        self.state_file.get("Blockers").filter(|b| {
            let v = b.trim().trim_end_matches(['.', '!']).trim();
            !v.is_empty() && !v.eq_ignore_ascii_case("none") && !v.eq_ignore_ascii_case("n/a")
        })
    }

    pub fn milestone(&self) -> Option<&str> {
        self.state_file.get("Active Milestone")
    }

    pub fn queue_done(&self) -> usize {
        self.active_task
            .queue
            .iter()
            .filter(|r| r.is_done())
            .count()
    }

    pub fn queue_total(&self) -> usize {
        self.active_task.queue.len()
    }

    pub fn progress_pct(&self) -> usize {
        let total = self.active_task.steps_total();
        if total == 0 {
            return 0;
        }
        (self.active_task.verified_steps().len() * 100) / total
    }
}
