//! Heuristic scoring for novel model names Vibe Router may surface.
//!
//! These exercise models this build has never seen — the whole point of the
//! heuristic path — plus the curated/heuristic boundary.

use va_core::knowledge::{
    auto_combo, find_benchmark, infer_class, normalize, parse_version, score_model,
    version_multiplier, Role, REASONING_BOOST, SPEED_BOOST,
};

const BASELINE: f32 = 0.5;

#[test]
fn scores_a_novel_reasoning_model() {
    // A name the curated table has never seen, but whose `r1` tag is real.
    let f = score_model("nebula-reasoner-r1:free", BASELINE);
    assert!(!f.curated, "an unknown family must not claim curated data");
    assert_eq!(f.class, "reasoner");
    assert!(
        f.reasoning > BASELINE,
        "reasoning tag must raise the score: {}",
        f.reasoning
    );
    assert!(f.factors.iter().any(|x| x.name == "reasoning-tag"));
}

#[test]
fn scores_a_novel_coding_model() {
    let f = score_model("qwen4-coder-experimental", BASELINE);
    assert_eq!(f.class, "coder");
    assert!(f.coding > BASELINE);
    assert!(f.factors.iter().any(|x| x.name == "code-tag"));
}

#[test]
fn scores_a_novel_fast_model() {
    let f = score_model("gemini-3.8-flash-preview", BASELINE);
    assert_eq!(f.class, "fast");
    assert!(f.speed > BASELINE);
    assert!(f.factors.iter().any(|x| x.name == "speed-tag"));
}

#[test]
fn scores_a_novel_flagship_model() {
    let f = score_model("somevendor-opus-2", BASELINE);
    assert_eq!(f.class, "reasoner", "opus is a reasoning tag");
    assert!(f.reasoning > BASELINE);
}

#[test]
fn untagged_model_gets_the_baseline_not_a_guess() {
    let f = score_model("mystery-abc-123", BASELINE);
    assert!(!f.curated);
    assert_eq!(f.class, "workhorse");
    assert!((f.reasoning - BASELINE).abs() < 1e-6);
    assert!((f.coding - BASELINE).abs() < 1e-6);
    assert!((f.speed - BASELINE).abs() < 1e-6);
    assert!(f.factors.is_empty(), "no signal, no invented factors");
}

#[test]
fn version_progression_is_parsed_and_applied() {
    let f = score_model("glm-5.3-air", BASELINE);
    assert!(
        f.factors.iter().any(|x| x.name == "version"),
        "5.3 must be recognised: {:?}",
        f.factors
    );
    let major_bump = score_model("glm-5.3-air", BASELINE);
    let minor_bump = score_model("glm-1.3-air", BASELINE);
    assert!(
        major_bump.reasoning > minor_bump.reasoning,
        "a higher version must score higher"
    );
}

#[test]
fn version_multiplier_is_monotonic() {
    assert!(version_multiplier((2, 0)) > version_multiplier((1, 0)));
    assert!(version_multiplier((3, 2)) > version_multiplier((3, 1)));
    assert_eq!(version_multiplier((0, 5)), 1.0, "0.x gets no bump");
    assert_eq!(
        version_multiplier((1, 0)),
        1.0,
        "the baseline version gets no bump"
    );
}

#[test]
fn version_parsing_rejects_non_versions() {
    assert_eq!(parse_version("gemini-3.8-flash"), Some((3, 8)));
    assert_eq!(
        parse_version("v1.0"),
        None,
        "a leading v is not part of the version"
    );
    assert_eq!(
        parse_version("llama-4"),
        None,
        "a major alone is not enough"
    );
    assert_eq!(parse_version("none"), None);
}

#[test]
fn tag_matching_respects_boundaries() {
    // `r1` must not be found inside `xr10`, or a numeric id would inflate
    // an unrelated model's reasoning score.
    assert_eq!(infer_class("xr10-v2"), "workhorse");
    assert_eq!(infer_class("r1-preview"), "reasoner");
}

#[test]
fn documented_boost_constants_are_applied() {
    let r = score_model("abcmodel-r1", BASELINE);
    let expected = (BASELINE * REASONING_BOOST).min(1.0);
    assert!((r.reasoning - expected).abs() < 1e-6, "got {}", r.reasoning);

    let c = score_model("abcmodel-coder", BASELINE);
    let expected_coding = (BASELINE * SPEED_BOOST).min(1.0);
    assert!(
        (c.coding - expected_coding).abs() < 1e-6,
        "got {}",
        c.coding
    );
}

#[test]
fn curated_models_beat_heuristic_guesses() {
    let curated = score_model("claude-sonnet", BASELINE);
    let guessed = score_model("claude-sonnet-ish-thing", BASELINE);
    assert!(curated.curated);
    assert!(!guessed.curated || guessed.coding <= curated.coding);
}

#[test]
fn normalized_names_compare_across_punctuation() {
    // Normalization folds every non-alphanumeric separator to `-`, so case and
    // separator style never affect a lookup.
    assert_eq!(normalize("DEEPSEEK-R1"), normalize("deepseek-r1"));
    assert_eq!(normalize("qwen3_coder"), normalize("qwen3-coder"));
    assert_eq!(normalize("provider/model:v1"), "provider-model-v1");
    // A provider suffix survives normalization as a trailing segment, which
    // is why curated lookup uses `contains` rather than equality.
    assert_eq!(normalize("DeepSeek-R1:free"), "deepseek-r1-free");
}

#[test]
fn benchmark_lookup_matches_by_substring() {
    assert!(find_benchmark("claude-sonnet-4-5").is_some());
    assert!(find_benchmark("anthropic/claude-sonnet").is_some());
    assert!(find_benchmark("totally-unknown-xyz").is_none());
}

#[test]
fn auto_combo_assigns_every_role_from_novel_models_alone() {
    // A combo the curated table has never seen must still fill every role.
    let models: Vec<String> = [
        "deepseek-r1:free-neox",
        "qwen4-coder-experimental",
        "gemini-3.8-flash-preview",
        "glm-5.3-air",
        "somevendor-opus-2",
        "abcmodel-turbo",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let plan = auto_combo(&models, BASELINE);
    assert_eq!(plan.len(), Role::all().len());
    for a in &plan {
        assert!(
            models.contains(&a.model),
            "unknown model assigned: {}",
            a.model
        );
    }
}

#[test]
fn auto_combo_is_deterministic() {
    let models: Vec<String> = ["deepseek-r1", "qwen3-coder", "gemini-2.0-flash"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let first = auto_combo(&models, BASELINE);
    let second = auto_combo(&models, BASELINE);
    assert_eq!(first, second, "the same input must produce the same plan");
}

#[test]
fn auto_combo_routes_reasoning_to_boss_and_code_to_code() {
    let models: Vec<String> = ["deepseek-r1", "qwen3-coder", "gemini-2.0-flash", "gpt-5"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let plan = auto_combo(&models, BASELINE);

    let boss = plan.iter().find(|a| a.role == Role::Boss).unwrap();
    let code = plan.iter().find(|a| a.role == Role::Code).unwrap();
    // The flagship is the strongest all-rounder, so it should not be wasted
    // on the code role when a dedicated coder exists.
    assert_ne!(
        boss.model, code.model,
        "boss and code must not share when a spare model exists"
    );
    assert_eq!(
        code.model, "qwen3-coder",
        "the code role must get the coding specialist"
    );
}

#[test]
fn a_single_model_covers_every_role() {
    let plan = auto_combo(&["llama-4-scout".to_string()], BASELINE);
    assert_eq!(plan.len(), Role::all().len());
    assert!(plan.iter().all(|a| a.model == "llama-4-scout"));
}

#[test]
fn empty_combo_produces_no_assignments() {
    assert!(auto_combo(&[], BASELINE).is_empty());
}
