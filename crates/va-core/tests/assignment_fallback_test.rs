//! Regression tests for the assignment endpoint ladder: the current Vibe
//! Router guards `/api/assignment` with an admin key but serves the same
//! payload publicly at `/v1/assignment`. Both clients must reach it without
//! credentials instead of declaring the gateway offline.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use va_core::router::fetch_live_assignment;
use va_core::router_client::{fetch_assignment, RouterAuth};

/// Mock gateway: `/api/assignment` → 401, `/v1/assignment` → 200 roles.
fn spawn_guarded_gateway() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for _ in 0..4 {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let mut buf = Vec::new();
            let mut byte = [0u8; 1];
            while let Ok(1) = stream.read(&mut byte) {
                buf.push(byte[0]);
                if buf.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            let request = String::from_utf8_lossy(&buf).to_string();
            let response = if request.starts_with("GET /v1/assignment") {
                let body = r#"{"offline_fallback_recommended":false,"roles":{"boss":"claude-opus","code":"qwen3-coder"}}"#;
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
            } else {
                "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_string()
            };
            let _ = stream.write_all(response.as_bytes());
        }
    });
    port
}

#[test]
fn gateway_client_falls_back_to_public_assignment_on_401() {
    let port = spawn_guarded_gateway();
    let a = fetch_assignment(
        "127.0.0.1",
        port,
        "/api/assignment",
        &RouterAuth::default(),
        2_000,
    );
    assert!(
        a.is_live(),
        "public /v1/assignment must rescue the 401: {a:?}"
    );
    assert_eq!(a.model_names().len(), 2);
}

#[test]
fn legacy_client_falls_back_to_public_assignment() {
    let port = spawn_guarded_gateway();
    let a = fetch_live_assignment(
        "127.0.0.1",
        port,
        "/api/assignment",
        Duration::from_millis(2_000),
    );
    let a = a.expect("legacy client must reach the public assignment");
    assert!(a.is_live);
    assert!(a.roles.iter().any(|r| r.mode == "code"));
}

#[test]
fn default_path_is_the_public_assignment() {
    assert_eq!(va_core::router_client::DEFAULT_PATH, "/v1/assignment");
    assert_eq!(va_core::router::DEFAULT_ROUTER_PATH, "/v1/assignment");
    let port = spawn_guarded_gateway();
    let a = fetch_assignment(
        "127.0.0.1",
        port,
        va_core::router_client::DEFAULT_PATH,
        &RouterAuth::default(),
        2_000,
    );
    assert!(a.is_live(), "zero-config default must work without a key");
}
