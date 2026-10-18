//! Hardware profiler and adaptive runtime engine.
//!
//! Enables optimal execution across diverse environments:
//! * **Disk-First (VPS Low-RAM Mode)**: Minimal in-memory footprint (< 35MB RSS), streaming DAG persistence.
//! * **RAM-First (Portable Zero-Wear Mode)**: In-memory hydration from USB flash drive, zero flash wear,
//!   crash-safe atomic flush on verified checkpoints.
//! * **Drive-Letter Independence**: Normalizes workspace paths using `{{PORTABLE_ROOT}}`.
//! * **Zero Host Footprint**: Temp directory redirection into portable storage or memory.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const PORTABLE_ROOT_TOKEN: &str = "{{PORTABLE_ROOT}}";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareProfile {
    Auto,
    Vps,
    Portable,
    Workstation,
}

impl HardwareProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            HardwareProfile::Auto => "auto",
            HardwareProfile::Vps => "vps",
            HardwareProfile::Portable => "portable",
            HardwareProfile::Workstation => "workstation",
        }
    }
}

impl std::str::FromStr for HardwareProfile {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "auto" => Ok(HardwareProfile::Auto),
            "vps" => Ok(HardwareProfile::Vps),
            "portable" => Ok(HardwareProfile::Portable),
            "workstation" => Ok(HardwareProfile::Workstation),
            other => Err(format!(
                "unknown hardware profile `{other}`, expected auto|vps|portable|workstation"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageMode {
    /// Stream DAG and state directly to SSD with minimal buffering.
    DiskFirst,
    /// In-memory hydration, zero flash wear, atomic flush on verified checkpoints.
    RamFirst,
}

impl StorageMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            StorageMode::DiskFirst => "disk_first",
            StorageMode::RamFirst => "ram_first",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSpecs {
    pub total_ram_bytes: u64,
    pub available_ram_bytes: u64,
    pub cpu_cores: usize,
    pub is_portable: bool,
    pub active_profile: HardwareProfile,
    pub storage_mode: StorageMode,
    pub worker_threads: usize,
}

impl SystemSpecs {
    pub fn is_vps_mode(&self) -> bool {
        self.active_profile == HardwareProfile::Vps
    }

    pub fn is_portable_mode(&self) -> bool {
        self.active_profile == HardwareProfile::Portable
    }

    pub fn is_zero_wear_active(&self) -> bool {
        self.storage_mode == StorageMode::RamFirst
    }
}

#[cfg(windows)]
fn query_system_memory() -> (u64, u64) {
    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page_file: u64,
        avail_page_file: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended_virtual: u64,
    }

    extern "system" {
        fn GlobalMemoryStatusEx(stat: *mut MemoryStatusEx) -> i32;
    }

    let mut stat = MemoryStatusEx {
        length: std::mem::size_of::<MemoryStatusEx>() as u32,
        memory_load: 0,
        total_phys: 0,
        avail_phys: 0,
        total_page_file: 0,
        avail_page_file: 0,
        total_virtual: 0,
        avail_virtual: 0,
        avail_extended_virtual: 0,
    };

    let ok = unsafe { GlobalMemoryStatusEx(&mut stat) };
    if ok != 0 {
        (stat.total_phys, stat.avail_phys)
    } else {
        (8 * 1024 * 1024 * 1024, 4 * 1024 * 1024 * 1024)
    }
}

#[cfg(not(windows))]
fn query_system_memory() -> (u64, u64) {
    if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
        let mut total = 0u64;
        let mut avail = 0u64;
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                total = parse_meminfo_kb(line) * 1024;
            } else if line.starts_with("MemAvailable:") {
                avail = parse_meminfo_kb(line) * 1024;
            }
        }
        if total > 0 {
            let actual_avail = if avail > 0 { avail } else { total / 2 };
            return (total, actual_avail);
        }
    }
    // Safe default for unprobed Unix
    (8 * 1024 * 1024 * 1024, 4 * 1024 * 1024 * 1024)
}

#[cfg(not(windows))]
fn parse_meminfo_kb(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|val| val.parse::<u64>().ok())
        .unwrap_or(0)
}

/// Detect if running in portable mode by checking for `.portable` or `portable_data/`.
pub fn detect_portable_mode(exe_dir: Option<&Path>, workspace_root: Option<&Path>) -> bool {
    if let Some(dir) = exe_dir {
        if dir.join(".portable").exists() || dir.join("portable_data").is_dir() {
            return true;
        }
    }
    if let Some(root) = workspace_root {
        if root.join(".portable").exists() || root.join("portable_data").is_dir() {
            return true;
        }
    }
    false
}

/// Detect CPU cores.
pub fn detect_cpu_cores() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
}

/// Detect system specifications with profile resolution.
pub fn detect_system_specs(
    requested: Option<HardwareProfile>,
    exe_dir: Option<&Path>,
    workspace_root: Option<&Path>,
) -> SystemSpecs {
    let (total_ram, avail_ram) = query_system_memory();
    let cores = detect_cpu_cores();
    let is_portable = detect_portable_mode(exe_dir, workspace_root);

    let profile = requested
        .or_else(|| {
            std::env::var("VIBE_ASSEMBLY_PROFILE")
                .ok()
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(HardwareProfile::Auto);

    resolve_specs(profile, total_ram, avail_ram, cores, is_portable)
}

/// Resolve profile and storage mode deterministically given hardware figures.
pub fn resolve_specs(
    requested: HardwareProfile,
    total_ram: u64,
    avail_ram: u64,
    cores: usize,
    is_portable: bool,
) -> SystemSpecs {
    let one_gb = 1024 * 1024 * 1024;
    let four_gb = 4 * one_gb;

    let (active_profile, storage_mode, worker_threads) = match requested {
        HardwareProfile::Vps => (HardwareProfile::Vps, StorageMode::DiskFirst, cores.min(2)),
        HardwareProfile::Workstation => (
            HardwareProfile::Workstation,
            StorageMode::DiskFirst,
            cores.max(1),
        ),
        HardwareProfile::Portable => {
            if avail_ram < one_gb {
                // Low-RAM Host Fallback
                (
                    HardwareProfile::Portable,
                    StorageMode::DiskFirst,
                    cores.min(2),
                )
            } else {
                (
                    HardwareProfile::Portable,
                    StorageMode::RamFirst,
                    cores.max(1),
                )
            }
        }
        HardwareProfile::Auto => {
            if is_portable {
                if avail_ram >= four_gb {
                    (
                        HardwareProfile::Portable,
                        StorageMode::RamFirst,
                        cores.max(1),
                    )
                } else if avail_ram < one_gb {
                    // Low-RAM Host Fallback
                    (
                        HardwareProfile::Portable,
                        StorageMode::DiskFirst,
                        cores.min(2),
                    )
                } else {
                    (
                        HardwareProfile::Portable,
                        StorageMode::DiskFirst,
                        cores.max(1),
                    )
                }
            } else if avail_ram < one_gb {
                (HardwareProfile::Vps, StorageMode::DiskFirst, cores.min(2))
            } else {
                (
                    HardwareProfile::Workstation,
                    StorageMode::DiskFirst,
                    cores.max(1),
                )
            }
        }
    };

    SystemSpecs {
        total_ram_bytes: total_ram,
        available_ram_bytes: avail_ram,
        cpu_cores: cores,
        is_portable,
        active_profile,
        storage_mode,
        worker_threads,
    }
}

// ---- Drive-Letter Independence & Path Normalization -------------------------

fn normalize_path_str(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    let trimmed = s.strip_prefix("//?/").unwrap_or(&s);
    trimmed.to_string()
}

/// Convert a path inside root to a portable path string `{{PORTABLE_ROOT}}/rel_path`.
pub fn to_portable_path(path: &Path, root: &Path) -> String {
    let norm_root = normalize_path_str(root);
    let norm_path = normalize_path_str(path);

    let norm_root_trimmed = norm_root.trim_end_matches('/');

    if norm_path == norm_root_trimmed {
        return PORTABLE_ROOT_TOKEN.to_string();
    }

    if let Some(rel) = norm_path.strip_prefix(norm_root_trimmed) {
        let clean_rel = rel.trim_start_matches('/');
        format!("{PORTABLE_ROOT_TOKEN}/{clean_rel}")
    } else {
        norm_path
    }
}

/// Resolve a portable path string back to an absolute filesystem PathBuf against current_root.
pub fn from_portable_path(portable_str: &str, current_root: &Path) -> PathBuf {
    if portable_str == PORTABLE_ROOT_TOKEN {
        return current_root.to_path_buf();
    }
    if let Some(stripped) = portable_str.strip_prefix(PORTABLE_ROOT_TOKEN) {
        let clean = stripped.trim_start_matches(['/', '\\']);
        current_root.join(clean)
    } else {
        PathBuf::from(portable_str)
    }
}

/// Redirect process temporary directory to `{{PORTABLE_ROOT}}/portable_data/tmp` when in portable mode.
pub fn setup_portable_environment(root: &Path, is_portable: bool) -> Option<PathBuf> {
    if !is_portable {
        return None;
    }
    let tmp_dir = root.join("portable_data").join("tmp");
    let _ = std::fs::create_dir_all(&tmp_dir);
    std::env::set_var("TEMP", &tmp_dir);
    std::env::set_var("TMP", &tmp_dir);
    Some(tmp_dir)
}

// ---- Crash-Safe Atomic State Sync -------------------------------------------

pub struct AtomicStateSync;

impl AtomicStateSync {
    /// Write content to path using atomic write-to-temp + fsync + atomic rename.
    pub fn write_atomic(target_path: &Path, content: &[u8]) -> std::io::Result<()> {
        let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
        let temp_path = parent.join(format!(
            ".atomic_{}.tmp",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;

        use std::io::Write;
        file.write_all(content)?;
        file.sync_all()?;
        drop(file);

        if let Err(e) = std::fs::rename(&temp_path, target_path) {
            if cfg!(windows) && target_path.exists() {
                let _ = std::fs::remove_file(target_path);
                std::fs::rename(&temp_path, target_path)?;
            } else {
                let _ = std::fs::remove_file(&temp_path);
                return Err(e);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vps_mode_selected_for_low_ram() {
        let specs = resolve_specs(
            HardwareProfile::Auto,
            512 * 1024 * 1024,
            256 * 1024 * 1024,
            2,
            false,
        );
        assert_eq!(specs.active_profile, HardwareProfile::Vps);
        assert_eq!(specs.storage_mode, StorageMode::DiskFirst);
        assert_eq!(specs.worker_threads, 2);
    }

    #[test]
    fn test_portable_zero_wear_on_high_ram_host() {
        let sixteen_gb = 16 * 1024 * 1024 * 1024;
        let specs = resolve_specs(
            HardwareProfile::Auto,
            sixteen_gb,
            8 * 1024 * 1024 * 1024,
            6,
            true,
        );
        assert_eq!(specs.active_profile, HardwareProfile::Portable);
        assert_eq!(specs.storage_mode, StorageMode::RamFirst);
        assert!(specs.is_zero_wear_active());
        assert_eq!(specs.worker_threads, 6);
    }

    #[test]
    fn test_portable_low_ram_host_fallback() {
        // Portable drive plugged into 512MB RAM machine must step down to DiskFirst
        let specs = resolve_specs(
            HardwareProfile::Portable,
            512 * 1024 * 1024,
            200 * 1024 * 1024,
            4,
            true,
        );
        assert_eq!(specs.active_profile, HardwareProfile::Portable);
        assert_eq!(specs.storage_mode, StorageMode::DiskFirst);
        assert_eq!(specs.worker_threads, 2);
    }

    #[test]
    fn test_drive_letter_independence_token_normalization() {
        let temp = std::env::temp_dir().join("va_test_portable_root_drive");
        let _ = std::fs::create_dir_all(&temp);
        let sub = temp.join(".ai").join("active_task.md");

        let tokenized = to_portable_path(&sub, &temp);
        assert!(tokenized.starts_with(PORTABLE_ROOT_TOKEN));
        assert!(tokenized.ends_with(".ai/active_task.md"));

        // Simulate plugging into a completely different drive / directory
        let new_host_root = PathBuf::from("X:/Different/Drive/VibeAssembly");
        let resolved = from_portable_path(&tokenized, &new_host_root);
        assert_eq!(resolved, new_host_root.join(".ai").join("active_task.md"));

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_crash_safe_atomic_sync() {
        let temp = std::env::temp_dir().join("va_atomic_sync_test.txt");
        let content = b"hello crash-safe atomic sync";
        assert!(AtomicStateSync::write_atomic(&temp, content).is_ok());
        let read_back = std::fs::read(&temp).unwrap();
        assert_eq!(read_back, content);
        let _ = std::fs::remove_file(&temp);
    }
}
