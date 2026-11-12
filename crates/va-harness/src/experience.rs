//! Token-frugal harness self-improvement (Harness² mechanics).
//!
//! The eight modular surfaces live in [`crate::components`]; the contrastive
//! `Propose -> Probe -> Compose` loop lives in [`crate::recursive`]. This module
//! owns the durable, dependency-free pieces they share:
//!
//! 1. **Contrastive recovery.** When a step fails, compare the invariant
//!    candidate (what must always hold) with the domain candidate (what this
//!    language/project prefers). The winner is chosen from evidence, not vibes.
//! 2. **EXPERIENCE.md reflector.** The chosen lesson is persisted to
//!    `.ai/EXPERIENCE.md` and rendered as a precondition for future tasks in
//!    the same language/domain.
//! 3. **Zero extra rollouts on the fast path.** Nothing here calls a model:
//!    the planner only pays for a recovery probe when a failure actually
//!    happens (or when `--recursive-k` explicitly asks for one).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A failure observed by the engine, used as contrastive evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepFailure {
    pub language: String,
    pub domain: String,
    pub file: Option<String>,
    pub block_hash: Option<u64>,
    pub command: String,
    pub detail: String,
}

impl StepFailure {
    /// Stable identity for stagnation detection and de-duplication.
    pub fn signature(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.file.as_deref().unwrap_or("-"),
            self.block_hash
                .map(|h| format!("{h:016x}"))
                .unwrap_or_else(|| "-".to_string()),
            self.command,
            self.detail.lines().next().unwrap_or("").trim()
        )
    }
}

/// One candidate explanation for a failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryCandidate {
    /// `invariant` or `domain`.
    pub kind: String,
    /// Human-readable proposed fix/lesson.
    pub lesson: String,
    /// Evidence lines that support the candidate.
    pub evidence: Vec<String>,
    /// How many pieces of evidence support it (the score).
    pub score: i64,
}

impl RecoveryCandidate {
    pub fn invariant(lesson: impl Into<String>, evidence: Vec<String>) -> Self {
        let score = evidence.len() as i64;
        Self {
            kind: "invariant".to_string(),
            lesson: lesson.into(),
            evidence,
            score,
        }
    }

    pub fn domain(lesson: impl Into<String>, evidence: Vec<String>) -> Self {
        let score = evidence.len() as i64;
        Self {
            kind: "domain".to_string(),
            lesson: lesson.into(),
            evidence,
            score,
        }
    }

    pub fn is_invariant(&self) -> bool {
        self.kind == "invariant"
    }
}

/// Result of a contrastive comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryDecision {
    /// The winning candidate.
    pub chosen: RecoveryCandidate,
    /// Why it won.
    pub rationale: String,
}

/// pFailure-triggered contrastive recovery.
pub struct ContrastiveRecovery;

impl ContrastiveRecovery {
    /// Compare invariant vs domain candidates. Ties favour the invariant: a
    /// correctness rule always outranks a stylistic preference.
    pub fn choose(invariant: &RecoveryCandidate, domain: &RecoveryCandidate) -> RecoveryDecision {
        if invariant.score >= domain.score {
            RecoveryDecision {
                chosen: invariant.clone(),
                rationale: format!(
                    "invariant candidate won ({} vs {} evidence items; ties favour correctness)",
                    invariant.score, domain.score
                ),
            }
        } else {
            RecoveryDecision {
                chosen: domain.clone(),
                rationale: format!(
                    "domain candidate won ({} vs {} evidence items)",
                    domain.score, invariant.score
                ),
            }
        }
    }

    /// Build the durable lesson from a failure plus the winning candidate.
    pub fn reflect(failure: &StepFailure, decision: &RecoveryDecision) -> ExperienceEntry {
        ExperienceEntry {
            language: failure.language.clone(),
            domain: failure.domain.clone(),
            signature: failure.signature(),
            kind: decision.chosen.kind.clone(),
            lesson: decision.chosen.lesson.clone(),
            evidence: decision.chosen.evidence.clone(),
            rationale: decision.rationale.clone(),
            date: today(),
        }
    }
}

/// One durable lesson stored in `.ai/EXPERIENCE.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperienceEntry {
    pub date: String,
    pub language: String,
    pub domain: String,
    pub signature: String,
    pub kind: String,
    pub lesson: String,
    pub evidence: Vec<String>,
    pub rationale: String,
}

impl ExperienceEntry {
    pub fn render_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "### {} - {} / {}\n",
            self.date, self.language, self.domain
        ));
        out.push_str(&format!("- **Signature:** `{}`\n", self.signature));
        out.push_str(&format!("- **Candidate:** {}\n", self.kind));
        out.push_str(&format!("- **Lesson:** {}\n", self.lesson));
        for item in &self.evidence {
            out.push_str(&format!("- **Evidence:** {}\n", item.replace('\n', " ")));
        }
        out.push_str(&format!("- **Rationale:** {}\n\n", self.rationale));
        out
    }
}

const EXPERIENCE_HEADER: &str = "# EXPERIENCE\n\nDurable, evidence-backed recovery lessons recorded by the\ncontrastive reflector. Newest entries first. This file is an input to future\ntasks in the same language/domain - it never overrides the ticket contract.\n\n";

/// Persistent `.ai/EXPERIENCE.md` store.
#[derive(Debug, Clone)]
pub struct ExperienceStore {
    path: PathBuf,
    entries: Vec<ExperienceEntry>,
}

impl ExperienceStore {
    pub fn path_for(root: &Path) -> PathBuf {
        root.join(".ai").join("EXPERIENCE.md")
    }

    /// Load existing entries; a missing file is an empty store.
    pub fn load(root: &Path) -> Self {
        let path = Self::path_for(root);
        let entries = std::fs::read_to_string(&path)
            .map(|raw| Self::parse(&raw))
            .unwrap_or_default();
        Self { path, entries }
    }

    /// Parse lessons from the markdown log (tolerant of hand edits).
    pub fn parse(raw: &str) -> Vec<ExperienceEntry> {
        let mut entries = Vec::new();
        let mut current: Option<ExperienceEntry> = None;
        for line in raw.lines() {
            if let Some(rest) = line.strip_prefix("### ") {
                if let Some(entry) = current.take() {
                    entries.push(entry);
                }
                let (date, scope) = rest.split_once(" - ").unwrap_or((rest, ""));
                let (language, domain) = scope.split_once(" / ").unwrap_or((scope, ""));
                current = Some(ExperienceEntry {
                    date: date.trim().to_string(),
                    language: language.trim().to_string(),
                    domain: domain.trim().to_string(),
                    signature: String::new(),
                    kind: String::new(),
                    lesson: String::new(),
                    evidence: Vec::new(),
                    rationale: String::new(),
                });
                continue;
            }
            let Some(entry) = current.as_mut() else {
                continue;
            };
            let trimmed = line.trim();
            if let Some(v) = trimmed.strip_prefix("- **Signature:**") {
                entry.signature = v.trim().trim_matches('`').to_string();
            } else if let Some(v) = trimmed.strip_prefix("- **Candidate:**") {
                entry.kind = v.trim().to_string();
            } else if let Some(v) = trimmed.strip_prefix("- **Lesson:**") {
                entry.lesson = v.trim().to_string();
            } else if let Some(v) = trimmed.strip_prefix("- **Evidence:**") {
                entry.evidence.push(v.trim().to_string());
            } else if let Some(v) = trimmed.strip_prefix("- **Rationale:**") {
                entry.rationale = v.trim().to_string();
            }
        }
        if let Some(entry) = current {
            entries.push(entry);
        }
        entries
    }

    pub fn entries(&self) -> &[ExperienceEntry] {
        &self.entries
    }

    /// Append one lesson, newest first, creating the file when absent.
    pub fn record(&mut self, entry: ExperienceEntry) -> Result<(), String> {
        self.entries.insert(0, entry.clone());
        self.write()
    }

    fn write(&self) -> Result<(), String> {
        let mut body = String::from(EXPERIENCE_HEADER);
        for entry in &self.entries {
            body.push_str(&entry.render_markdown());
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&self.path, body).map_err(|e| e.to_string())
    }

    /// Lessons relevant to a language/domain pair, newest first, de-duplicated.
    pub fn lessons_for(&self, language: &str, domain: &str) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        self.entries
            .iter()
            .filter(|e| {
                (language.is_empty() || e.language.eq_ignore_ascii_case(language))
                    && (domain.is_empty() || e.domain.eq_ignore_ascii_case(domain))
            })
            .filter(|e| seen.insert(e.signature.clone()))
            .map(|e| e.lesson.clone())
            .collect()
    }

    /// Precondition block appended to the Boss/Code prompt.
    pub fn render_precondition(&self, language: &str, domain: &str) -> String {
        let lessons = self.lessons_for(language, domain);
        if lessons.is_empty() {
            return String::new();
        }
        let mut out = String::from("Durable lessons from earlier tasks (EXPERIENCE.md):\n");
        for (i, lesson) in lessons.iter().take(8).enumerate() {
            out.push_str(&format!("{}. {lesson}\n", i + 1));
        }
        out
    }
}

fn today() -> String {
    // Dependency-free local date: days since the Unix epoch formatted as
    // YYYY-MM-DD. No chrono/time crate is available offline.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Howard Hinnant's `civil_from_days` algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("va-experience-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn contrastive_recovery_prefers_the_better_evidence_and_ties_go_to_invariant() {
        let inv = RecoveryCandidate::invariant("never swallow errors", vec!["a".into()]);
        let dom = RecoveryCandidate::domain("use project style", vec![]);
        assert!(ContrastiveRecovery::choose(&inv, &dom)
            .chosen
            .is_invariant());

        let inv = RecoveryCandidate::invariant("invariant", vec![]);
        let dom = RecoveryCandidate::domain("domain", vec!["a".into(), "b".into()]);
        assert!(!ContrastiveRecovery::choose(&inv, &dom)
            .chosen
            .is_invariant());

        let tie_inv = RecoveryCandidate::invariant("inv", vec!["a".into()]);
        let tie_dom = RecoveryCandidate::domain("dom", vec!["a".into()]);
        assert!(ContrastiveRecovery::choose(&tie_inv, &tie_dom)
            .chosen
            .is_invariant());
    }

    #[test]
    fn experience_round_trips_through_markdown() {
        let root = temp_root("roundtrip");
        let mut store = ExperienceStore::load(&root);
        let failure = StepFailure {
            language: "rust".into(),
            domain: "workspace".into(),
            file: Some("src/main.rs".into()),
            block_hash: Some(42),
            command: "cargo check".into(),
            detail: "E0382 use of moved value".into(),
        };
        let inv = RecoveryCandidate::invariant(
            "clone before moving the value",
            vec!["E0382".into(), "moved at line 12".into()],
        );
        let dom = RecoveryCandidate::domain("style note", vec![]);
        let decision = ContrastiveRecovery::choose(&inv, &dom);
        store
            .record(ContrastiveRecovery::reflect(&failure, &decision))
            .unwrap();

        let reloaded = ExperienceStore::load(&root);
        assert_eq!(reloaded.entries().len(), 1);
        let lesson = reloaded.entries()[0].lesson.clone();
        assert!(lesson.contains("clone"));
        assert!(reloaded
            .render_precondition("rust", "workspace")
            .contains("clone"));
        assert!(reloaded
            .render_precondition("python", "workspace")
            .is_empty());

        let raw = std::fs::read_to_string(ExperienceStore::path_for(&root)).unwrap();
        assert!(raw.starts_with("# EXPERIENCE"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn lessons_are_deduplicated_by_signature() {
        let root = temp_root("dedupe");
        let mut store = ExperienceStore::load(&root);
        let entry = ExperienceEntry {
            date: "2026-10-23".into(),
            language: "go".into(),
            domain: "api".into(),
            signature: "same".into(),
            kind: "invariant".into(),
            lesson: "check the error".into(),
            evidence: vec![],
            rationale: "test".into(),
        };
        store.record(entry.clone()).unwrap();
        store.record(entry).unwrap();
        assert_eq!(store.lessons_for("go", "api").len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }
}
