//! Compiler Diagnostic-to-Block Pinpointer (`va-harness::diagnostics`).
//!
//! Parses compiler diagnostic outputs (both structured `cargo --message-format=json`
//! and standard human terminal text), maps error line spans directly to the
//! containing code block from `va-harness::patcher`, and outputs structured
//! `PinnedDiagnostic` items for surgical agent retries.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::patcher::{segment_blocks, short_hash, Block};

/// A compiler diagnostic pinpointed to an exact hash-anchored code block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedDiagnostic {
    pub file_path: PathBuf,
    pub block_hash: u64,
    pub block_short_hash: String,
    pub line_range: (usize, usize),
    pub span_lines: (usize, usize),
    pub error_code: Option<String>,
    pub level: String,
    pub message: String,
    pub snippet: String,
}

impl PinnedDiagnostic {
    /// Format multiple pinned diagnostics into concise feedback for an agent prompt.
    pub fn format_feedback(diagnostics: &[PinnedDiagnostic]) -> String {
        if diagnostics.is_empty() {
            return String::new();
        }

        let mut out = String::new();
        out.push_str("Compiler Diagnostics (Pinpointed to Target Blocks):\n");

        for d in diagnostics {
            let code_str = match &d.error_code {
                Some(c) => format!("[{c}] "),
                None => String::new(),
            };

            let path_display = d.file_path.to_string_lossy().replace('\\', "/");
            out.push_str(&format!(
                "- {} {}{}: {}\n  --> Block: {} (lines {}..{})\n",
                d.level.to_uppercase(),
                code_str,
                path_display,
                d.message,
                d.block_short_hash,
                d.line_range.0,
                d.line_range.1,
            ));

            if !d.snippet.is_empty() {
                out.push_str("  Current Block Snippet:\n");
                out.push_str("  ----------------------------------------\n");
                for line in d.snippet.lines() {
                    out.push_str(&format!("  {line}\n"));
                }
                out.push_str("  ----------------------------------------\n");
            }
        }

        out
    }
}

/// Raw parsed diagnostic before block resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawDiagnostic {
    pub file_path: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
    pub error_code: Option<String>,
    pub level: String,
    pub message: String,
}

pub struct DiagnosticPinpointer;

impl DiagnosticPinpointer {
    /// Pin raw compiler output (JSON or text) to code blocks in the repository.
    pub fn pin_from_output(repo_root: &Path, output: &str) -> Vec<PinnedDiagnostic> {
        let raw = Self::parse_output(output);
        Self::pin_diagnostics(repo_root, &raw)
    }

    /// Parse compiler output (auto-detecting JSON lines or standard text).
    pub fn parse_output(output: &str) -> Vec<RawDiagnostic> {
        let mut results = Vec::new();
        let mut is_json = false;

        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    if val.get("reason").and_then(|r| r.as_str()) == Some("compiler-message") {
                        if let Some(msg) = val.get("message") {
                            is_json = true;
                            if let Some(diag) = parse_json_compiler_message(msg) {
                                results.push(diag);
                            }
                        }
                    }
                }
            }
        }

        if is_json && !results.is_empty() {
            return results;
        }

        // Fallback: parse standard text diagnostics
        parse_text_diagnostics(output)
    }

    /// Correlate raw diagnostics with file blocks on disk.
    pub fn pin_diagnostics(repo_root: &Path, raw_list: &[RawDiagnostic]) -> Vec<PinnedDiagnostic> {
        let mut pinned = Vec::new();

        for raw in raw_list {
            let full_path = if raw.file_path.is_absolute() {
                raw.file_path.clone()
            } else {
                repo_root.join(&raw.file_path)
            };

            let content = match std::fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let blocks = segment_blocks(&content);
            if blocks.is_empty() {
                continue;
            }

            // Find overlapping block(s)
            let matched_block = blocks
                .iter()
                .find(|b| {
                    // Check span overlap
                    b.start_line <= raw.line_end && raw.line_start <= b.end_line
                })
                .or_else(|| {
                    // Fallback to closest block
                    blocks.iter().min_by_key(|b| {
                        if raw.line_start < b.start_line {
                            b.start_line - raw.line_start
                        } else {
                            raw.line_start.saturating_sub(b.end_line)
                        }
                    })
                });

            if let Some(b) = matched_block {
                let snippet = extract_block_snippet(&content, b);
                let rel_path = raw
                    .file_path
                    .strip_prefix(repo_root)
                    .unwrap_or(&raw.file_path)
                    .to_path_buf();

                pinned.push(PinnedDiagnostic {
                    file_path: rel_path,
                    block_hash: b.hash,
                    block_short_hash: short_hash(b.hash),
                    line_range: (b.start_line, b.end_line),
                    span_lines: (raw.line_start, raw.line_end),
                    error_code: raw.error_code.clone(),
                    level: raw.level.clone(),
                    message: raw.message.clone(),
                    snippet,
                });
            }
        }

        pinned
    }
}

fn parse_json_compiler_message(msg: &serde_json::Value) -> Option<RawDiagnostic> {
    let level = msg
        .get("level")
        .and_then(|v| v.as_str())
        .unwrap_or("error")
        .to_string();
    let message = msg
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let error_code = msg
        .get("code")
        .and_then(|c| c.get("code"))
        .and_then(|c| c.as_str())
        .map(String::from);

    let spans = msg.get("spans").and_then(|v| v.as_array())?;
    let primary_span = spans
        .iter()
        .find(|s| {
            s.get("is_primary")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .or_else(|| spans.first())?;

    let file_name = primary_span.get("file_name").and_then(|v| v.as_str())?;
    let line_start = primary_span
        .get("line_start")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as usize;
    let line_end = primary_span
        .get("line_end")
        .and_then(|v| v.as_u64())
        .unwrap_or(line_start as u64) as usize;

    Some(RawDiagnostic {
        file_path: PathBuf::from(file_name),
        line_start,
        line_end,
        error_code,
        level,
        message,
    })
}

fn parse_text_diagnostics(output: &str) -> Vec<RawDiagnostic> {
    let mut diags = Vec::new();
    let lines: Vec<&str> = output.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        // Format A: error[E0432]: unresolved import ...
        //             --> src/lib.rs:10:5
        if line.contains("error[")
            || line.contains("error:")
            || line.contains("warning[")
            || line.contains("warning:")
        {
            let (level, code, msg) = parse_error_header(line);

            // Look ahead up to 5 lines for `--> file:line:col`
            let mut file_span: Option<(PathBuf, usize, usize)> = None;
            for j in 1..=5 {
                if i + j < lines.len() {
                    let next_line = lines[i + j].trim();
                    if let Some(rest) = next_line.strip_prefix("--> ") {
                        if let Some((p, l)) = parse_file_line_col(rest.trim()) {
                            file_span = Some((p, l, l));
                            break;
                        }
                    }
                }
            }

            if let Some((file_path, line_start, line_end)) = file_span {
                diags.push(RawDiagnostic {
                    file_path,
                    line_start,
                    line_end,
                    error_code: code,
                    level,
                    message: msg,
                });
            }
        } else if let Some((path, l, level, msg)) = parse_inline_file_line(line) {
            // Format B: path/to/file:10:5: error: message
            diags.push(RawDiagnostic {
                file_path: path,
                line_start: l,
                line_end: l,
                error_code: None,
                level,
                message: msg,
            });
        }

        i += 1;
    }

    diags
}

fn parse_error_header(line: &str) -> (String, Option<String>, String) {
    let level = if line.contains("warning") {
        "warning".to_string()
    } else {
        "error".to_string()
    };

    let mut code = None;
    if let Some(start) = line.find('[') {
        if let Some(end) = line[start..].find(']') {
            let inner = &line[start + 1..start + end];
            if inner.starts_with('E') || inner.starts_with("clippy::") {
                code = Some(inner.to_string());
            }
        }
    }

    let msg = if let Some((_, after)) = line.split_once(':') {
        after.trim().to_string()
    } else {
        line.trim().to_string()
    };

    (level, code, msg)
}

fn parse_file_line_col(s: &str) -> Option<(PathBuf, usize)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() >= 2 {
        let path = PathBuf::from(parts[0]);
        let line = parts[1].parse::<usize>().ok()?;
        Some((path, line))
    } else {
        None
    }
}

fn parse_inline_file_line(line: &str) -> Option<(PathBuf, usize, String, String)> {
    let parts: Vec<&str> = line.split(':').collect();
    if parts.len() >= 4 {
        let line_num = parts[1].trim().parse::<usize>().ok()?;
        let path = PathBuf::from(parts[0].trim());
        let level = parts[3].trim().to_lowercase();
        if level == "error" || level == "warning" {
            let msg = parts[4..].join(":").trim().to_string();
            return Some((path, line_num, level, msg));
        }
    }
    None
}

fn extract_block_snippet(content: &str, block: &Block) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if block.start_line == 0 || block.start_line > lines.len() {
        return String::new();
    }
    let start = block.start_line - 1;
    let end = block.end_line.min(lines.len());
    lines[start..end].join("\n")
}
