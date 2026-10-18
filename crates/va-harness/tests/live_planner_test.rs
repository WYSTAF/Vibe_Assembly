//! Tests for Live Router-Backed Planner (`va-harness::live_planner`).

use std::path::PathBuf;
use va_core::router_client::RequestContext;
use va_harness::engine::{Action, Engine};
use va_harness::live_planner::LiveRouterPlanner;
use va_harness::patcher::{segment_blocks, PatchOp};
use va_harness::session::{NodeKind, Session};

fn setup_planner_test_repo(tag: &str) -> (PathBuf, Session) {
    let temp = std::env::temp_dir().join(format!(
        "va_planner_test_{}_{}",
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::create_dir_all(&temp);
    let src = temp.join("lib.rs");
    std::fs::write(&src, "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n").unwrap();

    let mut session = Session::new();
    session.push(NodeKind::Prompt {
        role: "user".into(),
        text: "test live planner loop".into(),
    });
    session.commit().unwrap();

    (temp, session)
}

#[test]
fn test_live_planner_context_headers_and_retries() {
    // Verify standard role headers
    let boss_ctx = RequestContext {
        role: Some("boss".into()),
        node_id: Some(1),
        profile: Some("workstation".into()),
        is_retry: false,
    };
    let boss_headers = boss_ctx.headers();
    assert!(boss_headers.contains(&("X-Vibe-Role".into(), "boss".into())));
    assert!(boss_headers.contains(&("X-Vibe-Node-Id".into(), "1".into())));
    assert!(!boss_headers.iter().any(|(k, _)| k == "X-Vibe-Retry"));

    // Verify retry gate headers (emits X-Vibe-Retry: 1 and Cache-Control: no-cache)
    let retry_ctx = RequestContext {
        role: Some("code".into()),
        node_id: Some(2),
        profile: None,
        is_retry: true,
    };
    let retry_headers = retry_ctx.headers();
    assert!(retry_headers.contains(&("X-Vibe-Role".into(), "code".into())));
    assert!(retry_headers.contains(&("X-Vibe-Retry".into(), "1".into())));
    assert!(retry_headers.contains(&("Cache-Control".into(), "no-cache".into())));
}

#[test]
fn test_live_planner_mock_execution_and_telemetry() {
    let (repo, mut session) = setup_planner_test_repo("mock");
    let content = std::fs::read_to_string(repo.join("lib.rs")).unwrap();
    let blocks = segment_blocks(&content);
    let target = blocks[0].hash;

    let patch_action = Action::Patch {
        path: PathBuf::from("lib.rs"),
        op: PatchOp::new(
            target,
            "pub fn add(a: i32, b: i32) -> i32 {\n    (a + b) * 1\n}",
        ),
    };

    let actions = vec![patch_action, Action::Done];
    let mut planner = LiveRouterPlanner::with_mock_actions(&repo, actions);

    let mut engine = Engine::new(&repo);
    let records = engine.run(&mut session, &mut planner).unwrap();

    assert!(records.len() >= 2);
    assert!(records.iter().any(|r| r.action.contains("patch")));
    assert!(records.iter().any(|r| r.action == "done" && r.verified));

    // Verify session node has recorded telemetry
    let head = session.head().unwrap();
    session.record_telemetry(head, Some(350), Some(95)).unwrap();
    assert_eq!(session.total_cost_micros(), 350);
    assert_eq!(session.total_latency_ms(), 95);

    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn test_live_planner_repomap_generation() {
    let (repo, session) = setup_planner_test_repo("repomap");
    let mut planner = LiveRouterPlanner::new(&repo);

    // Running next_action triggers RepoMap generation
    let _ = va_harness::engine::Planner::next_action(&mut planner, &session);
    let map_text = planner.repomap().expect("RepoMap must be generated");
    assert!(map_text.contains("lib.rs"));
    assert!(map_text.contains("fn add"));

    let _ = std::fs::remove_dir_all(&repo);
}
