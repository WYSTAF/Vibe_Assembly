//! Judge: rejection of slop comments, diff minimalism enforcement, and the
//! single-retry budget.

use va_harness::judge::{DiffLine, Finding, FindingKind, Judge, JudgeConfig, Verdict};

fn added(path: &str, text: &str) -> DiffLine {
    DiffLine {
        marker: '+',
        path: path.to_string(),
        text: text.to_string(),
    }
}

fn removed(path: &str, text: &str) -> DiffLine {
    DiffLine {
        marker: '-',
        path: path.to_string(),
        text: text.to_string(),
    }
}

fn kinds(v: &Verdict) -> Vec<FindingKind> {
    match v {
        Verdict::Pass => Vec::new(),
        Verdict::Reject(f) => f.iter().map(|x| x.kind.clone()).collect(),
    }
}

#[test]
fn accepts_a_minimal_focused_diff() {
    let diff = vec![
        added("src/math.rs", "pub fn double(x: i32) -> i32 {"),
        added("src/math.rs", "    x * 2"),
        added("src/math.rs", "}"),
        removed("src/math.rs", "pub fn double(x: i32) -> i32 {"),
    ];
    assert_eq!(
        Judge::with_defaults().review(&diff, &[1, 2, 3, 1]),
        Verdict::Pass
    );
}

#[test]
fn rejects_slop_comments() {
    for phrase in [
        "// Fixed bug",
        "// added null check here",
        "// cleanup",
        "// quick fix",
        "// temporary",
        "// should work now",
    ] {
        let diff = vec![added("src/lib.rs", &format!("    {phrase}"))];
        let found = kinds(&Judge::with_defaults().review(&diff, &[7]));
        assert!(
            found.contains(&FindingKind::SlopComment),
            "should reject {phrase:?}, got {found:?}"
        );
    }
}

#[test]
fn allows_doc_comments_even_with_narration_words() {
    // Doc comments describe the API; they are not diff narration.
    for line in [
        "    /// Returns the improved value.",
        "//! Fixed the crate root.",
        "    /** Added a helper. */",
    ] {
        let diff = vec![added("src/lib.rs", line)];
        assert_eq!(
            Judge::with_defaults().review(&diff, &[1]),
            Verdict::Pass,
            "doc comment {line:?} must pass"
        );
    }
}

#[test]
fn rejects_unsafe_primitives_outside_tests() {
    for token in ["x.unwrap()", "panic!(\"no\")", "y.expect(\"z\")"] {
        let diff = vec![added("src/service.rs", &format!("    let v = {token};"))];
        let found = kinds(&Judge::with_defaults().review(&diff, &[5]));
        assert!(
            found.contains(&FindingKind::UnsafeInProduction),
            "should reject {token:?}, got {found:?}"
        );
    }
}

#[test]
fn allows_unsafe_primitives_in_test_paths() {
    for path in [
        "tests/integration.rs",
        "src/foo/test.rs",
        "benches/throughput.rs",
        "examples/demo.rs",
    ] {
        let diff = vec![added(path, "    let v = x.unwrap();")];
        assert_eq!(
            Judge::with_defaults().review(&diff, &[1]),
            Verdict::Pass,
            "{path} must be allowed"
        );
    }
}

#[test]
fn rejects_placeholders() {
    for marker in ["unimplemented!()", "todo!()", "panic!(\"not implemented\")"] {
        let diff = vec![added("src/engine.rs", &format!("    {marker}"))];
        let found = kinds(&Judge::with_defaults().review(&diff, &[9]));
        assert!(
            found.contains(&FindingKind::Placeholder),
            "should reject {marker:?}, got {found:?}"
        );
    }
}

#[test]
fn rejects_trailing_whitespace_on_added_lines() {
    let diff = vec![added("src/x.rs", "    let x = 1;   ")];
    assert!(kinds(&Judge::with_defaults().review(&diff, &[1]))
        .contains(&FindingKind::TrailingWhitespace));
}

#[test]
fn rejects_import_only_churn() {
    let diff = vec![
        added("src/a.rs", "use std::collections::HashMap;"),
        added("src/a.rs", "use std::fmt;"),
        removed("src/a.rs", "use std::io;"),
    ];
    assert!(kinds(&Judge::with_defaults().review(&diff, &[1, 2, 3]))
        .contains(&FindingKind::ImportChurn));
}

#[test]
fn allows_import_change_alongside_real_code() {
    // One import plus real logic is a legitimate change, not import churn.
    let diff = vec![
        added("src/a.rs", "use std::collections::HashMap;"),
        added("src/a.rs", "pub fn f() -> HashMap<u8, u8> {"),
        added("src/a.rs", "    HashMap::new()"),
        added("src/a.rs", "}"),
    ];
    assert_eq!(
        Judge::with_defaults().review(&diff, &[1, 2, 3, 4]),
        Verdict::Pass
    );
}

#[test]
fn rejects_excessive_churn() {
    let cfg = JudgeConfig {
        max_changed_lines: 12,
        ..Default::default()
    };
    let diff: Vec<DiffLine> = (0..60)
        .map(|i| added("src/big.rs", &format!("    let v{i} = {i};")))
        .collect();
    let nums: Vec<usize> = (0..60).collect();
    assert!(kinds(&Judge::new(cfg).review(&diff, &nums)).contains(&FindingKind::ExcessiveChurn));
}

#[test]
fn exactly_one_retry_then_abort() {
    let mut judge = Judge::with_defaults();
    assert!(judge.may_retry(), "a fresh judge permits the first attempt");
    assert!(
        judge.record_failure(),
        "first failure still allows one retry"
    );
    assert!(!judge.may_retry());
    assert!(
        !judge.record_failure(),
        "second failure must exhaust the budget"
    );
    assert_eq!(judge.retries_used(), 2);
}

#[test]
fn feedback_is_specific_and_line_anchored() {
    let diff = vec![added("src/deep/nested/mod.rs", "    let v = x.unwrap();")];
    match Judge::with_defaults().review(&diff, &[142]) {
        Verdict::Reject(findings) => {
            let text = Judge::feedback(&findings);
            assert!(text.contains("src/deep/nested/mod.rs:142"), "got: {text}");
            assert!(text.contains("unsafe-in-production"), "got: {text}");
        }
        other => panic!("expected reject, got {other:?}"),
    }
}

#[test]
fn multiple_findings_are_all_reported() {
    let diff = vec![
        added("src/a.rs", "    // fixed bug"),
        added("src/b.rs", "    let v = x.unwrap();"),
        added("src/c.rs", "    todo!()"),
    ];
    match Judge::with_defaults().review(&diff, &[1, 1, 1]) {
        Verdict::Reject(f) => {
            assert!(f.len() >= 3, "each violation must be reported: {f:?}");
            assert!(f.iter().any(|x| x.kind == FindingKind::SlopComment));
            assert!(f.iter().any(|x| x.kind == FindingKind::UnsafeInProduction));
            assert!(f.iter().any(|x| x.kind == FindingKind::Placeholder));
        }
        other => panic!("expected reject, got {other:?}"),
    }
}

#[test]
fn empty_diff_passes() {
    assert_eq!(Judge::with_defaults().review(&[], &[]), Verdict::Pass);
}

#[test]
fn finding_display_is_human_readable() {
    let f = Finding {
        kind: FindingKind::SlopComment,
        path: "src/x.rs".to_string(),
        line: 12,
        detail: "boilerplate".to_string(),
    };
    let text = f.to_string();
    assert!(text.starts_with("src/x.rs:12"));
    assert!(text.contains("slop-comment"));
}
