//! Built-in JSON-RPC 2.0 Model Context Protocol (MCP) server over stdio.
//!
//! Exposes Vibe Assembly's native tools to external agents (Hermes Agent, Claude Code, etc.):
//! * `vibe_assembly_list_blocks`: returns hash-anchored code blocks for a file.
//! * `vibe_assembly_patch_block`: applies deterministic hash-anchored edits with 1-hop recovery.
//! * `vibe_assembly_judge_diff`: evaluates a unified diff against anti-slop and minimalism rules.
//! * `vibe_assembly_session_dag`: queries DAG state, forks branches, or triggers rollback.
//! * `vibe_assembly_auto_combo`: returns optimal 6-role allocations from models or router.

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use va_core::knowledge::auto_combo;
use va_core::router_client::fetch_default;
use va_harness::diagnostics::{DiagnosticPinpointer, PinnedDiagnostic};
use va_harness::judge::{Judge, Verdict};
use va_harness::patcher::{apply_patch, segment_blocks, short_hash, PatchOp};
use va_harness::repomap::RepoMap;
use va_harness::review::parse_unified_diff;
use va_harness::session::Session;
use va_harness::tournament::{CandidatePatch, SpeculativeTournament};

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

/// Run the stdio MCP server loop.
pub fn run_mcp_server(workspace_root: Option<&Path>) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut handle = stdin.lock();
    let mut line = String::new();

    while handle.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            if let Some(resp) = handle_mcp_request(trimmed, workspace_root) {
                stdout.write_all(resp.as_bytes())?;
                stdout.write_all(b"\n")?;
                stdout.flush()?;
            }
        }
        line.clear();
    }
    Ok(())
}

/// Process a single JSON-RPC string, returning the serialized response (if not a notification).
pub fn handle_mcp_request(raw: &str, root: Option<&Path>) -> Option<String> {
    let req: JsonRpcRequest = match serde_json::from_str(raw) {
        Ok(r) => r,
        Err(e) => {
            let err_resp = json!({
                "jsonrpc": "2.0",
                "id": Value::Null,
                "error": {
                    "code": -32700,
                    "message": format!("Parse error: {e}")
                }
            });
            return Some(err_resp.to_string());
        }
    };

    let id = match req.id {
        Some(i) => i,
        None => {
            // Notification: no response required
            return None;
        }
    };

    let result = match req.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "vibe-assembly",
                "version": "2.0.0"
            }
        })),
        "notifications/initialized" => return None,
        "tools/list" => Ok(json!({
            "tools": [
                {
                    "name": "vibe_assembly_list_blocks",
                    "description": "List hash-anchored indentation-scoped code blocks for a file.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "file": {
                                "type": "string",
                                "description": "Path to the file to inspect"
                            }
                        },
                        "required": ["file"]
                    }
                },
                {
                    "name": "vibe_assembly_patch_block",
                    "description": "Apply a deterministic hash-anchored patch to a file block with 1-hop recovery.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "file": { "type": "string", "description": "Target file path" },
                            "target_hash": { "type": "string", "description": "Block hash in hex (e.g. '3A7F' or 64-bit)" },
                            "replacement": { "type": "string", "description": "Replacement source text" },
                            "expected_file_hash": { "type": "integer", "description": "Optional whole-file staleness hash" }
                        },
                        "required": ["file", "target_hash", "replacement"]
                    }
                },
                {
                    "name": "vibe_assembly_judge_diff",
                    "description": "Evaluate a unified diff against anti-slop and minimalism rules (blocks unwrap in production, comments, churn).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "diff": { "type": "string", "description": "Unified diff text" }
                        },
                        "required": ["diff"]
                    }
                },
                {
                    "name": "vibe_assembly_session_dag",
                    "description": "Inspect or mutate the transactional session trajectory DAG (status, fork, rollback, compact).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "action": { "type": "string", "enum": ["status", "fork", "rollback", "compact"] },
                            "node_id": { "type": "integer", "description": "Target node ID for fork or rollback" },
                            "reason": { "type": "string", "description": "Reason for fork" },
                            "from_id": { "type": "integer", "description": "Start node ID for compaction" },
                            "to_id": { "type": "integer", "description": "End node ID for compaction" },
                            "summary": { "type": "string", "description": "Summary payload for compaction" }
                        },
                        "required": ["action"]
                    }
                },
                {
                    "name": "vibe_assembly_auto_combo",
                    "description": "Compute optimal 6-role model allocation for Boss, Code, Debug, Judge, Chat, Hermes.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "models": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Optional list of model IDs; if omitted queries live router"
                            }
                        }
                    }
                },
                {
                    "name": "vibe_assembly_repomap",
                    "description": "Generate a compact, hash-anchored repository symbol map within a token budget.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "max_tokens": {
                                "type": "integer",
                                "description": "Maximum token budget for repomap (default 2048)"
                            },
                            "priority": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Priority file paths to render first"
                            }
                        }
                    }
                },
                {
                    "name": "vibe_assembly_diagnose_blocks",
                    "description": "Parse compiler errors (JSON or text) and pinpoint exact hash-anchored code blocks and snippets.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "command": {
                                "type": "string",
                                "description": "Optional compiler command (default: `cargo check --message-format=json`)"
                            },
                            "output": {
                                "type": "string",
                                "description": "Optional raw compiler output to pin directly"
                            }
                        }
                    }
                },
                {
                    "name": "vibe_assembly_tournament_step",
                    "description": "Run a speculative multi-branch tournament evaluating competing candidate patches and merge winner.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "candidates": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "id": { "type": "string" },
                                        "description": { "type": "string" },
                                        "patches": {
                                            "type": "array",
                                            "items": {
                                                "type": "object",
                                                "properties": {
                                                    "file": { "type": "string" },
                                                    "target_hash": { "type": "string" },
                                                    "replacement": { "type": "string" }
                                                },
                                                "required": ["file", "target_hash", "replacement"]
                                            }
                                        },
                                        "cost_micros": { "type": "integer" },
                                        "latency_ms": { "type": "integer" }
                                    },
                                    "required": ["id", "description", "patches"]
                                }
                            },
                            "verification_command": {
                                "type": "string",
                                "description": "Optional verification command (e.g. `cargo check`)"
                            }
                        },
                        "required": ["candidates"]
                    }
                }
            ]
        })),
        "tools/call" => {
            let params = req.params.unwrap_or(Value::Null);
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            execute_tool(tool_name, &args, root)
        }
        other => Err(json!({
            "code": -32601,
            "message": format!("Method not found: {other}")
        })),
    };

    let resp = match result {
        Ok(val) => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: Some(val),
            error: None,
        },
        Err(err) => JsonRpcResponse {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(err),
        },
    };

    serde_json::to_string(&resp).ok()
}

fn execute_tool(name: &str, args: &Value, root: Option<&Path>) -> Result<Value, Value> {
    match name {
        "vibe_assembly_list_blocks" => {
            let file_str = args.get("file").and_then(|v| v.as_str()).ok_or_else(
                || json!({"code": -32602, "message": "Missing required argument `file`"}),
            )?;
            let path = resolve_path(file_str, root);
            let content = std::fs::read_to_string(&path).map_err(|e| {
                json!({"code": -32000, "message": format!("Cannot read file {}: {e}", path.display())})
            })?;
            let blocks = segment_blocks(&content);
            let items: Vec<Value> = blocks
                .iter()
                .map(|b| {
                    json!({
                        "start_line": b.start_line,
                        "end_line": b.end_line,
                        "short_hash": short_hash(b.hash),
                        "full_hash": b.hash,
                        "signature": b.signature
                    })
                })
                .collect();

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&items).unwrap_or_default()
                }],
                "isError": false
            }))
        }
        "vibe_assembly_patch_block" => {
            let file_str = args.get("file").and_then(|v| v.as_str()).ok_or_else(
                || json!({"code": -32602, "message": "Missing required argument `file`"}),
            )?;
            let target_str = args.get("target_hash").and_then(|v| v.as_str()).ok_or_else(|| {
                json!({"code": -32602, "message": "Missing required argument `target_hash`"})
            })?;
            let replacement = args.get("replacement").and_then(|v| v.as_str()).ok_or_else(|| {
                json!({"code": -32602, "message": "Missing required argument `replacement`"})
            })?;
            let expected_file_hash = args.get("expected_file_hash").and_then(|v| v.as_u64());

            let path = resolve_path(file_str, root);
            let content = std::fs::read_to_string(&path).map_err(|e| {
                json!({"code": -32000, "message": format!("Cannot read file {}: {e}", path.display())})
            })?;

            let target_hash = parse_hash_val(target_str, &content).ok_or_else(
                || json!({"code": -32602, "message": format!("Invalid target hash: {target_str}")}),
            )?;

            let mut op = PatchOp::new(target_hash, replacement);
            if let Some(efh) = expected_file_hash {
                op = op.with_expected_file_hash(efh);
            }

            match apply_patch(&content, &op) {
                Ok(updated) => {
                    std::fs::write(&path, &updated).map_err(|e| {
                        json!({"code": -32000, "message": format!("Cannot write file {}: {e}", path.display())})
                    })?;
                    Ok(json!({
                        "content": [{
                            "type": "text",
                            "text": format!("Successfully patched {} ({} bytes)", path.display(), updated.len())
                        }],
                        "isError": false
                    }))
                }
                Err(e) => Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Patch rejected: {e}")
                    }],
                    "isError": true
                })),
            }
        }
        "vibe_assembly_judge_diff" => {
            let diff = args.get("diff").and_then(|v| v.as_str()).ok_or_else(
                || json!({"code": -32602, "message": "Missing required argument `diff`"}),
            )?;

            let (lines, numbers) = parse_unified_diff(diff);
            let judge = Judge::with_defaults();
            let verdict = judge.review(&lines, &numbers);

            let result_json = match verdict {
                Verdict::Pass => json!({
                    "verdict": "pass",
                    "findings": []
                }),
                Verdict::Reject(findings) => {
                    let feedback = Judge::feedback(&findings);
                    json!({
                        "verdict": "reject",
                        "feedback": feedback,
                        "findings": findings.iter().map(|f| format!("{f}")).collect::<Vec<_>>()
                    })
                }
            };

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&result_json).unwrap_or_default()
                }],
                "isError": false
            }))
        }
        "vibe_assembly_session_dag" => {
            let action = args
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("status");
            let session_path = root
                .map(|r| r.join(".ai").join("session.jsonl"))
                .unwrap_or_else(|| PathBuf::from(".ai/session.jsonl"));

            let mut session = if session_path.exists() {
                let raw = std::fs::read_to_string(&session_path).unwrap_or_default();
                Session::from_jsonl(&raw).unwrap_or_default()
            } else {
                Session::new()
            };

            match action {
                "status" => {
                    let path = session.current_path().unwrap_or_default();
                    let labels = session.kind_labels();
                    let cost = session.total_cost_micros();
                    let latency = session.total_latency_ms();

                    let status_val = json!({
                        "len": session.len(),
                        "head": session.head(),
                        "current_path": path,
                        "labels": labels,
                        "total_cost_micros": cost,
                        "total_latency_ms": latency
                    });

                    Ok(json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&status_val).unwrap_or_default()
                        }],
                        "isError": false
                    }))
                }
                "fork" => {
                    let node_id = args.get("node_id").and_then(|v| v.as_u64()).ok_or_else(
                        || json!({"code": -32602, "message": "Missing `node_id` for fork"}),
                    )?;
                    let reason = args
                        .get("reason")
                        .and_then(|v| v.as_str())
                        .unwrap_or("agent fork");
                    let fork_id = session.fork(node_id, reason).map_err(
                        |e| json!({"code": -32000, "message": format!("Fork error: {e}")}),
                    )?;
                    let _ = std::fs::write(&session_path, session.to_jsonl());

                    Ok(json!({
                        "content": [{
                            "type": "text",
                            "text": format!("Forked at node {node_id} -> new fork node {fork_id}")
                        }],
                        "isError": false
                    }))
                }
                "rollback" => {
                    let to_id = args.get("node_id").and_then(|v| v.as_u64());
                    if let Some(id) = to_id {
                        session.rollback(id).map_err(
                            |e| json!({"code": -32000, "message": format!("Rollback error: {e}")}),
                        )?;
                    } else {
                        session.rollback_to_last_verified().map_err(|e| {
                            json!({"code": -32000, "message": format!("Rollback to verified error: {e}")})
                        })?;
                    }
                    let _ = std::fs::write(&session_path, session.to_jsonl());

                    Ok(json!({
                        "content": [{
                            "type": "text",
                            "text": format!("Rolled back; new head is {:?}", session.head())
                        }],
                        "isError": false
                    }))
                }
                "compact" => {
                    let from_id = args.get("from_id").and_then(|v| v.as_u64()).ok_or_else(
                        || json!({"code": -32602, "message": "Missing `from_id` for compact"}),
                    )?;
                    let to_id = args.get("to_id").and_then(|v| v.as_u64()).ok_or_else(
                        || json!({"code": -32602, "message": "Missing `to_id` for compact"}),
                    )?;
                    let summary = args.get("summary").and_then(|v| v.as_str()).ok_or_else(
                        || json!({"code": -32602, "message": "Missing `summary` for compact"}),
                    )?;

                    let mut surface = session.context_surface().map_err(|e| {
                        json!({"code": -32000, "message": format!("Context surface error: {e}")})
                    })?;
                    surface.compact_range(from_id, to_id, summary).map_err(
                        |e| json!({"code": -32000, "message": format!("Compaction error: {e}")}),
                    )?;

                    Ok(json!({
                        "content": [{
                            "type": "text",
                            "text": format!("Compacted nodes {from_id}..{to_id}; surface now has {} items", surface.len())
                        }],
                        "isError": false
                    }))
                }
                other => Err(json!({
                    "code": -32602,
                    "message": format!("Unknown DAG action: {other}")
                })),
            }
        }
        "vibe_assembly_auto_combo" => {
            let models: Vec<String> =
                if let Some(arr) = args.get("models").and_then(|v| v.as_array()) {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                } else {
                    let gw = fetch_default();
                    if gw.is_live() {
                        gw.model_names()
                    } else {
                        vec![
                            "claude-opus".into(),
                            "qwen3-coder".into(),
                            "deepseek-r1".into(),
                            "gemini-2.0-flash".into(),
                        ]
                    }
                };

            let assignments = auto_combo(&models, 0.5);
            let mut out_map = BTreeMap::new();
            for a in assignments {
                out_map.insert(
                    a.role.as_str().to_string(),
                    json!({
                        "model": a.model,
                        "score": a.score
                    }),
                );
            }

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&out_map).unwrap_or_default()
                }],
                "isError": false
            }))
        }
        "vibe_assembly_repomap" => {
            let budget = args
                .get("max_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(2048) as usize;
            let priority_files: Vec<PathBuf> = args
                .get("priority")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(PathBuf::from))
                        .collect()
                })
                .unwrap_or_default();

            let r = root.unwrap_or_else(|| Path::new("."));
            let repomap = RepoMap::scan(r);
            let rendered = repomap.render_within_budget(budget, &priority_files);

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": rendered
                }],
                "isError": false
            }))
        }
        "vibe_assembly_diagnose_blocks" => {
            let r = root.unwrap_or_else(|| Path::new("."));
            let output_text = if let Some(raw) = args.get("output").and_then(|v| v.as_str()) {
                raw.to_string()
            } else {
                let cmd = args
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("cargo check --message-format=json");
                let (prog, arg) = if cfg!(windows) {
                    ("cmd", "/C")
                } else {
                    ("sh", "-c")
                };
                let out = std::process::Command::new(prog)
                    .arg(arg)
                    .arg(cmd)
                    .current_dir(r)
                    .output()
                    .map_err(|e| json!({"code": -32000, "message": format!("Command execution failed: {e}")}))?;
                format!(
                    "{}\n{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                )
            };

            let pinned = DiagnosticPinpointer::pin_from_output(r, &output_text);
            let feedback = PinnedDiagnostic::format_feedback(&pinned);

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": if feedback.is_empty() {
                        "No compiler diagnostics found on code blocks.".to_string()
                    } else {
                        feedback
                    }
                }],
                "isError": false
            }))
        }
        "vibe_assembly_tournament_step" => {
            let r = root.unwrap_or_else(|| Path::new("."));
            let candidates_val = args
                .get("candidates")
                .and_then(|v| v.as_array())
                .ok_or_else(
                    || json!({"code": -32602, "message": "Missing required argument `candidates`"}),
                )?;

            let mut candidates = Vec::new();
            for c_val in candidates_val {
                let id = c_val
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("cand")
                    .to_string();
                let description = c_val
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let patches_val = c_val.get("patches").and_then(|v| v.as_array()).ok_or_else(
                    || json!({"code": -32602, "message": "Candidate missing `patches` array"}),
                )?;

                let mut patches = Vec::new();
                for p_val in patches_val {
                    let file_str = p_val.get("file").and_then(|v| v.as_str()).ok_or_else(
                        || json!({"code": -32602, "message": "Patch missing `file`"}),
                    )?;
                    let target_str = p_val
                        .get("target_hash")
                        .and_then(|v| v.as_str())
                        .ok_or_else(
                            || json!({"code": -32602, "message": "Patch missing `target_hash`"}),
                        )?;
                    let replacement = p_val
                        .get("replacement")
                        .and_then(|v| v.as_str())
                        .ok_or_else(
                            || json!({"code": -32602, "message": "Patch missing `replacement`"}),
                        )?;

                    let file_path = PathBuf::from(file_str);
                    let full = r.join(&file_path);
                    let content = std::fs::read_to_string(&full).unwrap_or_default();
                    let target_hash = parse_hash_val(target_str, &content).unwrap_or(0);
                    let op = PatchOp::new(target_hash, replacement);
                    patches.push((file_path, op));
                }

                candidates.push(CandidatePatch {
                    id,
                    description,
                    patches,
                    cost_micros: c_val
                        .get("cost_micros")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                    latency_ms: c_val
                        .get("latency_ms")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                });
            }

            let session_path = r.join(".ai").join("session.jsonl");
            let mut session = if session_path.exists() {
                let raw = std::fs::read_to_string(&session_path).unwrap_or_default();
                Session::from_jsonl(&raw).unwrap_or_default()
            } else {
                Session::new()
            };

            let mut tournament = SpeculativeTournament::new(r);
            if let Some(cmd) = args.get("verification_command").and_then(|v| v.as_str()) {
                tournament = tournament.with_verification_command(cmd);
            }

            let outcome = tournament.run(&mut session, &candidates).map_err(
                |e| json!({"code": -32000, "message": format!("Tournament failed: {e}")}),
            )?;

            let _ = std::fs::write(&session_path, session.to_jsonl());

            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&outcome).unwrap_or_default()
                }],
                "isError": false
            }))
        }
        unknown => Err(json!({
            "code": -32601,
            "message": format!("Unknown tool: {unknown}")
        })),
    }
}

fn resolve_path(p: &str, root: Option<&Path>) -> PathBuf {
    let path = PathBuf::from(p);
    if path.is_absolute() {
        path
    } else if let Some(r) = root {
        r.join(path)
    } else {
        path
    }
}

fn parse_hash_val(text: &str, source: &str) -> Option<u64> {
    let t = text.trim().trim_start_matches('#');
    if t.len() <= 4 && !t.is_empty() {
        let blocks = segment_blocks(source);
        for b in blocks {
            if short_hash(b.hash).eq_ignore_ascii_case(t) {
                return Some(b.hash);
            }
        }
        return u64::from_str_radix(t, 16).ok().map(|h| h << 48);
    }
    u64::from_str_radix(t, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_initialize() {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        })
        .to_string();

        let resp_str = handle_mcp_request(&req, None).expect("must produce response");
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["serverInfo"]["name"], "vibe-assembly");
    }

    #[test]
    fn test_mcp_tools_list() {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        })
        .to_string();

        let resp_str = handle_mcp_request(&req, None).expect("must produce response");
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        let tools = resp["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"vibe_assembly_list_blocks"));
        assert!(names.contains(&"vibe_assembly_patch_block"));
        assert!(names.contains(&"vibe_assembly_judge_diff"));
        assert!(names.contains(&"vibe_assembly_session_dag"));
        assert!(names.contains(&"vibe_assembly_auto_combo"));
        assert!(names.contains(&"vibe_assembly_repomap"));
        assert!(names.contains(&"vibe_assembly_diagnose_blocks"));
        assert!(names.contains(&"vibe_assembly_tournament_step"));
    }

    #[test]
    fn test_mcp_judge_diff_call() {
        let clean_diff = "\
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,4 @@
 fn test() {
+    let x = 1;
 }
";
        let req = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "vibe_assembly_judge_diff",
                "arguments": {
                    "diff": clean_diff
                }
            }
        })
        .to_string();

        let resp_str = handle_mcp_request(&req, None).expect("must produce response");
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["result"]["isError"], false);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"verdict\": \"pass\""));
    }

    #[test]
    fn test_mcp_auto_combo_call() {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "vibe_assembly_auto_combo",
                "arguments": {
                    "models": ["claude-opus", "qwen3-coder", "deepseek-r1", "gemini-flash"]
                }
            }
        })
        .to_string();

        let resp_str = handle_mcp_request(&req, None).expect("must produce response");
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["result"]["isError"], false);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("boss"));
        assert!(text.contains("code"));
    }

    #[test]
    fn test_mcp_repomap_call() {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "vibe_assembly_repomap",
                "arguments": {
                    "max_tokens": 512
                }
            }
        })
        .to_string();

        let resp_str = handle_mcp_request(&req, None).expect("must produce response");
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["result"]["isError"], false);
    }

    #[test]
    fn test_mcp_diagnose_blocks_call() {
        let sample_output = "error[E0432]: unresolved import `foo`\n --> src/sample.rs:10:5\n";
        let req = json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "vibe_assembly_diagnose_blocks",
                "arguments": {
                    "output": sample_output
                }
            }
        })
        .to_string();

        let resp_str = handle_mcp_request(&req, None).expect("must produce response");
        let resp: Value = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp["result"]["isError"], false);
    }
}
