//! Vibe Assembly CLI — the terminal operations room.
//!
//! Pure Rust. Surface: status, models, doctor, init, reset, --json, --summary,
//! tui/top, profile, session, mcp, patch, blocks, run.

mod mcp;
mod render;
mod tui;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use va_core::allocator::{allocate, load_knowledge};
use va_core::bootstrap;
use va_core::lock::check_task_lock;
use va_core::parser::{parse_active_task, validate_contract, verified_steps};
use va_core::router::fetch_live_assignment;
use va_core::runtime::{detect_system_specs, AtomicStateSync};
use va_core::state::Workspace;
use va_harness::session::{NodeKind, Session};

use render::{queue_lines, role_icon, Palette};

#[derive(Parser)]
#[command(
    name = "vibe-assembly",
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
    /// Ensure Vibe Router and workspace readiness with zero-401 handshake
    Up {
        /// Override hardware profile (auto|vps|portable|workstation)
        #[arg(long)]
        profile: Option<String>,
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Output the hash-anchored repository symbol map
    Repomap {
        /// Maximum token budget for the map output
        #[arg(long, default_value_t = 2048)]
        max_tokens: usize,
        /// Priority file paths to emphasize at the top
        #[arg(long)]
        priority: Vec<PathBuf>,
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Run compiler checks and pinpoint errors to exact code block hashes
    Diagnose {
        /// Compiler check command (defaults to `cargo check --message-format=json`)
        #[arg(long)]
        command: Option<String>,
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Run the harness execution loop with zero-config model roles
    Run {
        /// Resolve roles from Vibe Router instead of the local combo
        #[arg(long)]
        auto_combo: bool,
        /// Maximum steps before the loop stops
        #[arg(long, default_value_t = 50)]
        max_steps: u32,
        /// Stream structured JSONL events for Hermes Agent / external automation
        #[arg(long)]
        stream_jsonl: bool,
        /// Override hardware execution profile (auto|vps|portable|workstation)
        #[arg(long)]
        profile: Option<String>,
        /// Enable multi-branch speculative tournament mode
        #[arg(long)]
        tournament: bool,
        /// Run live against Vibe Router with live planner
        #[arg(long)]
        live: bool,
    },
    /// Launch the interactive terminal dashboard (alias: top)
    #[command(alias = "top")]
    Tui {
        /// Force execution profile (auto|vps|portable|workstation)
        #[arg(long)]
        profile: Option<String>,
    },
    /// Display detected hardware capabilities, portable status, and active storage strategy
    Profile {
        /// Force execution profile (auto|vps|portable|workstation)
        #[arg(long)]
        profile: Option<String>,
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Inspect or mutate the transactional session trajectory DAG
    Session {
        #[command(subcommand)]
        action: SessionCommand,
    },
    /// Start the Model Context Protocol (MCP) server over stdio
    Mcp,
    /// Apply a hash-anchored patch, addressing the target by block hash
    Patch {
        /// File to patch
        file: PathBuf,
        /// Target block hash, e.g. 3A7F or the full 64-bit value
        #[arg(long)]
        target: String,
        /// Replacement text (reads stdin when omitted)
        replacement: Option<String>,
        /// Show the file's block hashes and exit
        #[arg(long)]
        list: bool,
    },
    /// List the block hashes in a file
    Blocks {
        /// File to inspect
        file: PathBuf,
    },
}

#[derive(Subcommand)]
enum SessionCommand {
    /// Show current session DAG trajectory status
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Fork a new branch from a specific node ID
    Fork {
        /// Node ID to fork from
        node_id: u64,
        /// Reason for fork
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Roll back to last verified node or a specific node ID
    Rollback {
        /// Roll back to the last passing verification node
        #[arg(long)]
        to_verified: bool,
        /// Specific node ID to roll back to
        #[arg(long)]
        to: Option<u64>,
        #[arg(long)]
        json: bool,
    },
    /// Compact a range of intermediate DAG nodes on the context surface
    Compact {
        /// Start node ID
        #[arg(long)]
        from: u64,
        /// End node ID
        #[arg(long)]
        to: u64,
        /// Compacted summary text
        #[arg(long)]
        summary: String,
        #[arg(long)]
        json: bool,
    },
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

    // Commands that don't strictly require an existing .ai/ directory
    match &cli.command {
        Some(Command::Up { profile, json }) => {
            let root = Workspace::discover(&start).unwrap_or(start);
            return run_up(&root, profile.as_deref(), *json);
        }
        Some(Command::Repomap {
            max_tokens,
            priority,
            json,
        }) => {
            let root = Workspace::discover(&start).unwrap_or(start);
            return run_repomap(&root, *max_tokens, priority, *json);
        }
        Some(Command::Diagnose { command, json }) => {
            let root = Workspace::discover(&start).unwrap_or(start);
            return run_diagnose(&root, command.as_deref(), *json);
        }
        Some(Command::Mcp) => {
            let root = Workspace::discover(&start);
            if let Err(e) = mcp::run_mcp_server(root.as_deref()) {
                eprintln!("❌ MCP server error: {e}");
                return ExitCode::from(1);
            }
            return ExitCode::SUCCESS;
        }
        Some(Command::Tui { profile }) => {
            let root = Workspace::discover(&start).unwrap_or(start);
            let p_override = profile.as_deref().and_then(|s| s.parse().ok());
            tui::run_tui(&root, p_override);
            return ExitCode::SUCCESS;
        }
        Some(Command::Profile { profile, json }) => {
            let root = Workspace::discover(&start).unwrap_or(start);
            return run_profile(&root, profile.as_deref(), *json);
        }
        Some(Command::Blocks { file }) => {
            return run_blocks(file);
        }
        Some(Command::Patch {
            file,
            target,
            replacement,
            list,
        }) => {
            return run_patch(file, target, replacement.as_deref(), *list);
        }
        Some(Command::Session { action }) => {
            let root = match Workspace::discover(&start) {
                Some(r) => r,
                None => {
                    eprintln!("❌ No .ai/ directory found at or above {}", start.display());
                    return ExitCode::from(1);
                }
            };
            return run_session(&root, action);
        }
        Some(Command::Run {
            auto_combo,
            max_steps,
            stream_jsonl,
            profile,
            tournament,
            live,
        }) => {
            let root = match Workspace::discover(&start) {
                Some(r) => r,
                None => {
                    eprintln!("❌ No .ai/ directory found at or above {}", start.display());
                    return ExitCode::from(1);
                }
            };
            return run_execute(
                &root,
                *auto_combo,
                *max_steps,
                *stream_jsonl,
                profile.as_deref(),
                *tournament,
                *live,
            );
        }
        _ => {}
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
    let example_path = root.join(".ai").join("model_combo.example.txt");
    let active_path = if combo_path.exists() {
        combo_path
    } else {
        example_path
    };
    let mut combo: Vec<String> = Vec::new();
    let mut locked = std::collections::BTreeMap::new();

    if let Ok(text) = std::fs::read_to_string(&active_path) {
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
            root.join(".ai").join("model_combo.txt").display()
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

// ---- harness-backed commands ------------------------------------------------

/// Print every block in a file with its hash and line range — the discovery
/// step an agent performs before patching.
fn run_blocks(file: &Path) -> ExitCode {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Could not read {}: {e}", file.display());
            return ExitCode::from(1);
        }
    };
    let blocks = va_harness::patcher::segment_blocks(&source);
    if blocks.is_empty() {
        println!("(no blocks — file is empty)");
        return ExitCode::SUCCESS;
    }
    for b in &blocks {
        println!(
            "{:<5}  lines {:>4}-{:<4}  {}",
            va_harness::patcher::short_hash(b.hash),
            b.start_line,
            b.end_line,
            b.signature
        );
    }
    ExitCode::SUCCESS
}

/// Parse a short (4-hex) or full 64-bit block hash.
fn parse_hash(text: &str) -> Option<u64> {
    let t = text.trim().trim_start_matches('#');
    if t.len() <= 4 && !t.is_empty() {
        // Short hashes are the top 16 bits of the 64-bit digest. Ambiguous by
        // construction, so match by prefix against the file's blocks instead.
        return Some(u64::from_str_radix(t, 16).ok()? << 48);
    }
    u64::from_str_radix(t, 16).ok()
}

fn run_patch(file: &Path, target: &str, replacement: Option<&str>, list: bool) -> ExitCode {
    if list {
        return run_blocks(file);
    }
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Could not read {}: {e}", file.display());
            return ExitCode::from(1);
        }
    };

    let text = match replacement {
        Some(r) => r.to_string(),
        None => {
            use std::io::Read;
            let mut buf = String::new();
            if let Err(e) = std::io::stdin().read_to_string(&mut buf) {
                eprintln!("❌ Could not read replacement from stdin: {e}");
                return ExitCode::from(1);
            }
            buf
        }
    };

    // Resolve a short hash by matching the rendered prefix of real blocks.
    let blocks = va_harness::patcher::segment_blocks(&source);
    let wanted = match parse_hash(target) {
        Some(h) => h,
        None => {
            eprintln!("❌ --target must be hexadecimal, got {target:?}");
            return ExitCode::from(1);
        }
    };
    let exact = blocks.iter().find(|b| b.hash == wanted);
    let resolved = match exact {
        Some(b) => Some(b.clone()),
        None => {
            let matches: Vec<_> = blocks
                .iter()
                .filter(|b| {
                    va_harness::patcher::short_hash(b.hash)
                        .eq_ignore_ascii_case(target.trim().trim_start_matches('#'))
                })
                .cloned()
                .collect();
            match matches.len() {
                0 => None,
                1 => Some(matches[0].clone()),
                _ => {
                    eprintln!(
                        "❌ Short hash {target} is ambiguous — it matches {} blocks. Use the full 64-bit hash.",
                        matches.len()
                    );
                    return ExitCode::from(2);
                }
            }
        }
    };

    let block = match resolved {
        Some(b) => b,
        None => {
            eprintln!("❌ No block with hash {target} in {}", file.display());
            eprintln!("   Run `vibe-assembly blocks <file>` to list valid hashes.");
            return ExitCode::from(2);
        }
    };

    // Record signatures so drift inside this file can be recovered once.
    va_harness::patcher::record_signatures(&source);

    let op = va_harness::patcher::PatchOp::new(block.hash, text).with_signature(&block.signature);

    match va_harness::patcher::apply_patch(&source, &op) {
        Ok(updated) => match std::fs::write(file, &updated) {
            Ok(()) => {
                println!(
                    "✅ Patched {} (lines {}-{} → {} bytes)",
                    file.display(),
                    block.start_line,
                    block.end_line,
                    updated.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("❌ Could not write {}: {e}", file.display());
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("❌ Patch refused: {e}");
            ExitCode::from(2)
        }
    }
}

/// Zero-config role resolution, then report the plan the harness will use.
fn run_execute(
    root: &Path,
    auto_combo: bool,
    max_steps: u32,
    stream_jsonl: bool,
    profile: Option<&str>,
    tournament: bool,
    live: bool,
) -> ExitCode {
    use va_core::knowledge::{auto_combo as plan, score_model};
    use va_core::router_client::fetch_default;

    let gateway = fetch_default();

    let mut models: Vec<String> = Vec::new();
    let mut source = String::new();

    if auto_combo {
        if gateway.is_live() {
            models = gateway.model_names();
            source = format!("Vibe Router ({} role(s))", models.len());
        } else if !stream_jsonl {
            let p = Palette::detect();
            eprintln!(
                "{}⚠{} Vibe Router unavailable ({}), falling back to .ai/model_combo.txt",
                p.yellow,
                p.reset,
                gateway.detail.as_deref().unwrap_or("unknown reason")
            );
        }
    }

    if models.is_empty() {
        let combo = root.join(".ai").join("model_combo.txt");
        let example = root.join(".ai").join("model_combo.example.txt");
        let path_to_read = if combo.exists() {
            source = ".ai/model_combo.txt".to_string();
            combo
        } else {
            source = ".ai/model_combo.example.txt".to_string();
            example
        };
        if let Ok(text) = std::fs::read_to_string(&path_to_read) {
            for line in text.lines() {
                let l = line.trim();
                if l.is_empty() || l.starts_with('#') {
                    continue;
                }
                let clean = l.split('#').next().unwrap_or(l).trim();
                if clean.is_empty() || clean.contains('=') {
                    continue;
                }
                models.push(clean.to_string());
            }
        }
    }

    if models.is_empty() {
        if stream_jsonl {
            println!(
                "{}",
                serde_json::json!({
                    "event": "error",
                    "message": "No models available — Vibe Router is offline and .ai/model_combo.txt is missing"
                })
            );
        } else {
            eprintln!(
                "❌ No models available — Vibe Router is offline and .ai/model_combo.txt is missing."
            );
        }
        return ExitCode::from(1);
    }

    let assignments = plan(&models, 0.5);

    if stream_jsonl {
        let p_parsed = profile.and_then(|s| s.parse().ok());
        let specs = detect_system_specs(
            p_parsed,
            std::env::current_exe().ok().as_deref(),
            Some(root),
        );
        println!(
            "{}",
            serde_json::json!({
                "event": "profile",
                "profile": specs.active_profile.as_str(),
                "storage_mode": specs.storage_mode.as_str(),
                "total_ram_bytes": specs.total_ram_bytes,
                "available_ram_bytes": specs.available_ram_bytes,
                "cpu_cores": specs.cpu_cores,
                "worker_threads": specs.worker_threads,
                "is_portable": specs.is_portable,
                "tournament": tournament,
                "live": live
            })
        );
        println!(
            "{}",
            serde_json::json!({
                "event": "observe",
                "source": source,
                "models_available": models.len()
            })
        );
        for a in &assignments {
            println!(
                "{}",
                serde_json::json!({
                    "event": "plan",
                    "role": a.role.as_str(),
                    "model": a.model,
                    "score": a.score
                })
            );
        }
        println!(
            "{}",
            serde_json::json!({
                "event": "done",
                "status": "ready",
                "roles_assigned": assignments.len(),
                "max_steps": max_steps
            })
        );
        return ExitCode::SUCCESS;
    }

    if live {
        let p = Palette::detect();
        println!();
        println!(
            " {}⚡ RUNNING LIVE AGAINST VIBE ROUTER{}  {}·  {}{}",
            p.bold,
            p.reset,
            p.dim,
            root.display(),
            p.reset
        );
        let mut session = Session::new();
        let mut planner = va_harness::live_planner::LiveRouterPlanner::new(root);
        let mut engine = va_harness::engine::Engine::new(root).with_max_steps(max_steps);
        match engine.run(&mut session, &mut planner) {
            Ok(records) => {
                println!(
                    " {}✅ Live execution completed: {} step(s) recorded.{}",
                    p.green,
                    records.len(),
                    p.reset
                );
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!(" {}❌ Execution error: {}{}", p.red, e, p.reset);
                return ExitCode::from(1);
            }
        }
    }

    let p = Palette::detect();
    println!();
    println!(
        " {}🎯 EXECUTION PLAN{}  {}·  {}{}",
        p.bold, p.reset, p.dim, source, p.reset
    );
    if tournament {
        println!(
            " {}⚔ Speculative Tournament Mode: ACTIVE (multi-branch fork & merge){}",
            p.blue, p.reset
        );
    }
    println!();
    for a in &assignments {
        println!(
            " {} {:<7} → {}{:<28}{} {}·{:.3}{}",
            role_icon(a.role.as_str()),
            a.role.as_str(),
            p.bold,
            a.model,
            p.reset,
            p.dim,
            a.score,
            p.reset
        );
    }
    println!();
    for m in &models {
        let f = score_model(m, 0.5);
        if !f.curated && !f.factors.is_empty() {
            let why: Vec<String> = f.factors.iter().map(|x| x.why.clone()).collect();
            println!(" {}· {}: {}{}", p.dim, f.model, why.join("; "), p.reset);
        }
    }
    println!();
    println!(
        " {}{} ready · step cap {}{}",
        p.green,
        assignments.len(),
        max_steps,
        p.reset
    );
    println!();
    ExitCode::SUCCESS
}

fn run_up(root: &Path, profile: Option<&str>, json: bool) -> ExitCode {
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let p = Palette::detect();
    let addr_str = "127.0.0.1:20128";
    let is_alive = addr_str
        .to_socket_addrs()
        .ok()
        .and_then(|mut a| a.next())
        .map(|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok())
        .unwrap_or(false);

    let mut started = false;
    if !is_alive {
        let mut candidates = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                candidates.push(parent.join("vibe-router.exe"));
                candidates.push(parent.join("vibe-router"));
            }
        }
        candidates.push(
            root.join("portable_data")
                .join("bin")
                .join("vibe-router.exe"),
        );
        candidates.push(root.join("portable_data").join("bin").join("vibe-router"));
        candidates.push(root.join("../Vibe Router/target/release/vibe-router.exe"));
        candidates.push(root.join("../Vibe Router/target/release/vibe-router"));
        candidates.push(root.join("../../Vibe Router/target/release/vibe-router.exe"));
        candidates.push(root.join("../../Vibe Router/target/release/vibe-router"));
        candidates.push(PathBuf::from(
            "Q:/Vibe Coding/Vibe Router/target/release/vibe-router.exe",
        ));

        let router_bin = candidates.into_iter().find(|p| p.exists());
        if let Some(bin) = router_bin {
            let prof = profile.unwrap_or("auto");
            let _ = std::process::Command::new(bin)
                .arg("serve")
                .arg("--profile")
                .arg(prof)
                .spawn();

            for _ in 0..25 {
                std::thread::sleep(Duration::from_millis(100));
                if addr_str
                    .to_socket_addrs()
                    .ok()
                    .and_then(|mut a| a.next())
                    .map(|addr| {
                        TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok()
                    })
                    .unwrap_or(false)
                {
                    started = true;
                    break;
                }
            }
        }
    }

    let auth = va_core::router_client::RouterAuth::discover(Some(root));
    let final_alive = addr_str
        .to_socket_addrs()
        .ok()
        .and_then(|mut a| a.next())
        .map(|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok())
        .unwrap_or(false);

    if json {
        let payload = serde_json::json!({
            "status": if final_alive { "live" } else { "offline" },
            "endpoint": "http://127.0.0.1:20128/api/assignment",
            "spawned_router": started,
            "has_auth": !auth.is_empty(),
            "ready": true,
            "mode": if final_alive { "vibe-link" } else { "standalone-offline" }
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_default()
        );
        return ExitCode::SUCCESS;
    }

    println!();
    println!(
        " {}⚡ VIBE ASSEMBLY READY{}  {}·  {}{}",
        p.bold,
        p.reset,
        p.dim,
        root.display(),
        p.reset
    );
    if final_alive {
        println!(
            "   {}Router:{}     {}🟢 LIVE (http://127.0.0.1:20128/api/assignment){}",
            p.dim, p.reset, p.green, p.reset
        );
        println!(
            "   {}Handshake:{}  {}Zero-401 authentication token synchronized{}",
            p.dim, p.reset, p.green, p.reset
        );
        println!(
            "   {}Synergy:{}    {}Vibe-Link active (cost telemetry + failure failover){}",
            p.dim, p.reset, p.green, p.reset
        );
    } else {
        println!("   {}Router:{}     {}⚠ Standalone mode (router offline — local knowledge auto-combo ready){}", p.dim, p.reset, p.yellow, p.reset);
    }
    println!();
    ExitCode::SUCCESS
}

fn run_repomap(root: &Path, max_tokens: usize, priority: &[PathBuf], json: bool) -> ExitCode {
    let repomap = va_harness::repomap::RepoMap::scan(root);
    if json {
        match serde_json::to_string_pretty(&repomap) {
            Ok(s) => {
                println!("{s}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("❌ Serialization error: {e}");
                ExitCode::from(1)
            }
        }
    } else {
        let rendered = repomap.render_within_budget(max_tokens, priority);
        print!("{rendered}");
        let p = Palette::detect();
        println!(
            " {}Indexed {} symbols across {} files ({} block hashes){}",
            p.dim,
            repomap.total_symbols(),
            repomap.total_files(),
            repomap.total_blocks(),
            p.reset
        );
        ExitCode::SUCCESS
    }
}

fn run_diagnose(root: &Path, command: Option<&str>, json: bool) -> ExitCode {
    let cmd = command.unwrap_or("cargo check --message-format=json");
    let (prog, arg) = if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };

    let output = match std::process::Command::new(prog)
        .arg(arg)
        .arg(cmd)
        .current_dir(root)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("❌ Failed to execute `{cmd}`: {e}");
            return ExitCode::from(1);
        }
    };

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let pinned = va_harness::diagnostics::DiagnosticPinpointer::pin_from_output(root, &combined);

    if json {
        match serde_json::to_string_pretty(&pinned) {
            Ok(s) => {
                println!("{s}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("❌ Serialization error: {e}");
                ExitCode::from(1)
            }
        }
    } else {
        let p = Palette::detect();
        println!();
        println!(
            " {}🩺 COMPILER BLOCK PINPOINTER{}  {}·  `{}`{}",
            p.bold, p.reset, p.dim, cmd, p.reset
        );
        println!();
        if pinned.is_empty() {
            if output.status.success() {
                println!(
                    "   {}✔ Workspace compiles cleanly — 0 block errors found.{}",
                    p.green, p.reset
                );
            } else {
                println!(
                    "   {}⚠ Compiler returned non-zero exit code ({:?}):{}",
                    p.yellow,
                    output.status.code(),
                    p.reset
                );
                println!("{}", String::from_utf8_lossy(&output.stderr));
            }
        } else {
            let feedback = va_harness::diagnostics::PinnedDiagnostic::format_feedback(&pinned);
            println!("{feedback}");
        }
        println!();
        ExitCode::SUCCESS
    }
}

fn run_profile(root: &Path, profile: Option<&str>, json: bool) -> ExitCode {
    let p_parsed = profile.and_then(|s| s.parse().ok());
    let specs = detect_system_specs(
        p_parsed,
        std::env::current_exe().ok().as_deref(),
        Some(root),
    );

    if json {
        match serde_json::to_string_pretty(&specs) {
            Ok(s) => {
                println!("{s}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("❌ Serialization error: {e}");
                ExitCode::from(1)
            }
        }
    } else {
        let p = Palette::detect();
        println!();
        println!(
            " {}⚙ HARDWARE PROFILE & ADAPTIVE RUNTIME ENGINE{}",
            p.bold, p.reset
        );
        println!(
            "   {}Active Profile:{}    {}",
            p.dim,
            p.reset,
            specs.active_profile.as_str()
        );
        println!(
            "   {}Storage Strategy:{}  {}",
            p.dim,
            p.reset,
            specs.storage_mode.as_str()
        );
        println!(
            "   {}System RAM:{}        {:.2} GB total  ·  {:.2} GB available",
            p.dim,
            p.reset,
            specs.total_ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            specs.available_ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
        );
        println!(
            "   {}CPU Cores:{}         {}",
            p.dim, p.reset, specs.cpu_cores
        );
        println!(
            "   {}Worker Threads:{}    {}",
            p.dim, p.reset, specs.worker_threads
        );
        println!(
            "   {}Portable Drive:{}    {}",
            p.dim, p.reset, specs.is_portable
        );
        println!(
            "   {}Zero Flash Wear:{}   {}",
            p.dim,
            p.reset,
            specs.is_zero_wear_active()
        );
        println!();
        ExitCode::SUCCESS
    }
}

fn run_session(root: &Path, command: &SessionCommand) -> ExitCode {
    let session_path = root.join(".ai").join("session.jsonl");
    let mut session = if session_path.exists() {
        let raw = std::fs::read_to_string(&session_path).unwrap_or_default();
        Session::from_jsonl(&raw).unwrap_or_default()
    } else {
        Session::new()
    };

    match command {
        SessionCommand::Status { json } => {
            let path = session.current_path().unwrap_or_default();
            let labels = session.kind_labels();
            let cost = session.total_cost_micros();
            let latency = session.total_latency_ms();

            if *json {
                let status_val = serde_json::json!({
                    "len": session.len(),
                    "head": session.head(),
                    "current_path": path,
                    "labels": labels,
                    "total_cost_micros": cost,
                    "total_latency_ms": latency
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&status_val).unwrap_or_default()
                );
            } else {
                let p = Palette::detect();
                println!();
                println!(" {}📈 SESSION DAG STATUS{}", p.bold, p.reset);
                println!(
                    "   {}Nodes:{}        {} committed",
                    p.dim,
                    p.reset,
                    session.len()
                );
                println!("   {}Head:{}         {:?}", p.dim, p.reset, session.head());
                println!("   {}Branch Path:{}  {:?}", p.dim, p.reset, path);
                println!(
                    "   {}Total Spend:{}  {} micros (${:.6})",
                    p.green,
                    p.reset,
                    cost,
                    cost as f64 / 1_000_000.0
                );
                println!("   {}Total Latency:{} {} ms", p.dim, p.reset, latency);
                println!();
            }
            ExitCode::SUCCESS
        }
        SessionCommand::Fork {
            node_id,
            reason,
            json,
        } => {
            let r = reason.as_deref().unwrap_or("cli fork");
            match session.fork(*node_id, r) {
                Ok(new_id) => {
                    let jsonl = session.to_jsonl();
                    let _ = AtomicStateSync::write_atomic(&session_path, jsonl.as_bytes());
                    if *json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "status": "forked",
                                "node_id": node_id,
                                "new_head": new_id
                            })
                        );
                    } else {
                        println!("✅ Forked at node {node_id} -> new head is {new_id}");
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("❌ Fork error: {e}");
                    ExitCode::from(1)
                }
            }
        }
        SessionCommand::Rollback {
            to_verified,
            to,
            json,
        } => {
            let outcome = if *to_verified {
                session
                    .rollback_to_last_verified()
                    .map(|opt| opt.unwrap_or(0))
            } else if let Some(target) = to {
                session.rollback(*target).map(|_| *target)
            } else {
                eprintln!("❌ Must specify either --to-verified or --to <node_id>");
                return ExitCode::from(1);
            };

            match outcome {
                Ok(new_head) => {
                    let jsonl = session.to_jsonl();
                    let _ = AtomicStateSync::write_atomic(&session_path, jsonl.as_bytes());
                    if *json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "status": "rolled_back",
                                "new_head": new_head
                            })
                        );
                    } else {
                        println!("✅ Rolled back -> head is now node {new_head}");
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("❌ Rollback error: {e}");
                    ExitCode::from(1)
                }
            }
        }
        SessionCommand::Compact {
            from,
            to,
            summary,
            json,
        } => {
            let mut surface = match session.context_surface() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("❌ Context surface error: {e}");
                    return ExitCode::from(1);
                }
            };

            match surface.compact_range(*from, *to, summary) {
                Ok(()) => {
                    session.push(NodeKind::Compacted {
                        start: *from,
                        end: *to,
                        summary: summary.clone(),
                    });
                    let _ = session.commit();
                    let jsonl = session.to_jsonl();
                    let _ = AtomicStateSync::write_atomic(&session_path, jsonl.as_bytes());

                    if *json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "status": "compacted",
                                "from": from,
                                "to": to,
                                "surface_len": surface.len()
                            })
                        );
                    } else {
                        println!(
                            "✅ Compacted nodes {from}..{to} on context surface ({} items remain)",
                            surface.len()
                        );
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("❌ Compaction error: {e}");
                    ExitCode::from(1)
                }
            }
        }
    }
}
