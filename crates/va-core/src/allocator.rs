//! Model-combo allocator — assigns user-available models to the five modes
//! using the curated knowledge base (.ai/model_knowledge.json).
//!
//! Pure native Rust implementation mirroring `bin/allocate-models.js`.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelKnowledgeEntry {
    pub name: String,
    pub aliases: Vec<String>,
    pub class: String,
    pub context_tokens: Option<usize>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeRequirement {
    pub primary_class: String,
    pub fallback_classes: Vec<String>,
    #[serde(default)]
    pub why: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelKnowledge {
    pub version: Option<usize>,
    pub classes: BTreeMap<String, String>,
    pub mode_requirements: BTreeMap<String, ModeRequirement>,
    pub models: Vec<ModelKnowledgeEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassifiedModel {
    pub raw: String,
    pub name: String,
    pub class: String,
    pub context_tokens: Option<usize>,
    pub matched: bool,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeAssignment {
    pub model: String,
    pub class: String,
    pub matched: bool,
    #[serde(default)]
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AllocationResult {
    pub assignments: BTreeMap<String, ModeAssignment>,
    pub unassigned: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn load_knowledge(root: &Path) -> Result<ModelKnowledge, String> {
    let p = if root.ends_with("model_knowledge.json") {
        root.to_path_buf()
    } else {
        root.join(".ai").join("model_knowledge.json")
    };

    let content =
        fs::read_to_string(&p).map_err(|e| format!("Could not read {}: {}", p.display(), e))?;
    serde_json::from_str(&content).map_err(|e| format!("Could not parse {}: {}", p.display(), e))
}

pub fn normalize_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '+' || *c == '-')
        .collect()
}

/// Match a user model string to a knowledge-base entry via aliases/substring.
pub fn classify_model(model_name: &str, knowledge: &ModelKnowledge) -> ClassifiedModel {
    let norm = normalize_name(model_name);
    let mut best: Option<(&ModelKnowledgeEntry, usize)> = None;

    for model in &knowledge.models {
        let mut keys: Vec<String> = Vec::with_capacity(model.aliases.len() + 1);
        keys.push(model.name.to_lowercase());
        for a in &model.aliases {
            keys.push(normalize_name(a));
        }

        let mut match_len = 0;
        for k in keys {
            let matched = norm.contains(&k) || (norm.len() >= 4 && k.contains(&norm));
            if matched && k.len() > match_len {
                match_len = k.len();
            }
        }

        if match_len > 0 {
            match best {
                None => best = Some((model, match_len)),
                Some((_, prev_len)) if match_len > prev_len => {
                    best = Some((model, match_len));
                }
                _ => {}
            }
        }
    }

    if let Some((m, _)) = best {
        return ClassifiedModel {
            raw: model_name.to_string(),
            name: m.name.clone(),
            class: m.class.clone(),
            context_tokens: m.context_tokens,
            matched: true,
            notes: m.notes.clone(),
        };
    }

    // Heuristics sweep for unknown names
    let reason_re = Regex::new(r"r\d|reason|think").unwrap();
    let coder_re = Regex::new(r"cod(e|er)|dev").unwrap();
    let fast_re = Regex::new(r"flash|nano|mini|lite|turbo|haiku").unwrap();
    let flagship_re = Regex::new(r"pro|ultra|opus|max|flagship").unwrap();

    let class = if reason_re.is_match(&norm) {
        "reasoner"
    } else if coder_re.is_match(&norm) {
        "coder"
    } else if fast_re.is_match(&norm) {
        "fast"
    } else if flagship_re.is_match(&norm) {
        "flagship"
    } else {
        "workhorse"
    };

    ClassifiedModel {
        raw: model_name.to_string(),
        name: model_name.to_string(),
        class: class.to_string(),
        context_tokens: None,
        matched: false,
        notes: None,
    }
}

/// Allocate user combo models to modes honoring primary requirements,
/// user locks, and fallback chains.
pub fn allocate(
    combo: &[String],
    knowledge: &ModelKnowledge,
    locked: &BTreeMap<String, String>,
) -> AllocationResult {
    let mut warnings = Vec::new();
    let classified: Vec<ClassifiedModel> =
        combo.iter().map(|m| classify_model(m, knowledge)).collect();

    let mut by_class: BTreeMap<String, Vec<ClassifiedModel>> = BTreeMap::new();
    for c in &classified {
        by_class.entry(c.class.clone()).or_default().push(c.clone());
    }

    // Rank within class: matched entries first, then longer names
    for pool in by_class.values_mut() {
        pool.sort_by(|a, b| {
            b.matched
                .cmp(&a.matched)
                .then_with(|| b.raw.len().cmp(&a.raw.len()))
        });
    }

    let mut assignments: BTreeMap<String, ModeAssignment> = BTreeMap::new();
    let mut used: HashSet<String> = HashSet::new();

    // Explicit user locks win first
    for mode in knowledge.mode_requirements.keys() {
        if let Some(lock_target) = locked.get(mode) {
            if let Some(found) = classified.iter().find(|c| &c.raw == lock_target) {
                assignments.insert(
                    mode.clone(),
                    ModeAssignment {
                        model: found.raw.clone(),
                        class: found.class.clone(),
                        matched: found.matched,
                        locked: true,
                    },
                );
                used.insert(found.raw.clone());
            } else {
                warnings.push(format!(
                    "lock for {} references \"{}\", which is not in the combo — allocating automatically instead.",
                    mode, lock_target
                ));
            }
        }
    }

    // Pass 1: Every mode claims its PRIMARY class when available
    for (mode, req) in &knowledge.mode_requirements {
        if assignments.contains_key(mode) {
            continue;
        }
        if let Some(pool) = by_class.get(&req.primary_class) {
            if let Some(picked) = pool.iter().find(|c| !used.contains(&c.raw)) {
                used.insert(picked.raw.clone());
                assignments.insert(
                    mode.clone(),
                    ModeAssignment {
                        model: picked.raw.clone(),
                        class: picked.class.clone(),
                        matched: picked.matched,
                        locked: false,
                    },
                );
            }
        }
    }

    // Pass 2: Remaining modes walk their fallback chain
    for (mode, req) in &knowledge.mode_requirements {
        if assignments.contains_key(mode) {
            continue;
        }
        let mut picked: Option<&ClassifiedModel> = None;
        for cls in &req.fallback_classes {
            if let Some(pool) = by_class.get(cls) {
                if let Some(p) = pool.iter().find(|c| !used.contains(&c.raw)) {
                    picked = Some(p);
                    break;
                }
            }
        }

        let final_picked = if let Some(p) = picked {
            p.clone()
        } else {
            // Global fallback: any unused model in classified
            if let Some(unused) = classified.iter().find(|c| !used.contains(&c.raw)) {
                warnings.push(format!(
                    "{}: no {}-class model in your combo — using {} as a fallback.",
                    mode, req.primary_class, unused.raw
                ));
                unused.clone()
            } else if !classified.is_empty() {
                let share_idx = assignments.len() % classified.len();
                let shared = &classified[share_idx];
                warnings.push(format!(
                    "{}: combo has only {} model(s); sharing \"{}\".",
                    mode,
                    classified.len(),
                    shared.raw
                ));
                shared.clone()
            } else {
                continue;
            }
        };

        used.insert(final_picked.raw.clone());
        assignments.insert(
            mode.clone(),
            ModeAssignment {
                model: final_picked.raw,
                class: final_picked.class,
                matched: final_picked.matched,
                locked: false,
            },
        );
    }

    let assigned_set: HashSet<String> = assignments.values().map(|a| a.model.clone()).collect();
    let unassigned: Vec<String> = classified
        .iter()
        .filter(|c| !assigned_set.contains(&c.raw))
        .map(|c| c.raw.clone())
        .collect();

    AllocationResult {
        assignments,
        unassigned,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_knowledge() -> ModelKnowledge {
        let mut mode_requirements = BTreeMap::new();
        mode_requirements.insert(
            "boss".to_string(),
            ModeRequirement {
                primary_class: "flagship".to_string(),
                fallback_classes: vec!["reasoner".to_string(), "workhorse".to_string()],
                why: None,
            },
        );
        mode_requirements.insert(
            "chat".to_string(),
            ModeRequirement {
                primary_class: "fast".to_string(),
                fallback_classes: vec!["workhorse".to_string(), "flagship".to_string()],
                why: None,
            },
        );
        mode_requirements.insert(
            "code".to_string(),
            ModeRequirement {
                primary_class: "coder".to_string(),
                fallback_classes: vec!["flagship".to_string(), "reasoner".to_string()],
                why: None,
            },
        );
        mode_requirements.insert(
            "debug".to_string(),
            ModeRequirement {
                primary_class: "reasoner".to_string(),
                fallback_classes: vec!["flagship".to_string(), "coder".to_string()],
                why: None,
            },
        );
        mode_requirements.insert(
            "hermes".to_string(),
            ModeRequirement {
                primary_class: "workhorse".to_string(),
                fallback_classes: vec!["fast".to_string(), "flagship".to_string()],
                why: None,
            },
        );

        let models = vec![
            ModelKnowledgeEntry {
                name: "DeepSeek R-series".to_string(),
                aliases: vec!["deepseek-r1".to_string(), "r1".to_string()],
                class: "reasoner".to_string(),
                context_tokens: Some(128000),
                notes: None,
            },
            ModelKnowledgeEntry {
                name: "Qwen Coder".to_string(),
                aliases: vec!["qwen3-coder".to_string(), "qwen-coder".to_string()],
                class: "coder".to_string(),
                context_tokens: Some(128000),
                notes: None,
            },
            ModelKnowledgeEntry {
                name: "Gemini Flash".to_string(),
                aliases: vec!["gemini-2.0-flash".to_string(), "gemini-flash".to_string()],
                class: "fast".to_string(),
                context_tokens: Some(1000000),
                notes: None,
            },
            ModelKnowledgeEntry {
                name: "Llama 4 Scout".to_string(),
                aliases: vec!["llama-4-scout".to_string()],
                class: "workhorse".to_string(),
                context_tokens: Some(128000),
                notes: None,
            },
            ModelKnowledgeEntry {
                name: "GPT-5 Mini".to_string(),
                aliases: vec!["gpt-5-mini".to_string()],
                class: "fast".to_string(),
                context_tokens: Some(128000),
                notes: None,
            },
        ];

        let mut classes = BTreeMap::new();
        classes.insert("reasoner".to_string(), "desc".to_string());
        classes.insert("coder".to_string(), "desc".to_string());
        classes.insert("flagship".to_string(), "desc".to_string());
        classes.insert("workhorse".to_string(), "desc".to_string());
        classes.insert("fast".to_string(), "desc".to_string());

        ModelKnowledge {
            version: Some(1),
            classes,
            mode_requirements,
            models,
        }
    }

    #[test]
    fn longest_matching_alias_wins() {
        let kb = sample_knowledge();
        assert_eq!(classify_model("deepseek-r1:free", &kb).class, "reasoner");
        assert_eq!(classify_model("qwen3-coder", &kb).class, "coder");
        assert_eq!(classify_model("gemini-2.0-flash", &kb).class, "fast");
    }

    #[test]
    fn unmatched_names_fall_back_to_heuristics() {
        let kb = sample_knowledge();
        assert_eq!(classify_model("mystery-reasoner-v9", &kb).class, "reasoner");
        assert_eq!(classify_model("somecoder-x", &kb).class, "coder");
        assert_eq!(
            classify_model("totally-unknown-model", &kb).class,
            "workhorse"
        );
    }

    #[test]
    fn stray_short_fragments_never_produce_confident_matches() {
        let kb = sample_knowledge();
        assert!(!classify_model("r", &kb).matched);
        assert!(!classify_model("q", &kb).matched);
    }

    #[test]
    fn assigns_primary_classes_first() {
        let kb = sample_knowledge();
        let combo = vec![
            "gemini-2.0-flash".to_string(),
            "deepseek-r1:free".to_string(),
            "qwen3-coder".to_string(),
            "llama-4-scout".to_string(),
            "gpt-5-mini".to_string(),
        ];
        let r = allocate(&combo, &kb, &BTreeMap::new());
        assert_eq!(r.assignments["debug"].model, "deepseek-r1:free");
        assert_eq!(r.assignments["code"].model, "qwen3-coder");
        assert_eq!(r.assignments["chat"].model, "gemini-2.0-flash");
        assert!(r.warnings.iter().any(|w| w.starts_with("boss:")));
    }

    #[test]
    fn user_locks_always_win() {
        let kb = sample_knowledge();
        let mut locked = BTreeMap::new();
        locked.insert("code".to_string(), "custom-code-model".to_string());
        let combo = vec!["custom-code-model".to_string(), "other".to_string()];
        let r = allocate(&combo, &kb, &locked);
        assert_eq!(r.assignments["code"].model, "custom-code-model");
        assert!(r.assignments["code"].locked);
    }

    #[test]
    fn degenerate_single_model_combo_shares() {
        let kb = sample_knowledge();
        let combo = vec!["llama-4".to_string()];
        let r = allocate(&combo, &kb, &BTreeMap::new());
        assert_eq!(r.assignments.len(), 5);
        assert!(r.warnings.len() >= 4);
    }

    #[test]
    fn empty_combo_yields_empty_assignments() {
        let kb = sample_knowledge();
        let r = allocate(&[], &kb, &BTreeMap::new());
        assert_eq!(r.assignments.len(), 0);
    }
}
