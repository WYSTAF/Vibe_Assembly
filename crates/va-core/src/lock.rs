//! Single-writer lockfile guard (.ai/.task.lock) around .ai/active_task.md.
//! Preserves ADR-007 integrity by preventing concurrent mode writes and race conditions.

use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockInfo {
    pub pid: u32,
    pub holder: String,
    pub acquired_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockStatus {
    Unlocked,
    Held(LockInfo),
    Stale(LockInfo),
}

/// Check if a PID corresponds to an actively running process.
#[cfg(windows)]
pub fn is_pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    unsafe {
        #[link(name = "kernel32")]
        extern "system" {
            fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> isize;
            fn CloseHandle(hObject: isize) -> i32;
        }
        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle != 0 {
            CloseHandle(handle);
            true
        } else {
            false
        }
    }
}

#[cfg(not(windows))]
pub fn is_pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    // Unix signal 0 check
    unsafe {
        extern "C" {
            fn kill(pid: i32, sig: i32) -> i32;
        }
        kill(pid as i32, 0) == 0
    }
}

pub fn lock_path(root: &Path) -> PathBuf {
    root.join(".ai").join(".task.lock")
}

pub fn check_task_lock(root: &Path) -> LockStatus {
    let p = lock_path(root);
    if !p.exists() {
        return LockStatus::Unlocked;
    }

    let content = match fs::read_to_string(&p) {
        Ok(c) => c,
        Err(_) => return LockStatus::Unlocked,
    };

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
        let pid = val.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let holder = val
            .get("holder")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let acquired_at = val
            .get("acquired_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let info = LockInfo {
            pid,
            holder,
            acquired_at,
        };

        if is_pid_alive(pid) {
            LockStatus::Held(info)
        } else {
            LockStatus::Stale(info)
        }
    } else {
        LockStatus::Unlocked
    }
}

#[derive(Debug)]
pub struct TaskLockGuard {
    lock_file: PathBuf,
    active: bool,
}

impl TaskLockGuard {
    pub fn release(mut self) {
        if self.active {
            let _ = fs::remove_file(&self.lock_file);
            self.active = false;
        }
    }
}

impl Drop for TaskLockGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = fs::remove_file(&self.lock_file);
            self.active = false;
        }
    }
}

/// Acquire the task lock exclusively. Fails closed if held by another active process.
pub fn acquire_task_lock(root: &Path, holder: &str) -> Result<TaskLockGuard, String> {
    let ai_dir = root.join(".ai");
    let _ = fs::create_dir_all(&ai_dir);
    let p = lock_path(root);

    let current_pid = std::process::id();
    let now = chrono_timestamp();

    let write_payload = |f: &mut fs::File| -> std::io::Result<()> {
        let payload = serde_json::json!({
            "pid": current_pid,
            "holder": holder,
            "acquired_at": now,
        });
        f.write_all(payload.to_string().as_bytes())
    };

    // Try creating exclusively
    match OpenOptions::new().write(true).create_new(true).open(&p) {
        Ok(mut f) => {
            let _ = write_payload(&mut f);
            Ok(TaskLockGuard {
                lock_file: p,
                active: true,
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // Check status of existing lock
            match check_task_lock(root) {
                LockStatus::Stale(info) => {
                    // Stale lock: remove and retry once
                    let _ = fs::remove_file(&p);
                    let mut f = OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&p)
                        .map_err(|err| {
                            format!(
                                "🛑 Lock Acquisition Failed: Collision recovering stale lock held by PID {}: {}",
                                info.pid, err
                            )
                        })?;
                    let _ = write_payload(&mut f);
                    Ok(TaskLockGuard {
                        lock_file: p,
                        active: true,
                    })
                }
                LockStatus::Held(info) => Err(format!(
                    "🛑 Lock Acquisition Failed: .ai/.task.lock is held by PID {} ({}) acquired at {}.\n\
                     Execution halted to preserve ADR-007 single-writer integrity for active_task.md.\n\
                     If this is a stale lock from a terminated task, remove \"{}\".",
                    info.pid,
                    info.holder,
                    info.acquired_at,
                    p.display()
                )),
                LockStatus::Unlocked => {
                    let mut f = OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&p)
                        .map_err(|err| format!("Failed to acquire lock: {}", err))?;
                    let _ = write_payload(&mut f);
                    Ok(TaskLockGuard {
                        lock_file: p,
                        active: true,
                    })
                }
            }
        }
        Err(e) => Err(format!("Failed to create lockfile {}: {}", p.display(), e)),
    }
}

fn chrono_timestamp() -> String {
    // Generate ISO8601-like timestamp without extra heavy crates
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}Z", duration.as_secs(), duration.subsec_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquires_and_releases_task_lock() {
        let dir = std::env::temp_dir().join("va-lock-rust-test");
        let ai = dir.join(".ai");
        let _ = fs::create_dir_all(&ai);

        let lock = acquire_task_lock(&dir, "rust-test-holder").expect("must acquire lock");
        assert!(lock_path(&dir).exists());

        match check_task_lock(&dir) {
            LockStatus::Held(info) => {
                assert_eq!(info.holder, "rust-test-holder");
                assert_eq!(info.pid, std::process::id());
            }
            _ => panic!("lock must be held"),
        }

        lock.release();
        assert!(!lock_path(&dir).exists());
        assert_eq!(check_task_lock(&dir), LockStatus::Unlocked);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn fails_closed_on_active_lock() {
        let dir = std::env::temp_dir().join("va-lock-rust-fail-closed");
        let ai = dir.join(".ai");
        let _ = fs::create_dir_all(&ai);

        let lock = acquire_task_lock(&dir, "holder-1").expect("must acquire");

        let second = acquire_task_lock(&dir, "holder-2");
        assert!(second.is_err());
        let err_msg = second.unwrap_err();
        assert!(err_msg.contains("Lock Acquisition Failed"));
        assert!(err_msg.contains("holder-1"));
        assert!(err_msg.contains("ADR-007"));

        lock.release();
        let _ = fs::remove_dir_all(&dir);
    }
}
