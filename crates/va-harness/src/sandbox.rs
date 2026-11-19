//! Sandboxed verification executor.
//!
//! Provides confined command execution for verification and tools:
//! * Configurable wall-clock timeouts.
//! * Working-directory confinement (prevents command execution outside workspace root).
//! * Environment sanitization (clears untrusted vars; passes whitelisted toolchain vars).
//! * Bounded stdout/stderr capture (prevents memory blowout from chatty processes).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Workspace root that confines execution.
    pub root: PathBuf,
    /// Wall-clock timeout for the command.
    pub timeout: Duration,
    /// Maximum bytes of stdout/stderr to capture before truncating.
    pub max_output_bytes: usize,
    /// Whitelisted environment variables passed through to the child process.
    pub env_whitelist: Vec<String>,
}

impl SandboxConfig {
    pub fn default_for(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            // Compile-style verification (cargo check, tsc, cmake) routinely
            // exceeds 30s on a cold workspace; a short default turns healthy
            // builds into false verification failures.
            timeout: Duration::from_secs(300),
            max_output_bytes: 64 * 1024,
            env_whitelist: vec![
                "PATH".into(),
                "SYSTEMROOT".into(),
                "SYSTEMDRIVE".into(),
                "WINDIR".into(),
                "COMSPEC".into(),
                "PATHEXT".into(),
                "TEMP".into(),
                "TMP".into(),
                "USERPROFILE".into(),
                "HOME".into(),
                "ALLUSERSPROFILE".into(),
                "PROGRAMDATA".into(),
                "NUMBER_OF_PROCESSORS".into(),
                "PROCESSOR_ARCHITECTURE".into(),
                "OS".into(),
                "CARGO_HOME".into(),
                "RUSTUP_HOME".into(),
                "RUST_LOG".into(),
                "LANG".into(),
                "LC_ALL".into(),
            ],
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_max_output_bytes(mut self, bytes: usize) -> Self {
        self.max_output_bytes = bytes;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub duration: Duration,
}

#[derive(Debug)]
pub enum SandboxError {
    OutsideRoot { path: PathBuf, root: PathBuf },
    SpawnFailed(std::io::Error),
    Io(std::io::Error),
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxError::OutsideRoot { path, root } => {
                write!(
                    f,
                    "confinement violation: path `{}` is outside workspace root `{}`",
                    path.display(),
                    root.display()
                )
            }
            SandboxError::SpawnFailed(e) => write!(f, "failed to spawn process: {e}"),
            SandboxError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for SandboxError {}

pub struct SandboxExecutor {
    config: SandboxConfig,
}

impl SandboxExecutor {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    /// Check if target_dir is strictly inside config.root.
    pub fn check_confinement(&self, target_dir: &Path) -> Result<PathBuf, SandboxError> {
        let canonical_root = self
            .config
            .root
            .canonicalize()
            .unwrap_or_else(|_| self.config.root.clone());
        let canonical_target = target_dir
            .canonicalize()
            .unwrap_or_else(|_| target_dir.to_path_buf());

        if canonical_target.starts_with(&canonical_root) {
            Ok(canonical_target)
        } else {
            Err(SandboxError::OutsideRoot {
                path: target_dir.to_path_buf(),
                root: self.config.root.clone(),
            })
        }
    }

    /// Execute a command line in cwd, confined and bounded.
    pub fn execute(
        &self,
        command: &str,
        cwd: Option<&Path>,
    ) -> Result<SandboxResult, SandboxError> {
        let work_dir = match cwd {
            Some(d) => self.check_confinement(d)?,
            None => self.config.root.clone(),
        };

        let (prog, arg) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let start = std::time::Instant::now();

        let mut cmd = std::process::Command::new(prog);
        cmd.arg(arg).arg(command);
        cmd.current_dir(&work_dir);

        // Sanitize environment: clear all, pass only whitelisted vars
        cmd.env_clear();
        for var in &self.config.env_whitelist {
            if let Ok(val) = std::env::var(var) {
                cmd.env(var, val);
            }
        }

        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(SandboxError::SpawnFailed)?;

        // Drain both pipes concurrently while the child runs. Reading only
        // after exit deadlocks any command that outwrites the OS pipe buffer
        // (~4-64KB): the child blocks on write, never exits, and gets killed
        // as a "timeout" even though it was healthy. Keep the first
        // max_output_bytes and discard the rest so capture stays bounded.
        let max = self.config.max_output_bytes;
        let out_handle = child
            .stdout
            .take()
            .map(|pipe| std::thread::spawn(move || drain_bounded(pipe, max)));
        let err_handle = child
            .stderr
            .take()
            .map(|pipe| std::thread::spawn(move || drain_bounded(pipe, max)));
        let join = |handle: Option<std::thread::JoinHandle<Vec<u8>>>| -> Vec<u8> {
            handle.and_then(|h| h.join().ok()).unwrap_or_default()
        };

        let timeout = self.config.timeout;

        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let stdout_buf = join(out_handle);
                    let stderr_buf = join(err_handle);
                    let stdout = truncate_output(&stdout_buf, self.config.max_output_bytes);
                    let stderr = truncate_output(&stderr_buf, self.config.max_output_bytes);

                    return Ok(SandboxResult {
                        success: status.success(),
                        exit_code: status.code(),
                        stdout,
                        stderr,
                        timed_out: false,
                        duration: start.elapsed(),
                    });
                }
                Ok(None) => {
                    if start.elapsed() >= timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        // Killing closes the pipes, so the drain threads finish.
                        let stdout_buf = join(out_handle);
                        let stderr_buf = join(err_handle);
                        return Ok(SandboxResult {
                            success: false,
                            exit_code: None,
                            stdout: truncate_output(&stdout_buf, self.config.max_output_bytes),
                            stderr: format!(
                                "Command timed out after {:?}\n{}",
                                timeout,
                                truncate_output(&stderr_buf, self.config.max_output_bytes)
                            ),
                            timed_out: true,
                            duration: start.elapsed(),
                        });
                    }
                    std::thread::sleep(Duration::from_millis(30));
                }
                Err(e) => return Err(SandboxError::Io(e)),
            }
        }
    }
}

/// Read a pipe to EOF, retaining at most `max_bytes + 1` (the extra byte
/// lets the caller detect and report overflow) and discarding the rest so a
/// chatty child never blocks on a full pipe.
fn drain_bounded(mut reader: impl Read, max_bytes: usize) -> Vec<u8> {
    let mut kept = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if kept.len() <= max_bytes {
                    let take = (max_bytes + 1 - kept.len()).min(n);
                    kept.extend_from_slice(&chunk[..take]);
                }
            }
        }
    }
    kept
}

fn truncate_output(bytes: &[u8], max_bytes: usize) -> String {
    if bytes.len() <= max_bytes {
        String::from_utf8_lossy(bytes).into_owned()
    } else {
        let truncated = String::from_utf8_lossy(&bytes[..max_bytes]);
        format!("{truncated}\n[... output truncated at {max_bytes} bytes ...]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confinement_inside_root() {
        let temp = std::env::temp_dir();
        let config = SandboxConfig::default_for(&temp);
        let sandbox = SandboxExecutor::new(config);
        assert!(sandbox.check_confinement(&temp).is_ok());
    }

    #[test]
    fn test_confinement_rejects_outside_root() {
        let root = std::env::temp_dir().join("va_sandbox_test_root_12345");
        let _ = std::fs::create_dir_all(&root);
        let outside = std::env::temp_dir().join("va_sandbox_test_outside_12345");
        let _ = std::fs::create_dir_all(&outside);

        let config = SandboxConfig::default_for(&root);
        let sandbox = SandboxExecutor::new(config);
        assert!(sandbox.check_confinement(&outside).is_err());

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn test_execution_success() {
        let temp = std::env::temp_dir();
        let config = SandboxConfig::default_for(&temp);
        let sandbox = SandboxExecutor::new(config);

        let cmd = "echo hello";
        let res = sandbox.execute(cmd, None).unwrap();
        assert!(res.success);
        assert!(res.stdout.contains("hello"));
        assert!(!res.timed_out);
    }

    #[test]
    fn test_bounded_output_truncates() {
        let bytes = b"0123456789abcdefghijklmnopqrstuvwxyz";
        let truncated = truncate_output(bytes, 10);
        assert!(truncated.starts_with("0123456789"));
        assert!(truncated.contains("output truncated at 10 bytes"));
    }

    #[test]
    fn chatty_commands_do_not_deadlock_on_full_pipes() {
        let temp = std::env::temp_dir();
        // ~120KB of stdout: far beyond any OS pipe buffer. Before concurrent
        // draining this blocked the child until the timeout killed it.
        let config = SandboxConfig::default_for(&temp).with_timeout(Duration::from_secs(15));
        let sandbox = SandboxExecutor::new(config);
        let command = if cfg!(windows) {
            "for /L %i in (1,1,3000) do @echo 0123456789012345678901234567890123456789"
        } else {
            "i=0; while [ $i -lt 3000 ]; do echo 0123456789012345678901234567890123456789; i=$((i+1)); done"
        };
        let res = sandbox.execute(command, None).unwrap();
        assert!(res.success, "chatty command must succeed: {}", res.stderr);
        assert!(!res.timed_out, "pipe deadlock masqueraded as a timeout");
        assert!(res.stdout.contains("output truncated"));
    }

    #[test]
    fn whitelist_keeps_windows_command_resolution_vars() {
        let config = SandboxConfig::default_for(&std::env::temp_dir());
        for var in ["PATHEXT", "COMSPEC", "SYSTEMDRIVE", "WINDIR"] {
            assert!(
                config.env_whitelist.iter().any(|v| v == var),
                "{var} must survive env_clear or cmd/npx resolution breaks"
            );
        }
        assert_eq!(config.timeout, Duration::from_secs(300));
    }
}
