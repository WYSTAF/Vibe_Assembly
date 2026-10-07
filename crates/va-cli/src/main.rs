//! Vibe Assembly CLI — the terminal operations room.
//!
//! Pure Rust. Mirrors the legacy `bin/va.js` surface: status, --models,
//! --doctor, --json, --summary, plus native workspace init.

mod render;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use va_core::allocator::{allocate, load_knowledge};
use va_core::bootstrap;
use va_core::lock::check_task_lock;
use va_core::parser::{parse_active_task, validate_contract, verified_steps};
use va_core::router::fetch_live_assignment;
use va_core::state::Workspace;

use render::{queue_lines, role_icon, Palette};

#[derive(Parser)]
#[command(
    name = "va",
    version,
    about = "Vibe Assembly — terminal operations room for the .ai/ workspace contract"
)]
struct Cli {
    /// Workspace root (defaults to searching upward from the current directory)
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    /// Emit machine-readable JSON
    #[arg(long)]
    json: bool,

    /// Print a Boss-ready wave report
    #[arg(long)]
    summary: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Show the ticket queue and resume point (default)
    Status,
    /// Resolve model allocations for the five modes
    Models,
    /// Run a workspace health check
    Doctor,
    /// Create a new Vibe Assembly workspace
    Init {
        /// Directory name for the new project
        name: String,
        /// Standard project template to apply
        #[arg(short, long)]
        template: Option<String>,
        /// Directory holding templates (defaults to ./templates)
        #[arg(long)]
        templates_dir: Option<PathBuf>,
    },
    /// Reset volatile .ai/ state to pristine template defaults
    Reset,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let start = cli
        .root
        .clone()
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));

    // `init` targets a new directory, so it must not require an existing workspace.
    if let Some(Command::Init {
        name,
        template,
        templates_dir,
    }) = &cli.command
    {
        let target = start.join(name);
        let tdir = templates_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("templates"));
        return match bootstrap::init_workspace(&target, name, template.as_deref(), Some(&tdir)) {
            Ok(()) => {
                println!("✅ Created Vibe Assembly workspace at {}", target.display());
                println!("   Next: open it in your editor and switch to 👑 Boss.");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("❌ {}", e);
                ExitCode::from(1)
            }
        };
    }

    let root = match Workspace::discover(&start) {
        Some(r) => r,
        None => {
            eprintln!(
                "❌ No .ai/ directory found at or above {}. Not a Vibe Assembly workspace?",
                start.display()
            );
            return ExitCode::from(1);
        }
    };

    let p = Palette::detect();

    match &cli.command {
        Some(Command::Reset) => {
            return match bootstrap::reset_template_state(&root) {
                Ok(()) => {
                    println!("✅ Reset .ai/ state in {}", root.display());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("❌ {}", e);
                    ExitCode::from(1)
                }
            };
        }
        Some(Command::Models) => return run_models(&root, &p),
        Some(Command::Doctor) => return run_doctor(&root, &p),
        _ => {}
    }

    if cli.summary {
        return run_summary(&root);
    }

    run_status(&root, &p, cli.json)
}

fn run_status(root: &Path, p: &Palette, json: bool) -> ExitCode {
    let ws = Workspace::load(root);

    if json {
        let payload = serde_json::json!({
            "root": ws.root,
            "wave_name": ws.active_task.wave_name,
            "active_id": ws.active_task.active_id,
            "contract_violation": ws.violation,
            "queue": ws.active_task.queue,
            "verified_steps": ws.active_task.verified_steps(),
            "suggested_phrase": ws.suggested_phrase(),
            "task_lock": check_task_lock(root),
            "router_assignment": ws.router_assignment,
        });
        match serde_json::to_string_pretty(&payload) {
            Ok(s) => println!("{}", s),
            Err(e) => {
                eprintln!("❌ Could not serialize status: {}", e);
                return ExitCode::from(2);
            }
        }
        return ExitCode::SUCCESS;
    }

    println!();
    println!(
        " {}⚡ VIBE ASSEMBLY{}  {}·  {}{}",
        p.bold,
        p.reset,
        p.dim,
        root.display(),
        p.reset
    );
    if let Some(wave) = &ws.active_task.wave_name {
        println!(" {}Wave:{} {}", p.dim, p.reset, wave);
    }

    if let va_core::lock::LockStatus::Held(info) = check_task_lock(root) {
        println!(
            " {}🔒 TASK LOCK:{} {}Held by PID {} ({}){}",
            p.yellow, p.reset, p.dim, info.pid, info.holder, p.reset
        );
    }

    if ws.router_assignment.is_live {
        println!(
            " {}🟢 Router:{} {} (live assignment active)",
            p.green, p.reset, ws.router_assignment.source
        );
    }

    if let Some(violation) = &ws.violation {
        println!();
        println!(" {}⛔ CONTRACT VIOLATION{} — {}", p.red, p.reset, violation);
        println!(
            " {}Return to 👑 Boss and fix .ai/active_task.md.{}",
            p.dim, p.reset
        );
        println!();
        return ExitCode::from(2);
    }

    if !ws.active_task.queue.is_empty() {
        println!();
        for line in queue_lines(&ws.active_task, p) {
            println!("{}", line);
        }
    }

    if let Some(a) = &ws.active_task.active {
        let verified = verified_steps(&a.steps, &a.relay_notes);
        println!();
        println!(" {}▶ {}{} — {}", p.bold, a.id, p.reset, a.title);
        if !a.files_allowed.is_empty() {
            println!(
                "   {}files:{} {}",
                p.dim,
                p.reset,
                a.files_allowed.join(", ")
            );
        }
        println!(
            "   {}steps:{} {}/{} verified",
            p.dim,
            p.reset,
            verified.len(),
            a.steps.len()
        );
        if let Some(last) = a.relay_notes.last() {
            let short: String = last.chars().take(100).collect();
            let ellipsis = if last.chars().count() > 100 {
                "…"
            } else {
                ""
            };
            println!(
                "   {}last evidence:{} {}{}",
                p.dim,
                p.reset,
                short.replace('\n', " "),
                ellipsis
            );
        }
    }

    println!();
    match ws.suggested_phrase() {
        Some(phrase) => {
            let resume = ws.active_task.resume_step();
            match resume {
                Some(n) if ws.active_task.has_work() => println!(
                    " {}⌨ NEXT:{} {}{}{}   {}(resume at step {} of {}){}",
                    p.green,
                    p.reset,
                    p.bold,
                    phrase,
                    p.reset,
                    p.dim,
                    n,
                    ws.active_task.steps_total(),
                    p.reset
                ),
                _ => println!(
                    " {}⌨ NEXT:{} {}{}{}",
                    p.green, p.reset, p.bold, phrase, p.reset
                ),
            }
        }
        None => println!(
            " {}○ All tickets done or none scoped — switch to 👑 Boss to plan the next wave.{}",
            p.yellow, p.reset
        ),
    }

    if let Some(blockers) = ws.blockers() {
        println!(" {}⚠ Blockers:{} {}", p.red, p.reset, blockers);
    }
    println!();
    ExitCode::SUCCESS
}

fn run_models(root: &Path, p: &Palette) -> ExitCode {
    // Prefer live router assignments; fall back to the local knowledge base.
    if let Some(live) = fetch_live_assignment(
        "127.0.0.1",
        20128,
        "/v1/assignment",
        std::time::Duration::from_millis(200),
    ) {
        println!();
        println!(
            " {}🧠 MODEL ALLOCATION{}  {}🟢 LIVE{}  {}·  {}{}",
            p.bold, p.reset, p.green, p.reset, p.dim, live.source, p.reset
        );
        println!();
        for role in &live.roles {
            let size = live
                .recommended_ticket_sizes
                .get(&role.mode)
                .map(|s| format!("  {}(size: {}){}", p.dim, s, p.reset))
                .unwrap_or_default();
            println!(
                " {} {:<7} → {}{}{}  [{}]{}",
                role_icon(&role.mode),
                role.mode,
                p.bold,
                role.model,
                p.reset,
                role.class,
                size
            );
        }
        println!();
        return ExitCode::SUCCESS;
    }

    // Offline fallback
    let knowledge = match load_knowledge(root) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("❌ {}", e);
            return ExitCode::from(1);
        }
    };

    let combo_path = root.join(".ai").join("model_combo.txt");
    let mut combo: Vec<String> = Vec::new();
    let mut locked = std::collections::BTreeMap::new();

    if let Ok(text) = std::fs::read_to_string(&combo_path) {
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let clean = match line.find('#') {
                Some(i) => line[..i].trim(),
                None => line,
            };
            if clean.is_empty() {
                continue;
            }
            if let Some((mode, model)) = clean.split_once('=') {
                locked.insert(mode.trim().to_lowercase(), model.trim().to_string());
            } else {
                combo.push(clean.to_string());
            }
        }
    }

    if combo.is_empty() && locked.is_empty() {
        eprintln!(
            "❌ No model combo found. Create {} with one router model per line.",
            combo_path.display()
        );
        eprintln!("   gemini-2.0-flash");
        eprintln!("   deepseek-r1:free");
        eprintln!("   qwen3-coder");
        return ExitCode::from(1);
    }

    let result = allocate(&combo, &knowledge, &locked);

    println!();
    println!(
        " {}🧠 MODEL ALLOCATION{}  {}·  combo of {}{}{}",
        p.bold,
        p.reset,
        p.dim,
        combo.len(),
        if locked.is_empty() {
            String::new()
        } else {
            format!(" + {} lock(s)", locked.len())
        },
        p.reset
    );
    println!();
    for (mode, a) in &result.assignments {
        let tag = if a.locked {
            format!("{}(locked){}", p.bold, p.reset)
        } else {
            format!("[{}]", a.class)
        };
        println!(
            " {} {:<7} → {}{}{}  {}",
            role_icon(mode),
            mode,
            p.bold,
            a.model,
            p.reset,
            tag
        );
    }
    if !result.unassigned.is_empty() {
        println!();
        println!(
            " {}unused: {}{}",
            p.dim,
            result.unassigned.join(", "),
            p.reset
        );
    }
    for w in &result.warnings {
        println!(" {}⚠ {}{}", p.yellow, w, p.reset);
    }
    println!();
    println!(
        " {}Set these in your router profile / mode settings. Re-run after editing model_combo.txt.{}",
        p.dim, p.reset
    );
    println!();
    ExitCode::SUCCESS
}

fn run_doctor(root: &Path, p: &Palette) -> ExitCode {
    let mut checks: Vec<(bool, String, String)> = Vec::new();

    let ai = root.join(".ai");
    for f in ["active_task.md", "current_state.md", "ticket_contract.md"] {
        let exists = ai.join(f).exists();
        checks.push((
            exists,
            format!(".ai/{} present", f),
            if exists {
                String::new()
            } else {
                "missing — core contract file".to_string()
            },
        ));
    }

    // Single-writer lockfile check
    match check_task_lock(root) {
        va_core::lock::LockStatus::Held(info) => checks.push((
            false,
            "single-writer task lock (.ai/.task.lock)".to_string(),
            format!("held by PID {} ({})", info.pid, info.holder),
        )),
        _ => checks.push((
            true,
            "single-writer task lock (.ai/.task.lock)".to_string(),
            "unlocked (available)".to_string(),
        )),
    }

    // Router connectivity (informational)
    let router_ok = fetch_live_assignment(
        "127.0.0.1",
        20128,
        "/v1/assignment",
        std::time::Duration::from_millis(200),
    )
    .is_some();
    checks.push((
        true,
        "Vibe Router connectivity".to_string(),
        if router_ok {
            "live".to_string()
        } else {
            "offline fallback ready (.ai/model_knowledge.json)".to_string()
        },
    ));

    // Contract consistency
    if let Ok(raw) = std::fs::read_to_string(ai.join("active_task.md")) {
        let at = parse_active_task(&raw);
        match validate_contract(&at) {
            None => checks.push((
                true,
                "ticket contract consistent".to_string(),
                format!(
                    "active: {}",
                    at.active_id.unwrap_or_else(|| "(none)".into())
                ),
            )),
            Some(v) => checks.push((false, "ticket contract consistent".to_string(), v)),
        }
    }

    // Fresh-template detection
    match std::fs::read_to_string(ai.join("project_config.md")) {
        Ok(c) => {
            let fresh = c.contains("CHANGE_ME_");
            checks.push((
                !fresh,
                "project configured".to_string(),
                if fresh {
                    "CHANGE_ME_* placeholders remain — finish 👑 Boss first setup".to_string()
                } else {
                    String::new()
                },
            ));
        }
        Err(_) => checks.push((
            false,
            ".ai/project_config.md present".to_string(),
            "missing".to_string(),
        )),
    }

    println!();
    println!(
        " {}🩺 VIBE ASSEMBLY DOCTOR{}  {}·  {}{}",
        p.bold,
        p.reset,
        p.dim,
        root.display(),
        p.reset
    );
    println!();

    let mut failures = 0;
    for (ok, label, detail) in &checks {
        let mark = if *ok {
            format!("{}✔{}", p.green, p.reset)
        } else {
            failures += 1;
            format!("{}✘{}", p.red, p.reset)
        };
        let suffix = if detail.is_empty() {
            String::new()
        } else {
            format!("{} — {}{}", p.dim, detail, p.reset)
        };
        println!(" {} {}{}", mark, label, suffix);
    }

    println!();
    if failures == 0 {
        println!(
            " {}All checks passed. Workspace is healthy.{}",
            p.green, p.reset
        );
    } else {
        println!(
            " {}{} issue(s) found.{} Fix the ✘ items above; ask 💬 Chat if unsure.",
            p.red, failures, p.reset
        );
    }
    println!();

    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    }
}

fn run_summary(root: &Path) -> ExitCode {
    let ws = Workspace::load(root);
    let at = &ws.active_task;
    let done = at.queue.iter().filter(|r| r.is_done()).count();

    println!(
        "# Wave Report — {}",
        at.wave_name
            .clone()
            .unwrap_or_else(|| "(unnamed wave)".into())
    );
    println!();
    println!("- **Workspace:** `{}`", root.display());
    println!("- **Progress:** {}/{} tickets done", done, at.queue.len());
    println!();
    println!("| Ticket | Title | Status |");
    println!("|--------|-------|--------|");
    for r in &at.queue {
        println!("| {} | {} | {} |", r.id, r.title, r.status);
    }
    ExitCode::SUCCESS
}
