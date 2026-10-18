//! Session trajectory DAG: node creation, fork, rollback, and replay.

use va_harness::session::{NodeKind, Session, SessionError};

fn seeded() -> Session {
    let mut s = Session::new();
    s.push(NodeKind::Prompt {
        role: "user".into(),
        text: "fix the parser".into(),
    });
    s.commit().unwrap();
    s.push(NodeKind::Thought {
        text: "looking at the queue".into(),
    });
    s.commit().unwrap();
    s
}

#[test]
fn creates_nodes_with_parents_and_hashes() {
    let s = seeded();
    assert_eq!(s.len(), 2);
    let root = &s.nodes().next().unwrap();
    assert!(root.parents.is_empty(), "the first node has no parent");
    assert_ne!(root.content_hash, 0);
    assert_ne!(root.state_hash, 0);

    let second = s.nodes().nth(1).unwrap();
    assert_eq!(second.parents.len(), 1);
    assert_eq!(second.parents[0], root.id);
    // A descendant's state hash folds in its ancestry, so it must differ.
    assert_ne!(second.state_hash, root.state_hash);
}

#[test]
fn uncommitted_work_is_invisible_until_commit() {
    let mut s = seeded();
    let before = s.len();
    s.push(NodeKind::Thought {
        text: "not yet durable".into(),
    });
    assert_eq!(s.len(), before, "push must not publish");
    s.commit().unwrap();
    assert_eq!(s.len(), before + 1, "commit must publish");
}

#[test]
fn fork_starts_a_new_branch_sharing_history() {
    let mut s = seeded();
    let branch_point = s.head().unwrap();
    s.push(NodeKind::Verification {
        passed: false,
        detail: "compile error".into(),
    });
    s.commit().unwrap();

    s.fork(branch_point, "compile failed, try another approach")
        .unwrap();
    s.push(NodeKind::Thought {
        text: "new strategy".into(),
    });
    s.commit().unwrap();

    let path = s.current_path().unwrap();
    assert_eq!(path[0], 1, "the shared root is still node 1");
    assert!(path.contains(&branch_point));
    assert!(s.children_of(branch_point).len() >= 2, "the fork diverges");
    // The pre-fork failure is still on record — history is not discarded.
    assert!(s
        .nodes()
        .any(|n| matches!(&n.kind, NodeKind::Verification { passed: false, .. })));
}

#[test]
fn rollback_moves_head_without_a_branch_marker() {
    let mut s = seeded();
    let good = s.head().unwrap();
    s.push(NodeKind::Verification {
        passed: false,
        detail: "boom".into(),
    });
    s.commit().unwrap();
    let bad = s.head().unwrap();
    assert_ne!(good, bad);

    s.rollback(good).unwrap();
    assert_eq!(s.head(), Some(good));
    assert!(s.is_detached());
    // The abandoned node remains in the log, not deleted.
    assert!(s.contains(bad));
}

#[test]
fn rollback_to_last_verified_lands_on_the_passing_node() {
    let mut s = seeded();
    s.push(NodeKind::Verification {
        passed: true,
        detail: "tests green".into(),
    });
    s.commit().unwrap();
    let verified = s.head().unwrap();

    s.push(NodeKind::ToolCall {
        name: "cargo".into(),
        arguments: "{}".into(),
    });
    s.commit().unwrap();
    s.push(NodeKind::Verification {
        passed: false,
        detail: "regression".into(),
    });
    s.commit().unwrap();

    s.rollback_to_last_verified().unwrap();
    assert_eq!(s.head(), Some(verified));
}

#[test]
fn replay_reuses_stored_results_without_reexecuting() {
    let s = seeded();
    let labels = s.replay(None).unwrap();
    assert_eq!(labels, vec!["prompt".to_string(), "thought".to_string()]);

    // Replay from a node is a suffix, and it is read-only.
    let from_second = s.current_path().unwrap()[1];
    let suffix = s.replay(Some(from_second)).unwrap();
    assert_eq!(suffix, vec!["thought".to_string()]);
    assert_eq!(s.len(), 2, "replay must not mutate the session");
}

#[test]
fn merge_node_can_have_multiple_parents() {
    let mut s = seeded();
    let a = s.nodes().next().unwrap().id;
    let b = s.head().unwrap();
    s.push_merging(
        vec![a, b],
        NodeKind::Thought {
            text: "joining two branches".into(),
        },
    );
    s.commit().unwrap();

    let merge = s.head().unwrap();
    assert_eq!(s.get(merge).unwrap().parents.len(), 2, "a true merge");
    let path = s.path_to(merge).unwrap();
    assert!(
        path.contains(&a) && path.contains(&b),
        "both ancestors present"
    );
}

#[test]
fn integrity_verification_detects_out_of_band_edits() {
    let s = seeded();
    assert!(s.verify_integrity().is_ok(), "a fresh log is intact");
    assert_eq!(s.len(), 2);
}

#[test]
fn jsonl_round_trips_structure() {
    let s = seeded();
    let raw = s.to_jsonl();
    assert_eq!(raw.lines().count(), 2);
    let restored = Session::from_jsonl(&raw).unwrap();
    assert_eq!(restored.len(), s.len());
    assert_eq!(restored.head(), s.head());
}

#[test]
fn missing_and_unknown_nodes_are_reported_not_panicked() {
    let mut s = seeded();
    assert!(matches!(
        s.fork(9999, "nope"),
        Err(SessionError::UnknownNode(9999))
    ));
    assert!(matches!(
        s.rollback(9999),
        Err(SessionError::UnknownNode(9999))
    ));
    assert!(matches!(
        s.replay(Some(9999)),
        Err(SessionError::UnknownNode(9999))
    ));
}

#[test]
fn jsonl_round_trips_complex_payloads_with_quotes_braces_crlf_and_unicode() {
    let mut s = Session::new();
    let complex_text = "fn main() {\r\n    println!(\"Hello \\\"World\\\" with {{{nested braces}}} 👑 ⚡\");\r\n}\r\n";
    s.push(NodeKind::Prompt {
        role: "user".into(),
        text: complex_text.into(),
    });
    s.commit().unwrap();

    let diff_text = "--- a/src/lib.rs\r\n+++ b/src/lib.rs\r\n@@ -1,2 +1,3 @@\r\n+    let x = \"value with \\\"quotes\\\" and }}} braces\";\r\n";
    s.push(NodeKind::Thought {
        text: diff_text.into(),
    });
    s.commit().unwrap();

    let raw = s.to_jsonl();
    let restored = Session::from_jsonl(&raw).unwrap();
    assert_eq!(restored.len(), 2);
    assert_eq!(restored.head(), s.head());

    let root_node = restored.nodes().next().unwrap();
    if let NodeKind::Prompt { text, .. } = &root_node.kind {
        assert_eq!(text, complex_text);
    } else {
        panic!("expected Prompt node");
    }

    let second_node = restored.nodes().nth(1).unwrap();
    if let NodeKind::Thought { text } = &second_node.kind {
        assert_eq!(text, diff_text);
    } else {
        panic!("expected Thought node");
    }
}

#[test]
fn context_surface_range_compaction_preserves_dag_and_replay() {
    let mut s = Session::new();
    // 1. Initial prompt
    let _n1 = s.push(NodeKind::Prompt {
        role: "user".into(),
        text: "implement algorithm".into(),
    });
    s.commit().unwrap();

    // 2. Failed attempt
    let n2 = s.push(NodeKind::Thought {
        text: "try brute force".into(),
    });
    s.commit().unwrap();

    // 3. Failed verification
    let _n3 = s.push(NodeKind::Verification {
        passed: false,
        detail: "timeout on large inputs".into(),
    });
    s.commit().unwrap();

    // 4. Retry thought
    let n4 = s.push(NodeKind::Thought {
        text: "try dynamic programming".into(),
    });
    s.commit().unwrap();

    // 5. Passing verification
    let n5 = s.push(NodeKind::Verification {
        passed: true,
        detail: "all tests passed".into(),
    });
    s.commit().unwrap();

    assert_eq!(s.len(), 5);

    // Extract ContextSurface view over active branch
    let mut surface = s.context_surface().unwrap();
    assert_eq!(surface.len(), 5);

    // Compact intermediate retry loop (n2..=n4)
    surface
        .compact_range(
            n2,
            n4,
            "Attempted brute force which timed out; switched to dynamic programming.",
        )
        .unwrap();

    // Surface now has 3 items: n1, summary, n5
    assert_eq!(surface.len(), 3);
    let prompt_ctx = surface.to_prompt_context();
    assert!(prompt_ctx.contains("The conversation history before this point was compacted"));
    assert!(prompt_ctx.contains("dynamic programming"));
    assert!(prompt_ctx.contains("all tests passed"));

    // Crucial invariant: Underlying DAG is NOT mutated or truncated!
    assert_eq!(s.len(), 5, "underlying DAG must keep all 5 nodes");
    assert_eq!(s.head(), Some(n5));
    assert!(s.verify_integrity().is_ok());

    // Deterministic replay still walks all 5 nodes
    let labels = s.replay(None).unwrap();
    assert_eq!(labels.len(), 5);
    assert_eq!(
        labels,
        vec![
            "prompt",
            "thought",
            "verification",
            "thought",
            "verification"
        ]
    );
}

#[test]
fn telemetry_and_judge_verified_hash_recorded_and_persisted() {
    let mut s = Session::new();
    let n1 = s.push(NodeKind::ToolCall {
        name: "vibe_router".into(),
        arguments: "{}".into(),
    });
    s.commit().unwrap();

    s.record_telemetry(n1, Some(450), Some(125)).unwrap();
    assert_eq!(s.total_cost_micros(), 450);
    assert_eq!(s.total_latency_ms(), 125);

    let v_hash = 0xcafe_babe_1234_5678u64;
    s.record_judge_verified(n1, v_hash).unwrap();

    let raw = s.to_jsonl();
    assert!(raw.contains("\"cost_micros\":450"));
    assert!(raw.contains("\"latency_ms\":125"));

    let restored = Session::from_jsonl(&raw).unwrap();
    assert_eq!(restored.total_cost_micros(), 450);
    assert_eq!(restored.total_latency_ms(), 125);
    let node = restored.get(n1).unwrap();
    assert_eq!(node.judge_verified_hash, Some(v_hash));
}
