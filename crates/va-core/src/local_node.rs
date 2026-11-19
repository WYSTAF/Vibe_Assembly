//! Vibe Local inference node client: HTTP probe, LAN UDP discovery, and
//! zero-cost local telemetry.
//!
//! Vibe Local serves an OpenAI-compatible API on `127.0.0.1:20129` and
//! announces itself (and LAN Pocket Workers) over UDP on port `20131`.
//! Everything here is dependency-free `std::net` so the workspace stays
//! offline-buildable.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{IpAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

pub const DEFAULT_LOCAL_HOST: &str = "127.0.0.1";
pub const DEFAULT_LOCAL_PORT: u16 = 20129;
pub const DEFAULT_DISCOVERY_PORT: u16 = 20131;
/// Datagram payload workers answer to; anything else is ignored.
pub const DISCOVERY_MAGIC: &str = "VIBE_DISCOVER_V1";
pub const DEFAULT_PROBE_TIMEOUT_MS: u64 = 400;

/// A reachable Vibe Local node or an announced LAN Pocket Worker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalNodeStatus {
    pub host: String,
    pub port: u16,
    /// Base URL, e.g. `http://127.0.0.1:20129`.
    pub endpoint: String,
    pub reachable: bool,
    pub models: Vec<String>,
    /// `vulkan`, `cuda`, or `cpu` when the node reports one.
    pub backend: Option<String>,
    pub device: Option<String>,
    /// True when the node was discovered over LAN UDP rather than probed
    /// directly on this machine.
    pub lan: bool,
}

/// Resolve the local node endpoint from the environment.
pub fn local_endpoint() -> (String, u16) {
    let host = std::env::var("VIBE_LOCAL_HOST").unwrap_or_else(|_| DEFAULT_LOCAL_HOST.to_string());
    let port = std::env::var("VIBE_LOCAL_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_LOCAL_PORT);
    (host, port)
}

/// Resolve the LAN discovery port from the environment.
pub fn discovery_port() -> u16 {
    std::env::var("VIBE_LOCAL_DISCOVERY_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_DISCOVERY_PORT)
}

/// Optional shared token for the local node, never logged.
pub fn local_token() -> Option<String> {
    std::env::var("VIBE_LOCAL_TOKEN")
        .ok()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

/// Auth headers for local requests: the token travels both as
/// `X-Vibe-Local-Token` and as `Authorization: Bearer`.
pub fn local_auth_headers() -> Vec<(String, String)> {
    match local_token() {
        Some(token) => vec![
            ("X-Vibe-Local-Token".to_string(), token.clone()),
            ("Authorization".to_string(), format!("Bearer {token}")),
        ],
        None => Vec::new(),
    }
}

/// Normalize a reported backend; unknown values fail closed to `None`.
pub fn normalize_backend(raw: &str) -> Option<String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "vulkan" => Some("vulkan".to_string()),
        "cuda" => Some("cuda".to_string()),
        "cpu" => Some("cpu".to_string()),
        _ => None,
    }
}

/// Probe `GET /v1/models` on a Vibe Local node over raw HTTP/1.1.
///
/// Returns `None` when the node is unreachable, answers non-2xx, or returns
/// a body that is not a valid OpenAI-style model catalogue.
pub fn probe_vibe_local(host: &str, port: u16, timeout_ms: u64) -> Option<LocalNodeStatus> {
    let (status, headers, body) =
        http_get_local(host, port, "/v1/models", Duration::from_millis(timeout_ms)).ok()?;
    if !(200..300).contains(&status) {
        return None;
    }
    let json: serde_json::Value = serde_json::from_str(body.trim()).ok()?;
    if json.get("data").and_then(|v| v.as_array()).is_none()
        && json.get("models").and_then(|v| v.as_array()).is_none()
    {
        return None;
    }
    // A live node with no models loaded is still reachable.
    let models = crate::router_client::parse_v1_models(&body).unwrap_or_default();
    Some(LocalNodeStatus {
        host: host.to_string(),
        port,
        endpoint: format!("http://{host}:{port}"),
        reachable: true,
        models,
        backend: headers
            .get("x-vibe-local-backend")
            .and_then(|b| normalize_backend(b)),
        device: headers
            .get("x-vibe-local-device")
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty()),
        lan: false,
    })
}

/// Broadcast a `VIBE_DISCOVER_V1` UDP probe and collect worker announcements.
///
/// Sends to the limited broadcast address plus loopback, then listens until
/// the receive window closes. No replies is a normal outcome (empty list).
pub fn discover_lan_workers(discovery_port: u16, timeout_ms: u64) -> Vec<LocalNodeStatus> {
    let Ok(socket) = UdpSocket::bind("0.0.0.0:0") else {
        return Vec::new();
    };
    let _ = socket.set_broadcast(true);
    let window = Duration::from_millis(timeout_ms.clamp(50, 2_000));
    let deadline = Instant::now() + window;
    for target in [
        format!("255.255.255.255:{discovery_port}"),
        format!("127.0.0.1:{discovery_port}"),
    ] {
        let _ = socket.send_to(DISCOVERY_MAGIC.as_bytes(), &target);
    }

    let mut workers: Vec<LocalNodeStatus> = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        if socket.set_read_timeout(Some(remaining)).is_err() {
            break;
        }
        match socket.recv_from(&mut buf) {
            Ok((len, from)) => {
                let Ok(text) = std::str::from_utf8(&buf[..len]) else {
                    continue;
                };
                if let Some(node) = parse_worker_announcement(text, Some(from.ip())) {
                    if !workers
                        .iter()
                        .any(|w| w.host == node.host && w.port == node.port)
                    {
                        workers.push(node);
                    }
                }
            }
            // Windows surfaces ICMP port-unreachable from the broadcast as a
            // reset on the next receive; keep listening until the window ends.
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => continue,
            Err(_) => break,
        }
    }
    // Vibe Local announcements carry no model list; fill models/backend/device
    // from the node's own catalogue so discovered workers are actually usable.
    for worker in &mut workers {
        match probe_vibe_local(&worker.host, worker.port, 300) {
            Some(probed) => {
                if worker.models.is_empty() {
                    worker.models = probed.models;
                }
                worker.backend = worker.backend.take().or(probed.backend);
                worker.device = worker.device.take().or(probed.device);
                worker.reachable = true;
            }
            None => worker.reachable = false,
        }
    }
    workers
}

/// Parse one worker announcement: a JSON object or `key=value` pairs
/// separated by newlines, `;`, or `&`. Fails closed without a usable
/// host/port pair. Vibe Local announces `http_port` and its hostname; the
/// UDP sender IP wins over the announced host because it is the proven
/// reachable address (hostnames may not resolve across machines).
pub fn parse_worker_announcement(
    payload: &str,
    from_ip: Option<IpAddr>,
) -> Option<LocalNodeStatus> {
    let raw = payload.trim();
    if raw.is_empty() {
        return None;
    }

    let mut fields: BTreeMap<String, String> = BTreeMap::new();
    let mut models: Vec<String> = Vec::new();
    if let Ok(serde_json::Value::Object(obj)) = serde_json::from_str::<serde_json::Value>(raw) {
        for (key, value) in obj {
            match key.as_str() {
                "models" => match value {
                    serde_json::Value::Array(list) => {
                        for item in list {
                            if let Some(id) = item.as_str() {
                                models.push(id.to_string());
                            }
                        }
                    }
                    serde_json::Value::String(s) => {
                        models.extend(s.split(',').map(|m| m.trim().to_string()));
                    }
                    _ => {}
                },
                _ => {
                    if let Some(s) = value.as_str() {
                        fields.insert(key.to_ascii_lowercase(), s.to_string());
                    } else if let Some(n) = value.as_u64() {
                        fields.insert(key.to_ascii_lowercase(), n.to_string());
                    }
                }
            }
        }
    } else {
        for pair in raw.split(['\n', ';', '&']) {
            let Some((key, value)) = pair.split_once('=') else {
                continue;
            };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            if key == "models" {
                models.extend(value.split(',').map(|m| m.trim().to_string()));
            } else {
                fields.insert(key, value);
            }
        }
    }

    let port: u16 = fields
        .get("port")
        .or_else(|| fields.get("http_port"))
        .and_then(|p| p.parse().ok())?;
    if port == 0 {
        return None;
    }
    let host = from_ip
        .map(|ip| ip.to_string())
        .or_else(|| fields.get("host").filter(|h| !h.trim().is_empty()).cloned())?;
    models.retain(|m| !m.is_empty());
    models.dedup();

    Some(LocalNodeStatus {
        host: host.clone(),
        port,
        endpoint: format!("http://{host}:{port}"),
        reachable: true,
        models,
        backend: fields.get("backend").and_then(|b| normalize_backend(b)),
        device: fields
            .get("device")
            .or_else(|| fields.get("device_name"))
            .filter(|d| !d.is_empty())
            .cloned(),
        lan: true,
    })
}

/// Minimal raw HTTP/1.1 GET with the local-node auth headers.
fn http_get_local(
    host: &str,
    port: u16,
    path: &str,
    timeout: Duration,
) -> Result<(u16, BTreeMap<String, String>, String), String> {
    let addr = format!("{host}:{port}")
        .to_socket_addrs()
        .map_err(|e| format!("resolve {host}:{port}: {e}"))?
        .next()
        .ok_or_else(|| format!("no address for {host}:{port}"))?;
    let mut stream =
        TcpStream::connect_timeout(&addr, timeout).map_err(|e| format!("connect: {e}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|e| e.to_string())?;

    let mut req =
        format!("GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nAccept: application/json\r\n");
    for (k, v) in local_auth_headers() {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str("Connection: close\r\n\r\n");
    stream
        .write_all(req.as_bytes())
        .map_err(|e| format!("write: {e}"))?;

    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .map_err(|e| format!("read: {e}"))?;
    let text = String::from_utf8_lossy(&buf).to_string();
    let (head, body) = text
        .split_once("\r\n\r\n")
        .ok_or_else(|| "malformed HTTP response".to_string())?;
    let status = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse::<u16>().ok())
        .ok_or_else(|| "no status code".to_string())?;
    let mut headers = BTreeMap::new();
    for line in head.lines().skip(1) {
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }
    Ok((status, headers, body.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_worker_announcements() {
        let payload = r#"{"host":"192.168.1.40","port":20129,"backend":"Vulkan",
            "device":"RTX 3060","models":["qwen3-8b","llama-3.1-8b"]}"#;
        let node = parse_worker_announcement(payload, None).unwrap();
        assert_eq!(node.host, "192.168.1.40");
        assert_eq!(node.port, 20129);
        assert_eq!(node.endpoint, "http://192.168.1.40:20129");
        assert_eq!(node.backend.as_deref(), Some("vulkan"));
        assert_eq!(node.device.as_deref(), Some("RTX 3060"));
        assert_eq!(node.models, vec!["qwen3-8b", "llama-3.1-8b"]);
        assert!(node.lan && node.reachable);
    }

    #[test]
    fn parses_key_value_announcements_and_sender_ip_fallback() {
        let payload = "port=20129\nbackend=cuda\nmodels=qwen3-8b, phi-4\n";
        let ip: IpAddr = "10.0.0.7".parse().unwrap();
        let node = parse_worker_announcement(payload, Some(ip)).unwrap();
        assert_eq!(node.host, "10.0.0.7");
        assert_eq!(node.backend.as_deref(), Some("cuda"));
        assert_eq!(node.models, vec!["qwen3-8b", "phi-4"]);
    }

    #[test]
    fn parses_the_real_vibe_local_announcement_shape() {
        // Vibe Local replies with http_port + hostname and no model list.
        let payload = r#"{"service":"vibe-local","version":"0.1.0","http_port":20129,"lan":true,"host":"DESKTOP-ABC123"}"#;
        let ip: IpAddr = "192.168.1.40".parse().unwrap();
        let node = parse_worker_announcement(payload, Some(ip)).unwrap();
        assert_eq!(node.port, 20129);
        assert_eq!(
            node.host, "192.168.1.40",
            "sender IP beats unresolvable hostname"
        );
        assert_eq!(node.endpoint, "http://192.168.1.40:20129");
        assert!(
            node.models.is_empty(),
            "models arrive via the HTTP enrichment probe"
        );
        assert!(node.lan);
    }

    #[test]
    fn announcements_fail_closed_without_host_or_port() {
        assert!(parse_worker_announcement("backend=cpu", None).is_none());
        assert!(parse_worker_announcement("host=1.2.3.4", None).is_none());
        assert!(parse_worker_announcement("host=1.2.3.4;port=0", None).is_none());
        assert!(parse_worker_announcement("host=1.2.3.4;port=abc", None).is_none());
        assert!(parse_worker_announcement("", None).is_none());
        assert!(parse_worker_announcement("not an announcement", None).is_none());
    }

    #[test]
    fn unknown_backends_fail_closed_to_none() {
        assert_eq!(normalize_backend("VULKAN").as_deref(), Some("vulkan"));
        assert_eq!(normalize_backend(" cuda ").as_deref(), Some("cuda"));
        assert_eq!(normalize_backend("CPU").as_deref(), Some("cpu"));
        assert_eq!(normalize_backend("metal"), None);
        assert_eq!(normalize_backend(""), None);
        let payload = r#"{"host":"h","port":20129,"backend":"metal"}"#;
        let node = parse_worker_announcement(payload, None).unwrap();
        assert_eq!(node.backend, None);
    }

    #[test]
    fn discovery_without_listeners_returns_empty() {
        // Port 1 (tcpmux) has no Vibe worker; the window must close cleanly.
        let workers = discover_lan_workers(1, 120);
        assert!(workers.is_empty());
    }

    #[test]
    fn probe_of_a_closed_port_is_none() {
        assert!(probe_vibe_local("127.0.0.1", 1, 150).is_none());
    }
}
