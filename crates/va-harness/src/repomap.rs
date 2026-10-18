//! Hash-Anchored Repository Map (`va-harness::repomap`).
//!
//! Provides a compact, token-budgeted structural symbol map of workspace files,
//! binding every function, struct, trait, and class directly to its deterministic
//! 64-bit FNV-1a block hash from `va-harness::patcher`.
//!
//! Allows coding agents to discover exact target block hashes for edits without
//! wasting context reading whole source trees.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use crate::patcher::{hash_file, segment_blocks, short_hash, Block};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Impl,
    Type,
    Module,
    Constant,
    Class,
    Interface,
    Other,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "fn",
            SymbolKind::Method => "method",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Trait => "trait",
            SymbolKind::Impl => "impl",
            SymbolKind::Type => "type",
            SymbolKind::Module => "mod",
            SymbolKind::Constant => "const",
            SymbolKind::Class => "class",
            SymbolKind::Interface => "interface",
            SymbolKind::Other => "symbol",
        }
    }
}

/// A structural symbol bound to a deterministic block hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolEntry {
    pub name: String,
    pub signature: String,
    pub kind: SymbolKind,
    pub file_path: PathBuf,
    pub line: usize,
    pub block_hash: u64,
    pub block_short_hash: String,
    pub block_range: (usize, usize),
}

/// File-level structural metadata in the repository map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMapEntry {
    pub file_path: PathBuf,
    pub file_hash: u64,
    pub file_short_hash: String,
    pub blocks: Vec<Block>,
    pub symbols: Vec<SymbolEntry>,
}

/// The hash-anchored repository map.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoMap {
    pub root: PathBuf,
    pub files: BTreeMap<PathBuf, FileMapEntry>,
}

impl RepoMap {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            files: BTreeMap::new(),
        }
    }

    /// Scan a workspace root honoring standard exclusions.
    pub fn scan(root: &Path) -> Self {
        let mut map = Self::new(root);
        let mut exclusions = HashSet::new();
        exclusions.insert("target".to_string());
        exclusions.insert(".git".to_string());
        exclusions.insert("references".to_string());
        exclusions.insert("portable_data".to_string());
        exclusions.insert("node_modules".to_string());
        exclusions.insert(".cargo".to_string());

        // Parse root .gitignore if available
        let gitignore_path = root.join(".gitignore");
        if let Ok(content) = std::fs::read_to_string(gitignore_path) {
            for line in content.lines() {
                let l = line.trim();
                if l.is_empty() || l.starts_with('#') {
                    continue;
                }
                let clean = l.trim_matches('/').trim_matches('\\');
                if !clean.is_empty() && !clean.contains('*') {
                    exclusions.insert(clean.to_string());
                }
            }
        }

        walk_dir_recursive(root, root, &exclusions, &mut map);
        map
    }

    /// Add a single source file to the map.
    pub fn add_file(&mut self, rel_path: &Path, content: &str) {
        let file_hash = hash_file(content);
        let file_short_hash = short_hash(file_hash);
        let blocks = segment_blocks(content);
        let symbols = extract_symbols(rel_path, content, &blocks);

        self.files.insert(
            rel_path.to_path_buf(),
            FileMapEntry {
                file_path: rel_path.to_path_buf(),
                file_hash,
                file_short_hash,
                blocks,
                symbols,
            },
        );
    }

    /// Total count of indexed symbols.
    pub fn total_symbols(&self) -> usize {
        self.files.values().map(|f| f.symbols.len()).sum()
    }

    /// Total count of indexed blocks.
    pub fn total_blocks(&self) -> usize {
        self.files.values().map(|f| f.blocks.len()).sum()
    }

    /// Total files indexed.
    pub fn total_files(&self) -> usize {
        self.files.len()
    }

    /// Find symbols matching a query name (substring or exact).
    pub fn find_symbols(&self, query: &str) -> Vec<&SymbolEntry> {
        let q = query.to_lowercase();
        let mut results = Vec::new();
        for file in self.files.values() {
            for sym in &file.symbols {
                if sym.name.to_lowercase().contains(&q) || sym.signature.to_lowercase().contains(&q)
                {
                    results.push(sym);
                }
            }
        }
        results
    }

    /// Find the block containing a line number in a file.
    pub fn find_block_for_line(&self, rel_path: &Path, line: usize) -> Option<&Block> {
        self.files.get(rel_path).and_then(|f| {
            f.blocks
                .iter()
                .find(|b| b.start_line <= line && line <= b.end_line)
        })
    }

    /// Render a compact, deterministic structural map within a token budget.
    ///
    /// Priority files are rendered first, followed by alphabetical workspace files.
    /// Token budget estimation uses standard ~4 characters per token heuristic.
    pub fn render_within_budget(&self, max_tokens: usize, priority_files: &[PathBuf]) -> String {
        let token_budget = if max_tokens == 0 { 2048 } else { max_tokens };
        let char_limit = token_budget * 4;

        let mut ordered_paths: Vec<&PathBuf> = Vec::new();
        let mut seen = HashSet::new();

        // 1. Priority files first
        for p in priority_files {
            let norm = normalize_rel_path(p);
            if self.files.contains_key(&norm) && seen.insert(norm.clone()) {
                if let Some((k, _)) = self.files.get_key_value(&norm) {
                    ordered_paths.push(k);
                }
            }
        }

        // 2. Remaining files in alphabetical order
        for k in self.files.keys() {
            if seen.insert(k.clone()) {
                ordered_paths.push(k);
            }
        }

        let mut out = String::with_capacity(char_limit.min(64 * 1024));
        let mut truncated = false;

        for path in ordered_paths {
            if let Some(entry) = self.files.get(path) {
                if entry.symbols.is_empty() && entry.blocks.is_empty() {
                    continue;
                }

                let path_display = path.to_string_lossy().replace('\\', "/");
                let file_header = format!(
                    "{} [file: {}, {} blocks]:\n",
                    path_display,
                    entry.file_short_hash,
                    entry.blocks.len()
                );

                if out.len() + file_header.len() > char_limit {
                    truncated = true;
                    break;
                }
                out.push_str(&file_header);

                if entry.symbols.is_empty() {
                    // Show block outline if no specific named symbols
                    for b in &entry.blocks {
                        let sig_short = if b.signature.len() > 60 {
                            format!("{}…", &b.signature[..57])
                        } else {
                            b.signature.clone()
                        };
                        let line = format!(
                            "  block {:04X} (lines {:>3}..{:<3}): {}\n",
                            (b.hash >> 48) as u16,
                            b.start_line,
                            b.end_line,
                            sig_short
                        );
                        if out.len() + line.len() > char_limit {
                            truncated = true;
                            break;
                        }
                        out.push_str(&line);
                    }
                } else {
                    for sym in &entry.symbols {
                        let line = format!(
                            "  {} {} [block: {}, lines: {}..{}]\n",
                            sym.kind.as_str(),
                            sym.name,
                            sym.block_short_hash,
                            sym.block_range.0,
                            sym.block_range.1
                        );
                        if out.len() + line.len() > char_limit {
                            truncated = true;
                            break;
                        }
                        out.push_str(&line);
                    }
                }

                if truncated {
                    break;
                }
                out.push('\n');
            }
        }

        if truncated {
            out.push_str(&format!(
                "\n[... repomap truncated to fit token budget of {token_budget} tokens ...]\n"
            ));
        }

        out
    }
}

fn normalize_rel_path(p: &Path) -> PathBuf {
    let s = p.to_string_lossy().replace('\\', "/");
    let trimmed = s.trim_start_matches("./");
    PathBuf::from(trimmed)
}

fn walk_dir_recursive(
    current: &Path,
    root: &Path,
    exclusions: &HashSet<String>,
    map: &mut RepoMap,
) {
    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if exclusions.contains(&file_name) {
            continue;
        }

        if path.is_dir() {
            if file_name.starts_with('.') && file_name != ".ai" {
                continue;
            }
            walk_dir_recursive(&path, root, exclusions, map);
        } else if path.is_file() && is_indexable_source(&path) {
            if let Ok(rel) = path.strip_prefix(root) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    map.add_file(rel, &content);
                }
            }
        }
    }
}

fn is_indexable_source(path: &Path) -> bool {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    matches!(
        ext,
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "toml" | "md"
    )
}

/// Extract structural symbols from source code and bind them to code blocks.
pub fn extract_symbols(file_path: &Path, content: &str, blocks: &[Block]) -> Vec<SymbolEntry> {
    let ext = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let lines: Vec<&str> = content.lines().collect();
    let mut symbols = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let line_num = idx + 1;
        let trimmed = line.trim();

        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with("/*")
        {
            continue;
        }

        let symbol_opt = match ext {
            "rs" => extract_rust_symbol(trimmed, line_num),
            "ts" | "tsx" | "js" | "jsx" => extract_js_ts_symbol(trimmed, line_num),
            "py" => extract_python_symbol(trimmed, line_num),
            "go" => extract_go_symbol(trimmed, line_num),
            _ => None,
        };

        if let Some((name, sig, kind)) = symbol_opt {
            // Find containing block
            let containing_block = blocks
                .iter()
                .find(|b| b.start_line <= line_num && line_num <= b.end_line)
                .or_else(|| blocks.first());

            if let Some(b) = containing_block {
                symbols.push(SymbolEntry {
                    name,
                    signature: sig,
                    kind,
                    file_path: file_path.to_path_buf(),
                    line: line_num,
                    block_hash: b.hash,
                    block_short_hash: short_hash(b.hash),
                    block_range: (b.start_line, b.end_line),
                });
            }
        }
    }

    symbols
}

fn extract_rust_symbol(line: &str, _line_num: usize) -> Option<(String, String, SymbolKind)> {
    // Strip common modifiers
    let mut s = line;
    for modifier in [
        "pub(crate)",
        "pub(super)",
        "pub",
        "async",
        "unsafe",
        "extern \"C\"",
        "const",
    ] {
        if let Some(rest) = s.strip_prefix(modifier) {
            s = rest.trim_start();
        }
    }

    if let Some(rest) = s.strip_prefix("fn ") {
        let name = extract_ident(rest)?;
        return Some((name, clean_sig(line), SymbolKind::Function));
    }
    if let Some(rest) = s.strip_prefix("struct ") {
        let name = extract_ident(rest)?;
        return Some((name, clean_sig(line), SymbolKind::Struct));
    }
    if let Some(rest) = s.strip_prefix("enum ") {
        let name = extract_ident(rest)?;
        return Some((name, clean_sig(line), SymbolKind::Enum));
    }
    if let Some(rest) = s.strip_prefix("trait ") {
        let name = extract_ident(rest)?;
        return Some((name, clean_sig(line), SymbolKind::Trait));
    }
    if let Some(rest) = s.strip_prefix("impl") {
        let sig = clean_sig(line);
        let name = rest
            .trim_start()
            .split('{')
            .next()
            .unwrap_or(rest)
            .trim()
            .to_string();
        return Some((name, sig, SymbolKind::Impl));
    }
    if let Some(rest) = s.strip_prefix("type ") {
        let name = extract_ident(rest)?;
        return Some((name, clean_sig(line), SymbolKind::Type));
    }
    if let Some(rest) = s.strip_prefix("mod ") {
        let name = extract_ident(rest)?;
        return Some((name, clean_sig(line), SymbolKind::Module));
    }

    None
}

fn extract_js_ts_symbol(line: &str, _line_num: usize) -> Option<(String, String, SymbolKind)> {
    let mut s = line;
    if let Some(rest) = s.strip_prefix("export ") {
        s = rest.trim_start();
    }
    if let Some(rest) = s.strip_prefix("default ") {
        s = rest.trim_start();
    }
    if let Some(rest) = s.strip_prefix("async ") {
        s = rest.trim_start();
    }

    if let Some(rest) = s.strip_prefix("function ") {
        let name = extract_ident(rest)?;
        return Some((name, clean_sig(line), SymbolKind::Function));
    }
    if let Some(rest) = s.strip_prefix("class ") {
        let name = extract_ident(rest)?;
        return Some((name, clean_sig(line), SymbolKind::Class));
    }
    if let Some(rest) = s.strip_prefix("interface ") {
        let name = extract_ident(rest)?;
        return Some((name, clean_sig(line), SymbolKind::Interface));
    }
    if let Some(rest) = s.strip_prefix("type ") {
        let name = extract_ident(rest)?;
        return Some((name, clean_sig(line), SymbolKind::Type));
    }

    None
}

fn extract_python_symbol(line: &str, _line_num: usize) -> Option<(String, String, SymbolKind)> {
    let mut s = line;
    if let Some(rest) = s.strip_prefix("async ") {
        s = rest.trim_start();
    }

    if let Some(rest) = s.strip_prefix("def ") {
        let name = extract_ident(rest)?;
        return Some((name, clean_sig(line), SymbolKind::Function));
    }
    if let Some(rest) = s.strip_prefix("class ") {
        let name = extract_ident(rest)?;
        return Some((name, clean_sig(line), SymbolKind::Class));
    }

    None
}

fn extract_go_symbol(line: &str, _line_num: usize) -> Option<(String, String, SymbolKind)> {
    if let Some(rest) = line.strip_prefix("func ") {
        let clean = rest.trim_start();
        if clean.starts_with('(') {
            // Method receiver: func (r *Type) MethodName(...)
            if let Some((_, after)) = clean.split_once(')') {
                let name = extract_ident(after.trim_start())?;
                return Some((name, clean_sig(line), SymbolKind::Method));
            }
        }
        let name = extract_ident(clean)?;
        return Some((name, clean_sig(line), SymbolKind::Function));
    }
    if let Some(rest) = line.strip_prefix("type ") {
        let clean = rest.trim_start();
        let name = extract_ident(clean)?;
        if clean.contains("struct") {
            return Some((name, clean_sig(line), SymbolKind::Struct));
        } else if clean.contains("interface") {
            return Some((name, clean_sig(line), SymbolKind::Interface));
        }
    }
    None
}

fn extract_ident(s: &str) -> Option<String> {
    let s = s.trim_start();
    let end = s
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(s.len());
    let ident = &s[..end];
    if ident.is_empty() {
        None
    } else {
        Some(ident.to_string())
    }
}

fn clean_sig(line: &str) -> String {
    let trimmed = line.trim();
    trimmed
        .strip_suffix('{')
        .unwrap_or(trimmed)
        .trim()
        .to_string()
}
