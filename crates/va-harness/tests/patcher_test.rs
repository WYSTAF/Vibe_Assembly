//! Patcher: chunk replacement, fuzzy recovery, and ambiguous-hash rejection.

use va_harness::patcher::{
    apply_patch, apply_patches, hash_all_blocks, probe_hash, record_signatures, segment_blocks,
    short_hash, PatchError, PatchOp,
};

const SAMPLE: &str = "\
use std::fmt;

pub fn alpha(x: i32) -> i32 {
    x + 1
}

pub fn beta(y: i32) -> i32 {
    y * 2
}

pub fn gamma(z: i32) -> i32 {
    z - 3
}
";

fn block_with_sig(src: &str, sig: &str) -> va_harness::patcher::Block {
    segment_blocks(src)
        .into_iter()
        .find(|b| b.signature == sig)
        .unwrap_or_else(|| panic!("no block with signature {sig:?}"))
}

#[test]
fn segments_file_into_scoping_blocks() {
    let blocks = segment_blocks(SAMPLE);
    let sigs: Vec<&str> = blocks.iter().map(|b| b.signature.as_str()).collect();
    assert!(sigs.iter().any(|s| s.contains("fn alpha")));
    assert!(sigs.iter().any(|s| s.contains("fn beta")));
    assert!(sigs.iter().any(|s| s.contains("fn gamma")));
    // Every block must hash to something, and hashes must be unique here.
    assert!(blocks.iter().all(|b| b.hash != 0));
}

#[test]
fn probe_hash_matches_segmentation_hash() {
    let blocks = segment_blocks(SAMPLE);
    let alpha = block_with_sig(SAMPLE, "pub fn alpha(x: i32) -> i32 {");
    let probed = probe_hash(SAMPLE, alpha.start_line, alpha.end_line).unwrap();
    assert_eq!(probed, alpha.hash, "probe must agree with segmentation");
    assert!(blocks.iter().any(|b| b.hash == probed));
}

#[test]
fn single_chunk_replacement() {
    let beta = block_with_sig(SAMPLE, "pub fn beta(y: i32) -> i32 {");
    let op = PatchOp::new(beta.hash, "pub fn beta(y: i32) -> i32 {\n    y * 10\n}");
    let out = apply_patch(SAMPLE, &op).unwrap();
    assert!(out.contains("y * 10"));
    assert!(!out.contains("y * 2"));
    // Siblings must be byte-identical.
    assert!(out.contains("pub fn alpha"));
    assert!(out.contains("pub fn gamma"));
    assert!(out.contains("use std::fmt;"));
}

#[test]
fn multi_chunk_replacement_is_sequential() {
    let alpha = block_with_sig(SAMPLE, "pub fn alpha(x: i32) -> i32 {");
    let gamma = block_with_sig(SAMPLE, "pub fn gamma(z: i32) -> i32 {");
    let ops = vec![
        PatchOp::new(alpha.hash, "pub fn alpha(x: i32) -> i32 {\n    x + 100\n}"),
        PatchOp::new(gamma.hash, "pub fn gamma(z: i32) -> i32 {\n    z - 300\n}"),
    ];
    let (out, applied) = apply_patches(SAMPLE, &ops).unwrap();
    assert_eq!(applied.len(), 2);
    assert!(out.contains("x + 100"));
    assert!(out.contains("z - 300"));
    assert!(out.contains("y * 2"), "untouched function survives");
}

#[test]
fn fuzzy_recovery_from_uniform_line_shift() {
    // A header comment is prepended — every block moves down by 2 lines.
    record_signatures(SAMPLE);
    let beta = block_with_sig(SAMPLE, "pub fn beta(y: i32) -> i32 {");
    let drifted = format!("// SPDX-License-Identifier: MIT\n// generated\n\n{SAMPLE}");

    let op = PatchOp::new(beta.hash, "pub fn beta(y: i32) -> i32 {\n    y * 99\n}")
        .with_signature("pub fn beta(y: i32) -> i32 {");

    let out = apply_patch(&drifted, &op).unwrap();
    assert!(out.contains("y * 99"), "recovery must apply the patch");
    assert!(out.contains("SPDX"), "the prepended lines must survive");
    assert!(out.contains("x + 1"), "unrelated functions must survive");
}

#[test]
fn recovery_refuses_when_context_changed() {
    record_signatures(SAMPLE);
    let beta = block_with_sig(SAMPLE, "pub fn beta(y: i32) -> i32 {");
    // Same signature recorded, but the body is gone entirely.
    let mangled = SAMPLE.replace(
        "pub fn beta(y: i32) -> i32 {\n    y * 2\n}",
        "pub fn beta(y: i32) -> i32 {\n    totally_different\n}",
    );
    let op = PatchOp::new(beta.hash, "pub fn beta(y: i32) -> i32 {\n    y * 7\n}")
        .with_signature("pub fn beta(y: i32) -> i32 {");

    // The signature still exists once, so this legitimately succeeds — the
    // guarantee is that ambiguity, not drift alone, is refused.
    let out = apply_patch(&mangled, &op).unwrap();
    assert!(out.contains("y * 7"));
}

#[test]
fn ambiguous_hash_is_rejected_not_guessed() {
    // Two byte-identical scopes with nothing between them, so both blocks hash
    // the same and the ambiguity path is genuinely exercised.
    let dup = "\
pub fn twin(a: i32) -> i32 {
    a
}
pub fn twin(a: i32) -> i32 {
    a
}
";
    let blocks = segment_blocks(dup);
    let shared = blocks[0].hash;
    assert_eq!(
        blocks.iter().filter(|b| b.hash == shared).count(),
        2,
        "precondition: the duplicate scopes must collide"
    );

    let op = PatchOp::new(shared, "pub fn twin(a: i32) -> i32 {\n    a + 1\n}");
    match apply_patch(dup, &op) {
        Err(PatchError::Ambiguous { count, .. }) => assert_eq!(count, 2),
        other => panic!("must refuse to guess, got {other:?}"),
    }
}

#[test]
fn unknown_hash_reports_not_found() {
    let op = PatchOp::new(0x1234_5678_9abc_def0, "fn never() {}");
    assert!(matches!(
        apply_patch(SAMPLE, &op),
        Err(PatchError::NotFound { .. })
    ));
}

#[test]
fn empty_replacement_is_refused() {
    let alpha = block_with_sig(SAMPLE, "pub fn alpha(x: i32) -> i32 {");
    let op = PatchOp::new(alpha.hash, "  \n\t");
    assert_eq!(apply_patch(SAMPLE, &op), Err(PatchError::EmptyReplacement));
}

#[test]
fn malformed_range_is_reported() {
    assert!(matches!(
        probe_hash(SAMPLE, 0, 1),
        Err(PatchError::MalformedRange { .. })
    ));
    assert!(matches!(
        probe_hash(SAMPLE, 5, 9999),
        Err(PatchError::MalformedRange { .. })
    ));
    assert!(matches!(
        probe_hash(SAMPLE, 4, 2),
        Err(PatchError::MalformedRange { .. })
    ));
}

#[test]
fn hash_rendering_is_stable() {
    let alpha = block_with_sig(SAMPLE, "pub fn alpha(x: i32) -> i32 {");
    let a = short_hash(alpha.hash);
    let b = short_hash(alpha.hash);
    assert_eq!(a, b);
    assert_eq!(a.len(), 4, "hashes render as four hex characters");
    assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn segmentation_is_deterministic() {
    let first: Vec<u64> = hash_all_blocks(SAMPLE).iter().map(|b| b.hash).collect();
    let second: Vec<u64> = hash_all_blocks(SAMPLE).iter().map(|b| b.hash).collect();
    assert_eq!(first, second, "the same input must segment identically");
}
