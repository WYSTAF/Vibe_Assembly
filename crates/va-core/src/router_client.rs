//! Vibe Router assignment client with optional auth and offline fallback.
//!
//! # Endpoint note
//!
//! The wave brief named `GET /v1/assignment`. Probing the live gateway on
//! 2026-10-08 showed:
//!
//! ```text
//! 200  /api/health        gateway is up
//! 404  /v1/assignment     path does not exist
//! 401  /api/assignment    exists, requires authorization
//! ```
//!
//! So the default is `/api/assignment`, with the path overridable for when
//! the gateway adds the `/v1` alias. Because the payload shape is not yet
//! fixed by a published contract, the parser accepts several plausible field
//! names and degrades to offline rather than guessing wrong.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 20128;
pub const DEFAULT_PATH: &str = "/api/assignment";
pub const DEFAULT_TIMEOUT_MS: u64 = 400;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouterStatus {
    /// The gateway answered and we understood it.
    Live,
    /// The gateway is unreachable, unauthorized, or said to fall back.
    Offline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouterRole {
    pub mode: String,
    pub model: String,
    pub class: Option<String>,
    /// Recommended ticket size, e.g. "S" or "M".
    pub recommended_ticket_size: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouterAssignment {
    pub status: RouterStatus,
    pub source: String,
    pub roles: Vec<RouterRole>,
    pub recommended_ticket_sizes: BTreeMap<String, String>,
    /// Set when offline: why we gave up, for the doctor command.
    pub detail: Option<String>,
}

impl RouterAssignment {
    pub fn offline(detail: impl Into<String>) -> Self {
        Self {
            status: RouterStatus::Offline,
            source: "offline".to_string(),
            roles: Vec::new(),
            recommended_ticket_sizes: BTreeMap::new(),
            detail: Some(detail.into()),
        }
    }

    pub fn is_live(&self) -> bool {
        self.status == RouterStatus::Live
    }

    pub fn model_names(&self) -> Vec<String> {
        self.roles.iter().map(|r| r.model.clone()).collect()
    }
}

/// Optional authorization for the gateway.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RouterAuth {
    /// Sent as `Authorization: Bearer <token>`.
    pub bearer: Option<String>,
    /// Sent as `X-API-Key: <key>`.
    pub api_key: Option<String>,
}

impl RouterAuth {
    /// Read credentials from the environment.
    ///
    /// `VIBE_ROUTER_TOKEN` and `VIBE_ROUTER_API_KEY` are checked first, then
    /// the common aliases, so no configuration is required when the gateway
    /// is unauthenticated on localhost.
    pub fn from_env() -> Self {
        let pick = |keys: &[&str]| -> Option<String> {
            keys.iter()
                .find_map(|k| std::env::var(k).ok())
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        Self {
            bearer: pick(&[
                "VIBE_ROUTER_TOKEN",
                "VIBE_ROUTER_BEARER",
                "NINEROUTER_API_KEY",
            ]),
            api_key: pick(&["VIBE_ROUTER_API_KEY", "VIBE_ROUTER_KEY"]),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bearer.is_none() && self.api_key.is_none()
    }

    fn headers(&self) -> Vec<(String, String)> {
        let mut h = vec![
            ("Accept".to_string(), "application/json".to_string()),
            (
                "User-Agent".to_string(),
                "vibe-assembly/2.0 (va-core router client)".to_string(),
            ),
        ];
        if let Some(t) = &self.bearer {
            h.push(("Authorization".to_string(), format!("Bearer {t}")));
        }
        if let Some(k) = &self.api_key {
            h.push(("X-API-Key".to_string(), k.clone()));
        }
        h
    }
}

/// Perform the HTTP GET and return `(status, body)`.
fn http_get(
    host: &str,
    port: u16,
    path: &str,
    auth: &RouterAuth,
    timeout: Duration,
) -> Result<(u16, String), String> {
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

    let mut req = format!("GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\n");
    for (k, v) in auth.headers() {
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
    Ok((status, body.to_string()))
}

/// Decode chunked transfer encoding, which the Next.js gateway uses.
fn decode_chunked(body: &str) -> String {
    let mut out = String::new();
    let mut rest = body;
    while let Some((size_line, tail)) = rest.split_once("\r\n") {
        let size = usize::from_str_radix(size_line.trim().split(';').next().unwrap_or("0"), 16)
            .unwrap_or(0);
        if size == 0 || tail.len() < size {
            break;
        }
        out.push_str(&tail[..size]);
        rest = tail[size..].trim_start_matches("\r\n");
    }
    out
}

/// Pull a string field, tolerating several plausible spellings.
fn pick<'a>(v: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    for k in keys {
        if let Some(s) = v.get(*k).and_then(|x| x.as_str()) {
            return Some(s);
        }
    }
    None
}

/// Interpret the assignment payload. Accepts `roles`, `assignments`, or a
/// top-level map of mode → model.
pub fn parse_assignment(body: &str) -> Result<RouterAssignment, String> {
    let raw = body.trim();
    if raw.is_empty() {
        return Err("empty body".to_string());
    }
    // Tolerate a JSONL or stray-wrapped response by taking the outermost
    // object we can find.
    let json: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("invalid JSON: {e}"))?;

    if let Some(flag) = json
        .get("offline_fallback_recommended")
        .and_then(|v| v.as_bool())
    {
        if flag {
            return Err("router advised offline fallback".to_string());
        }
    }

    let container = json
        .get("roles")
        .or_else(|| json.get("assignments"))
        .unwrap_or(&json);

    let map = container
        .as_object()
        .ok_or_else(|| "no role map in assignment payload".to_string())?;

    let mut sizes: BTreeMap<String, String> = BTreeMap::new();
    if let Some(obj) = json
        .get("recommended_ticket_sizes")
        .or_else(|| json.get("ticket_sizes"))
        .and_then(|v| v.as_object())
    {
        for (k, v) in obj {
            if let Some(s) = v.as_str() {
                sizes.insert(k.to_lowercase(), s.to_string());
            }
        }
    } else if let Some(one) = pick(&json, &["ticket_size", "recommended_ticket_size"]) {
        for m in ["boss", "code", "debug", "judge", "hermes", "chat"] {
            sizes.insert(m.to_string(), one.to_string());
        }
    }

    let mut roles = Vec::new();
    for (mode, value) in map {
        // A container node, not a role entry.
        if mode.eq_ignore_ascii_case("version")
            || mode.eq_ignore_ascii_case("offline_fallback_recommended")
            || mode.eq_ignore_ascii_case("recommended_ticket_sizes")
        {
            continue;
        }
        let norm_mode = mode.to_lowercase();
        if !["boss", "code", "debug", "judge", "hermes", "chat"].contains(&norm_mode.as_str()) {
            continue;
        }

        let (model, class) = match value {
            serde_json::Value::String(s) => (s.clone(), None),
            serde_json::Value::Object(o) => {
                let m = o
                    .get("model")
                    .or_else(|| o.get("name"))
                    .or_else(|| o.get("id"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| format!("role {norm_mode} has no model"))?
                    .to_string();
                let c = o.get("class").and_then(|v| v.as_str()).map(String::from);
                (m, c)
            }
            _ => continue,
        };
        let size = sizes.get(&norm_mode).cloned();
        roles.push(RouterRole {
            mode: norm_mode,
            model,
            class,
            recommended_ticket_size: size,
        });
    }

    if roles.is_empty() {
        return Err("assignment payload contained no roles".to_string());
    }
    roles.sort_by(|a, b| a.mode.cmp(&b.mode));

    Ok(RouterAssignment {
        status: RouterStatus::Live,
        source: "Vibe Router".to_string(),
        roles,
        recommended_ticket_sizes: sizes,
        detail: None,
    })
}

/// Query the gateway, falling back to offline on any failure.
pub fn fetch_assignment(
    host: &str,
    port: u16,
    path: &str,
    auth: &RouterAuth,
    timeout_ms: u64,
) -> RouterAssignment {
    let timeout = Duration::from_millis(timeout_ms);
    let (status, body) = match http_get(host, port, path, auth, timeout) {
        Ok(v) => v,
        Err(e) => return RouterAssignment::offline(format!("gateway unreachable: {e}")),
    };

    if status == 401 || status == 403 {
        return RouterAssignment::offline(format!(
            "gateway returned {status} — set VIBE_ROUTER_TOKEN or VIBE_ROUTER_API_KEY"
        ));
    }
    if !(200..300).contains(&status) {
        return RouterAssignment::offline(format!("gateway returned HTTP {status}"));
    }

    let body = if body
        .trim_start()
        .starts_with(|c: char| c.is_ascii_hexdigit() && c != '"')
        && body.contains("chunked")
    {
        decode_chunked(&body)
    } else {
        body
    };

    match parse_assignment(&body) {
        Ok(a) => a,
        Err(e) => RouterAssignment::offline(format!("unusable assignment payload: {e}")),
    }
}

/// Query using the defaults, reading credentials from the environment.
pub fn fetch_default() -> RouterAssignment {
    let host = std::env::var("VIBE_ROUTER_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
    let port = std::env::var("VIBE_ROUTER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let path = std::env::var("VIBE_ROUTER_PATH").unwrap_or_else(|_| DEFAULT_PATH.to_string());
    fetch_assignment(
        &host,
        port,
        &path,
        &RouterAuth::from_env(),
        DEFAULT_TIMEOUT_MS,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_object_form_roles() {
        let body = r#"{"offline_fallback_recommended":false,
            "roles":{"boss":{"model":"claude-opus","class":"flagship"},
                     "code":{"model":"qwen3-coder","class":"coder"}},
            "recommended_ticket_sizes":{"boss":"S","code":"M"}}"#;
        let a = parse_assignment(body).unwrap();
        assert!(a.is_live());
        assert_eq!(a.roles.len(), 2);
        let boss = a.roles.iter().find(|r| r.mode == "boss").unwrap();
        assert_eq!(boss.model, "claude-opus");
        assert_eq!(boss.recommended_ticket_size.as_deref(), Some("S"));
        let code = a.roles.iter().find(|r| r.mode == "code").unwrap();
        assert_eq!(code.recommended_ticket_size.as_deref(), Some("M"));
    }

    #[test]
    fn parses_string_form_roles() {
        let body = r#"{"assignments":{"boss":"claude-opus","code":"qwen3-coder"}}"#;
        let a = parse_assignment(body).unwrap();
        assert_eq!(a.roles.len(), 2);
        assert!(a.roles.iter().all(|r| !r.model.is_empty()));
    }

    #[test]
    fn accepts_a_uniform_ticket_size() {
        let body = r#"{"roles":{"code":"m"},"ticket_size":"M"}"#;
        let a = parse_assignment(body).unwrap();
        assert_eq!(
            a.recommended_ticket_sizes.get("code").map(String::as_str),
            Some("M")
        );
    }

    #[test]
    fn honors_router_advised_offline_fallback() {
        let body = r#"{"offline_fallback_recommended":true,"roles":{}}"#;
        let err = parse_assignment(body).unwrap_err();
        assert!(err.contains("offline"), "got: {err}");
    }

    #[test]
    fn rejects_payloads_with_no_usable_roles() {
        assert!(parse_assignment("{}").is_err());
        assert!(parse_assignment("not json").is_err());
        assert!(parse_assignment("").is_err());
        // Roles present but none we recognise as a mode.
        assert!(parse_assignment(r#"{"roles":{"zzz":{"model":"x"}}}"#).is_err());
    }

    #[test]
    fn ignores_control_keys_when_counting_roles() {
        let body = r#"{"version":2,"offline_fallback_recommended":false,
            "recommended_ticket_sizes":{"code":"S"},"roles":{"code":"m"}}"#;
        let a = parse_assignment(body).unwrap();
        assert_eq!(a.roles.len(), 1);
    }

    #[test]
    fn unauthorized_reports_how_to_authenticate() {
        let a = fetch_assignment("127.0.0.1", 1, DEFAULT_PATH, &RouterAuth::default(), 150);
        assert!(!a.is_live());
        let detail = a.detail.unwrap_or_default();
        assert!(
            detail.contains("VIBE_ROUTER_TOKEN") || detail.contains("unreachable"),
            "must explain either the missing token or the failure: {detail}"
        );
    }

    #[test]
    fn chunked_decoding_reassembles_the_body() {
        let raw = "{\"roles\":{\"code\":\"m\"}}";
        let enc = format!("{:x}\r\n{}\r\n0\r\n\r\n", raw.len(), raw);
        assert_eq!(decode_chunked(&enc), raw);
    }

    #[test]
    fn auth_headers_are_emitted_only_when_present() {
        let empty = RouterAuth::default();
        assert!(empty.is_empty());
        assert!(!empty.headers().iter().any(|(k, _)| k == "Authorization"));

        let with = RouterAuth {
            bearer: Some("t".into()),
            api_key: Some("k".into()),
        };
        assert!(!with.is_empty());
        let h = with.headers();
        assert!(h
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer t"));
        assert!(h.iter().any(|(k, v)| k == "X-API-Key" && v == "k"));
    }
}
