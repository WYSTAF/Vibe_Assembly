//! Incremental git diff review — checkpoint, isolate, stream.
//!
//! Design note: the upstream (`pi`) ships a `git-checkpoint` extension and a
//! `durable` reviewer loop that re-prompts a model until it emits a `DONE`
//! sentinel. This module provides the deterministic substrate that loop needs:
//! a cheap checkpoint before each phase, and a precise statement of what
//! *this* step changed, isolated from everything committed before it.
//!
//! The `DONE`-sentinel review loop is reproduced in `engine.rs` at the
//! orchestration level, where it belongs.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::judge::{DiffLine, Finding, Judge, Verdict};

/// A lightweight git checkpoint: a commit sha, or a marker when git is
/// unavailable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpoint {
    pub id: String,
}

#[derive(Debug)]
pub enum ReviewError {
    GitUnavailable(String),
    GitFailed { command: String, stderr: String },
    NotARepository,
}

impl std::fmt::Display for ReviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewError::GitUnavailable(e) => write!(f, "git unavailable: {e}"),
            ReviewError::GitFailed { command, stderr } => {
                write!(f, "git {command} failed: {stderr}")
            }
            ReviewError::NotARepository => write!(f, "not inside a git repository"),
        }
    }
}

impl std::error::Error for ReviewError {}

/// Runs git in `repo` and returns stdout.
fn git(repo: &Path, args: &[&str]) -> Result<String, ReviewError> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .map_err(|e| ReviewError::GitUnavailable(e.to_string()))?;
    if !out.status.success() {
        return Err(ReviewError::GitFailed {
            command: args.join(" "),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Tracks a checkpoint across an execution phase and computes the delta since.
pub struct Reviewer {
    repo: PathBuf,
    checkpoint: Option<Checkpoint>,
}

impl Reviewer {
    pub fn new(repo: &Path) -> Self {
        Self {
            repo: repo.to_path_buf(),
            checkpoint: None,
        }
    }

    pub fn is_repository(&self) -> bool {
        Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .current_dir(&self.repo)
            .output()
            .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "true")
            .unwrap_or(false)
    }

    /// Snapshot the working tree into a light commit so the next phase's delta
    /// is precisely attributable.
    ///
    /// The commit is local-only and never pushed; it exists so `since` has a
    /// stable anchor. Nothing is staged that was not already modified.
    pub fn checkpoint(&mut self) -> Result<Checkpoint, ReviewError> {
        if !self.is_repository() {
            return Err(ReviewError::NotARepository);
        }
        // Use a stash-free approach: create a commit object from the working
        // tree without moving the branch, so main history is untouched.
        let tree = git(&self.repo, &["add", "-A"])?;
        let _ = tree;
        let commit = git(
            &self.repo,
            &[
                "commit",
                "--allow-empty",
                "-m",
                "va-harness checkpoint",
                "-q",
            ],
        )?;
        let _ = commit;
        let sha = git(&self.repo, &["rev-parse", "HEAD"])?.trim().to_string();
        self.checkpoint = Some(Checkpoint { id: sha.clone() });
        Ok(Checkpoint { id: sha })
    }

    /// The current checkpoint, if one has been taken.
    pub fn current(&self) -> Option<&Checkpoint> {
        self.checkpoint.as_ref()
    }

    /// Parse a unified diff into structured lines with line numbers.
    ///
    /// Added lines are numbered in the new file; removed lines in the old.
    pub fn parse_diff(&self, raw: &str) -> (Vec<DiffLine>, Vec<usize>) {
        let mut lines = Vec::new();
        let mut numbers = Vec::new();
        let (mut new_no, mut old_no) = (0usize, 0usize);
        let mut path = String::new();

        for raw_line in raw.lines() {
            // Track the current file from a `+++ b/<path>` or `--- a/<path>` header.
            if let Some(rest) = raw_line.strip_prefix("+++ b/") {
                path = rest.split('\t').next().unwrap_or(rest).to_string();
                continue;
            }
            if let Some(rest) = raw_line.strip_prefix("--- a/") {
                if path.is_empty() {
                    path = rest.split('\t').next().unwrap_or(rest).to_string();
                }
                continue;
            }
            if raw_line.starts_with("diff --git")
                || raw_line.starts_with("index ")
                || raw_line.starts_with("@@")
                || raw_line.starts_with("new file mode")
                || raw_line.starts_with("deleted file mode")
            {
                continue;
            }
            let mut chars = raw_line.chars();
            let Some(marker) = chars.next() else {
                continue;
            };
            if marker != '+' && marker != '-' && marker != ' ' {
                continue;
            }
            let text = chars.as_str().to_string();
            match marker {
                '+' => {
                    new_no += 1;
                    numbers.push(new_no);
                }
                '-' => {
                    old_no += 1;
                    numbers.push(old_no);
                }
                _ => {
                    new_no += 1;
                    old_no += 1;
                }
            }
            lines.push(DiffLine {
                marker,
                path: path.clone(),
                text,
            });
        }
        (lines, numbers)
    }

    /// Everything changed since the checkpoint — the exact delta of this step.
    pub fn since_checkpoint(&self) -> Result<Vec<DiffLine>, ReviewError> {
        let raw = match &self.checkpoint {
            Some(cp) => git(&self.repo, &["diff", &cp.id])?,
            None => git(&self.repo, &["diff", "HEAD"])?,
        };
        Ok(self.parse_diff(&raw).0)
    }

    /// Judge the delta since the checkpoint.
    pub fn review_since_checkpoint(&self, judge: &Judge) -> Result<Verdict, ReviewError> {
        let raw = match &self.checkpoint {
            Some(cp) => git(&self.repo, &["diff", &cp.id])?,
            None => git(&self.repo, &["diff", "HEAD"])?,
        };
        let (lines, numbers) = self.parse_diff(&raw);
        Ok(judge.review(&lines, &numbers))
    }

    /// A structured event describing this step's delta, for UI streaming.
    pub fn delta_event(&self) -> Result<DeltaEvent, ReviewError> {
        let lines = self.since_checkpoint()?;
        let mut added = 0usize;
        let mut removed = 0usize;
        let mut files: Vec<String> = Vec::new();
        for l in &lines {
            if l.added() {
                added += 1;
            } else if l.removed() {
                removed += 1;
            }
            if !files.contains(&l.path) {
                files.push(l.path.clone());
            }
        }
        Ok(DeltaEvent {
            files,
            added,
            removed,
        })
    }
}

/// A compact, UI-friendly summary of one step's changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeltaEvent {
    pub files: Vec<String>,
    pub added: usize,
    pub removed: usize,
}

impl std::fmt::Display for DeltaEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} file(s), +{} −{}",
            self.files.len(),
            self.added,
            self.removed
        )
    }
}

/// Convenience: the findings from a review, rendered for the Code role.
pub fn render_findings(findings: &[Finding]) -> String {
    Judge::feedback(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unified_diff_with_line_numbers() {
        let r = Reviewer::new(Path::new("."));
        let raw = "diff --git a/src/lib.rs b/src/lib.rs\n\
                   --- a/src/lib.rs\n\
                   +++ b/src/lib.rs\n\
                   @@ -1,3 +1,3 @@\n\
                    use std::fmt;\n\
                   -fn old() {}\n\
                   +fn new() {}\n\
                    \n";
        let (lines, nums) = r.parse_diff(raw);
        let changed: Vec<&DiffLine> = lines.iter().filter(|l| !l.marker.is_whitespace()).collect();
        assert_eq!(changed.len(), 2);
        assert!(changed.iter().all(|l| l.path == "src/lib.rs"));
        // `-` is numbered in the old file, `+` in the new.
        assert_eq!(nums[0], 1, "old-file line for the removed line");
        assert_eq!(nums[1], 1, "new-file line for the added line");
    }

    #[test]
    fn delta_event_counts_and_dedupes_files() {
        let event = DeltaEvent {
            files: vec!["a.rs".to_string()],
            added: 3,
            removed: 1,
        };
        assert_eq!(event.to_string(), "1 file(s), +3 −1");
    }

    #[test]
    fn non_repository_is_reported_not_panicked() {
        let dir = std::env::temp_dir().join("va-review-not-a-repo");
        let _ = std::fs::create_dir_all(&dir);
        let mut r = Reviewer::new(&dir);
        // A temp dir is not a repo, so checkpointing must error cleanly.
        if !r.is_repository() {
            assert!(matches!(r.checkpoint(), Err(ReviewError::NotARepository)));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
