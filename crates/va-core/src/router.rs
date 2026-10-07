//! Vibe Router assignment client with offline fallback.
//! Queries http://127.0.0.1:20128/v1/assignment or falls back to
//! .ai/model_knowledge.json and .ai/model_combo.txt.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

pub const DEFAULT_ROUTER_HOST: &str = "127.0.0.1";
pub const DEFAULT_ROUTER_PORT: u16 = 20128;
pub const DEFAULT_ROUTER_PATH: &str = "/v1/assignment";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleAllocation {
    pub mode: String,
    pub model: String,
    pub class: String,
    pub recommended_ticket_size: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouterAssignment {
    pub is_live: bool,
    pub source: String,
    pub roles: Vec<RoleAllocation>,
    pub recommended_ticket_sizes: BTreeMap<String, String>,
}

impl Default for RouterAssignment {
    fn default() -> Self {
        Self {
            is_live: false,
            source: "offline (default)".to_string(),
            roles: Vec::new(),
            recommended_ticket_sizes: BTreeMap::new(),
        }
    }
}

/// Attempt to fetch dynamic role allocations from Vibe Router via HTTP 1.1 GET.
/// Uses standard library TcpStream for zero extra dependencies and fast timeout.
pub fn fetch_live_assignment(
    host: &str,
    port: u16,
    path: &str,
    timeout: Duration,
) -> Option<RouterAssignment> {
    let addrs: Vec<SocketAddr> = format!("{}:{}", host, port)
        .to_socket_addrs()
        .ok()?
        .collect();
    if addrs.is_empty() {
        return None;
    }

    let mut stream = TcpStream::connect_timeout(&addrs[0], timeout).ok()?;
    stream.set_read_timeout(Some(timeout)).ok()?;
    stream.set_write_timeout(Some(timeout)).ok()?;

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}:{}\r\nUser-Agent: vibe-assembly-control/1.2.0\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
        path, host, port
    );

    stream.write_all(request.as_bytes()).ok()?;

    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response);

    let text = String::from_utf8_lossy(&response);
    let (header_part, body_part) = text.split_once("\r\n\r\n")?;

    // Check HTTP status code
    let first_line = header_part.lines().next()?;
    if !first_line.contains("200") {
        return None;
    }

    let val: serde_json::Value = serde_json::from_str(body_part.trim()).ok()?;

    // If router advises offline fallback, honor it immediately
    if val
        .get("offline_fallback_recommended")
        .and_then(|v| v.as_bool())
        == Some(true)
    {
        return None;
    }

    let raw_roles = val.get("roles").or_else(|| val.get("assignments"))?;
    let raw_roles_obj = raw_roles.as_object()?;
    if raw_roles_obj.is_empty() {
        return None;
    }

    let raw_sizes = val
        .get("recommended_ticket_sizes")
        .or_else(|| val.get("ticket_sizes"));
    let mut sizes = BTreeMap::new();
    if let Some(s_obj) = raw_sizes.and_then(|v| v.as_object()) {
        for (k, v) in s_obj {
            if let Some(s) = v.as_str() {
                sizes.insert(k.to_lowercase(), s.to_string());
            }
        }
    } else if let Some(s_str) = raw_sizes.and_then(|v| v.as_str()) {
        for m in &["boss", "code", "debug", "chat", "hermes"] {
            sizes.insert((*m).to_string(), s_str.to_string());
        }
    }

    let mut roles = Vec::new();
    for (mode, r_val) in raw_roles_obj {
        let (model, class) = if let Some(s) = r_val.as_str() {
            (s.to_string(), "router".to_string())
        } else if let Some(obj) = r_val.as_object() {
            let m = obj
                .get("model")
                .or_else(|| obj.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let c = obj
                .get("class")
                .and_then(|v| v.as_str())
                .unwrap_or("router")
                .to_string();
            (m, c)
        } else {
            continue;
        };

        let mode_norm = mode.to_lowercase();
        let size = sizes
            .get(&mode_norm)
            .cloned()
            .unwrap_or_else(|| "S".to_string());

        roles.push(RoleAllocation {
            mode: mode_norm,
            model,
            class,
            recommended_ticket_size: size,
        });
    }

    if roles.is_empty() {
        return None;
    }

    Some(RouterAssignment {
        is_live: true,
        source: format!("Vibe Router ({}:{})", host, port),
        roles,
        recommended_ticket_sizes: sizes,
    })
}

/// Resolve role allocations, first querying Vibe Router, and gracefully falling
/// back to .ai/model_knowledge.json and .ai/model_combo.txt on failure.
pub fn resolve_assignment(root: &Path) -> RouterAssignment {
    let (host, port, path) = parse_env_router_endpoint();

    // Fast probe with 200ms timeout so offline workspaces don't lag
    if let Some(live) = fetch_live_assignment(&host, port, &path, Duration::from_millis(200)) {
        return live;
    }

    // Offline fallback using local knowledge base and combo
    let ai_dir = root.join(".ai");
    let kb_path = ai_dir.join("model_knowledge.json");
    let combo_path = ai_dir.join("model_combo.txt");

    let mut combo = Vec::new();
    let mut locks: BTreeMap<String, String> = BTreeMap::new();

    if let Ok(combo_text) = fs::read_to_string(&combo_path) {
        for line in combo_text.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            let clean = if let Some(h) = t.find('#') {
                t[..h].trim()
            } else {
                t
            };
            if clean.is_empty() {
                continue;
            }
            if let Some((mode, model)) = clean.split_once('=') {
                locks.insert(mode.trim().to_lowercase(), model.trim().to_string());
            } else {
                combo.push(clean.to_string());
            }
        }
    }

    let mut default_sizes = BTreeMap::new();
    for m in &["boss", "code", "debug", "chat", "hermes"] {
        default_sizes.insert((*m).to_string(), "S".to_string());
    }

    // Try parsing model_knowledge.json
    if let Ok(kb_raw) = fs::read_to_string(&kb_path) {
        if let Ok(kb) = serde_json::from_str::<serde_json::Value>(&kb_raw) {
            let mut roles = Vec::new();
            if let Some(reqs) = kb.get("mode_requirements").and_then(|v| v.as_object()) {
                for (mode, req) in reqs {
                    let mode_norm = mode.to_lowercase();
                    let primary_class = req
                        .get("primary_class")
                        .and_then(|v| v.as_str())
                        .unwrap_or("workhorse");

                    // Model name: if locked, use locked; else pick from combo or default
                    let model = if let Some(l) = locks.get(&mode_norm) {
                        l.clone()
                    } else if let Some(first) = combo.first() {
                        first.clone()
                    } else {
                        format!("default-{}-model", primary_class)
                    };

                    roles.push(RoleAllocation {
                        mode: mode_norm,
                        model,
                        class: primary_class.to_string(),
                        recommended_ticket_size: "S".to_string(),
                    });
                }
            }

            if !roles.is_empty() {
                return RouterAssignment {
                    is_live: false,
                    source: "offline (.ai/model_knowledge.json)".to_string(),
                    roles,
                    recommended_ticket_sizes: default_sizes,
                };
            }
        }
    }

    // Minimal baseline fallback when files are absent
    let modes = [
        ("boss", "flagship"),
        ("chat", "fast"),
        ("code", "coder"),
        ("debug", "reasoner"),
        ("hermes", "workhorse"),
    ];

    let roles = modes
        .iter()
        .map(|(m, c)| RoleAllocation {
            mode: (*m).to_string(),
            model: locks
                .get(*m)
                .cloned()
                .unwrap_or_else(|| format!("fallback-{}", c)),
            class: (*c).to_string(),
            recommended_ticket_size: "S".to_string(),
        })
        .collect();

    RouterAssignment {
        is_live: false,
        source: "offline (default)".to_string(),
        roles,
        recommended_ticket_sizes: default_sizes,
    }
}

fn parse_env_router_endpoint() -> (String, u16, String) {
    if let Ok(url) = std::env::var("VIBE_ROUTER_URL") {
        if let Some(after) = url.strip_prefix("http://") {
            let (host_port, path) = after.split_once('/').unwrap_or((after, "v1/assignment"));
            let path = format!("/{}", path);
            if let Some((h, p)) = host_port.split_once(':') {
                if let Ok(port) = p.parse::<u16>() {
                    return (h.to_string(), port, path);
                }
            } else {
                return (host_port.to_string(), 80, path);
            }
        }
    }
    (
        DEFAULT_ROUTER_HOST.to_string(),
        DEFAULT_ROUTER_PORT,
        DEFAULT_ROUTER_PATH.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_offline_fallback_when_unreachable() {
        let dir = std::env::temp_dir().join("va-router-offline-test");
        let ai = dir.join(".ai");
        let _ = fs::create_dir_all(&ai);

        let res = resolve_assignment(&dir);
        assert!(!res.is_live);
        assert!(res.source.contains("offline"));
        assert!(!res.roles.is_empty());
        assert!(res.roles.iter().any(|r| r.mode == "code"));

        let _ = fs::remove_dir_all(&dir);
    }
}
