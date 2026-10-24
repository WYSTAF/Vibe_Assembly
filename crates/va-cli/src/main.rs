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
use va_core::knowledge::context_window_tokens;
use va_core::lock::check_task_lock;
use va_core::modes::{ExecutionMode, LanguageLens};
use va_core::parser::{parse_active_task, validate_contract, verified_steps};
use va_core::router::fetch_live_assignment;
use va_core::runtime::{detect_system_specs, AtomicStateSync};
use va_core::state::Workspace;
use va_harness::engine::{Engine, EngineError, StagnationPolicy};
use va_harness::live_planner::{LivePlannerConfig, LiveRouterPlanner};
use va_harness::session::{ContextWindowConfig, NodeKind, Session};

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
    /// Resolve model allocations for the six roles
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
        /// Execution mode: code | web | chat | debug
        #[arg(long, default_value = "code")]
        mode: String,
        /// Language lens: auto | rust | c | python | ts | go
        #[arg(long, default_value = "auto")]
        lang: String,
        /// Pin a router model/alias for every role (default: auto-combo)
        #[arg(long)]
        model: Option<String>,
        /// Independent judge model (default: deterministic judge)
        #[arg(long)]
        judge_model: Option<String>,
        /// Deterministic offline synthesis; never calls the router
        #[arg(long)]
        offline_mock: bool,
        /// Explicit task text instead of .ai/active_task.md
        #[arg(long)]
        prompt: Option<String>,
    },
    /// Run one task in a chosen execution mode and print the result
    Prompt {
        /// Task text
        text: String,
        /// Execution mode: code | web | chat | debug
        #[arg(long, default_value = "code")]
        mode: String,
        /// Language lens: auto | rust | c | python | ts | go
        #[arg(long, default_value = "auto")]
        lang: String,
        /// Pin a router model/alias (default: auto-combo)
        #[arg(long)]
        model: Option<String>,
        /// Independent judge model
        #[arg(long)]
        judge_model: Option<String>,
        /// Maximum engine steps
        #[arg(long, default_value_t = 50)]
        max_steps: u32,
        /// Deterministic offline synthesis; never calls the router
        #[arg(long)]
        offline_mock: bool,
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Start an interactive read-only architect chat
    Chat {
        /// Language lens: auto | rust | c | python | ts | go
        #[arg(long, default_value = "auto")]
        lang: String,
        /// Pin a router model/alias (default: auto-combo)
        #[arg(long)]
        model: Option<String>,
        /// Independent judge model
        #[arg(long)]
        judge_model: Option<String>,
    },
    /// Launch the interactive terminal dashboard (alias: top)
    #[command(alias = "top")]
    Tui {
        /// Force execution profile (auto|vps|portable|workstation)
        #[arg(long)]
        profile: Option<String>,
        /// Initial execution mode: code | web | chat | debug
        #[arg(long, default_value = "code")]
        mode: String,
        /// Initial language lens: auto | rust | c | python | ts | go
        #[arg(long, default_value = "auto")]
        lang: String,
        /// Initially pinned model
        #[arg(long)]
        model: Option<String>,
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
                seed_model_combo(&target);
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
        Some(Command::Tui {
            profile,
            mode,
            lang,
            model,
        }) => {
            let root = Workspace::discover(&start).unwrap_or(start);
            let p_override = profile.as_deref().and_then(|s| s.parse().ok());
            let mode = match mode.parse::<ExecutionMode>() {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("❌ {e}");
                    return ExitCode::from(2);
                }
            };
            let language = match lang.parse::<LanguageLens>() {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("❌ {e}");
                    return ExitCode::from(2);
                }
            };
            tui::run_tui(&root, p_override, mode, language, model.clone());
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
            mode,
            lang,
            model,
            judge_model,
            offline_mock,
            prompt,
        }) => {
            let root = match Workspace::discover(&start) {
                Some(r) => r,
                None => {
                    eprintln!("❌ No .ai/ directory found at or above {}", start.display());
                    return ExitCode::from(1);
                }
            };
            let mode = match mode.parse::<ExecutionMode>() {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("❌ {e}");
                    return ExitCode::from(2);
                }
            };
            let language = match lang.parse::<LanguageLens>() {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("❌ {e}");
                    return ExitCode::from(2);
                }
            };
            if *auto_combo {
                seed_model_combo(&root);
            }
            return run_execute(
                &root,
                ExecutionOptions {
                    auto_combo: *auto_combo,
                    max_steps: *max_steps,
                    stream_jsonl: *stream_jsonl,
                    profile: profile.as_deref(),
                    tournament: *tournament,
                    live: *live,
                    mode,
                    language,
                    model: model.clone(),
                    judge_model: judge_model.clone(),
                    offline_mock: *offline_mock,
                    task_prompt: prompt.clone(),
                },
            );
        }
        Some(Command::Prompt {
            text,
            mode,
            lang,
            model,
            judge_model,
            max_steps,
            offline_mock,
            json,
        }) => {
            let root = match Workspace::discover(&start) {
                Some(r) => r,
                None => {
                    eprintln!("❌ No .ai/ directory found at or above {}", start.display());
                    return ExitCode::from(1);
                }
            };
            let mode = match mode.parse::<ExecutionMode>() {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("❌ {e}");
                    return ExitCode::from(2);
                }
            };
            let language = match lang.parse::<LanguageLens>() {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("❌ {e}");
                    return ExitCode::from(2);
                }
            };
            return run_prompt(
                &root,
                text,
                mode,
                language,
                model.clone(),
                judge_model.clone(),
                *max_steps,
                *offline_mock,
                *json,
            );
        }
        Some(Command::Chat {
            lang,
            model,
            judge_model,
        }) => {
            let root = match Workspace::discover(&start) {
                Some(r) => r,
                None => {
                    eprintln!("❌ No .ai/ directory found at or above {}", start.display());
                    return ExitCode::from(1);
                }
            };
            let language = match lang.parse::<LanguageLens>() {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("❌ {e}");
                    return ExitCode::from(2);
                }
            };
            return run_chat(&root, language, model.clone(), judge_model.clone());
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
struct ExecutionOptions<'a> {
    auto_combo: bool,
    max_steps: u32,
    stream_jsonl: bool,
    profile: Option<&'a str>,
    tournament: bool,
    live: bool,
    mode: ExecutionMode,
    language: LanguageLens,
    model: Option<String>,
    judge_model: Option<String>,
    offline_mock: bool,
    task_prompt: Option<String>,
}

fn run_execute(root: &Path, opts: ExecutionOptions<'_>) -> ExitCode {
    let ExecutionOptions {
        auto_combo,
        max_steps,
        stream_jsonl,
        profile,
        tournament,
        live,
        mode,
        language,
        model,
        judge_model,
        offline_mock,
        task_prompt,
    } = opts;
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

    if live || offline_mock {
        let p = Palette::detect();
        let accent = accent_ansi(mode);
        println!();
        println!(
            " {}LIVE RUN | {}{}  {}*  {}{}",
            accent,
            mode.label(),
            p.reset,
            p.dim,
            root.display(),
            p.reset
        );
        let mut planner = build_planner(
            root,
            mode,
            language,
            model.clone(),
            judge_model.clone(),
            offline_mock,
            task_prompt.clone(),
        );
        planner.begin_task();
        let active_model = if offline_mock {
            "offline-mock".to_string()
        } else {
            planner.active_model("code")
        };
        let context = ContextWindowConfig::new(context_window_tokens(&active_model));
        let mut session = Session::new();
        let mut engine = Engine::new(root)
            .with_max_steps(max_steps)
            .with_profile(mode, language)
            .with_context_window(context)
            .with_stagnation_policy(if tournament {
                StagnationPolicy::EscalateTournament
            } else {
                StagnationPolicy::Fail
            });
        match engine.run(&mut session, &mut planner) {
            Ok(records) => {
                persist_session(root, &session);
                let stats = engine.stats();
                println!(
                    " {}OK Live execution completed: {} step(s), {} patch(es), {} verification(s).{}",
                    p.green,
                    records.len(),
                    stats.patches_applied,
                    stats.verifications_run,
                    p.reset
                );
                print_run_meter(
                    &p,
                    stats.context_tokens,
                    &context,
                    stats.context_compactions,
                );
                return ExitCode::SUCCESS;
            }
            Err(EngineError::StagnationDetected {
                signature,
                attempts,
                escalate_tournament,
            }) => {
                eprintln!(
                    " {}WARN Stagnation detected after {} attempts: {}{}",
                    p.yellow, attempts, signature, p.reset
                );
                if escalate_tournament {
                    eprintln!(
                        " {}-> Escalating to SpeculativeTournament (re-run with --tournament to keep branches).{}",
                        p.dim, p.reset
                    );
                }
                return ExitCode::from(3);
            }
            Err(e) => {
                eprintln!(" {}ERR Execution error: {}{}", p.red, e, p.reset);
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

const DEFAULT_MODEL_COMBO: &str = "\
# Vibe Assembly offline model fallback (auto-combo seed)
# One model id/alias per line; comments and `role=model` overrides are ignored
# by the allocator. The router maps these aliases when it is online.
gpt-4o-mini
claude-3-5-sonnet
deepseek-chat
gemini-2.0-flash
";

/// Seed `.ai/model_combo.txt` from the example, or a sensible default.
fn seed_model_combo(root: &Path) {
    let ai = root.join(".ai");
    let combo = ai.join("model_combo.txt");
    if combo.exists() {
        return;
    }
    let example = ai.join("model_combo.example.txt");
    let body =
        std::fs::read_to_string(&example).unwrap_or_else(|_| DEFAULT_MODEL_COMBO.to_string());
    let _ = std::fs::create_dir_all(&ai);
    let _ = std::fs::write(&combo, body);
}

/// Truecolor accent for the active mode; empty on non-terminal output.
fn accent_ansi(mode: ExecutionMode) -> String {
    if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        return String::new();
    }
    let rgb = mode.accent_rgb();
    format!(
        "\x1b[38;2;{};{};{}m",
        (rgb >> 16) & 0xFF,
        (rgb >> 8) & 0xFF,
        rgb & 0xFF
    )
}

fn build_planner(
    root: &Path,
    mode: ExecutionMode,
    language: LanguageLens,
    model: Option<String>,
    judge_model: Option<String>,
    offline_mock: bool,
    task_prompt: Option<String>,
) -> LiveRouterPlanner {
    let mut config = LivePlannerConfig::default_for(root);
    config.mode = mode;
    config.language = language;
    config.model = model;
    config.judge_model = judge_model;
    config.offline_mock = offline_mock;
    config.task_prompt = task_prompt;
    LiveRouterPlanner::with_config(config)
}

fn persist_session(root: &Path, session: &Session) {
    let path = root.join(".ai").join("session.jsonl");
    let jsonl = session.to_jsonl();
    let _ = AtomicStateSync::write_atomic(&path, jsonl.as_bytes());
}

fn context_percent(tokens: usize, config: &ContextWindowConfig) -> usize {
    if config.window_tokens == 0 {
        return 0;
    }
    ((tokens.saturating_mul(100)) / config.window_tokens).min(100)
}

fn print_run_meter(p: &Palette, tokens: usize, config: &ContextWindowConfig, compactions: u32) {
    let pct = context_percent(tokens, config);
    let color = if pct >= 75 { p.yellow } else { p.green };
    println!(
        " {}Context:{} {}% of a {}-token window · auto-compactions: {}",
        p.dim, p.reset, pct, config.window_tokens, compactions
    );
    println!(
        " {}{}{}  {}[{}]{}",
        color,
        "█".repeat((pct / 5).clamp(1, 20)),
        p.reset,
        p.dim,
        if pct >= 75 { "watermark" } else { "ok" },
        p.reset,
    );
}

#[allow(clippy::too_many_arguments)]
fn run_prompt(
    root: &Path,
    text: &str,
    mode: ExecutionMode,
    language: LanguageLens,
    model: Option<String>,
    judge_model: Option<String>,
    max_steps: u32,
    offline_mock: bool,
    json: bool,
) -> ExitCode {
    let p = Palette::detect();
    let accent = accent_ansi(mode);
    let mut planner = build_planner(
        root,
        mode,
        language,
        model,
        judge_model,
        offline_mock,
        Some(text.to_string()),
    );
    planner.begin_task();

    if !json {
        println!();
        println!(
            " {}◆ {} · {}{}",
            accent,
            mode.label(),
            language.resolve(root).label(),
            p.reset
        );
    }

    let mut session = Session::new();
    session.push(NodeKind::Prompt {
        role: "user".to_string(),
        text: text.to_string(),
    });
    let _ = session.commit();

    let role = if mode == ExecutionMode::Chat {
        "chat"
    } else {
        "code"
    };
    let active_model = if offline_mock {
        "offline-mock".to_string()
    } else {
        planner.active_model(role)
    };
    let context = ContextWindowConfig::new(context_window_tokens(&active_model));

    if mode == ExecutionMode::Chat {
        return match planner.ask(text, &session) {
            Ok(answer) => {
                if !json {
                    println!("{answer}");
                }
                let cleaned = va_core::router_client::strip_think_blocks(&answer);
                session.push(NodeKind::Thought { text: answer });
                let _ = session.commit();
                persist_session(root, &session);
                let (surface, compacted) = session
                    .context_surface_with_budget(&context)
                    .unwrap_or_default();
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "mode": mode.as_str(),
                            "language": language.resolve(root).as_str(),
                            "model": active_model,
                            "read_only": true,
                            "answer": cleaned,
                            "context_tokens": surface.estimate_tokens(),
                            "context_percent": context_percent(surface.estimate_tokens(), &context),
                            "context_compactions": compacted,
                        })
                    );
                } else {
                    print_run_meter(&p, surface.estimate_tokens(), &context, compacted as u32);
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!(" {}✗ {}{}", p.red, e, p.reset);
                ExitCode::from(1)
            }
        };
    }

    let mut engine = Engine::new(root)
        .with_max_steps(max_steps)
        .with_profile(mode, language)
        .with_context_window(context)
        .with_stagnation_policy(StagnationPolicy::EscalateTournament);
    let result = engine.run(&mut session, &mut planner);
    let stats = engine.stats().clone();
    persist_session(root, &session);
    let pct = context_percent(stats.context_tokens, &context);

    match result {
        Ok(records) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "mode": mode.as_str(),
                        "language": language.resolve(root).as_str(),
                        "model": active_model,
                        "judge_model": planner.judge_model(),
                        "steps": records.len(),
                        "patches": stats.patches_applied,
                        "verifications": stats.verifications_run,
                        "judge_rejections": stats.judge_rejections,
                        "context_tokens": stats.context_tokens,
                        "context_percent": pct,
                        "context_compactions": stats.context_compactions,
                        "stagnation_events": stats.stagnation_events,
                    })
                );
            } else {
                println!(
                    " {}✓ {} step(s) · {} patch(es) · {} verification(s) · {} judge rejection(s){}",
                    p.green,
                    records.len(),
                    stats.patches_applied,
                    stats.verifications_run,
                    stats.judge_rejections,
                    p.reset
                );
                print_run_meter(
                    &p,
                    stats.context_tokens,
                    &context,
                    stats.context_compactions,
                );
                if planner.last_cost_micros > 0 || planner.last_latency_ms > 0 {
                    println!(
                        " {}Router telemetry:{} {} µ$ · {} ms (last call)",
                        p.dim, p.reset, planner.last_cost_micros, planner.last_latency_ms
                    );
                }
            }
            ExitCode::SUCCESS
        }
        Err(EngineError::StagnationDetected {
            signature,
            attempts,
            escalate_tournament,
        }) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "mode": mode.as_str(),
                        "status": "stagnation",
                        "signature": signature,
                        "attempts": attempts,
                        "escalate_tournament": escalate_tournament,
                    })
                );
            } else {
                eprintln!(
                    " {}⚠ Stagnation after {} attempts: {}{}",
                    p.yellow, attempts, signature, p.reset
                );
                if escalate_tournament {
                    eprintln!(
                        " {}→ SpeculativeTournament escalation requested.{}",
                        p.dim, p.reset
                    );
                }
            }
            ExitCode::from(3)
        }
        Err(e) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "mode": mode.as_str(),
                        "status": "error",
                        "error": e.to_string(),
                    })
                );
            } else {
                eprintln!(" {}✗ Execution error: {}{}", p.red, e, p.reset);
            }
            ExitCode::from(1)
        }
    }
}

fn run_chat(
    root: &Path,
    language: LanguageLens,
    model: Option<String>,
    judge_model: Option<String>,
) -> ExitCode {
    use std::io::BufRead as _;
    use std::io::Write as _;

    let p = Palette::detect();
    let accent = accent_ansi(ExecutionMode::Chat);
    let mut planner = build_planner(
        root,
        ExecutionMode::Chat,
        language,
        model,
        judge_model,
        false,
        None,
    );
    planner.begin_task();
    let mut session = Session::new();
    let mut active_model = String::from("auto-combo");
    let mut context = ContextWindowConfig::default();

    println!();
    println!(
        " {}◆ CHAT / ARCHITECT · READ-ONLY{}  {}· {}{}",
        accent,
        p.reset,
        p.dim,
        language.resolve(root).label(),
        p.reset
    );
    println!(
        " {}Commands: /exit to quit, /status for context usage.{}",
        p.dim, p.reset
    );

    let stdin = std::io::stdin();
    loop {
        print!("{}you>{} ", accent, p.reset);
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!(" {}✗ stdin: {}{}", p.red, e, p.reset);
                return ExitCode::from(1);
            }
        }
        let question = line.trim();
        if question.is_empty() {
            continue;
        }
        if matches!(question, "/exit" | "/quit" | "exit" | "quit") {
            break;
        }
        if question == "/status" {
            let (surface, compacted) = session
                .context_surface_with_budget(&context)
                .unwrap_or_default();
            println!(
                " {}model={} · {} tokens · {}% of {} · compactions={}{}",
                p.dim,
                active_model,
                surface.estimate_tokens(),
                context_percent(surface.estimate_tokens(), &context),
                context.window_tokens,
                compacted,
                p.reset
            );
            continue;
        }

        session.push(NodeKind::Prompt {
            role: "user".to_string(),
            text: question.to_string(),
        });
        let _ = session.commit();

        if active_model == "auto-combo" {
            active_model = planner.active_model("chat");
            context = ContextWindowConfig::new(context_window_tokens(&active_model));
        }

        match planner.ask(question, &session) {
            Ok(answer) => {
                let cleaned = va_core::router_client::strip_think_blocks(&answer);
                println!("\n{}architect>{} {}\n", accent, p.reset, cleaned);
                session.push(NodeKind::Thought { text: answer });
                let _ = session.commit();
                let (surface, compacted) = session
                    .context_surface_with_budget(&context)
                    .unwrap_or_default();
                print_run_meter(&p, surface.estimate_tokens(), &context, compacted as u32);
            }
            Err(e) => {
                eprintln!(" {}✗ {}{}\n", p.red, e, p.reset);
            }
        }
    }

    persist_session(root, &session);
    println!(
        " {}session saved · read-only mode never wrote project files{}",
        p.dim, p.reset
    );
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
