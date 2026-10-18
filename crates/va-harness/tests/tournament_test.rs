//! Tests for Multi-Branch Speculative Tournament Mode (`va-harness::tournament`).

use std::path::PathBuf;
use va_harness::patcher::{segment_blocks, PatchOp};
use va_harness::session::{NodeKind, Session};
use va_harness::tournament::{CandidatePatch, SpeculativeTournament};

fn setup_tournament_test_repo(tag: &str) -> (PathBuf, Session) {
    let temp = std::env::temp_dir().join(format!(
        "va_tournament_test_{}_{}",
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::create_dir_all(&temp);
    let src = temp.join("lib.rs");
    std::fs::write(&src, "pub fn compute(x: i32) -> i32 {\n    x + 1\n}\n").unwrap();

    let mut session = Session::new();
    session.push(NodeKind::Prompt {
        role: "user".into(),
        text: "optimize compute".into(),
    });
    session.commit().unwrap();
    session.push(NodeKind::Verification {
        passed: true,
        detail: "baseline passed".into(),
    });
    session.commit().unwrap();

    (temp, session)
}

#[test]
fn test_tournament_selects_clean_minimal_candidate() {
    let (repo, mut session) = setup_tournament_test_repo("clean");
    let content = std::fs::read_to_string(repo.join("lib.rs")).unwrap();
    let blocks = segment_blocks(&content);
    let target = blocks[0].hash;

    // Candidate A: introduces a slop comment ("// fixed bug in compute") -> should be disqualified by judge
    let candidate_a = CandidatePatch {
        id: "cand_a_slop".into(),
        description: "Solution with slop comment".into(),
        patches: vec![(
            PathBuf::from("lib.rs"),
            PatchOp::new(
                target,
                "pub fn compute(x: i32) -> i32 {\n    // fixed bug in compute\n    x * 2\n}",
            ),
        )],
        cost_micros: 200,
        latency_ms: 100,
    };

    // Candidate B: clean focused fix, 2 lines modified
    let candidate_b = CandidatePatch {
        id: "cand_b_clean".into(),
        description: "Minimal clean solution".into(),
        patches: vec![(
            PathBuf::from("lib.rs"),
            PatchOp::new(target, "pub fn compute(x: i32) -> i32 {\n    x * 2\n}"),
        )],
        cost_micros: 150,
        latency_ms: 80,
    };

    // Candidate C: excessive churn, more lines
    let candidate_c = CandidatePatch {
        id: "cand_c_churn".into(),
        description: "Larger churn solution".into(),
        patches: vec![(
            PathBuf::from("lib.rs"),
            PatchOp::new(
                target,
                "pub fn compute(x: i32) -> i32 {\n    let factor = 2;\n    let intermediate = x * factor;\n    intermediate\n}",
            ),
        )],
        cost_micros: 300,
        latency_ms: 120,
    };

    let mut tournament = SpeculativeTournament::new(&repo);
    let outcome = tournament
        .run(
            &mut session,
            &[candidate_a, candidate_b.clone(), candidate_c],
        )
        .unwrap();

    // Candidate A must be disqualified by judge
    let eval_a = outcome
        .evaluations
        .iter()
        .find(|e| e.candidate_id == "cand_a_slop")
        .unwrap();
    assert!(eval_a.disqualified);
    assert!(!eval_a.judge_passed);

    // Candidate B must win (fewer lines churn and lower cost than C)
    assert_eq!(outcome.winner_id.as_deref(), Some("cand_b_clean"));
    assert!(outcome.merge_node_id.is_some());

    // Multi-parent merge node must exist in the session DAG
    let merge_id = outcome.merge_node_id.unwrap();
    let merge_node = session.get(merge_id).unwrap();
    assert!(
        merge_node.parents.len() >= 3,
        "merge node must link all candidate branches"
    );

    // Winning patch must be durability applied to the workspace file
    let final_content = std::fs::read_to_string(repo.join("lib.rs")).unwrap();
    assert!(final_content.contains("x * 2"));
    assert!(!final_content.contains("// fixed bug"));

    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn test_tournament_all_disqualified_returns_no_winner() {
    let (repo, mut session) = setup_tournament_test_repo("all_disq");
    let content = std::fs::read_to_string(repo.join("lib.rs")).unwrap();
    let blocks = segment_blocks(&content);
    let target = blocks[0].hash;

    // Both candidates contain slop / placeholders
    let c1 = CandidatePatch {
        id: "c1".into(),
        description: "Placeholder".into(),
        patches: vec![(
            PathBuf::from("lib.rs"),
            PatchOp::new(target, "pub fn compute(x: i32) -> i32 {\n    todo!()\n}"),
        )],
        cost_micros: 100,
        latency_ms: 50,
    };

    let c2 = CandidatePatch {
        id: "c2".into(),
        description: "Slop".into(),
        patches: vec![(
            PathBuf::from("lib.rs"),
            PatchOp::new(
                target,
                "pub fn compute(x: i32) -> i32 {\n    // added check\n    x\n}",
            ),
        )],
        cost_micros: 100,
        latency_ms: 50,
    };

    let mut tournament = SpeculativeTournament::new(&repo);
    let outcome = tournament.run(&mut session, &[c1, c2]).unwrap();

    assert!(outcome.winner_id.is_none());
    assert!(outcome.merge_node_id.is_none());
    assert!(outcome.evaluations.iter().all(|e| e.disqualified));

    let _ = std::fs::remove_dir_all(&repo);
}
