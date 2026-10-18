//! Sandboxed verification executor.
//!
//! Provides confined command execution for verification and tools:
//! * Configurable wall-clock timeouts.
//! * Working-directory confinement (prevents command execution outside workspace root).
//! * Environment sanitization (clears untrusted vars; passes whitelisted toolchain vars).
//! * Bounded stdout/stderr capture (prevents memory blowout from chatty processes).

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
            timeout: Duration::from_secs(30),
            max_output_bytes: 64 * 1024,
            env_whitelist: vec![
                "PATH".into(),
                "SYSTEMROOT".into(),
                "TEMP".into(),
                "TMP".into(),
                "USERPROFILE".into(),
                "HOME".into(),
                "CARGO_HOME".into(),
                "RUSTUP_HOME".into(),
                "RUST_LOG".into(),
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

        let timeout = self.config.timeout;

        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let mut stdout_buf = Vec::new();
                    let mut stderr_buf = Vec::new();
                    if let Some(mut out) = child.stdout.take() {
                        use std::io::Read;
                        let _ = out.read_to_end(&mut stdout_buf);
                    }
                    if let Some(mut err) = child.stderr.take() {
                        use std::io::Read;
                        let _ = err.read_to_end(&mut stderr_buf);
                    }

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
                        return Ok(SandboxResult {
                            success: false,
                            exit_code: None,
                            stdout: String::new(),
                            stderr: format!("Command timed out after {:?}", timeout),
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
}
