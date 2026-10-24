//! Lightweight Terminal Dashboard (TUI) for Vibe Assembly (`vibe-assembly tui` / `vibe-assembly top`).
//!
//! Renders:
//! * Active HardwareProfile & Storage Strategy (Disk-First VPS, RAM-First Portable Zero-Wear, Workstation).
//! * 6-role model assignments and router connectivity state (Vibe-Link, 9router/Universal Gateway, or Offline).
//! * Live Session DAG trajectory, verification status, judge verdicts, and micro-dollar spend.

use std::io::Write;
use std::path::Path;

use va_core::knowledge::{auto_combo, context_window_tokens};
use va_core::modes::{ExecutionMode, LanguageLens};
use va_core::router_client::fetch_default;
use va_core::runtime::{detect_system_specs, HardwareProfile, StorageMode};
use va_core::state::Workspace;
use va_harness::session::{ContextWindowConfig, Session};

use crate::render::{role_icon, Palette};

pub fn run_tui(
    root: &Path,
    requested_profile: Option<HardwareProfile>,
    mut mode: ExecutionMode,
    mut language: LanguageLens,
    mut model: Option<String>,
) {
    let interactive = std::io::IsTerminal::is_terminal(&std::io::stdin());
    loop {
        render_dashboard(root, requested_profile, mode, language, model.clone());
        if !interactive {
            break;
        }
        println!();
        println!(" [1] Mode  [2] Language  [3] Model  [4] Refresh  [5] Quit");
        print!(" select> ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        match line.trim() {
            "1" => {
                for (i, m) in ExecutionMode::ALL.iter().enumerate() {
                    println!("   [{}] {} ({})", i + 1, m.label(), m.accent_hex());
                }
                print!("   mode> ");
                let _ = std::io::stdout().flush();
                let mut choice = String::new();
                let _ = std::io::stdin().read_line(&mut choice);
                if let Ok(idx) = choice.trim().parse::<usize>() {
                    if let Some(m) = ExecutionMode::ALL.get(idx.saturating_sub(1)) {
                        mode = *m;
                    }
                }
            }
            "2" => {
                for (i, l) in LanguageLens::ALL.iter().enumerate() {
                    println!("   [{}] {}", i + 1, l.label());
                }
                print!("   language> ");
                let _ = std::io::stdout().flush();
                let mut choice = String::new();
                let _ = std::io::stdin().read_line(&mut choice);
                if let Ok(idx) = choice.trim().parse::<usize>() {
                    if let Some(l) = LanguageLens::ALL.get(idx.saturating_sub(1)) {
                        language = *l;
                    }
                }
            }
            "3" => {
                print!("   model (blank = auto-combo)> ");
                let _ = std::io::stdout().flush();
                let mut choice = String::new();
                let _ = std::io::stdin().read_line(&mut choice);
                let clean = choice.trim().to_string();
                model = if clean.is_empty() { None } else { Some(clean) };
            }
            "4" => {}
            "5" | "q" | "quit" => break,
            _ => {}
        }
    }
}

fn render_dashboard(
    root: &Path,
    requested_profile: Option<HardwareProfile>,
    mode: ExecutionMode,
    language: LanguageLens,
    model: Option<String>,
) {
    let p = Palette::detect();
    let specs = detect_system_specs(
        requested_profile,
        std::env::current_exe().ok().as_deref(),
        Some(root),
    );
    let ws = Workspace::load(root);
    let gateway = fetch_default();
    let accent = if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        let rgb = mode.accent_rgb();
        format!(
            "\x1b[38;2;{};{};{}m",
            (rgb >> 16) & 0xFF,
            (rgb >> 8) & 0xFF,
            rgb & 0xFF
        )
    } else {
        String::new()
    };

    println!();
    println!(
        "{}┌────────────────────────────────────────────────────────────────────────┐{}",
        p.dim, p.reset
    );
    println!(
        "{}│{}  {}⚡ VIBE ASSEMBLY MISSION CONTROL{}  {}·  TERMINAL DASHBOARD            {}│{}",
        p.dim, p.reset, p.bold, p.reset, p.dim, p.dim, p.reset
    );
    println!(
        "{}└────────────────────────────────────────────────────────────────────────┘{}",
        p.dim, p.reset
    );

    // Section 0: active execution profile, accent theme, and context meter.
    let session_path = root.join(".ai").join("session.jsonl");
    let session = if session_path.exists() {
        let raw = std::fs::read_to_string(&session_path).unwrap_or_default();
        Session::from_jsonl(&raw).unwrap_or_default()
    } else {
        Session::new()
    };
    let active_model = model.clone().unwrap_or_else(|| "auto-combo".to_string());
    let window = context_window_tokens(&active_model);
    let context = ContextWindowConfig::new(window);
    let (surface, compacted) = session
        .context_surface_with_budget(&context)
        .unwrap_or_default();
    let tokens = surface.estimate_tokens();
    let pct = tokens
        .saturating_mul(100)
        .checked_div(window)
        .unwrap_or(0)
        .min(100);
    let bar_width = (pct / 5).clamp(1, 20);
    println!();
    println!(" {}◆ EXECUTION PROFILE{}", accent, p.reset);
    println!(
        "   {}Mode:{}        {}{}{}",
        p.dim,
        p.reset,
        accent,
        mode.label(),
        p.reset
    );
    println!(
        "   {}Accent:{}      {}{}{}",
        p.dim,
        p.reset,
        accent,
        mode.accent_hex(),
        p.reset
    );
    println!(
        "   {}Language:{}    {}",
        p.dim,
        p.reset,
        language.resolve(root).label()
    );
    println!("   {}Model:{}       {}", p.dim, p.reset, active_model);
    println!(
        "   {}Context:{}     {}% of {}-token window  {}{}{}  ({} tokens, {} auto-compaction(s))",
        p.dim,
        p.reset,
        pct,
        window,
        accent,
        "█".repeat(bar_width),
        p.reset,
        tokens,
        compacted
    );

    // Section 1: Hardware & Storage Profile
    println!();
    println!(
        " {}⚙ HARDWARE PROFILE & RUNTIME STRATEGY{}",
        p.bold, p.reset
    );
    let profile_label = match specs.active_profile {
        HardwareProfile::Vps => format!("{}VPS (Low-RAM Mode){}", p.yellow, p.reset),
        HardwareProfile::Portable => format!(
            "{}Portable Flash Drive (Drive-Letter Independent){}",
            p.blue, p.reset
        ),
        HardwareProfile::Workstation => format!("{}Workstation (Host Native){}", p.green, p.reset),
        HardwareProfile::Auto => format!("{}Auto Detected{}", p.dim, p.reset),
    };
    let storage_label = match specs.storage_mode {
        StorageMode::DiskFirst => {
            format!("{}Disk-First (< 35MB RSS Streaming){}", p.yellow, p.reset)
        }
        StorageMode::RamFirst => format!(
            "{}RAM-First (Zero Flash Wear + Atomic Flush){}",
            p.green, p.reset
        ),
    };
    let total_gb = specs.total_ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let avail_gb = specs.available_ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

    println!("   {}Profile:{}      {}", p.dim, p.reset, profile_label);
    println!("   {}Storage Mode:{} {}", p.dim, p.reset, storage_label);
    println!(
        "   {}System RAM:{}   {:.1} GB total  ·  {:.1} GB available",
        p.dim, p.reset, total_gb, avail_gb
    );
    println!(
        "   {}Concurrency:{}  {} CPU core(s)  ·  {} worker thread(s) configured",
        p.dim, p.reset, specs.cpu_cores, specs.worker_threads
    );
    if specs.is_portable {
        println!(
            "   {}Zero-Wear:{}    {}ACTIVE (in-memory hydration from USB flash drive){}",
            p.green, p.bold, p.green, p.reset
        );
    }

    // Section 2: Vibe Router Connectivity & 6-Role Allocations
    println!();
    println!(
        " {}🧠 CONNECTIVITY & 6-ROLE MODEL ASSIGNMENTS{}",
        p.bold, p.reset
    );
    let (conn_label, models) = if gateway.is_live() {
        let label = format!("{}🟢 LIVE ({}){}", p.green, gateway.source, p.reset);
        (label, gateway.model_names())
    } else {
        let reason = gateway.detail.as_deref().unwrap_or("offline fallback");
        let label = format!("{}⚠ OFFLINE ({}){}", p.yellow, reason, p.reset);
        let fallback_models = vec![
            "claude-opus".into(),
            "qwen3-coder".into(),
            "deepseek-r1".into(),
            "gemini-2.0-flash".into(),
        ];
        (label, fallback_models)
    };
    println!("   {}Router State:{} {}", p.dim, p.reset, conn_label);

    let assignments = auto_combo(&models, 0.5);
    println!();
    println!(
        "   {}Role     Icon  Model                         Score   Class / Target{}",
        p.dim, p.reset
    );
    println!(
        "   {}─────────────────────────────────────────────────────────────────{}",
        p.dim, p.reset
    );
    for a in &assignments {
        println!(
            "   {:<8} {}  {}{:<28}{} {:.3}   [{}]",
            a.role.as_str(),
            role_icon(a.role.as_str()),
            p.bold,
            a.model,
            p.reset,
            a.score,
            a.role.as_str()
        );
    }

    // Section 3: Live Session Trajectory DAG & Spend
    let session_path = root.join(".ai").join("session.jsonl");
    let session = if session_path.exists() {
        let raw = std::fs::read_to_string(&session_path).unwrap_or_default();
        Session::from_jsonl(&raw).unwrap_or_default()
    } else {
        Session::new()
    };

    println!();
    println!(
        " {}📈 SESSION TRAJECTORY DAG & SPEND TELEMETRY{}",
        p.bold, p.reset
    );
    println!(
        "   {}DAG Nodes:{}     {} committed node(s)",
        p.dim,
        p.reset,
        session.len()
    );
    println!("   {}Head Node:{}     {:?}", p.dim, p.reset, session.head());
    let path = session.current_path().unwrap_or_default();
    println!("   {}Active Branch:{} {:?}", p.dim, p.reset, path);

    let cost_micros = session.total_cost_micros();
    let cost_usd = cost_micros as f64 / 1_000_000.0;
    let latency_ms = session.total_latency_ms();
    println!(
        "   {}Total Spend:{}   ${:.6} ({} micro-dollars)",
        p.green, p.reset, cost_usd, cost_micros
    );
    println!("   {}Total Latency:{} {} ms", p.dim, p.reset, latency_ms);

    if let Ok(Some(last_v)) = session.last_verified() {
        println!(
            "   {}Last Verified:{} Node #{} (verified checkpoint intact)",
            p.green, p.reset, last_v
        );
    } else {
        println!(
            "   {}Last Verified:{} (none yet in active trajectory)",
            p.dim, p.reset
        );
    }

    // Section 4: Hash-Anchored RepoMap & Speculative Tournament
    let repomap = va_harness::repomap::RepoMap::scan(root);
    println!();
    println!(
        " {}🗺 HASH-ANCHORED REPOMAP & TOURNAMENT ENGINE{}",
        p.bold, p.reset
    );
    println!(
        "   {}Indexed Symbols:{} {} across {} files ({} code blocks)",
        p.dim,
        p.reset,
        repomap.total_symbols(),
        repomap.total_files(),
        repomap.total_blocks()
    );
    println!(
        "   {}Tournament Mode:{} Speculative multi-branch fork & merge (active)",
        p.dim, p.reset
    );

    // Section 5: Active Task & Queue
    if let Some(wave) = &ws.active_task.wave_name {
        println!();
        println!(" {}📋 WORKSPACE CONTRACT STATUS{}", p.bold, p.reset);
        println!("   {}Active Wave:{}   {}", p.dim, p.reset, wave);
        println!(
            "   {}Active Ticket:{} {:?}",
            p.dim, p.reset, ws.active_task.active_id
        );
        println!(
            "   {}Remaining:{}     {} ticket(s) in queue",
            p.dim,
            p.reset,
            ws.active_task.queue.len()
        );
    }

    println!();
    println!(
        "{}────────────────────────────────────────────────────────────────────────{}",
        p.dim, p.reset
    );
    println!(
        " {}Press Ctrl+C to exit. Refresh anytime with `vibe-assembly tui`.{}",
        p.dim, p.reset
    );
    println!();
}
