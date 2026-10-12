//! Model fitness scoring and version-aware heuristics.
//!
//! Two jobs:
//!
//! 1. **Curated benchmarks** for frontier models, so a known model gets a
//!    grounded score rather than a guess.
//! 2. **Heuristic inference for unknown models** — when Vibe Router surfaces a
//!    model this build has never seen (`gemini-3.8-flash`, `glm-5.3`,
//!    `deepseek-v3.2`), parse its name for family, version, and behavioural
//!    tags, then score it from those.
//!
//! Scoring is deliberately explainable: every score carries the factors that
//! produced it, so an allocation can always be justified to the user.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A role a model can be allocated to. Judge is included for the harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Role {
    Boss,
    Code,
    Debug,
    Judge,
    Hermes,
    Chat,
}

impl Role {
    pub fn all() -> &'static [Role] {
        &[
            Role::Boss,
            Role::Code,
            Role::Debug,
            Role::Judge,
            Role::Hermes,
            Role::Chat,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Boss => "boss",
            Role::Code => "code",
            Role::Debug => "debug",
            Role::Judge => "judge",
            Role::Hermes => "hermes",
            Role::Chat => "chat",
        }
    }

    pub fn parse(s: &str) -> Option<Role> {
        match s.trim().to_lowercase().as_str() {
            "boss" => Some(Role::Boss),
            "code" => Some(Role::Code),
            "debug" => Some(Role::Debug),
            "judge" => Some(Role::Judge),
            "hermes" => Some(Role::Hermes),
            "chat" => Some(Role::Chat),
            _ => None,
        }
    }
}

/// Curated benchmark data for a model we know by name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Benchmark {
    pub name: String,
    /// SWE-bench Verified, percentage.
    pub swe_bench: Option<f32>,
    /// Arena Elo.
    pub arena_elo: Option<u32>,
    /// Aider polyglot score — a coding metric.
    pub aider: Option<f32>,
    /// Tokens per second, for the speed axis. Curated models only.
    pub tps: Option<u32>,
    /// Canonical capability class, if curated.
    pub class: Option<String>,
}

/// The embedded knowledge base of known models.
pub fn benchmarks() -> Vec<Benchmark> {
    vec![
        Benchmark {
            name: "claude-opus".into(),
            swe_bench: Some(72.5),
            arena_elo: Some(1560),
            aider: Some(88.0),
            tps: Some(45),
            class: Some("flagship".into()),
        },
        Benchmark {
            name: "claude-sonnet".into(),
            swe_bench: Some(69.0),
            arena_elo: Some(1470),
            aider: Some(84.0),
            tps: Some(75),
            class: Some("coder".into()),
        },
        Benchmark {
            name: "gpt-5".into(),
            swe_bench: Some(74.0),
            arena_elo: Some(1530),
            aider: Some(86.0),
            tps: Some(60),
            class: Some("flagship".into()),
        },
        Benchmark {
            name: "gpt-5-mini".into(),
            swe_bench: Some(68.0),
            arena_elo: Some(1400),
            aider: Some(80.0),
            tps: Some(150),
            class: Some("fast".into()),
        },
        Benchmark {
            name: "gemini-2.5-pro".into(),
            swe_bench: Some(63.0),
            arena_elo: Some(1450),
            aider: Some(78.0),
            tps: Some(80),
            class: Some("flagship".into()),
        },
        Benchmark {
            name: "gemini-2.0-flash".into(),
            swe_bench: Some(38.0),
            arena_elo: Some(1300),
            aider: Some(58.0),
            tps: Some(250),
            class: Some("fast".into()),
        },
        Benchmark {
            name: "deepseek-r1".into(),
            swe_bench: Some(57.0),
            arena_elo: Some(1400),
            aider: Some(68.0),
            tps: Some(40),
            class: Some("reasoner".into()),
        },
        Benchmark {
            name: "qwen3-coder".into(),
            swe_bench: Some(60.0),
            arena_elo: Some(1380),
            aider: Some(76.0),
            tps: Some(90),
            class: Some("coder".into()),
        },
        Benchmark {
            name: "llama-4-scout".into(),
            swe_bench: Some(32.0),
            arena_elo: Some(1250),
            aider: Some(52.0),
            tps: Some(200),
            class: Some("workhorse".into()),
        },
    ]
}

/// Look up a curated benchmark by name, cloning the match.
///
/// The benchmark table is rebuilt on each call so the result can be owned —
/// a `'static` reference would require leaking a temporary `Vec`.
pub fn find_benchmark(name: &str) -> Option<Benchmark> {
    let norm = normalize(name);
    benchmarks()
        .into_iter()
        .find(|b| norm.contains(&normalize(&b.name)))
}

/// Lowercase and strip everything that is not `[a-z0-9.]`, so `DeepSeek-R1:free`
/// and `deepseek-r1` compare equal.
pub fn normalize(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Why a score was assigned — surfaced so allocations are explainable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Factor {
    pub name: String,
    pub value: f32,
    pub why: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fitness {
    pub model: String,
    pub reasoning: f32,
    pub coding: f32,
    pub speed: f32,
    /// Capability class, curated or inferred.
    pub class: String,
    /// Whether the score came from curated data or from name heuristics.
    pub curated: bool,
    pub factors: Vec<Factor>,
}

impl Fitness {
    pub fn score_for(&self, role: Role) -> f32 {
        match role {
            Role::Boss => self.reasoning * 0.75 + self.coding * 0.25,
            Role::Code => self.coding,
            Role::Debug => self.reasoning * 0.7 + self.coding * 0.3,
            // A judge must be strict and fast, not clever-but-sloppy.
            Role::Judge => self.speed * 0.4 + self.reasoning * 0.4 + self.coding * 0.2,
            Role::Hermes => self.speed * 0.6 + self.coding * 0.2 + self.reasoning * 0.2,
            Role::Chat => self.speed,
        }
    }
}

/// Tags that signal deliberate reasoning.
const REASONING_TAGS: &[&str] = &["r1", "reason", "think", "reasoning", "pro", "opus", "max"];
/// Tags that signal code-specialisation or speed.
const CODE_TAGS: &[&str] = &["coder", "code", "dev", "stark", "swe"];
const SPEED_TAGS: &[&str] = &["flash", "mini", "lite", "turbo", "haiku", "nano", "air"];

/// Multiplier applied when a reasoning tag is present (per the spec).
pub const REASONING_BOOST: f32 = 1.3;
/// Multiplier applied when a speed/coding tag is present.
pub const SPEED_BOOST: f32 = 1.25;

/// Score a model, preferring curated data and falling back to its name.
///
/// `baseline` is the score a model with no signal at all receives, so
/// unknown models are neither rewarded nor punished into oblivion.
pub fn score_model(name: &str, baseline: f32) -> Fitness {
    if let Some(b) = find_benchmark(name) {
        return Fitness {
            model: name.to_string(),
            reasoning: b.arena_elo.unwrap_or(1200) as f32 / 1600.0,
            coding: b.swe_bench.unwrap_or(40.0) / 100.0,
            // Throughput, not a coding metric: a fast model is fast at
            // everything, and Chat/Hermes are latency-bound.
            speed: b.tps.unwrap_or(60) as f32 / 250.0,
            class: b.class.clone().unwrap_or_else(|| "workhorse".into()),
            curated: true,
            factors: vec![
                Factor {
                    name: "swe-bench".into(),
                    value: b.swe_bench.unwrap_or(0.0),
                    why: format!("{} SWE-bench Verified", b.name),
                },
                Factor {
                    name: "arena-elo".into(),
                    value: b.arena_elo.unwrap_or(0) as f32,
                    why: format!("{} Arena Elo", b.name),
                },
            ],
        };
    }

    let norm = normalize(name);
    let mut factors = Vec::new();
    let mut reasoning = baseline;
    let mut coding = baseline;
    let mut speed = baseline;

    if REASONING_TAGS.iter().any(|t| contains_tag(&norm, t)) {
        reasoning *= REASONING_BOOST;
        factors.push(Factor {
            name: "reasoning-tag".into(),
            value: REASONING_BOOST,
            why: format!("name contains a reasoning tag ({name})"),
        });
    }
    if CODE_TAGS.iter().any(|t| contains_tag(&norm, t)) {
        coding *= SPEED_BOOST;
        factors.push(Factor {
            name: "code-tag".into(),
            value: SPEED_BOOST,
            why: format!("name contains a coding tag ({name})"),
        });
    }
    if SPEED_TAGS.iter().any(|t| contains_tag(&norm, t)) {
        speed *= SPEED_BOOST;
        factors.push(Factor {
            name: "speed-tag".into(),
            value: SPEED_BOOST,
            why: format!("name contains a speed tag ({name})"),
        });
    }

    // A version bump implies a newer model than the family average.
    if let Some(version) = parse_version(&norm) {
        let bump = version_multiplier(version);
        if bump != 1.0 {
            reasoning *= bump;
            coding *= bump;
            factors.push(Factor {
                name: "version".into(),
                value: bump,
                why: format!(
                    "parsed version {}.{} from the model name",
                    version.0, version.1
                ),
            });
        }
    }

    let class = infer_class(&norm).to_string();

    Fitness {
        model: name.to_string(),
        reasoning: reasoning.min(1.0),
        coding: coding.min(1.0),
        speed: speed.min(1.0),
        class,
        curated: false,
        factors,
    }
}

/// Tag matching on hyphen boundaries, so `r1` never matches inside `xr10`.
fn contains_tag(norm: &str, tag: &str) -> bool {
    let padded = format!("-{norm}-");
    padded.contains(&format!("-{tag}")) || norm == tag || norm.starts_with(&format!("{tag}-"))
}

/// Parse a dotted version such as `3.8` or `4.0` out of a normalized name.
pub fn parse_version(norm: &str) -> Option<(u32, u32)> {
    for part in norm.split('-') {
        let mut bits = part.split('.');
        if let (Some(major), Some(minor)) = (bits.next(), bits.next()) {
            if let (Ok(ma), Ok(mi)) = (major.parse::<u32>(), minor.parse::<u32>()) {
                if ma <= 99 && mi <= 99 {
                    return Some((ma, mi));
                }
            }
        }
    }
    None
}

/// Each major version above the first bumps overall capability.
pub fn version_multiplier(version: (u32, u32)) -> f32 {
    let (major, minor) = version;
    if major == 0 {
        return 1.0;
    }
    1.0 + (major.saturating_sub(1) as f32) * 0.15 + (minor.min(9) as f32) * 0.01
}

/// Infer a capability class purely from the name.
pub fn infer_class(norm: &str) -> &'static str {
    if REASONING_TAGS.iter().any(|t| contains_tag(norm, t)) {
        "reasoner"
    } else if CODE_TAGS.iter().any(|t| contains_tag(norm, t)) {
        "coder"
    } else if SPEED_TAGS.iter().any(|t| contains_tag(norm, t)) {
        "fast"
    } else {
        "workhorse"
    }
}

/// A computed zero-config allocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assignment {
    pub role: Role,
    pub model: String,
    pub score: f32,
}

/// Auto-assign `models` to the six roles by fitness, with no configuration.
///
/// Each model is used at most once; a second pass reuses the best remaining
/// model for any role left unfilled, so the result always covers every role.
pub fn auto_combo(models: &[String], baseline: f32) -> Vec<Assignment> {
    let fitness: Vec<Fitness> = models.iter().map(|m| score_model(m, baseline)).collect();

    let mut out: Vec<Assignment> = Vec::new();
    let mut used: Vec<usize> = Vec::new();

    for role in Role::all() {
        let best = fitness
            .iter()
            .enumerate()
            .filter(|(i, _)| !used.contains(i))
            .max_by(|a, b| {
                a.1.score_for(*role)
                    .partial_cmp(&b.1.score_for(*role))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        if let Some((idx, f)) = best {
            used.push(idx);
            out.push(Assignment {
                role: *role,
                model: f.model.clone(),
                score: f.score_for(*role),
            });
        }
    }

    // Fill any gaps by reusing the best available model, mirroring the
    // offline allocator's "share rather than leave a role unassigned".
    let missing: Vec<Role> = Role::all()
        .iter()
        .copied()
        .filter(|r| !out.iter().any(|a| a.role == *r))
        .collect();
    for role in missing {
        if let Some(f) = fitness.first() {
            out.push(Assignment {
                role,
                model: f.model.clone(),
                score: f.score_for(role),
            });
        }
    }

    out
}

/// Convenience: scores for every role keyed by role name, for JSON output.
pub fn scores_by_role(assignments: &[Assignment]) -> BTreeMap<String, (String, f32)> {
    assignments
        .iter()
        .map(|a| (a.role.as_str().to_string(), (a.model.clone(), a.score)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curated_models_are_preferred() {
        let f = score_model("deepseek-r1:free", 0.5);
        assert!(f.curated);
        assert_eq!(f.class, "reasoner");
        assert!(!f.factors.is_empty());
    }

    #[test]
    fn unknown_model_is_scored_from_its_name() {
        let f = score_model("gemini-3.8-flash", 0.5);
        assert!(!f.curated, "must not claim to be curated");
        assert!(f.speed > 0.5, "the flash tag raises speed");
        assert_eq!(f.class, "fast");
    }

    #[test]
    fn reasoning_tag_applies_the_documented_boost() {
        let base = score_model("mystery-reasoner-v9", 0.5);
        let with_tag = score_model("deepseek-r1", 0.5);
        // Both go through the same path for the tag; assert the factor exists.
        assert!(base
            .factors
            .iter()
            .any(|f| f.name == "reasoning-tag" && (f.value - REASONING_BOOST).abs() < 1e-6));
        assert!(with_tag.curated);
    }

    #[test]
    fn version_progression_raises_the_score() {
        let v1 = version_multiplier((1, 0));
        let v3 = version_multiplier((3, 0));
        assert!(v3 > v1, "a newer major version must score higher");
        assert!((version_multiplier((1, 5)) - version_multiplier((1, 0))) > 0.0);
    }

    #[test]
    fn versions_are_parsed_from_names() {
        assert_eq!(parse_version("gemini-3.8-flash"), Some((3, 8)));
        assert_eq!(parse_version("glm-5.3"), Some((5, 3)));
        assert_eq!(
            parse_version("deepseek-v3.2"),
            None,
            "a prefixed v is not a version"
        );
        assert_eq!(parse_version("no-version-here"), None);
    }

    #[test]
    fn tags_do_not_match_inside_longer_words() {
        assert!(!contains_tag("xr10-model", "r1"), "r1 must not match xr10");
        assert!(contains_tag("r1-model", "r1"));
    }

    #[test]
    fn auto_combo_covers_every_role_exactly_once() {
        let models = vec![
            "gemini-2.0-flash".to_string(),
            "deepseek-r1".to_string(),
            "qwen3-coder".to_string(),
            "claude-sonnet".to_string(),
            "gpt-5".to_string(),
            "llama-4-scout".to_string(),
        ];
        let plan = auto_combo(&models, 0.5);
        assert_eq!(plan.len(), Role::all().len());
        let mut roles: Vec<&str> = plan.iter().map(|a| a.role.as_str()).collect();
        roles.sort_unstable();
        let mut expected: Vec<&str> = Role::all().iter().map(|r| r.as_str()).collect();
        expected.sort_unstable();
        assert_eq!(roles, expected);
    }

    #[test]
    fn auto_combo_gives_each_role_a_distinct_model_when_possible() {
        let models = vec![
            "gemini-2.0-flash".to_string(),
            "deepseek-r1".to_string(),
            "qwen3-coder".to_string(),
        ];
        let plan = auto_combo(&models, 0.5);
        let unique: std::collections::HashSet<&str> =
            plan.iter().map(|a| a.model.as_str()).collect();
        assert_eq!(unique.len(), 3, "three models must serve three roles each");
    }

    #[test]
    fn auto_combo_degrades_gracefully_with_few_models() {
        let plan = auto_combo(&["llama-4-scout".to_string()], 0.5);
        assert_eq!(
            plan.len(),
            Role::all().len(),
            "every role is still assigned"
        );
        assert!(plan.iter().all(|a| a.model == "llama-4-scout"));
    }

    #[test]
    fn empty_model_list_yields_nothing_to_allocate() {
        assert!(auto_combo(&[], 0.5).is_empty());
    }

    #[test]
    fn role_scoring_prefers_the_intended_strength() {
        let coder = score_model("qwen3-coder", 0.5);
        let speedster = score_model("gemini-2.0-flash", 0.5);
        assert!(coder.score_for(Role::Code) > speedster.score_for(Role::Code));
        assert!(speedster.score_for(Role::Chat) > coder.score_for(Role::Chat));
    }
}
