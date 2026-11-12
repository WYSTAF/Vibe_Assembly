//! The eight modular harness surfaces (`c1`..`c8`).
//!
//! Every task starts from the pristine, immutable baseline loaded from
//! `va-core::prompts`. Per-task overlays live in
//! `.ai/harness_overlays/<task_id>/` (or in RAM when the runtime is in portable
//! mode) and are never allowed to leak into the next task: `ComponentTree::begin`
//! and `reset` enforce strict reset-to-base, pre-conditioned only by verified
//! lessons from `.ai/EXPERIENCE.md`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::experience::ExperienceStore;

/// Stable ids of the eight harness surfaces, in render order.
pub const SURFACE_IDS: [&str; 8] = [
    "c1_system_prompt",
    "c2_guardrails",
    "c3_memory",
    "c4_tool_guidance",
    "c5_subagents",
    "c6_skills",
    "c7_scripts",
    "c8_plugins",
];

/// One modular harness surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessSurface {
    pub id: String,
    pub content: String,
}

/// The eight surfaces that make up the harness overlay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessLayers {
    surfaces: BTreeMap<String, String>,
}

impl Default for HarnessLayers {
    fn default() -> Self {
        Self::base()
    }
}

impl HarnessLayers {
    /// Pristine baseline, loaded from the immutable `va-core::prompts` contract.
    /// Every new task starts here; no overlay survives a reset.
    pub fn base() -> Self {
        let mut surfaces = BTreeMap::new();
        surfaces.insert(
            "c1_system_prompt".into(),
            va_core::prompts::MASTER_CONSTITUTION.to_string(),
        );
        surfaces.insert(
            "c2_guardrails".into(),
            format!(
                "{}\nNever write outside files_allowed. Never self-approve a failing \
                 verification. Stop after two identical failures and escalate.",
                va_core::prompts::CODE_INSTRUCTIONS
            ),
        );
        surfaces.insert(
            "c3_memory".into(),
            "Use the immutable trajectory DAG for history. Use .ai/EXPERIENCE.md for durable \
             lessons. Never re-read a file whose block hash you already hold."
                .into(),
        );
        surfaces.insert(
            "c4_tool_guidance".into(),
            format!(
                "{}\nAddress edits by block hash, never by line number. Verify with the \
                 language lens command before claiming success.",
                va_core::prompts::CODE_ROLE_DEFINITION
            ),
        );
        surfaces.insert(
            "c5_subagents".into(),
            format!(
                "{}\nA single writer owns the workspace. Observers (chat/audit) are read-only \
                 and may not patch.",
                va_core::prompts::CHAT_ROLE_DEFINITION
            ),
        );
        surfaces.insert(
            "c6_skills".into(),
            "Keep instructions short and composable; load only the skill needed for the current \
             ticket."
                .into(),
        );
        surfaces.insert(
            "c7_scripts".into(),
            format!(
                "{}\nPrefer deterministic, re-runnable scripts over ad-hoc shell sequences. \
                 Capture exit codes as evidence.",
                va_core::prompts::DEBUG_INSTRUCTIONS
            ),
        );
        surfaces.insert(
            "c8_plugins".into(),
            "Plugins must be optional. A missing plugin degrades gracefully and never blocks the \
             core loop."
                .into(),
        );
        Self { surfaces }
    }

    /// Strict reset-to-base. Called at the start of every task.
    pub fn reset_to_base(&mut self) {
        *self = Self::base();
    }

    /// Apply an overlay to one surface (used only within the current task).
    pub fn set(&mut self, id: &str, content: impl Into<String>) -> Result<(), String> {
        if !SURFACE_IDS.contains(&id) {
            return Err(format!("unknown harness surface `{id}`"));
        }
        self.surfaces.insert(id.to_string(), content.into());
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&str> {
        self.surfaces.get(id).map(String::as_str)
    }

    pub fn surfaces(&self) -> Vec<HarnessSurface> {
        self.surfaces
            .iter()
            .map(|(id, content)| HarnessSurface {
                id: id.clone(),
                content: content.clone(),
            })
            .collect()
    }

    /// True when every surface still equals the pristine base.
    pub fn is_pristine(&self) -> bool {
        *self == Self::base()
    }

    /// Render the overlay for a system prompt, in c1..c8 order.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for id in SURFACE_IDS {
            if let Some(content) = self.surfaces.get(id) {
                out.push_str(&format!("[{id}]\n{content}\n\n"));
            }
        }
        out
    }

    /// Directory holding one task's ephemeral overlays.
    pub fn overlay_dir(root: &Path, task_id: &str) -> PathBuf {
        root.join(".ai").join("harness_overlays").join(task_id)
    }

    /// Persist one surface overlay for a task. Creates the task directory.
    pub fn save_overlay(root: &Path, task_id: &str, id: &str, content: &str) -> Result<(), String> {
        if !SURFACE_IDS.contains(&id) {
            return Err(format!("unknown harness surface `{id}`"));
        }
        let dir = Self::overlay_dir(root, task_id);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(format!("{id}.md")), content).map_err(|e| e.to_string())
    }

    /// Load base plus any overlays stored for this task. A missing directory
    /// yields the pristine base.
    pub fn load_with_overlays(root: &Path, task_id: &str) -> Result<Self, String> {
        let mut layers = Self::base();
        let dir = Self::overlay_dir(root, task_id);
        if !dir.exists() {
            return Ok(layers);
        }
        for id in SURFACE_IDS {
            let path = dir.join(format!("{id}.md"));
            if let Ok(content) = std::fs::read_to_string(&path) {
                layers.set(id, content)?;
            }
        }
        Ok(layers)
    }
}

/// A per-task component tree with strict reset-to-base isolation.
///
/// Overlays are scoped to one task id. In portable mode they stay in RAM and
/// are never written to flash; otherwise they persist under
/// `.ai/harness_overlays/<task_id>/`.
pub struct ComponentTree {
    root: PathBuf,
    task_id: String,
    portable: bool,
    layers: HarnessLayers,
    precondition: String,
}

impl ComponentTree {
    /// Begin a task: reset to base, pre-condition c3 with verified lessons, then
    /// load only this task's overlays.
    pub fn begin(
        root: &Path,
        task_id: &str,
        portable: bool,
        experience: &ExperienceStore,
        language: &str,
        domain: &str,
    ) -> Self {
        let precondition = experience.render_precondition(language, domain);
        let mut layers = if portable {
            HarnessLayers::base()
        } else {
            HarnessLayers::load_with_overlays(root, task_id).unwrap_or_default()
        };
        if !precondition.is_empty() {
            if let Some(base) = layers.get("c3_memory").map(str::to_string) {
                let _ = layers.set("c3_memory", format!("{base}\n{precondition}"));
            }
        }
        Self {
            root: root.to_path_buf(),
            task_id: task_id.to_string(),
            portable,
            layers,
            precondition,
        }
    }

    pub fn layers(&self) -> &HarnessLayers {
        &self.layers
    }

    pub fn layers_mut(&mut self) -> &mut HarnessLayers {
        &mut self.layers
    }

    pub fn precondition(&self) -> &str {
        &self.precondition
    }

    /// Apply an overlay within this task only.
    pub fn set_overlay(&mut self, id: &str, content: &str) -> Result<(), String> {
        self.layers.set(id, content)?;
        if !self.portable {
            HarnessLayers::save_overlay(&self.root, &self.task_id, id, content)?;
        }
        Ok(())
    }

    /// Strict reset-to-base, re-applying only the verified precondition.
    pub fn reset(&mut self) {
        self.layers.reset_to_base();
        if !self.precondition.is_empty() {
            if let Some(base) = self.layers.get("c3_memory").map(str::to_string) {
                let _ = self
                    .layers
                    .set("c3_memory", format!("{base}\n{}", self.precondition));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("va-components-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn eight_surfaces_are_all_present_and_non_empty() {
        let layers = HarnessLayers::base();
        assert_eq!(layers.surfaces().len(), 8);
        for id in SURFACE_IDS {
            assert!(!layers.get(id).unwrap_or("").is_empty(), "{id} is empty");
        }
    }

    #[test]
    fn base_is_sourced_from_the_prompts_contract() {
        let layers = HarnessLayers::base();
        assert!(layers
            .get("c1_system_prompt")
            .unwrap()
            .contains("Master Workspace Constitution"));
    }

    #[test]
    fn strict_reset_to_base_discards_overlays() {
        let mut layers = HarnessLayers::base();
        layers.set("c1_system_prompt", "poisoned overlay").unwrap();
        assert!(!layers.is_pristine());
        layers.reset_to_base();
        assert!(layers.is_pristine());
        assert_ne!(layers.get("c1_system_prompt").unwrap(), "poisoned overlay");
    }

    #[test]
    fn unknown_surfaces_are_rejected() {
        let mut layers = HarnessLayers::base();
        assert!(layers.set("c9_unknown", "x").is_err());
    }

    #[test]
    fn overlays_round_trip_and_are_task_scoped() {
        let root = temp_root("overlays");
        HarnessLayers::save_overlay(&root, "task-a", "c4_tool_guidance", "overlay a").unwrap();
        let a = HarnessLayers::load_with_overlays(&root, "task-a").unwrap();
        assert_eq!(a.get("c4_tool_guidance").unwrap(), "overlay a");
        // A different task never sees task-a's overlay.
        let b = HarnessLayers::load_with_overlays(&root, "task-b").unwrap();
        assert!(b.is_pristine());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn component_tree_reset_isolates_tasks_and_keeps_precondition() {
        let root = temp_root("tree");
        let mut experience = ExperienceStore::load(&root);
        experience
            .record(crate::experience::ExperienceEntry {
                date: "2026-11-08".into(),
                language: "rust".into(),
                domain: "workspace".into(),
                signature: "sig".into(),
                kind: "invariant".into(),
                lesson: "check the error".into(),
                evidence: vec![],
                rationale: "test".into(),
            })
            .unwrap();

        let mut tree = ComponentTree::begin(&root, "t1", false, &experience, "rust", "workspace");
        assert!(tree
            .layers()
            .get("c3_memory")
            .unwrap()
            .contains("check the error"));
        tree.set_overlay("c2_guardrails", "task overlay").unwrap();
        assert_eq!(tree.layers().get("c2_guardrails").unwrap(), "task overlay");

        // A new task starts clean but keeps the verified lesson.
        let next = ComponentTree::begin(&root, "t2", false, &experience, "rust", "workspace");
        assert_ne!(next.layers().get("c2_guardrails").unwrap(), "task overlay");
        assert!(next
            .layers()
            .get("c3_memory")
            .unwrap()
            .contains("check the error"));

        // Reset within a task discards the overlay but keeps the precondition.
        tree.reset();
        assert_ne!(tree.layers().get("c2_guardrails").unwrap(), "task overlay");
        assert!(tree
            .layers()
            .get("c3_memory")
            .unwrap()
            .contains("check the error"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn portable_mode_keeps_overlays_in_ram() {
        let root = temp_root("portable");
        let experience = ExperienceStore::load(&root);
        let mut tree = ComponentTree::begin(&root, "t1", true, &experience, "rust", "workspace");
        tree.set_overlay("c6_skills", "ram only").unwrap();
        assert!(!HarnessLayers::overlay_dir(&root, "t1").exists());
        assert_eq!(tree.layers().get("c6_skills").unwrap(), "ram only");
        let _ = std::fs::remove_dir_all(&root);
    }
}
