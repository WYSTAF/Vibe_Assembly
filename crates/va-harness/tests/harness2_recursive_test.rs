//! Integration tests for the Harness² modules: c1..c8 reset-to-base isolation
//! and the Propose -> Probe -> Compose contrastive loop's DAG branching.

use std::path::PathBuf;

use va_harness::components::{ComponentTree, HarnessLayers, SURFACE_IDS};
use va_harness::experience::{ExperienceEntry, ExperienceStore};
use va_harness::judge::{DiffLine, Verdict};
use va_harness::recursive::{
    CandidateKind, RecursiveConfig, RecursiveLoop, SandboxJudgeVerifier, StepVerifier,
};
use va_harness::sandbox::SandboxResult;
use va_harness::session::{NodeKind, Session};

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("va-h2-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct MockVerifier {
    pass: bool,
    judge_pass: bool,
}

impl StepVerifier for MockVerifier {
    fn verify(&mut self, _command: &str) -> SandboxResult {
        SandboxResult {
            success: self.pass,
            exit_code: Some(if self.pass { 0 } else { 1 }),
            stdout: String::new(),
            stderr: if self.pass {
                String::new()
            } else {
                "error[E0382]: use of moved value".into()
            },
            timed_out: false,
            duration: std::time::Duration::from_millis(1),
        }
    }
    fn judge(&mut self, _diff: &[DiffLine]) -> Verdict {
        if self.judge_pass {
            Verdict::Pass
        } else {
            Verdict::Reject(vec![])
        }
    }
}

#[test]
fn c1_c8_reset_to_base_isolates_successive_tasks() {
    let root = temp_root("isolation");
    let experience = ExperienceStore::load(&root);

    let mut task_a = ComponentTree::begin(&root, "a", false, &experience, "rust", "workspace");
    task_a
        .set_overlay("c1_system_prompt", "task-a poison")
        .unwrap();
    task_a.set_overlay("c6_skills", "task-a skill").unwrap();
    assert!(!task_a.layers().is_pristine());

    // A brand-new task must not inherit task-a's overlays.
    let task_b = ComponentTree::begin(&root, "b", false, &experience, "rust", "workspace");
    assert!(task_b.layers().is_pristine());
    for id in SURFACE_IDS {
        assert_eq!(
            task_b.layers().get(id).unwrap(),
            HarnessLayers::base().get(id).unwrap(),
            "{id} leaked across tasks"
        );
    }

    // Reset within task-a returns to base.
    task_a.reset();
    assert!(task_a.layers().is_pristine());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn reset_to_base_keeps_only_verified_experience_precondition() {
    let root = temp_root("precondition");
    let mut experience = ExperienceStore::load(&root);
    experience
        .record(ExperienceEntry {
            date: "2026-11-08".into(),
            language: "rust".into(),
            domain: "workspace".into(),
            signature: "sig".into(),
            kind: "invariant".into(),
            lesson: "always check the error".into(),
            evidence: vec!["E0382".into()],
            rationale: "test".into(),
        })
        .unwrap();

    let mut tree = ComponentTree::begin(&root, "t", false, &experience, "rust", "workspace");
    tree.set_overlay("c2_guardrails", "overlay").unwrap();
    tree.reset();
    // Overlay gone, verified lesson retained in c3.
    assert_ne!(tree.layers().get("c2_guardrails").unwrap(), "overlay");
    assert!(tree
        .layers()
        .get("c3_memory")
        .unwrap()
        .contains("always check the error"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn fast_path_records_a_verified_node_without_probing() {
    let root = temp_root("fast");
    let mut session = Session::new();
    let mut verifier = MockVerifier {
        pass: true,
        judge_pass: true,
    };
    let loop_ = RecursiveLoop::new(&root, RecursiveConfig::default());
    let outcome = loop_.execute_step(&mut session, &mut verifier, "cargo check", &[], None);
    assert!(outcome.fast_path);
    assert_eq!(outcome.probes, 0);
    assert!(outcome.passed);
    // No fork markers on the fast path.
    let forks = session
        .nodes()
        .filter(|n| matches!(n.kind, NodeKind::Fork { .. }))
        .count();
    assert_eq!(forks, 0);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn failure_forks_two_branches_and_composes_a_merge_node() {
    let root = temp_root("branch");
    let mut session = Session::new();
    let mut verifier = MockVerifier {
        pass: false,
        judge_pass: false,
    };
    let loop_ = RecursiveLoop::new(&root, RecursiveConfig::default());
    let outcome = loop_.execute_step(&mut session, &mut verifier, "cargo check", &[], None);

    assert!(!outcome.fast_path);
    assert_eq!(outcome.probes, 2);
    assert!(outcome.winner.is_some());
    assert!(outcome.lesson_recorded);

    // Two contrastive fork branches exist.
    let forks: Vec<_> = session
        .nodes()
        .filter(|n| matches!(n.kind, NodeKind::Fork { .. }))
        .collect();
    assert_eq!(forks.len(), 2);

    // The composed node is a true multi-parent merge.
    let merged = outcome.merged_node.unwrap();
    let node = session.get(merged).unwrap();
    assert_eq!(node.parents.len(), 2);
    assert!(matches!(node.kind, NodeKind::Verification { .. }));

    // The DAG integrity holds after branching + merge.
    assert!(session.verify_integrity().is_ok());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn opt_in_k_probes_even_when_the_first_try_passes() {
    let root = temp_root("optin");
    let mut session = Session::new();
    let mut verifier = MockVerifier {
        pass: true,
        judge_pass: true,
    };
    let loop_ = RecursiveLoop::new(&root, RecursiveConfig { k: 2 });
    let outcome = loop_.execute_step(&mut session, &mut verifier, "cargo check", &[], None);
    assert!(!outcome.fast_path);
    assert_eq!(outcome.probes, 2);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn compose_ties_favour_the_invariant_candidate() {
    let root = temp_root("tie");
    let loop_ = RecursiveLoop::new(&root, RecursiveConfig::default());
    let same = va_harness::recursive::ProbeOutcome {
        passed: true,
        judge_passed: true,
        error_count: 0,
        churn: 2,
        cost_micros: 10,
    };
    assert_eq!(loop_.compose(&same, &same), CandidateKind::Invariant);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn sandbox_judge_verifier_runs_a_real_confined_command() {
    let root = temp_root("sandbox");
    let mut verifier = SandboxJudgeVerifier::new(&root);
    // A trivially successful confined command.
    let result = verifier.verify("cargo --version");
    assert!(result.success);
    let _ = std::fs::remove_dir_all(&root);
}
