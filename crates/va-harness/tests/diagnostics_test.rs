//! Tests for Compiler Diagnostic-to-Block Pinpointer (`va-harness::diagnostics`).

use std::path::PathBuf;
use va_harness::diagnostics::{DiagnosticPinpointer, PinnedDiagnostic, RawDiagnostic};
use va_harness::patcher::{segment_blocks, short_hash};

fn sample_source_file() -> &'static str {
    r#"// Header comment
use std::collections::HashMap;

pub fn calculate_metrics(values: &[i32]) -> i32 {
    let sum: i32 = values.iter().sum();
    sum
}

pub fn broken_function(x: &str) -> usize {
    x.invalid_method_call()
}

pub fn helper() -> bool {
    true
}
"#
}

#[test]
fn test_parse_json_compiler_diagnostics() {
    let json_output = r#"
{"reason":"compiler-artifact","package_id":"sample 0.1.0","target":{"kind":["lib"],"name":"sample"}}
{"reason":"compiler-message","package_id":"sample 0.1.0","target":{"kind":["lib"],"name":"sample"},"message":{"children":[],"code":{"code":"E0599","explanation":"no method named..."},"level":"error","message":"no method named `invalid_method_call` found for reference `&str` in the current scope","rendered":"error[E0599]: ...","spans":[{"byte_end":185,"byte_start":164,"column_end":27,"column_start":7,"file_name":"src/sample.rs","is_primary":true,"line_end":10,"line_start":10,"suggested_replacement":null,"suggestion_applicability":null,"text":[{"highlight_end":27,"highlight_start":7,"text":"    x.invalid_method_call()"}]}]}}
"#;

    let parsed = DiagnosticPinpointer::parse_output(json_output);
    assert_eq!(parsed.len(), 1);
    let diag = &parsed[0];
    assert_eq!(diag.file_path, PathBuf::from("src/sample.rs"));
    assert_eq!(diag.line_start, 10);
    assert_eq!(diag.line_end, 10);
    assert_eq!(diag.error_code.as_deref(), Some("E0599"));
    assert_eq!(diag.level, "error");
    assert!(diag
        .message
        .contains("no method named `invalid_method_call`"));
}

#[test]
fn test_parse_text_compiler_diagnostics() {
    let text_output = r#"
error[E0432]: unresolved import `va_harness::missing`
 --> src/lib.rs:5:5
  |
5 | use va_harness::missing;
  |     ^^^^^^^^^^^^^^^^^^^ no `missing` in the root

warning: unused variable: `unused_var`
  --> src/lib.rs:15:9
   |
15 |     let unused_var = 42;
   |         ^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_unused_var`
"#;

    let parsed = DiagnosticPinpointer::parse_output(text_output);
    assert!(parsed.len() >= 2);

    let err = parsed.iter().find(|d| d.level == "error").unwrap();
    assert_eq!(err.file_path, PathBuf::from("src/lib.rs"));
    assert_eq!(err.line_start, 5);
    assert_eq!(err.error_code.as_deref(), Some("E0432"));
    assert!(err.message.contains("unresolved import"));

    let warn = parsed.iter().find(|d| d.level == "warning").unwrap();
    assert_eq!(warn.file_path, PathBuf::from("src/lib.rs"));
    assert_eq!(warn.line_start, 15);
    assert!(warn.message.contains("unused variable"));
}

#[test]
fn test_block_hash_pinpointing_and_snippet() {
    let temp_dir = std::env::temp_dir().join(format!(
        "va_diag_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::create_dir_all(&temp_dir);
    let file_path = temp_dir.join("sample.rs");
    let content = sample_source_file();
    std::fs::write(&file_path, content).unwrap();

    let blocks = segment_blocks(content);
    let broken_block = blocks
        .iter()
        .find(|b| b.signature.contains("broken_function"))
        .unwrap();

    let raw = vec![RawDiagnostic {
        file_path: PathBuf::from("sample.rs"),
        line_start: 10,
        line_end: 10,
        error_code: Some("E0599".into()),
        level: "error".into(),
        message: "no method named `invalid_method_call`".into(),
    }];

    let pinned = DiagnosticPinpointer::pin_diagnostics(&temp_dir, &raw);
    assert_eq!(pinned.len(), 1);

    let p = &pinned[0];
    assert_eq!(p.block_hash, broken_block.hash);
    assert_eq!(p.block_short_hash, short_hash(broken_block.hash));
    assert_eq!(
        p.line_range,
        (broken_block.start_line, broken_block.end_line)
    );
    assert!(p.snippet.contains("pub fn broken_function"));
    assert!(p.snippet.contains("invalid_method_call"));

    // Check feedback rendering
    let feedback = PinnedDiagnostic::format_feedback(&pinned);
    assert!(feedback.contains("[E0599]"));
    assert!(feedback.contains(&p.block_short_hash));
    assert!(feedback.contains("Current Block Snippet"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
