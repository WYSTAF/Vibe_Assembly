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
vibe-assembly models          # model allocation for the five modes
vibe-assembly doctor          # workspace health check
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
│   │                       #   knowledge (model scoring), bootstrap
│   ├── va-harness/         # Execution engine: session, patcher, review, judge, engine
│   ├── va-cli/             # `vibe-assembly` binary — terminal operations room
│   └── va-desktop/         # `vibe-assembly-desktop` — native GPUI dashboard
├── templates/              # Seeded project starters (planner, todo, website)
└── references/             # Upstream harnesses (not built)
```

## ⚙️ The execution harness

`va-harness` is the part that makes an agent's edits safe to trust.

| Module | Role |
|---|---|
| `session` | Append-only transactional trajectory DAG. Per-node state hashing, `fork`, `rollback`, `replay`. Replay reuses stored results — no tool re-runs. |
| `patcher` | Edits address **per-block hashes**, never line numbers. Recovers from a uniform line shift; hard-rejects an ambiguous hash rather than guessing. |
| `review` | Git checkpoint per phase, isolating each step's exact delta for review. |
| `judge` | Deterministic gate before commit: slop comments, placeholders, `unwrap()` in production, import-only churn, excessive churn. One targeted retry, then abort. |
| `engine` | Observe → Think → Patch → Verify → Judge. Model-agnostic via a `Planner` trait. |

```bash
# Discover block hashes in a file
vibe-assembly blocks src/main.rs

# Patch by hash (4-hex short form or the full 64-bit value)
vibe-assembly patch src/main.rs --target 159A <<< 'pub fn beta(y: i32) -> i32 {
    y * 42
}'

# Zero-config role assignment from Vibe Router
vibe-assembly run --auto-combo
```

**Vibe Router.** The client targets `/api/assignment` (the `/v1` path does not
exist on the gateway) and honours `VIBE_ROUTER_TOKEN` (Bearer) or
`VIBE_ROUTER_API_KEY` (`X-API-Key`). Without credentials it falls back to
`.ai/model_combo.txt` and says so. Models the curated table has never seen are
scored from their name — version progression and behavioural tags — so
`gemini-3.8-flash` or `glm-5.3` still allocates sensibly.

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
