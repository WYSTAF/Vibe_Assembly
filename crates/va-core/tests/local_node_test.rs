//! Live-socket tests for the Vibe Local probe and LAN UDP discovery against
//! local mock servers (no real node required).

use std::io::{Read, Write};
use std::net::{TcpListener, UdpSocket};
use std::thread;

use va_core::local_node::{discover_lan_workers, probe_vibe_local, DISCOVERY_MAGIC};

fn spawn_models_server(response: &'static str) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = Vec::new();
            let mut byte = [0u8; 1];
            while let Ok(1) = stream.read(&mut byte) {
                buf.push(byte[0]);
                if buf.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            let request = String::from_utf8_lossy(&buf).to_string();
            assert!(request.starts_with("GET /v1/models HTTP/1.1"));
            let _ = stream.write_all(response.as_bytes());
        }
    });
    port
}

#[test]
fn probe_reads_models_backend_and_endpoint_from_a_live_node() {
    let port = spawn_models_server(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: application/json\r\n\
         X-Vibe-Local-Backend: vulkan\r\n\
         X-Vibe-Local-Device: RTX 3060\r\n\
         Connection: close\r\n\r\n\
         {\"data\":[{\"id\":\"qwen3-8b\"},{\"id\":\"llama-3.1-8b\"}]}",
    );
    let node = probe_vibe_local("127.0.0.1", port, 2_000).expect("node must be reachable");
    assert!(node.reachable);
    assert!(!node.lan);
    assert_eq!(node.endpoint, format!("http://127.0.0.1:{port}"));
    assert_eq!(node.models, vec!["qwen3-8b", "llama-3.1-8b"]);
    assert_eq!(node.backend.as_deref(), Some("vulkan"));
    assert_eq!(node.device.as_deref(), Some("RTX 3060"));
}

#[test]
fn probe_fails_closed_on_http_errors_and_garbage_bodies() {
    let port = spawn_models_server("HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\nnope");
    assert!(probe_vibe_local("127.0.0.1", port, 2_000).is_none());

    let port =
        spawn_models_server("HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n{\"unexpected\":true}");
    assert!(probe_vibe_local("127.0.0.1", port, 2_000).is_none());
}

#[test]
fn udp_discovery_collects_worker_announcements() {
    let responder = UdpSocket::bind("127.0.0.1:0").unwrap();
    let port = responder.local_addr().unwrap().port();
    thread::spawn(move || {
        let mut buf = [0u8; 256];
        if let Ok((len, from)) = responder.recv_from(&mut buf) {
            assert_eq!(std::str::from_utf8(&buf[..len]).unwrap(), DISCOVERY_MAGIC);
            let announcement = r#"{"host":"127.0.0.1","port":20129,"backend":"cuda","device":"GTX 1660","models":["phi-4"]}"#;
            let _ = responder.send_to(announcement.as_bytes(), from);
        }
    });

    let workers = discover_lan_workers(port, 1_500);
    assert_eq!(workers.len(), 1, "exactly one mock worker must answer");
    let worker = &workers[0];
    assert!(worker.lan && worker.reachable);
    assert_eq!(worker.port, 20129);
    assert_eq!(worker.backend.as_deref(), Some("cuda"));
    assert_eq!(worker.device.as_deref(), Some("GTX 1660"));
    assert_eq!(worker.models, vec!["phi-4"]);
}
