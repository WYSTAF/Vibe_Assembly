//! End-to-end live-planner test: a local mock OpenAI-compatible server must
//! drive a real Boss → Code → Verify → Done trajectory and produce a real
//! model-authored block patch. No offline synthesis is allowed here.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use va_core::modes::{ExecutionMode, LanguageLens, ReasoningEffort};
use va_core::router_client::{chat_completion, ChatMessage, RequestContext, RouterAuth};
use va_harness::engine::{Engine, StagnationPolicy};
use va_harness::live_planner::{LivePlannerConfig, LiveRouterPlanner};
use va_harness::patcher::segment_blocks;
use va_harness::session::{ContextWindowConfig, Session};

fn temp_repo(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("va-live-http-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join(".ai")).unwrap();
    dir
}

fn read_request(stream: &mut std::net::TcpStream) -> (String, String) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = stream.read(&mut chunk).unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_headers_end(&buf) {
            let head = String::from_utf8_lossy(&buf[..pos]).to_string();
            let length = head
                .lines()
                .find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    if k.eq_ignore_ascii_case("content-length") {
                        v.trim().parse::<usize>().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            if buf.len() >= pos + 4 + length {
                let body = String::from_utf8_lossy(&buf[pos + 4..pos + 4 + length]).to_string();
                return (head, body);
            }
        }
    }
    (String::new(), String::new())
}

fn find_headers_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// Spawn a mock router that answers `requests` connections using `route`.
fn mock_router<F>(route: F) -> (u16, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>)
where
    F: Fn(&str) -> String + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let roles = Arc::new(Mutex::new(Vec::new()));
    let roles_out = Arc::clone(&roles);
    let handle = thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let (head, _body) = read_request(&mut stream);
            let role = head
                .lines()
                .find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    if k.eq_ignore_ascii_case("x-vibe-role") {
                        Some(v.trim().to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            roles_out.lock().unwrap().push(role.clone());
            let body = route(&role);
            let wrapped = serde_json::json!({
                "choices": [{"message": {"role": "assistant", "content": body}}]
            })
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                 X-Vibe-Router-Cost-Micros: 321\r\nX-Vibe-Router-Latency-Ms: 42\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                wrapped.len(),
                wrapped
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    (port, roles, handle)
}

#[test]
fn live_planner_boss_code_verify_against_real_http() {
    let repo = temp_repo("planner");
    let source = "pub fn greet() {\n    let msg = \"hi\";\n    println!(\"{msg}\");\n}\n";
    std::fs::write(repo.join("src").join("lib.rs"), source).unwrap();
    let block = segment_blocks(source)
        .into_iter()
        .find(|b| b.signature.starts_with("pub fn greet"))
        .expect("greet block");
    let short = format!("{:04X}", (block.hash >> 48) as u16);

    let (port, roles, server) = mock_router(move |role| {
        match role {
        "boss" => format!(
            "{{\"files\":[\"src/lib.rs\"],\"targets\":[{{\"file\":\"src/lib.rs\",\"block_hash\":\"{short}\"}}],\"verify\":\"exit 0\",\"done\":false}}"
        ),
        _ => "{\"patches\":[{\"file\":\"src/lib.rs\",\"target_hash\":\"SHORT\",\"replacement\":\"pub fn greet() {\\n    println!(\\\"hello from the live planner\\\");\\n}\"}],\"done\":false}".replace("SHORT", &short),
    }
    });

    let mut config = LivePlannerConfig::default_for(&repo);
    config.model = Some("mock-model".to_string());
    config.router_host = "127.0.0.1".to_string();
    config.router_port = port;
    config.router_path = "/v1/chat/completions".to_string();
    config.timeout_ms = 5_000;
    config.mode = ExecutionMode::Code;
    config.language = LanguageLens::Rust;
    config.verification_command = Some("exit 0".to_string());
    config.task_prompt = Some("Greet the user from the live planner.".to_string());
    let mut planner = LiveRouterPlanner::with_config(config);

    let mut session = Session::new();
    session.push(va_harness::session::NodeKind::Prompt {
        role: "user".to_string(),
        text: "Greet the user from the live planner.".to_string(),
    });
    session.commit().unwrap();

    let mut engine = Engine::new(&repo)
        .with_max_steps(8)
        .with_profile(ExecutionMode::Code, LanguageLens::Rust)
        .with_context_window(ContextWindowConfig::new(32_000))
        .with_stagnation_policy(StagnationPolicy::Fail);

    let result = engine.run(&mut session, &mut planner);
    assert!(result.is_ok(), "live run failed: {result:?}");
    server.join().unwrap();

    let updated = std::fs::read_to_string(repo.join("src").join("lib.rs")).unwrap();
    assert!(
        updated.contains("hello from the live planner"),
        "model-authored patch must be applied; got:\n{updated}"
    );
    assert!(
        !updated.contains("// Step") && !updated.contains("offline-mock"),
        "no placeholder synthesis may leak into a live run"
    );

    let roles = roles.lock().unwrap().clone();
    assert_eq!(
        roles,
        vec!["boss".to_string(), "code".to_string()],
        "expected Boss then Code call"
    );
    assert_eq!(planner.last_cost_micros, 321);
    assert_eq!(planner.last_latency_ms, 42);

    let stats = engine.stats();
    assert_eq!(stats.patches_applied, 1);
    assert_eq!(stats.verifications_run, 1);
    assert!(session.replay(None).is_ok());

    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn live_planner_applies_multiple_patches_to_the_same_file() {
    let repo = temp_repo("multi-patch");
    let source = "pub fn one() -> u32 {\n    1\n}\n\npub fn two() -> u32 {\n    2\n}\n";
    std::fs::write(repo.join("src").join("lib.rs"), source).unwrap();
    let blocks = segment_blocks(source);
    let one = blocks
        .iter()
        .find(|b| b.signature.starts_with("pub fn one"))
        .expect("one block");
    let two = blocks
        .iter()
        .find(|b| b.signature.starts_with("pub fn two"))
        .expect("two block");
    let short_one = format!("{:04X}", (one.hash >> 48) as u16);
    let short_two = format!("{:04X}", (two.hash >> 48) as u16);

    let (port, _roles, server) = mock_router(move |role| {
        if role == "boss" {
            format!(
                "{{\"targets\":[{{\"file\":\"src/lib.rs\",\"block_hash\":\"{short_one}\"}},\
                 {{\"file\":\"src/lib.rs\",\"block_hash\":\"{short_two}\"}}],\
                 \"verify\":\"exit 0\",\"done\":false}}"
            )
        } else {
            format!(
                "{{\"patches\":[\
                 {{\"file\":\"src/lib.rs\",\"target_hash\":\"{short_one}\",\
                 \"replacement\":\"pub fn one() -> u32 {{\\n    10\\n}}\"}},\
                 {{\"file\":\"src/lib.rs\",\"target_hash\":\"{short_two}\",\
                 \"replacement\":\"pub fn two() -> u32 {{\\n    20\\n}}\"}}],\"done\":false}}"
            )
        }
    });

    let mut config = LivePlannerConfig::default_for(&repo);
    config.model = Some("mock-model".to_string());
    config.router_host = "127.0.0.1".to_string();
    config.router_port = port;
    config.router_path = "/v1/chat/completions".to_string();
    config.timeout_ms = 5_000;
    config.mode = ExecutionMode::Code;
    config.language = LanguageLens::Rust;
    config.verification_command = Some("exit 0".to_string());
    config.task_prompt = Some("Return ten and twenty.".to_string());
    let mut planner = LiveRouterPlanner::with_config(config);

    let mut session = Session::new();
    let mut engine = Engine::new(&repo)
        .with_max_steps(8)
        .with_profile(ExecutionMode::Code, LanguageLens::Rust)
        .with_context_window(ContextWindowConfig::new(32_000))
        .with_stagnation_policy(StagnationPolicy::Fail);
    let result = engine.run(&mut session, &mut planner);
    assert!(result.is_ok(), "multi-patch live run failed: {result:?}");
    server.join().unwrap();

    let updated = std::fs::read_to_string(repo.join("src").join("lib.rs")).unwrap();
    assert!(
        updated.contains("    10"),
        "first patch missing:\n{updated}"
    );
    assert!(
        updated.contains("    20"),
        "second patch missing:\n{updated}"
    );
    assert_eq!(engine.stats().patches_applied, 2);
    assert_eq!(engine.stats().verifications_run, 1);
    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn live_planner_telemetry_accepts_vr_alias_headers() {
    let repo = temp_repo("telemetry");
    let source = "pub fn answer() -> u32 {\n    41\n}\n";
    std::fs::write(repo.join("src").join("lib.rs"), source).unwrap();
    let block = segment_blocks(source).first().cloned().unwrap();
    let short = format!("{:04X}", (block.hash >> 48) as u16);

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let requests = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&requests);
    let handle = thread::spawn(move || {
        let bodies: HashMap<&str, String> = HashMap::from([
            (
                "boss",
                format!(
                    "{{\"targets\":[{{\"file\":\"src/lib.rs\",\"block_hash\":\"{short}\"}}],\"verify\":\"exit 0\"}}"
                ),
            ),
            (
                "code",
                format!(
                    "{{\"patches\":[{{\"file\":\"src/lib.rs\",\"target_hash\":\"{short}\",\"replacement\":\"pub fn answer() -> u32 {{\\n    42\\n}}\"}}]}}"
                ),
            ),
        ]);
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let (head, _) = read_request(&mut stream);
            counter.fetch_add(1, Ordering::SeqCst);
            let role = head
                .lines()
                .find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    if k.eq_ignore_ascii_case("x-vibe-role") {
                        Some(v.trim().to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            let body = bodies.get(role.as_str()).cloned().unwrap_or_default();
            let wrapped = serde_json::json!({
                "choices": [{"message": {"role": "assistant", "content": body}}]
            })
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                 X-VR-Cost-Micros: 99\r\nX-VR-Latency-Ms: 7\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                wrapped.len(),
                wrapped
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });

    let mut config = LivePlannerConfig::default_for(&repo);
    config.model = Some("mock-model".into());
    config.router_port = port;
    config.timeout_ms = 5_000;
    config.mode = ExecutionMode::Code;
    config.language = LanguageLens::Rust;
    config.verification_command = Some("exit 0".to_string());
    config.task_prompt = Some("Return 42.".to_string());
    let mut planner = LiveRouterPlanner::with_config(config);
    let mut session = Session::new();
    let mut engine = Engine::new(&repo)
        .with_max_steps(8)
        .with_profile(ExecutionMode::Code, LanguageLens::Rust);
    let result = engine.run(&mut session, &mut planner);
    assert!(result.is_ok(), "{result:?}");
    handle.join().unwrap();
    assert_eq!(requests.load(Ordering::SeqCst), 2);
    assert_eq!(planner.last_cost_micros, 99);
    assert_eq!(planner.last_latency_ms, 7);
    let updated = std::fs::read_to_string(repo.join("src").join("lib.rs")).unwrap();
    assert!(updated.contains("42"));
    let _ = std::fs::remove_dir_all(&repo);
}

/// One-connection mock that captures the request body and answers a minimal
/// chat completion, so payload fields can be asserted verbatim.
fn capture_one_completion() -> (u16, Arc<Mutex<String>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let captured = Arc::new(Mutex::new(String::new()));
    let out = Arc::clone(&captured);
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let (_head, body) = read_request(&mut stream);
        *out.lock().unwrap() = body;
        let wrapped = serde_json::json!({
            "choices": [{"message": {"role": "assistant", "content": "ok"}}]
        })
        .to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{}",
            wrapped.len(),
            wrapped
        );
        let _ = stream.write_all(response.as_bytes());
    });
    (port, captured, handle)
}

#[test]
fn reasoning_effort_payload_emission_follows_model_capability() {
    let cases = [
        ("gpt-5.2-codex", ReasoningEffort::XHigh, Some("xhigh")),
        ("claude-sonnet-4.6", ReasoningEffort::XHigh, Some("high")),
        ("claude-sonnet-4.6", ReasoningEffort::High, Some("high")),
        ("gpt-4o-mini", ReasoningEffort::High, None),
        ("gemini-2.5-flash", ReasoningEffort::Medium, None),
        ("claude-opus-4.6", ReasoningEffort::Auto, None),
    ];
    for (model, effort, expected) in cases {
        let (port, captured, server) = capture_one_completion();
        let ctx = RequestContext {
            reasoning_effort: effort,
            ..RequestContext::default()
        };
        let result = chat_completion(
            "127.0.0.1",
            port,
            "/v1/chat/completions",
            &RouterAuth::default(),
            &ctx,
            model,
            &[ChatMessage::user("hi")],
            0.2,
            None,
            5_000,
        );
        assert!(result.is_ok(), "{model} / {effort}: {result:?}");
        server.join().unwrap();
        let body: serde_json::Value =
            serde_json::from_str(&captured.lock().unwrap()).expect("request body must be JSON");
        match expected {
            Some(level) => assert_eq!(
                body["reasoning_effort"], level,
                "{model} requested {effort} must wire {level}"
            ),
            None => assert!(
                body.get("reasoning_effort").is_none(),
                "{model} requested {effort} must omit the field, got {body}"
            ),
        }
    }
}

#[test]
fn live_planner_config_effort_reaches_every_router_call() {
    let repo = temp_repo("effort-config");
    let source = "pub fn effort() -> u32 {\n    1\n}\n";
    std::fs::write(repo.join("src").join("lib.rs"), source).unwrap();
    let block = segment_blocks(source).first().cloned().unwrap();
    let short = format!("{:04X}", (block.hash >> 48) as u16);

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let bodies = Arc::new(Mutex::new(Vec::new()));
    let bodies_out = Arc::clone(&bodies);
    let handle = thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let (head, body) = read_request(&mut stream);
            bodies_out.lock().unwrap().push(body);
            let role = head
                .lines()
                .find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    if k.eq_ignore_ascii_case("x-vibe-role") {
                        Some(v.trim().to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            let content = if role == "boss" {
                format!(
                    "{{\"targets\":[{{\"file\":\"src/lib.rs\",\"block_hash\":\"{short}\"}}],\"verify\":\"exit 0\"}}"
                )
            } else {
                format!(
                    "{{\"patches\":[{{\"file\":\"src/lib.rs\",\"target_hash\":\"{short}\",\"replacement\":\"pub fn effort() -> u32 {{\\n    2\\n}}\"}}]}}"
                )
            };
            let wrapped = serde_json::json!({
                "choices": [{"message": {"role": "assistant", "content": content}}]
            })
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                wrapped.len(),
                wrapped
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });

    let mut config = LivePlannerConfig::default_for(&repo);
    config.model = Some("claude-sonnet-4.6".into());
    config.router_port = port;
    config.timeout_ms = 5_000;
    config.mode = ExecutionMode::Code;
    config.language = LanguageLens::Rust;
    config.verification_command = Some("exit 0".to_string());
    config.task_prompt = Some("Return two.".to_string());
    config.reasoning_effort = ReasoningEffort::XHigh;
    let mut planner = LiveRouterPlanner::with_config(config);
    let mut session = Session::new();
    let mut engine = Engine::new(&repo)
        .with_max_steps(8)
        .with_profile(ExecutionMode::Code, LanguageLens::Rust);
    let result = engine.run(&mut session, &mut planner);
    assert!(result.is_ok(), "{result:?}");
    handle.join().unwrap();

    let bodies = bodies.lock().unwrap();
    assert_eq!(bodies.len(), 2, "boss and code calls must both be captured");
    for body in bodies.iter() {
        let json: serde_json::Value = serde_json::from_str(body).unwrap();
        assert_eq!(
            json["reasoning_effort"], "high",
            "XHigh on sonnet must coerce to high on every call: {body}"
        );
    }
    let _ = std::fs::remove_dir_all(&repo);
}
