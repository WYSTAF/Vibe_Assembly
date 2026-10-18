//! Tests for Hash-Anchored Repository Map (`va-harness::repomap`).

use std::path::{Path, PathBuf};
use va_harness::patcher::{segment_blocks, short_hash};
use va_harness::repomap::{RepoMap, SymbolKind};

fn sample_rust_code() -> &'static str {
    r#"
use std::collections::HashMap;

pub struct Config {
    pub name: String,
    pub timeout: u64,
}

pub trait Runner {
    fn run(&self) -> bool;
}

impl Runner for Config {
    fn run(&self) -> bool {
        true
    }
}

pub fn execute_task(cfg: &Config) -> Result<(), String> {
    let _ = cfg;
    Ok(())
}

fn internal_helper(x: i32) -> i32 {
    x * 2
}
"#
}

#[test]
fn test_symbol_extraction_and_block_hash_correspondence() {
    let mut map = RepoMap::new(Path::new("."));
    let rel_path = PathBuf::from("src/sample.rs");
    let content = sample_rust_code();
    map.add_file(&rel_path, content);

    // Verify symbols extracted
    let symbols = &map.files.get(&rel_path).unwrap().symbols;
    assert!(symbols.len() >= 5);

    let struct_sym = symbols.iter().find(|s| s.name == "Config").unwrap();
    assert_eq!(struct_sym.kind, SymbolKind::Struct);

    let trait_sym = symbols.iter().find(|s| s.name == "Runner").unwrap();
    assert_eq!(trait_sym.kind, SymbolKind::Trait);

    let fn_sym = symbols.iter().find(|s| s.name == "execute_task").unwrap();
    assert_eq!(fn_sym.kind, SymbolKind::Function);

    // Exact correspondence between RepoMap block hashes and patcher segment_blocks
    let blocks = segment_blocks(content);
    for sym in symbols {
        let matching_block = blocks
            .iter()
            .find(|b| b.start_line <= sym.line && sym.line <= b.end_line)
            .unwrap();
        assert_eq!(sym.block_hash, matching_block.hash);
        assert_eq!(sym.block_short_hash, short_hash(matching_block.hash));
        assert_eq!(
            sym.block_range,
            (matching_block.start_line, matching_block.end_line)
        );
    }
}

#[test]
fn test_multilingual_symbol_extraction() {
    let mut map = RepoMap::new(Path::new("."));

    // TypeScript
    let ts_code = "export interface User {\n    id: string;\n}\n\nexport class AuthManager {\n    login() {}\n}\n\nexport function verifyToken(t: string): boolean {\n    return true;\n}\n";
    map.add_file(&PathBuf::from("web/auth.ts"), ts_code);

    let ts_file = map.files.get(&PathBuf::from("web/auth.ts")).unwrap();
    assert!(ts_file
        .symbols
        .iter()
        .any(|s| s.name == "User" && s.kind == SymbolKind::Interface));
    assert!(ts_file
        .symbols
        .iter()
        .any(|s| s.name == "AuthManager" && s.kind == SymbolKind::Class));
    assert!(ts_file
        .symbols
        .iter()
        .any(|s| s.name == "verifyToken" && s.kind == SymbolKind::Function));

    // Python
    let py_code = "class WorkerPool:\n    def __init__(self):\n        pass\n\ndef run_job(job_id):\n    return 42\n";
    map.add_file(&PathBuf::from("scripts/worker.py"), py_code);

    let py_file = map.files.get(&PathBuf::from("scripts/worker.py")).unwrap();
    assert!(py_file
        .symbols
        .iter()
        .any(|s| s.name == "WorkerPool" && s.kind == SymbolKind::Class));
    assert!(py_file
        .symbols
        .iter()
        .any(|s| s.name == "run_job" && s.kind == SymbolKind::Function));
}

#[test]
fn test_render_within_token_budget_and_priority() {
    let mut map = RepoMap::new(Path::new("."));
    map.add_file(&PathBuf::from("crates/first.rs"), sample_rust_code());
    map.add_file(&PathBuf::from("crates/second.rs"), "pub fn second() {}\n");
    map.add_file(&PathBuf::from("crates/third.rs"), "pub fn third() {}\n");

    // Priority file should appear first
    let priority = vec![PathBuf::from("crates/third.rs")];
    let rendered = map.render_within_budget(2048, &priority);

    let third_pos = rendered.find("crates/third.rs").unwrap();
    let first_pos = rendered.find("crates/first.rs").unwrap();
    assert!(
        third_pos < first_pos,
        "priority file must be rendered before other files"
    );

    // Test token budget truncation
    let tightly_budgeted = map.render_within_budget(15, &priority);
    assert!(
        tightly_budgeted.contains("repomap truncated to fit token budget"),
        "must truncate when exceeding token budget"
    );
}

#[test]
fn test_find_symbols_and_block_query() {
    let mut map = RepoMap::new(Path::new("."));
    let path = PathBuf::from("src/core.rs");
    map.add_file(&path, sample_rust_code());

    let found = map.find_symbols("execute");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "execute_task");

    let block = map.find_block_for_line(&path, found[0].line).unwrap();
    assert_eq!(block.hash, found[0].block_hash);
}
