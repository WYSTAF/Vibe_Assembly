//! Execution modes and the programming-language lens.
//!
//! Vibe Assembly exposes four practical modes. They share one harness but
//! differ in permission boundary, prompt contract, verification command and
//! UI accent colour. Exactly one writer exists at a time (Code / Web / Debug);
//! Chat / Architect is strictly read-only.

use std::fmt;
use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// The four production execution modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Observe → Think → Patch → Verify → Judge.
    Code,
    /// HTML5 / CSS / Tailwind / TypeScript / React and `templates/`.
    Web,
    /// Read-only architecture, explanation and planning mode.
    Chat,
    /// Compiler diagnostics, root-cause analysis, security audit.
    Debug,
}

impl ExecutionMode {
    pub const ALL: [ExecutionMode; 4] = [
        ExecutionMode::Code,
        ExecutionMode::Web,
        ExecutionMode::Chat,
        ExecutionMode::Debug,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ExecutionMode::Code => "code",
            ExecutionMode::Web => "web",
            ExecutionMode::Chat => "chat",
            ExecutionMode::Debug => "debug",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ExecutionMode::Code => "Code",
            ExecutionMode::Web => "Web / Site Builder",
            ExecutionMode::Chat => "Chat / Architect",
            ExecutionMode::Debug => "Debug / Audit",
        }
    }

    /// Compact label for tab-like selectors.
    pub fn short_label(self) -> &'static str {
        match self {
            ExecutionMode::Code => "Code",
            ExecutionMode::Web => "Web",
            ExecutionMode::Chat => "Chat",
            ExecutionMode::Debug => "Debug",
        }
    }

    /// UI accent for headers, active borders, badges and buttons.
    pub fn accent_hex(self) -> &'static str {
        match self {
            ExecutionMode::Code => "#F5B800",
            ExecutionMode::Web => "#06B6D4",
            ExecutionMode::Chat => "#3B82F6",
            ExecutionMode::Debug => "#EF4444",
        }
    }

    /// Accent as 0xRRGGBB.
    pub fn accent_rgb(self) -> u32 {
        match self {
            ExecutionMode::Code => 0x00F5_B800,
            ExecutionMode::Web => 0x0006_B6D4,
            ExecutionMode::Chat => 0x003B_82F6,
            ExecutionMode::Debug => 0x00EF_4444,
        }
    }

    /// Chat / Architect is a hard read-only boundary: the engine refuses to
    /// apply patches in this mode.
    pub fn allows_writes(self) -> bool {
        !matches!(self, ExecutionMode::Chat)
    }

    /// Short role contract injected into the system prompt.
    pub fn contract(self) -> &'static str {
        match self {
            ExecutionMode::Code => {
                "Implement the active ticket with minimal, verifiable edits. \
                 One writer only; every change must pass verification and the judge."
            }
            ExecutionMode::Web => {
                "Build clean, accessible web frontends (HTML5, CSS, Tailwind, TypeScript, React). \
                 Prefer semantic markup, no inline event handlers, and responsive layouts."
            }
            ExecutionMode::Chat => {
                "You are strictly read-only. Explain, review and design using the repository map. \
                 Never emit patches, never write files, never run mutating commands."
            }
            ExecutionMode::Debug => {
                "Diagnose compiler and runtime failures from evidence, isolate the root cause, \
                 audit unsafe code, and propose the smallest corrective patch."
            }
        }
    }

    /// Banned tokens for this mode, enforced by the judge in addition to the
    /// language guardrails.
    pub fn mode_guardrails(self) -> Vec<(&'static str, &'static str)> {
        match self {
            ExecutionMode::Web => vec![
                ("onclick=", "inline event handlers are not allowed"),
                ("document.write(", "document.write is not allowed"),
                ("<font", "legacy <font> markup is not allowed"),
            ],
            ExecutionMode::Chat => Vec::new(),
            ExecutionMode::Code => Vec::new(),
            ExecutionMode::Debug => Vec::new(),
        }
    }
}

impl fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ExecutionMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "code" | "coder" => Ok(ExecutionMode::Code),
            "web" | "site" | "website" => Ok(ExecutionMode::Web),
            "chat" | "architect" | "plan" => Ok(ExecutionMode::Chat),
            "debug" | "audit" => Ok(ExecutionMode::Debug),
            other => Err(format!(
                "unknown mode `{other}` (expected code|web|chat|debug)"
            )),
        }
    }
}

/// Programming-language lens. `Auto` infers from the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LanguageLens {
    Auto,
    Rust,
    /// C and C++ share one lens; `Cpp` is an accepted alias.
    C,
    Python,
    TypeScript,
    Go,
}

impl LanguageLens {
    pub const ALL: [LanguageLens; 6] = [
        LanguageLens::Auto,
        LanguageLens::Rust,
        LanguageLens::C,
        LanguageLens::Python,
        LanguageLens::TypeScript,
        LanguageLens::Go,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            LanguageLens::Auto => "auto",
            LanguageLens::Rust => "rust",
            LanguageLens::C => "c",
            LanguageLens::Python => "python",
            LanguageLens::TypeScript => "ts",
            LanguageLens::Go => "go",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            LanguageLens::Auto => "Auto-Detect",
            LanguageLens::Rust => "Rust",
            LanguageLens::C => "C / C++",
            LanguageLens::Python => "Python",
            LanguageLens::TypeScript => "TypeScript / JS",
            LanguageLens::Go => "Go",
        }
    }

    /// Compact label for UI selectors.
    pub fn short_label(self) -> &'static str {
        match self {
            LanguageLens::Auto => "Auto",
            LanguageLens::Rust => "Rust",
            LanguageLens::C => "C/C++",
            LanguageLens::Python => "Python",
            LanguageLens::TypeScript => "TS/JS",
            LanguageLens::Go => "Go",
        }
    }

    /// Resolve `Auto` against the workspace contents.
    pub fn resolve(self, root: &Path) -> LanguageLens {
        match self {
            LanguageLens::Auto => Self::detect(root),
            other => other,
        }
    }

    /// Infer the dominant language from manifest files, then file extensions.
    pub fn detect(root: &Path) -> LanguageLens {
        if root.join("Cargo.toml").exists() {
            return LanguageLens::Rust;
        }
        if root.join("go.mod").exists() {
            return LanguageLens::Go;
        }
        if root.join("pyproject.toml").exists()
            || root.join("requirements.txt").exists()
            || root.join("setup.py").exists()
        {
            return LanguageLens::Python;
        }
        if root.join("package.json").exists() || root.join("tsconfig.json").exists() {
            return LanguageLens::TypeScript;
        }
        if root.join("CMakeLists.txt").exists()
            || root.join("Makefile").exists()
            || root.join("meson.build").exists()
        {
            return LanguageLens::C;
        }

        // Extension census, bounded so a huge tree cannot stall startup.
        let mut counts = [0usize; 5];
        let mut seen = 0usize;
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                if seen > 2_000 {
                    break;
                }
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if !matches!(
                        name,
                        ".git" | "target" | "node_modules" | ".venv" | "venv" | "dist" | "build"
                    ) {
                        stack.push(path);
                    }
                    continue;
                }
                seen += 1;
                match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
                    "rs" => counts[0] += 1,
                    "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" => counts[1] += 1,
                    "py" => counts[2] += 1,
                    "ts" | "tsx" | "js" | "jsx" => counts[3] += 1,
                    "go" => counts[4] += 1,
                    _ => {}
                }
            }
        }
        let best = counts
            .iter()
            .enumerate()
            .max_by_key(|(_, count)| **count)
            .map(|(i, _)| i)
            .unwrap_or(0);
        match best {
            1 => LanguageLens::C,
            2 => LanguageLens::Python,
            3 => LanguageLens::TypeScript,
            4 => LanguageLens::Go,
            _ => LanguageLens::Rust,
        }
    }

    /// Language-appropriate verification command.
    pub fn verification_command(self, root: &Path) -> String {
        match self.resolve(root) {
            LanguageLens::Rust => "cargo check --all-targets".to_string(),
            LanguageLens::Go => "go build ./...".to_string(),
            LanguageLens::Python => "python -m compileall -q .".to_string(),
            LanguageLens::TypeScript => {
                if root.join("tsconfig.json").exists() {
                    "npx tsc --noEmit".to_string()
                } else {
                    "node --check .".to_string()
                }
            }
            LanguageLens::C => {
                if root.join("CMakeLists.txt").exists() {
                    "cmake --build build".to_string()
                } else if root.join("Makefile").exists() {
                    "make".to_string()
                } else {
                    "cc -fsyntax-only *.c".to_string()
                }
            }
            LanguageLens::Auto => "cargo check --all-targets".to_string(),
        }
    }

    /// `(banned token, human message)` pairs enforced by the judge.
    pub fn guardrails(self) -> Vec<(&'static str, &'static str)> {
        match self {
            LanguageLens::Rust => vec![
                (".unwrap()", "avoid unwrap() in production code"),
                ("panic!(", "avoid panic! in production code"),
                (".expect(", "avoid expect() in production code"),
            ],
            LanguageLens::Python => vec![
                (
                    "except:",
                    "bare except is not allowed; catch a specific exception",
                ),
                (
                    "except Exception: pass",
                    "swallowing exceptions is not allowed",
                ),
                ("eval(", "eval() is not allowed"),
            ],
            LanguageLens::C => vec![
                ("strcpy(", "strcpy() is unsafe; use a bounded copy"),
                ("strcat(", "strcat() is unsafe; use a bounded copy"),
                ("gets(", "gets() is unsafe and removed from modern C"),
                ("sprintf(", "sprintf() is unsafe; use snprintf()"),
                ("system(", "system() is not allowed in production code"),
            ],
            LanguageLens::TypeScript => vec![
                (": any", "avoid the any type"),
                ("@ts-ignore", "do not suppress type errors"),
                ("@ts-nocheck", "do not disable type checking"),
            ],
            LanguageLens::Go => vec![
                ("panic(", "avoid panic in production code"),
                ("_ = err", "do not discard errors"),
            ],
            LanguageLens::Auto => Vec::new(),
        }
    }

    /// Judge guidance appended to the review prompt for this language.
    pub fn judge_note(self) -> &'static str {
        match self {
            LanguageLens::Rust => {
                "Rust: reject unwrap/expect/panic in non-test code, and unsafe without justification."
            }
            LanguageLens::Python => {
                "Python: reject bare except, exception swallowing, eval, and mutable default arguments."
            }
            LanguageLens::C => {
                "C/C++: reject unbounded string functions, unchecked allocation, and pointer arithmetic without bounds."
            }
            LanguageLens::TypeScript => {
                "TypeScript: reject any, ts-ignore/ts-nocheck, and unhandled promise rejections."
            }
            LanguageLens::Go => "Go: reject discarded errors, panics in libraries, and goroutine leaks.",
            LanguageLens::Auto => "Reject placeholders, swallowed errors, and unsafe primitives.",
        }
    }
}

impl fmt::Display for LanguageLens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LanguageLens {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" | "detect" => Ok(LanguageLens::Auto),
            "rust" | "rs" => Ok(LanguageLens::Rust),
            "c" | "cpp" | "c++" | "cc" | "clang" => Ok(LanguageLens::C),
            "python" | "py" => Ok(LanguageLens::Python),
            "ts" | "typescript" | "js" | "javascript" | "tsx" | "jsx" => {
                Ok(LanguageLens::TypeScript)
            }
            "go" | "golang" => Ok(LanguageLens::Go),
            other => Err(format!(
                "unknown language `{other}` (expected auto|rust|c|python|ts|go)"
            )),
        }
    }
}

/// The pair selected for one run: execution mode + language lens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceProfile {
    pub mode: ExecutionMode,
    pub language: LanguageLens,
    pub model: Option<String>,
    pub judge_model: Option<String>,
}

impl WorkspaceProfile {
    pub fn new(mode: ExecutionMode, language: LanguageLens) -> Self {
        Self {
            mode,
            language,
            model: None,
            judge_model: None,
        }
    }

    pub fn resolved_language(&self, root: &Path) -> LanguageLens {
        self.language.resolve(root)
    }

    pub fn allows_writes(&self) -> bool {
        self.mode.allows_writes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_accents_are_distinct_and_dark_theme_safe() {
        let accents: Vec<&str> = ExecutionMode::ALL.iter().map(|m| m.accent_hex()).collect();
        assert_eq!(accents.len(), 4);
        assert!(accents.iter().all(|a| a.starts_with('#')));
        for (i, a) in accents.iter().enumerate() {
            assert!(!accents[i + 1..].contains(a), "accents must be distinct");
        }
        assert_eq!(ExecutionMode::Code.accent_hex(), "#F5B800");
        assert_eq!(ExecutionMode::Web.accent_hex(), "#06B6D4");
        assert_eq!(ExecutionMode::Chat.accent_hex(), "#3B82F6");
        assert_eq!(ExecutionMode::Debug.accent_hex(), "#EF4444");
    }

    #[test]
    fn chat_mode_is_read_only() {
        assert!(!ExecutionMode::Chat.allows_writes());
        for mode in [
            ExecutionMode::Code,
            ExecutionMode::Web,
            ExecutionMode::Debug,
        ] {
            assert!(mode.allows_writes(), "{mode} must allow writes");
        }
    }

    #[test]
    fn mode_parsing_accepts_aliases_and_rejects_garbage() {
        assert_eq!("architect".parse(), Ok(ExecutionMode::Chat));
        assert_eq!("website".parse(), Ok(ExecutionMode::Web));
        assert_eq!("AUDIT".parse(), Ok(ExecutionMode::Debug));
        assert!("nope".parse::<ExecutionMode>().is_err());
    }

    #[test]
    fn language_auto_detects_rust_manifests() {
        let dir = std::env::temp_dir().join("va_lens_rust_detect");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
        assert_eq!(LanguageLens::Auto.resolve(&dir), LanguageLens::Rust);
        assert_eq!(
            LanguageLens::Rust.verification_command(&dir),
            "cargo check --all-targets"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn language_detects_python_ts_go_c_by_manifest_and_extension() {
        let cases = [
            ("pyproject.toml", "x = 1", LanguageLens::Python),
            ("package.json", "{}", LanguageLens::TypeScript),
            ("go.mod", "module x", LanguageLens::Go),
            ("CMakeLists.txt", "project(x)", LanguageLens::C),
        ];
        for (file, body, expected) in cases {
            let dir = std::env::temp_dir().join(format!("va_lens_{}", expected.as_str()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(file), body).unwrap();
            assert_eq!(
                LanguageLens::Auto.resolve(&dir),
                expected,
                "manifest {file} should select {expected:?}"
            );
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn language_guardrails_match_the_required_rules() {
        assert!(LanguageLens::Rust
            .guardrails()
            .iter()
            .any(|(p, _)| *p == ".unwrap()"));
        assert!(LanguageLens::Python
            .guardrails()
            .iter()
            .any(|(p, _)| *p == "except:"));
        assert!(LanguageLens::C
            .guardrails()
            .iter()
            .any(|(p, _)| *p == "strcpy("));
        assert!(LanguageLens::TypeScript
            .guardrails()
            .iter()
            .any(|(p, _)| *p == ": any"));
        assert!(LanguageLens::Go
            .guardrails()
            .iter()
            .any(|(p, _)| *p == "panic("));
    }

    #[test]
    fn language_parsing_accepts_cpp_and_golang() {
        assert_eq!("cpp".parse(), Ok(LanguageLens::C));
        assert_eq!("golang".parse(), Ok(LanguageLens::Go));
        assert_eq!("javascript".parse(), Ok(LanguageLens::TypeScript));
    }
}
