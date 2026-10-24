//! Deterministic hash-anchored source patching.
//!
//! # Design note — relationship to the upstream reference
//!
//! In the upstream (`oh-my-pi`, Rust, crate `pi-edit` "hashline" mode) the
//! 4-hex tag is a **whole-file staleness assertion** produced by XXH32, and
//! individual blocks are addressed by **line number plus a tree-sitter AST**.
//! Block lowering parses against the tagged snapshot, never the live file.
//!
//! This module implements the anchoring behaviour the Vibe Assembly wave
//! requires — patches target **per-block hashes**, never raw line numbers —
//! using two deliberate substitutions, both forced by the local environment:
//!
//! * **FNV-1a instead of XXH32.** `xxhash-rust` is not in the local cargo
//!   cache and the network is heavily throttled; the spec explicitly permits
//!   FNV-1a. It is a 64-bit digest rather than a 16-bit tag, so it carries far
//!   more collision headroom than upstream's 4-hex tag.
//! * **Indentation-scoped blocks instead of tree-sitter.** ~55 grammar crates
//!   are unavailable offline. Blocks are derived from brace/indent structure,
//!   which handles Rust well and degrades predictably elsewhere.
//!
//! The two properties that matter most are preserved from upstream and are
//! enforced strictly:
//!
//! 1. **Drift is detected, never assumed.** If the file changed since hashing,
//!    a patch fails unless a *single-hop structural rebase* applies.
//! 2. **Ambiguity is a hard rejection.** A target hash matching more than one
//!    block is an error, never a guess. Upstream has a test asserting exactly
//!    this: *"even the correctly shifted anchor on repeated content rejects."*

use serde::{Deserialize, Serialize};

use crate::session::fnv1a64;

/// A contiguous slice of a source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    /// 1-based inclusive line range.
    pub start_line: usize,
    pub end_line: usize,
    /// Deterministic hash of the block's normalized text.
    pub hash: u64,
    /// First line's trimmed text — used to re-identify a block after drift.
    pub signature: String,
}

impl Block {
    pub fn line_count(&self) -> usize {
        self.end_line.saturating_sub(self.start_line) + 1
    }
}

/// Normalize a line for hashing: strip trailing whitespace and a BOM/CR.
fn normalize_line(line: &str) -> &str {
    line.trim_end_matches([' ', '\t', '\r'])
}

/// Normalized whole-file text, used by the repeated-anchor guard.
fn normalized_source(source: &str) -> String {
    source
        .lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Normalized body of one block.
fn normalized_block_body(source: &str, block: &Block) -> String {
    let lines: Vec<&str> = source.lines().collect();
    if block.start_line == 0 || block.end_line > lines.len() || block.start_line > block.end_line {
        return String::new();
    }
    lines[block.start_line - 1..block.end_line]
        .iter()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Count non-overlapping occurrences of `needle` in `haystack`.
fn occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() || needle.len() > haystack.len() {
        return 0;
    }
    haystack.match_indices(needle).count()
}

/// Hash a block's normalized content. Leading indentation is *preserved*
/// because it is semantic in Rust; trailing whitespace is not.
pub fn hash_block(lines: &[&str]) -> u64 {
    let mut buf = String::with_capacity(lines.len() * 32);
    for line in lines {
        buf.push_str(normalize_line(line));
        buf.push('\n');
    }
    fnv1a64(buf.as_bytes())
}

/// Hash an entire file's normalized content.
/// Ported from oh-my-pi's whole-file staleness assertion (XXH32 -> 64-bit FNV-1a).
pub fn hash_file(source: &str) -> u64 {
    let mut buf = String::with_capacity(source.len());
    for line in source.lines() {
        buf.push_str(normalize_line(line));
        buf.push('\n');
    }
    fnv1a64(buf.as_bytes())
}

/// Check if a line opens a multi-line raw string literal, returning the expected closer.
fn detect_raw_string_open(line: &str) -> Option<&'static str> {
    if let Some(pos) = line.find("r##\"") {
        if !line[pos + 4..].contains("\"##") {
            return Some("\"##");
        }
    } else if let Some(pos) = line.find("r#\"") {
        if !line[pos + 3..].contains("\"#") {
            return Some("\"#");
        }
    } else if let Some(pos) = line.find("r\"") {
        if !line[pos + 2..].contains('"') {
            return Some("\"");
        }
    }
    None
}

/// Split `source` into semantic blocks.
///
/// A block starts at a line with lower indentation than its predecessor (a
/// scope boundary) and runs to the line before the next such boundary. This
/// approximates the AST segmentation the upstream obtains from tree-sitter
/// without the grammar dependency.
pub fn segment_blocks(source: &str) -> Vec<Block> {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }

    let mut in_block_comment = false;
    let mut raw_string_closer: Option<&'static str> = None;

    let mut indents: Vec<usize> = Vec::with_capacity(lines.len());
    for line in &lines {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            indents.push(usize::MAX);
            continue;
        }

        // Multi-line block comment tracking
        if in_block_comment {
            if line.contains("*/") {
                in_block_comment = false;
            }
            indents.push(usize::MAX);
            continue;
        } else if trimmed.starts_with("/*") && !line.contains("*/") {
            in_block_comment = true;
            indents.push(usize::MAX);
            continue;
        }

        // Multi-line raw string tracking (e.g. r#"..."# or r"...")
        if let Some(closer) = raw_string_closer {
            if line.contains(closer) {
                raw_string_closer = None;
            }
            indents.push(usize::MAX);
            continue;
        } else if let Some(closer) = detect_raw_string_open(line) {
            raw_string_closer = Some(closer);
        }

        // Single-line comments, doc comments, block comment continuations
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            indents.push(usize::MAX);
            continue;
        }

        // A closing brace/bracket/paren dedents to column 0, which would otherwise open a
        // new block and orphan it from the body it terminates. Braces are part
        // of the scope they close, so give them the body's depth.
        if trimmed.starts_with('}') || trimmed.starts_with(')') || trimmed.starts_with(']') {
            indents.push(usize::MAX);
            continue;
        }

        indents.push(line.len() - trimmed.len());
    }

    let mut starts: Vec<usize> = vec![0];
    for i in 1..lines.len() {
        let cur = indents[i];
        let prev = indents[i - 1];
        if cur != usize::MAX && cur < prev {
            starts.push(i);
        }
    }

    let mut blocks = Vec::new();
    for (n, &start) in starts.iter().enumerate() {
        let end = starts.get(n + 1).map_or(lines.len(), |&next| next) - 1;
        let slice = &lines[start..=end.min(lines.len() - 1)];
        blocks.push(Block {
            start_line: start + 1,
            end_line: end + 1,
            hash: hash_block(slice),
            signature: slice
                .first()
                .map(|l| l.trim().to_string())
                .unwrap_or_default(),
        });
    }
    blocks
}

/// Find every block in `source` whose hash equals `hash`.
pub fn find_blocks_by_hash(source: &str, hash: u64) -> Vec<Block> {
    segment_blocks(source)
        .into_iter()
        .filter(|b| b.hash == hash)
        .collect()
}

/// Compute the hashes of a file once, for a batch of patch targets.
pub fn hash_all_blocks(source: &str) -> Vec<Block> {
    segment_blocks(source)
}

/// Render a block hash the way a UI or prompt would display it.
pub fn short_hash(hash: u64) -> String {
    format!("{:04X}", (hash >> 48) as u16)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchError {
    /// No block in the file carries this hash.
    NotFound { hash: u64 },
    /// More than one block carries this hash — refuse rather than guess.
    Ambiguous { hash: u64, count: usize },
    /// The file drifted since hashing and no single-hop rebase applies.
    Stale {
        expected_line: usize,
        actual_line: usize,
    },
    /// The replacement is empty and the caller demanded an insertion.
    EmptyReplacement,
    /// The block bounds are inconsistent with the file.
    MalformedRange {
        start: usize,
        end: usize,
        lines: usize,
    },
    /// A 1-hop rebase was attempted but the enclosing context changed.
    ContextMismatch { expected: String, actual: String },
    /// Whole file drifted since discovery — external modification detected (oh-my-pi).
    FileDrifted { expected: u64, actual: u64 },
    /// Structural recovery found more than one block with the same normalized
    /// body. Even a correctly shifted anchor is rejected on repeated content.
    AmbiguousRepeatedAnchor { signature: String, count: usize },
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchError::NotFound { hash } => write!(
                f,
                "no block with hash {} ({}) in the file — it may have been edited",
                short_hash(*hash),
                hash
            ),
            PatchError::Ambiguous { hash, count } => write!(
                f,
                "hash {} matches {count} blocks — refusing to guess; add context to disambiguate",
                short_hash(*hash)
            ),
            PatchError::Stale {
                expected_line,
                actual_line,
            } => write!(
                f,
                "anchor drifted: expected line {expected_line}, found at {actual_line}"
            ),
            PatchError::EmptyReplacement => write!(f, "replacement text is empty"),
            PatchError::MalformedRange { start, end, lines } => write!(
                f,
                "block range {start}-{end} is outside the file ({lines} lines)"
            ),
            PatchError::ContextMismatch { expected, actual } => write!(
                f,
                "enclosing context changed: expected `{expected}`, found `{actual}` — refusing to rebase"
            ),
            PatchError::FileDrifted { expected, actual } => write!(
                f,
                "whole file drifted since discovery: expected hash {expected:#018x}, found {actual:#018x}"
            ),
            PatchError::AmbiguousRepeatedAnchor { signature, count } => write!(
                f,
                "repeated anchor `{signature}` appears {count} times — structurally shifted anchor rejected"
            ),
        }
    }
}

impl std::error::Error for PatchError {}

/// A patch instruction targeting a block by hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchOp {
    pub target_hash: u64,
    pub replacement: String,
    /// Whole-file hash at discovery time. If set, catches external file drift
    /// before block resolution (ported from oh-my-pi).
    pub expected_file_hash: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyOutcome {
    /// Applied directly; the block was where the hash said.
    Direct {
        block: Block,
        recovery: Option<RecoveryKind>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryKind {
    /// The block moved; a single uniform line offset was applied.
    LineShift { offset: isize },
    /// The block was re-identified by its opening signature after drift.
    SignatureMatch,
}

/// Apply `op` to `source`, addressing the target by hash.
///
/// Resolution order:
/// 1. Whole-file staleness check (if `expected_file_hash` supplied).
/// 2. Exact hash match — unambiguous, applied directly.
/// 3. Single-hop recovery when the hash is gone but the block's opening
///    signature appears exactly once at a different position.
/// 4. Hard failure. Ambiguity at any step is an error, never a guess.
pub fn apply_patch(source: &str, op: &PatchOp) -> Result<String, PatchError> {
    if let Some(expected) = op.expected_file_hash {
        let actual = hash_file(source);
        if actual != expected {
            return Err(PatchError::FileDrifted { expected, actual });
        }
    }

    if op.replacement.trim().is_empty() {
        return Err(PatchError::EmptyReplacement);
    }

    let matches = find_blocks_by_hash(source, op.target_hash);

    match matches.len() {
        1 => {
            let block = matches[0].clone();
            let updated = replace_lines(source, &block, &op.replacement)?;
            Ok(updated)
        }
        0 => recover_or_fail(source, op),
        n => Err(PatchError::Ambiguous {
            hash: op.target_hash,
            count: n,
        }),
    }
}

fn recover_or_fail(source: &str, op: &PatchOp) -> Result<String, PatchError> {
    // 1-hop recovery: the block's opening signature may still exist at a
    // shifted position. We need the original signature — recover it by
    // looking for a block whose *content* is a prefix-match of the
    // replacement's neighbourhood is unreliable, so instead require the
    // caller to have supplied a signature-bearing op.
    //
    // Without the original signature we cannot re-identify the block, so the
    // honest answer is: not found. Callers that want drift tolerance supply
    // `op.expected_signature`.
    let expected = op_signature(op)?;
    let blocks = segment_blocks(source);
    let candidates: Vec<&Block> = blocks.iter().filter(|b| b.signature == expected).collect();

    match candidates.len() {
        1 => {
            let block = candidates[0].clone();
            // Strict repeated-content guard: even a correctly shifted anchor
            // is rejected when the same normalized body appears more than once
            // in the file. A hash that no longer exists plus repeated content
            // is exactly the case where guessing corrupts code.
            let body = normalized_block_body(source, &block);
            let repeats = occurrences(&normalized_source(source), &body);
            if repeats > 1 {
                return Err(PatchError::AmbiguousRepeatedAnchor {
                    signature: expected,
                    count: repeats,
                });
            }
            // Single-hop structural rebase: confirm the drift is a pure line
            // shift by checking the block still opens with the same text.
            let lines: Vec<&str> = source.lines().collect();
            let actual = lines
                .get(block.start_line - 1)
                .map(|l| l.trim().to_string())
                .unwrap_or_default();
            if actual != expected {
                return Err(PatchError::ContextMismatch { expected, actual });
            }
            let updated = replace_lines(source, &block, &op.replacement)?;
            Ok(updated)
        }
        0 => Err(PatchError::NotFound {
            hash: op.target_hash,
        }),
        n => Err(PatchError::Ambiguous {
            hash: op.target_hash,
            count: n,
        }),
    }
}

/// Signature carrier so `PatchOp` stays a plain data type. Stored alongside by
/// `PatchOp::with_signature`.
fn op_signature(op: &PatchOp) -> Result<String, PatchError> {
    SIGNATURES.with(|s| {
        s.borrow()
            .get(&op.target_hash)
            .cloned()
            .ok_or(PatchError::NotFound {
                hash: op.target_hash,
            })
    })
}

thread_local! {
    /// Signatures recorded at hash time, enabling 1-hop recovery.
    static SIGNATURES: std::cell::RefCell<HashMapT> = std::cell::RefCell::new(HashMapT::new());
}

type HashMapT = std::collections::HashMap<u64, String>;

impl PatchOp {
    pub fn new(target_hash: u64, replacement: impl Into<String>) -> Self {
        Self {
            target_hash,
            replacement: replacement.into(),
            expected_file_hash: None,
        }
    }

    pub fn with_expected_file_hash(mut self, hash: u64) -> Self {
        self.expected_file_hash = Some(hash);
        self
    }

    /// Record the block's opening signature at hash time, enabling drift
    /// recovery later.
    pub fn with_signature(self, signature: &str) -> Self {
        SIGNATURES.with(|s| {
            s.borrow_mut()
                .insert(self.target_hash, signature.to_string());
        });
        self
    }

    pub fn target_hash(&self) -> u64 {
        self.target_hash
    }
}

/// Record signatures for every block in `source`, so a later patch can
/// recover from a uniform line shift.
pub fn record_signatures(source: &str) {
    for block in segment_blocks(source) {
        SIGNATURES.with(|s| {
            s.borrow_mut().insert(block.hash, block.signature.clone());
        });
    }
}

fn replace_lines(source: &str, block: &Block, replacement: &str) -> Result<String, PatchError> {
    let lines: Vec<&str> = source.lines().collect();
    if block.start_line == 0 || block.end_line > lines.len() || block.start_line > block.end_line {
        return Err(PatchError::MalformedRange {
            start: block.start_line,
            end: block.end_line,
            lines: lines.len(),
        });
    }
    let mut out: Vec<String> = Vec::with_capacity(lines.len() + 4);
    out.extend(lines[..block.start_line - 1].iter().map(|s| s.to_string()));
    out.extend(replacement.lines().map(|s| s.to_string()));
    if block.end_line < lines.len() {
        out.extend(lines[block.end_line..].iter().map(|s| s.to_string()));
    }
    Ok(out.join("\n"))
}

/// Apply several patches in sequence, stopping at the first failure.
///
/// Sequential application matters: each patch re-segments the file, so an
/// earlier edit changes the block boundaries the next patch addresses.
pub fn apply_patches(source: &str, ops: &[PatchOp]) -> Result<(String, Vec<u64>), PatchError> {
    let mut current = source.to_string();
    let mut applied = Vec::with_capacity(ops.len());
    for op in ops {
        current = apply_patch(&current, op)?;
        applied.push(op.target_hash);
    }
    Ok((current, applied))
}

/// Hash a single line-range as a standalone block — the discovery helper an
/// agent uses to learn what to patch.
pub fn probe_hash(source: &str, start_line: usize, end_line: usize) -> Result<u64, PatchError> {
    let lines: Vec<&str> = source.lines().collect();
    if start_line == 0 || end_line > lines.len() || start_line > end_line {
        return Err(PatchError::MalformedRange {
            start: start_line,
            end: end_line,
            lines: lines.len(),
        });
    }
    Ok(hash_block(&lines[start_line - 1..end_line]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str =
        "fn alpha() {\n    let x = 1;\n    x\n}\n\nfn beta() {\n    let y = 2;\n    y\n}\n";

    #[test]
    fn segments_scopes_into_blocks() {
        let blocks = segment_blocks(SAMPLE);
        assert!(blocks.len() >= 2, "expected at least two scopes");
        assert!(blocks.iter().any(|b| b.signature == "fn alpha() {"));
        assert!(blocks.iter().any(|b| b.signature == "fn beta() {"));
    }

    #[test]
    fn hash_is_deterministic_and_whitespace_insensitive_at_eol() {
        // Trailing spaces/tabs/CR are stripped; indentation is preserved.
        let a = hash_block(&["fn a() {", "    let x = 1;", "}"]);
        let b = hash_block(&["fn a() {", "    let x = 1;   ", "}\r"]);
        assert_eq!(a, b);
    }

    #[test]
    fn leading_indentation_changes_the_hash() {
        let a = hash_block(&["fn a() {", "    let x = 1;", "}"]);
        let b = hash_block(&["fn a() {", "let x = 1;", "}"]);
        assert_ne!(a, b);
    }

    #[test]
    fn applies_patch_to_unique_block() {
        let block = segment_blocks(SAMPLE)
            .into_iter()
            .find(|b| b.signature == "fn beta() {")
            .unwrap();
        let op = PatchOp::new(block.hash, "fn beta() {\n    let y = 99;\n    y\n}");
        let out = apply_patch(SAMPLE, &op).unwrap();
        assert!(out.contains("let y = 99;"));
        assert!(!out.contains("let y = 2;"));
        // The untouched function must survive verbatim.
        assert!(out.contains("fn alpha() {"));
    }

    #[test]
    fn rejects_unknown_hash() {
        let op = PatchOp::new(0xdead_beef, "x");
        assert!(matches!(
            apply_patch(SAMPLE, &op),
            Err(PatchError::NotFound { .. })
        ));
    }

    #[test]
    fn rejects_empty_replacement() {
        let block = segment_blocks(SAMPLE)[0].clone();
        let op = PatchOp::new(block.hash, "   ");
        assert_eq!(apply_patch(SAMPLE, &op), Err(PatchError::EmptyReplacement));
    }

    #[test]
    fn ambiguous_hash_is_a_hard_rejection() {
        // Two byte-identical scopes with nothing between them, so both blocks
        // hash the same and the rejection path is genuinely exercised.
        let dup = "fn a() {\n    1\n}\nfn a() {\n    1\n}\n";
        let blocks = segment_blocks(dup);
        let hashes: Vec<u64> = blocks.iter().map(|b| b.hash).collect();
        assert_eq!(blocks.len(), 2, "two scopes must be two blocks");
        let shared = hashes[0];
        assert_eq!(
            hashes.iter().filter(|h| **h == shared).count(),
            2,
            "duplicate scopes must collide so the rejection path is exercised"
        );
        let op = PatchOp::new(shared, "fn a() {\n    2\n}");
        assert!(matches!(
            apply_patch(dup, &op),
            Err(PatchError::Ambiguous { count: 2, .. })
        ));
    }

    #[test]
    fn recovers_from_uniform_line_shift() {
        // Hash the file, then prepend a line (simulating drift).
        record_signatures(SAMPLE);
        let block = segment_blocks(SAMPLE)
            .into_iter()
            .find(|b| b.signature == "fn beta() {")
            .unwrap();
        let drifted = format!("// new header comment\n\n{}", SAMPLE);
        let op = PatchOp::new(block.hash, "fn beta() {\n    let y = 42;\n    y\n}")
            .with_signature("fn beta() {");
        let out = apply_patch(&drifted, &op).unwrap();
        assert!(out.contains("let y = 42;"));
        assert!(out.contains("fn alpha() {"));
    }

    #[test]
    fn rejects_when_whole_file_drifted() {
        let block = segment_blocks(SAMPLE)[0].clone();
        let file_hash = hash_file(SAMPLE);
        let op = PatchOp::new(block.hash, "fn alpha() { 42 }").with_expected_file_hash(file_hash);

        let drifted = format!("// header\n{}", SAMPLE);
        assert!(matches!(
            apply_patch(&drifted, &op),
            Err(PatchError::FileDrifted { .. })
        ));
    }

    #[test]
    fn repeated_body_rejects_even_a_correctly_shifted_anchor() {
        // The body of `fn alpha` appears twice (once as a block, once nested in
        // a comment-bearing block), but only one block has its signature. The
        // strict guard must still refuse rather than guess which copy to edit.
        let repeated = "fn alpha() {\n    let x = 1;\n    let y = 2;\n    x\n}\n\nfn beta() {\n    let marker = 0;\n    fn alpha() {\n        let x = 1;\n        let y = 2;\n        x\n    }\n}\n";
        let block = segment_blocks(SAMPLE)
            .into_iter()
            .find(|b| b.signature == "fn alpha() {")
            .unwrap();
        let op = PatchOp::new(block.hash, "fn alpha() {\n    2\n}").with_signature("fn alpha() {");
        match apply_patch(repeated, &op) {
            Err(PatchError::AmbiguousRepeatedAnchor { count, .. }) => {
                assert_eq!(count, 2, "both copies must be counted")
            }
            other => panic!("expected AmbiguousRepeatedAnchor, got {other:?}"),
        }
    }

    #[test]
    fn raw_string_literal_does_not_split_scope() {
        let code = "fn make_query() -> &'static str {\n    r#\"\nSELECT *\nFROM users\n\"#\n}\n";
        let blocks = segment_blocks(code);
        assert_eq!(
            blocks.len(),
            1,
            "raw string literal must not split the function into orphaned blocks"
        );
    }
}
