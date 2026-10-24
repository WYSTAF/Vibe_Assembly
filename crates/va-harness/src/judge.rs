//! Strict anti-slop and minimalism quality gatekeeper.
//!
//! Inspects a diff before commit authorization and rejects changes that are
//! noisy rather than necessary. The Code role gets exactly one targeted
//! retry; a second failure aborts.
//!
//! Design note: the upstream (`pi`) implements its reviewer as an LLM loop that
//! re-prompts until a `DONE` sentinel appears. That mechanism is reproduced in
//! `review.rs` at the orchestration level. This module is the *deterministic*
//! gate that runs before any model is consulted — cheap, exhaustive, and
//! impossible to talk past.

use std::collections::BTreeMap;

/// One line of a unified diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    /// `+`, `-`, or ` ` (context).
    pub marker: char,
    pub path: String,
    pub text: String,
}

impl DiffLine {
    pub fn added(&self) -> bool {
        self.marker == '+'
    }
    pub fn removed(&self) -> bool {
        self.marker == '-'
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Approved for commit.
    Pass,
    /// Rejected; the Code role gets one targeted retry.
    Reject(Vec<Finding>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingKind {
    /// Boilerplate AI narration ("// Fixed bug", "// Added null check").
    SlopComment,
    /// A placeholder left behind (TODO-as-stub, unimplemented!, empty body).
    Placeholder,
    /// `unwrap()` / `panic!` / `expect()` outside tests.
    UnsafeInProduction,
    /// Whole-file or large-scale reformat riding along with a change.
    ExcessiveChurn,
    /// Trailing whitespace introduced.
    TrailingWhitespace,
    /// An import added, removed, or reordered.
    ImportChurn,
    /// A language- or mode-specific guardrail token (see `modes.rs`).
    LanguageGuardrail,
}

impl FindingKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            FindingKind::SlopComment => "slop-comment",
            FindingKind::Placeholder => "placeholder",
            FindingKind::UnsafeInProduction => "unsafe-in-production",
            FindingKind::ExcessiveChurn => "excessive-churn",
            FindingKind::TrailingWhitespace => "trailing-whitespace",
            FindingKind::ImportChurn => "import-churn",
            FindingKind::LanguageGuardrail => "language-guardrail",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub kind: FindingKind,
    pub path: String,
    pub line: usize,
    pub detail: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}: {} — {}",
            self.path,
            self.line,
            self.kind.as_str(),
            self.detail
        )
    }
}

#[derive(Debug, Clone)]
pub struct JudgeConfig {
    /// Maximum added+removed lines before the diff is called churn.
    pub max_changed_lines: usize,
    /// Allow `unwrap()` etc. in these paths (tests, benches, examples).
    pub unsafe_allowed_in: Vec<String>,
    /// Allow boilerplate-looking comments inside doc comments (`///`).
    pub allow_doc_comments: bool,
    /// `(token, message)` guardrails injected by the language lens and the
    /// active execution mode.
    pub extra_guardrails: Vec<(String, String)>,
}

impl Default for JudgeConfig {
    fn default() -> Self {
        Self {
            max_changed_lines: 400,
            unsafe_allowed_in: vec![
                "test".to_string(),
                "tests".to_string(),
                "bench".to_string(),
                "benches".to_string(),
                "examples".to_string(),
            ],
            allow_doc_comments: true,
            extra_guardrails: Vec::new(),
        }
    }
}

impl JudgeConfig {
    /// Attach the active language/mode guardrails.
    pub fn with_guardrails(
        mut self,
        guardrails: impl IntoIterator<Item = (String, String)>,
    ) -> Self {
        self.extra_guardrails = guardrails.into_iter().collect();
        self
    }
}

/// Phrases that mark a comment as narration about the change rather than
/// documentation of the code. Matched case-insensitively on the comment body.
const SLOP_PHRASES: &[&str] = &[
    "added null check",
    "added check",
    "fix bug",
    "fixed bug",
    "bug fix",
    "update code",
    "updated code",
    "improved",
    "improve performance",
    "cleanup",
    "clean up",
    "tweak",
    "refactor",
    "this should work",
    "should work now",
    "todo: fix",
    "as requested",
    "as per the request",
    "note to self",
    "temporary",
    "temp fix",
    "quick fix",
];

/// Stub bodies that signal an unfinished implementation.
const PLACEHOLDER_MARKERS: &[&str] = &["unimplemented!", "todo!()", "panic!(\"not implemented\")"];

pub struct Judge {
    config: JudgeConfig,
    /// How many retries the Code role has consumed.
    retries_used: u32,
}

impl Judge {
    pub fn new(config: JudgeConfig) -> Self {
        Self {
            config,
            retries_used: 0,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(JudgeConfig::default())
    }

    pub fn retries_used(&self) -> u32 {
        self.retries_used
    }

    fn is_unsafe_allowed(&self, path: &str) -> bool {
        self.config
            .unsafe_allowed_in
            .iter()
            .any(|p| path.contains(p.as_str()))
    }

    /// Inspect a diff and return a verdict.
    ///
    /// `line_numbers` maps each diff line to its 1-based line number in the
    /// new file (added lines) or old file (removed lines).
    pub fn review(&self, diff: &[DiffLine], line_numbers: &[usize]) -> Verdict {
        let mut findings = Vec::new();

        let changed = diff.iter().filter(|l| !l.marker.is_whitespace()).count();
        if changed > self.config.max_changed_lines {
            findings.push(Finding {
                kind: FindingKind::ExcessiveChurn,
                path: "*".to_string(),
                line: 0,
                detail: format!(
                    "{changed} changed lines exceeds the {} limit — split the change",
                    self.config.max_changed_lines
                ),
            });
        }

        let mut per_file: BTreeMap<&str, usize> = BTreeMap::new();
        for (i, line) in diff.iter().enumerate() {
            if line.marker.is_whitespace() {
                continue;
            }
            *per_file.entry(line.path.as_str()).or_default() += 1;
            let lineno = line_numbers.get(i).copied().unwrap_or(0);

            if line.added() {
                if let Some(f) = self.check_added(line, lineno) {
                    findings.push(f);
                }
            }
        }

        // Import churn: a file whose only changes are import lines.
        for (path, count) in &per_file {
            let touching: Vec<&DiffLine> = diff
                .iter()
                .filter(|l| !l.marker.is_whitespace() && l.path == *path)
                .collect();
            if touching.is_empty() {
                continue;
            }
            let all_imports = touching.iter().all(|l| is_import_line(&l.text));
            if all_imports && *count > 0 {
                findings.push(Finding {
                    kind: FindingKind::ImportChurn,
                    path: (*path).to_string(),
                    line: 0,
                    detail: "every changed line in this file is an import — imports must change for a reason".to_string(),
                });
            }
        }

        if findings.is_empty() {
            Verdict::Pass
        } else {
            Verdict::Reject(findings)
        }
    }

    fn check_added(&self, line: &DiffLine, lineno: usize) -> Option<Finding> {
        let text = &line.text;
        let trimmed = text.trim_start();

        // Trailing whitespace on a newly added line.
        if text.len() != text.trim_end().len() && !text.is_empty() {
            return Some(Finding {
                kind: FindingKind::TrailingWhitespace,
                path: line.path.clone(),
                line: lineno,
                detail: "added line has trailing whitespace".to_string(),
            });
        }

        // Placeholders.
        for marker in PLACEHOLDER_MARKERS {
            if text.contains(marker) && !self.is_unsafe_allowed(&line.path) {
                return Some(Finding {
                    kind: FindingKind::Placeholder,
                    path: line.path.clone(),
                    line: lineno,
                    detail: format!("unfinished stub: `{marker}`"),
                });
            }
        }

        // Unsafe primitives outside test paths.
        if !self.is_unsafe_allowed(&line.path) {
            for token in ["unwrap()", "panic!(", ".expect("] {
                if text.contains(token) {
                    return Some(Finding {
                        kind: FindingKind::UnsafeInProduction,
                        path: line.path.clone(),
                        line: lineno,
                        detail: format!("`{token}` in a production path — use Result/Option"),
                    });
                }
            }
            // Language-lens and mode guardrails.
            for (token, message) in &self.config.extra_guardrails {
                if text.contains(token.as_str()) {
                    return Some(Finding {
                        kind: FindingKind::LanguageGuardrail,
                        path: line.path.clone(),
                        line: lineno,
                        detail: format!("`{token}` — {message}"),
                    });
                }
            }
        }

        // Slop comments. Doc comments (`///`, `//!`, `/**`) are exempt because
        // they document the API rather than narrate the diff.
        let is_comment =
            trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*');
        if is_comment {
            let is_doc = trimmed.starts_with("///")
                || trimmed.starts_with("//!")
                || trimmed.starts_with("/**");
            if !(is_doc && self.config.allow_doc_comments) {
                let body = trimmed
                    .trim_start_matches('/')
                    .trim_start_matches('*')
                    .to_lowercase();
                for phrase in SLOP_PHRASES {
                    if body.contains(phrase) {
                        return Some(Finding {
                            kind: FindingKind::SlopComment,
                            path: line.path.clone(),
                            line: lineno,
                            detail: format!(
                                "boilerplate comment narrating the change: \"{phrase}\""
                            ),
                        });
                    }
                }
            }
        }

        None
    }

    /// Record that the Code role failed a review. Returns `false` once the
    /// single permitted retry is spent.
    pub fn record_failure(&mut self) -> bool {
        self.retries_used += 1;
        self.retries_used <= 1
    }

    /// Whether the Code role may attempt another submission.
    pub fn may_retry(&self) -> bool {
        self.retries_used < 1
    }

    /// Format findings as targeted, actionable feedback — not a wall of text.
    pub fn feedback(findings: &[Finding]) -> String {
        findings
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn is_import_line(text: &str) -> bool {
    let t = text.trim();
    t.starts_with("use ")
        || t.starts_with("pub use ")
        || t.starts_with("import ")
        || t.starts_with("from ")
        || t.starts_with("extern crate")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn added(path: &str, text: &str) -> DiffLine {
        DiffLine {
            marker: '+',
            path: path.to_string(),
            text: text.to_string(),
        }
    }

    #[test]
    fn passes_a_clean_diff() {
        let diff = vec![
            added("src/main.rs", "fn main() {"),
            added("src/main.rs", "    println!(\"hi\");"),
            added("src/main.rs", "}"),
        ];
        let nums = vec![1, 2, 3];
        assert_eq!(Judge::with_defaults().review(&diff, &nums), Verdict::Pass);
    }

    #[test]
    fn rejects_slop_comment() {
        let diff = vec![added("src/lib.rs", "    // Fixed bug here")];
        let v = Judge::with_defaults().review(&diff, &[10]);
        match v {
            Verdict::Reject(f) => {
                assert_eq!(f.len(), 1);
                assert_eq!(f[0].kind, FindingKind::SlopComment);
            }
            other => panic!("expected reject, got {other:?}"),
        }
    }

    #[test]
    fn allows_doc_comments_with_narration_words() {
        let diff = vec![added("src/lib.rs", "    /// Returns the improved value.")];
        assert_eq!(Judge::with_defaults().review(&diff, &[1]), Verdict::Pass);
    }

    #[test]
    fn rejects_unsafe_in_production_but_allows_in_tests() {
        let prod = vec![added("src/service.rs", "    let v = x.unwrap();")];
        assert!(matches!(
            Judge::with_defaults().review(&prod, &[5]),
            Verdict::Reject(_)
        ));

        let test = vec![added("tests/it.rs", "    let v = x.unwrap();")];
        assert_eq!(Judge::with_defaults().review(&test, &[5]), Verdict::Pass);
    }

    #[test]
    fn rejects_placeholders() {
        let diff = vec![added("src/engine.rs", "    todo!()")];
        match Judge::with_defaults().review(&diff, &[7]) {
            Verdict::Reject(f) => assert_eq!(f[0].kind, FindingKind::Placeholder),
            other => panic!("expected reject, got {other:?}"),
        }
    }

    #[test]
    fn rejects_import_only_churn() {
        let diff = vec![
            added("src/a.rs", "use std::collections::HashMap;"),
            added("src/a.rs", "use std::fmt;"),
        ];
        match Judge::with_defaults().review(&diff, &[1, 2]) {
            Verdict::Reject(f) => {
                assert!(f.iter().any(|x| x.kind == FindingKind::ImportChurn));
            }
            other => panic!("expected reject, got {other:?}"),
        }
    }

    #[test]
    fn rejects_excessive_churn() {
        let mut diff = Vec::new();
        for i in 0..50 {
            diff.push(added("src/big.rs", &format!("    let v{i} = {i};")));
        }
        let cfg = JudgeConfig {
            max_changed_lines: 10,
            ..Default::default()
        };
        match Judge::new(cfg).review(&diff, &vec![1; 50]) {
            Verdict::Reject(f) => {
                assert!(f.iter().any(|x| x.kind == FindingKind::ExcessiveChurn));
            }
            other => panic!("expected reject, got {other:?}"),
        }
    }

    #[test]
    fn allows_exactly_one_retry() {
        let mut judge = Judge::with_defaults();
        assert!(judge.may_retry());
        assert!(
            judge.record_failure(),
            "first failure still permits a retry"
        );
        assert!(!judge.may_retry());
        assert!(
            !judge.record_failure(),
            "second failure exhausts the budget"
        );
    }

    #[test]
    fn feedback_is_line_delimited_and_specific() {
        let f = vec![Finding {
            kind: FindingKind::UnsafeInProduction,
            path: "src/x.rs".to_string(),
            line: 42,
            detail: "unwrap in production".to_string(),
        }];
        let text = Judge::feedback(&f);
        assert!(text.contains("src/x.rs:42"));
        assert!(text.contains("unwrap in production"));
    }
}
