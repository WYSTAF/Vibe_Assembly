# 🏗️ Vibe Assembly

> **Zero-Dollar, Infinite-Context Vibe Coding Architecture — now 100% native Rust.**

Build massive software projects using free API tiers without ever hitting context
limits or AI hallucinations. Vibe Assembly is a strict multi-agent workspace
contract with native tooling: a state engine, a CLI, and a GPUI desktop dashboard.

There is no Node.js, no npm, and no `package.json`. Everything is Rust.

## 🛑 The problem

When coding with LLMs, keeping the entire project history in a single chat
exhausts the token limits of free APIs (OpenRouter, Cloudflare Workers AI,
Gemini free tier). The AI loses track of the architecture, hallucinates, and
breaks the codebase. This is a **context** problem, not a model problem.

## 💡 The solution

Stop using one AI employee. Use **five, who never share a chat**. They
communicate only through regulated markdown files in `.ai/`.

| Mode | Role | Owns |
|---|---|---|
| 👑 Boss | Architect + PM. Writes ticket blueprints. | All `.ai/` planning state. **Never production code.** |
| 💬 Chat | Product brainstorming. | Conversation only. |
| 💻 Code | Implements only what Boss authorized. | One pointed ticket, inside `files_allowed`. |
| 🐞 Debug | Surgical fixes when things break. | `known_bugs.md`. |
| 📣 Hermes | Docs and release notes, from disk evidence. | Root `*.md`, `docs/`, `plans/`, `changelog.md`. |

Because each mode starts with a fresh context, none of them hits the limit. The
limit moves from "chat length" to "ticket size" — and tickets are small by design.

## 🧠 The `.ai/` contract

`.ai/active_task.md` holds the queue and **exactly one Active Ticket pointer**,
which is the sole execution authority ([ADR-007](.ai/decisions.md)). Each ticket
carries an allowlist, verification-gated steps, and append-only `relay_notes`.

After each step, Code appends one note with the step number, changed paths, exact
command, and exit code. A step counts as **verified** only when a *single* note
both references that step (word-boundary matched) and carries success evidence.
The **resume point** is the first unverified step — so an interrupted session
resumes exactly where it stopped.

## ⚡ Quick start

```bash
# Create a workspace
cargo run -p va-cli -- init my-project

# Inspect it
cd my-project
vibe-assembly status          # ticket queue + resume point
vibe-assembly models          # model allocation for the modes
vibe-assembly doctor          # workspace health check
vibe-assembly profile         # hardware specs & adaptive storage mode
vibe-assembly tui             # lightweight terminal dashboard (alias: top)
vibe-assembly --json          # machine-readable status

# Launch the desktop dashboard
cargo run -p va-desktop
```

## 📦 Workspace layout

```
Vibe Assembly/
├── .ai/                    # The AI contract — authoritative state
├── crates/
│   ├── va-core/            # State engine: parser, allocator, router, lock, prompts,
│   │                       #   knowledge (model scoring), runtime (hardware profiler)
│   ├── va-harness/         # Execution engine: session, patcher, review, judge, sandbox, engine
│   ├── va-cli/             # `vibe-assembly` binary — terminal operations room & MCP server
│   └── va-desktop/         # `vibe-assembly-desktop` — native GPUI dashboard
├── templates/              # Seeded project starters (planner, todo, website)
└── references/             # Upstream harnesses (not built)
```

## ⚙️ The execution harness

`va-harness` is the part that makes an agent's edits safe to trust.

| Module | Role |
|---|---|
| `session` | Append-only transactional trajectory DAG. Per-node state hashing, `fork`, `rollback`, `replay`, `ContextSurface` compaction. Replay reuses stored results — no tool re-runs. |
| `patcher` | Edits address **per-block hashes**, never line numbers. Whole-file drift assertion (from `oh-my-pi`), 1-hop recovery from line shifts, and hard rejection on ambiguity. |
| `review` | Git checkpoint per phase, isolating each step's exact delta for review. |
| `judge` | Deterministic gate before commit: slop comments, placeholders, `unwrap()` in production, import-only churn, excessive churn. One targeted retry, then abort. |
| `sandbox` | Confined command execution with wall-clock timeouts, directory confinement, and bounded output capture. |
| `engine` | Observe → Think → Patch → Verify → Judge loop with durable `onYield` completion gate and `judge_verified_hash`. |

```bash
# Discover block hashes in a file
vibe-assembly blocks src/main.rs

# Patch by hash (4-hex short form or the full 64-bit value)
vibe-assembly patch src/main.rs --target 159A <<< 'pub fn beta(y: i32) -> i32 {
    y * 42
}'

# Zero-config role assignment with JSONL stream for Hermes Agent / external automation
vibe-assembly run --auto-combo --stream-jsonl

# Manage trajectory DAG
vibe-assembly session status --json
vibe-assembly session rollback --to-verified
vibe-assembly session compact --from 2 --to 4 --summary "Collapsed failed attempts"

# Start Model Context Protocol (MCP) server over stdio
vibe-assembly mcp
```

## 🚀 Hardware-aware adaptive runtime

Vibe Assembly adapts seamlessly to execution hardware:

- **Mode 1: Disk-First (VPS Ultra-Low-RAM Mode)** (`--profile vps` or available RAM < 1GB):
  Worker threads constrained to 2 cores; session DAG, state updates, and checkpoints stream directly to disk with minimal buffering, maintaining total resident memory (RSS) strictly under ~35MB.
- **Mode 2: RAM-First (Portable Zero-Wear Mode)** (`--profile portable` on host with >= 4GB RAM):
  Hydrates workspace state and DAG into host RAM; performs intermediate edits, retries, and hash checks 100% in memory to prevent USB flash drive write amplification; flushes state to storage on verified checkpoints using crash-safe atomic write-to-temp + `fsync` + rename.
- **Drive-Letter Independence**: Workspace-relative paths are normalized using the `{{PORTABLE_ROOT}}` token and resolved dynamically against the executable, allowing USB drives to switch host drive letters without broken links.
- **Universal Gateway Fallback**: If the Vibe Router assignment endpoint returns `404 Not Found` (common on standard OpenAI proxies like `9router`, LiteLLM, or Ollama), the client automatically queries `GET /v1/models` and computes optimal 6-role allocations via heuristic auto-combo.
- **Vibe-Link Synergy**: Outbound requests attach contextual headers (`X-Vibe-Role`, `X-Vibe-Node-Id`, `X-Vibe-Profile`, `X-Vibe-Retry`), support automatic zero-401 handshake via local `.vibe_router_token`, and ingest micro-dollar cost and latency telemetry (`X-VR-Cost-Micros`, `X-VR-Latency-Ms`) directly into session DAG nodes.

## 🧩 Feature parity with the legacy JS tooling

Every capability from the original Node.js implementation is now native Rust:

| Legacy (JS) | Native (Rust) |
|---|---|
| `bin/lib/parser.js` | `va-core/src/parser.rs` |
| `bin/allocate-models.js` | `va-core/src/allocator.rs` |
| `bin/lib/router.js` | `va-core/src/router.rs` |
| `bin/lib/lock.js` | `va-core/src/lock.rs` |
| `bin/reset-template-state.js` | `va-core/src/bootstrap.rs` |
| `bin/va.js` (the `va` CLI) | `vibe-assembly` binary — `va-cli/src/main.rs` |
| `.roo/rules/*.md`, `.roomodes` | `va-core/src/prompts.rs` |
| `app/desktop-rust/` | `va-desktop/` |

## 🔨 Building on Windows

GPUI pulls ~700 transitive crates. On Windows, `rustc` can abort with
`STATUS_STACK_BUFFER_OVERRUN` compiling a few heavy dependencies. Raise the
stack before invoking cargo:

```bash
bash -c 'ulimit -s unlimited; cargo build'
```

`RUST_MIN_STACK` does **not** fix this — the overflow is in the compiler
process, not a worker thread. Build in debug; `opt-level=2` uses far more
codegen stack.

## 🧪 Verification

```bash
cargo fmt --all
cargo clippy --workspace --all-targets   # 0 warnings
cargo test --workspace
```

## 📄 License

MIT
