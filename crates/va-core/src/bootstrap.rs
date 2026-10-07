//! Project initialization, template application, and pristine state reset.
//! Native Rust implementation of `reset-template-state.js` and `bin/cli.js`.

use std::fs;
use std::path::Path;

use crate::lock::acquire_task_lock;

pub const DEFAULT_ACTIVE_TASK: &str = r#"# Active Task Assignment Blueprint

## Wave
- **Name:** First Feature
- **Objective:** Define and deliver the first feature of this project. Replace this blueprint with your real task via 👑 Boss Mode.
- **Status:** awaiting_boss
- **User paste:** n/a — Boss has not scoped any tickets yet.

## Status Snapshot
- **current_ticket:** none
- **completed_tickets:** none
- **remaining_tickets:** none
- **resume_rule:** Open the explicit Active Ticket, read its `relay_notes`, and continue at the first step without successful verification evidence.

## Active Ticket
- **id:** none

## Queue
| order | id | title | status | effort | depends_on |
|------:|----|-------|--------|--------|------------|

(No tickets yet. 👑 Boss fills this table per `.ai/ticket_contract.md`.)
"#;

pub const DEFAULT_CURRENT_STATE: &str = r#"# Current Operational State
- **Active Milestone:** Project Initialization
- **Current Assignment:** none — awaiting first Boss blueprint.
- **Planning Status:** No tickets defined yet.
- **Implementation Status:** Not started.
- **Verification Status:** Nothing to verify yet.
- **Resume Anchor:** n/a — no active tickets.
- **Blockers:** None.
"#;

pub const DEFAULT_PROGRESS: &str = r#"# Project Progress

## Active Wave
- (none yet — 👑 Boss records the first wave here)

## Scope Guard
- (👑 Boss narrows this when the first wave is scoped)

## Historical Waves
- (empty)
"#;

pub const DEFAULT_SESSION_CONTEXT: &str = r#"# ⏳ Active Session Context Handoff
Last Session Status: Fresh template — no sessions yet.
Active Blockers: None
Next Immediate Action Step: Switch to 💬 Chat to shape your idea, or 👑 Boss to scope the first feature.
"#;

pub const DEFAULT_KNOWN_BUGS: &str = r#"# 🐞 Forensic Anomaly Log
No active anomalies detected. Code and Debug modes can append records directly here.
"#;

pub const DEFAULT_CHANGELOG: &str = r#"# Changelog (agent OS)
- Template initialized.
"#;

pub const DEFAULT_VERIFICATION: &str = r#"# Verification Record & Proven Baselines
Current Feature Proven: Base AI Operating System Initialized
Test Suites & Checks Executed:
[x] Template initialized with native Rust tooling.

Runtime Environment Specs:
Engine Host: Standalone Rust Engine
Persistence: None

Last Date Proven Working: never — nothing executed yet.
"#;

pub fn pristine_template_files() -> Vec<(&'static str, &'static str)> {
    vec![
        ("active_task.md", DEFAULT_ACTIVE_TASK),
        ("current_state.md", DEFAULT_CURRENT_STATE),
        ("progress.md", DEFAULT_PROGRESS),
        ("session_context.md", DEFAULT_SESSION_CONTEXT),
        ("known_bugs.md", DEFAULT_KNOWN_BUGS),
        ("changelog.md", DEFAULT_CHANGELOG),
        ("verification.md", DEFAULT_VERIFICATION),
    ]
}

/// Reset volatile .ai/ state to pristine template defaults while holding
/// the single-writer task lock to preserve ADR-007.
pub fn reset_template_state(project_dir: &Path) -> Result<(), String> {
    let ai_dir = project_dir.join(".ai");
    if !ai_dir.is_dir() {
        return Err(format!(
            "No .ai/ directory found in {}",
            project_dir.display()
        ));
    }

    let lock = acquire_task_lock(project_dir, "reset-template-state")?;
    for (filename, content) in pristine_template_files() {
        let p = ai_dir.join(filename);
        let _ = fs::write(&p, content);
    }
    lock.release();
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TemplateInfo {
    pub name: String,
    pub description: String,
    pub entry: String,
}

pub fn load_template_catalog(templates_dir: &Path) -> Vec<TemplateInfo> {
    let catalog_path = templates_dir.join("catalog.json");
    if let Ok(c) = fs::read_to_string(&catalog_path) {
        #[derive(serde::Deserialize)]
        struct Cat {
            templates: Vec<TemplateInfo>,
        }
        if let Ok(cat) = serde_json::from_str::<Cat>(&c) {
            return cat.templates;
        }
    }
    Vec::new()
}

pub fn is_safe_project_name(name: &str) -> bool {
    if name.is_empty() || name == "." || name == ".." {
        return false;
    }
    if name.contains('/') || name.contains('\\') {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

pub fn apply_project_template(
    template_name: &str,
    templates_dir: &Path,
    project_dir: &Path,
) -> Result<(), String> {
    let src = templates_dir.join(template_name);
    if !src.is_dir() {
        return Err(format!(
            "Template '{}' not found at {}",
            template_name,
            src.display()
        ));
    }

    let app_src = src.join("app");
    let app_dst = project_dir.join("app");
    if app_src.is_dir() {
        copy_dir_recursive(&app_src, &app_dst)?;
    }

    let seed_ai = src.join(".ai");
    let target_ai = project_dir.join(".ai");
    if seed_ai.is_dir() {
        let _ = fs::create_dir_all(&target_ai);
        if let Ok(entries) = fs::read_dir(&seed_ai) {
            for entry in entries.flatten() {
                let target = target_ai.join(entry.file_name());
                let _ = fs::copy(entry.path(), target);
            }
        }
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create dir {}: {}", dst.display(), e))?;

    let entries =
        fs::read_dir(src).map_err(|e| format!("Failed to read dir {}: {}", src.display(), e))?;

    for entry in entries.flatten() {
        let ft = entry.file_type().map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ft.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy file {}: {}", src_path.display(), e))?;
        }
    }
    Ok(())
}

pub fn init_metadata(project_name: &str, project_dir: &Path) {
    let config_path = project_dir.join(".ai").join("project_config.md");
    if let Ok(c) = fs::read_to_string(&config_path) {
        let updated = c.replace("CHANGE_ME_PROJECT_NAME", project_name);
        let _ = fs::write(&config_path, updated);
    }

    let overview_path = project_dir.join(".ai").join("project_overview.md");
    if let Ok(c) = fs::read_to_string(&overview_path) {
        let updated = c.replace("CHANGE_ME_PROJECT_NAME", project_name);
        let _ = fs::write(&overview_path, updated);
    }
}

pub fn init_workspace(
    project_dir: &Path,
    project_name: &str,
    template: Option<&str>,
    templates_dir: Option<&Path>,
) -> Result<(), String> {
    if !is_safe_project_name(project_name) {
        return Err(format!(
            "Invalid project name '{}'. Allowed: letters, numbers, dots, underscores, hyphens.",
            project_name
        ));
    }

    if project_dir.exists() {
        return Err(format!(
            "Directory '{}' already exists. Choose another name or remove it.",
            project_dir.display()
        ));
    }

    fs::create_dir_all(project_dir).map_err(|e| format!("Could not create directory: {}", e))?;

    let ai_dir = project_dir.join(".ai");
    fs::create_dir_all(&ai_dir).map_err(|e| format!("Could not create .ai directory: {}", e))?;

    // Write pristine default files
    for (filename, content) in pristine_template_files() {
        let p = ai_dir.join(filename);
        let _ = fs::write(p, content);
    }

    if let (Some(t_name), Some(t_dir)) = (template, templates_dir) {
        apply_project_template(t_name, t_dir, project_dir)?;
    }

    init_metadata(project_name, project_dir);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_safe_project_names() {
        assert!(is_safe_project_name("my-app"));
        assert!(is_safe_project_name("app_123.test"));
        assert!(!is_safe_project_name(""));
        assert!(!is_safe_project_name("."));
        assert!(!is_safe_project_name(".."));
        assert!(!is_safe_project_name("foo/bar"));
        assert!(!is_safe_project_name("foo\\bar"));
    }

    #[test]
    fn resets_template_state_with_task_lock() {
        let temp_dir = std::env::temp_dir().join("va-bootstrap-test-reset");
        let ai = temp_dir.join(".ai");
        let _ = fs::create_dir_all(&ai);
        let _ = fs::write(ai.join("active_task.md"), "mutated content");

        reset_template_state(&temp_dir).expect("must reset successfully");
        let content = fs::read_to_string(ai.join("active_task.md")).unwrap();
        assert!(content.contains("First Feature"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
