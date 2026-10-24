# Vibe Assembly
> **Zero-dollar, infinite-context vibe coding architecture - now 100% native Rust.**

Build large software projects on free or local model tiers without exhausting a
single chat context. Vibe Assembly is a strict multi-agent workspace contract
with native tooling: a state engine, an execution harness, a CLI/TUI, and a GPUI
desktop dashboard.

Everything is Rust. There is no browser runtime, no webview, and no separate
package-manager toolchain.

## The problem

Keeping an entire project history in one chat exhausts token limits, loses
architectural detail, and invites stale edits. The failure is contextual, not a
model-quality problem.

## The solution

Use focused roles that communicate through regulated markdown state in `.ai/`,
and execute every edit through a verifiable harness.

| Role / Mode | Owns |
|---|---|
| Boss (phase inside Code/Web/Debug) | Ticket blueprints and `.ai/` planning state. Never production code. |
| Code (`--mode code`) | Implements only the active ticket, inside `files_allowed`. |
| Web (`--mode web`) | HTML5, CSS, Tailwind, TypeScript, React, and `templates/`. |
| Debug (`--mode debug`) | Compiler diagnostics, root-cause analysis, security audit. |
| Chat (`--mode chat`) | Read-only architecture, explanation, and planning. |
| Judge | Deterministic anti-slop gate, independent of the coding model. |
| Hermes | Documentation and release notes, written from disk evidence. |

Because each task starts fresh and tickets are small, the limit moves from
"chat length" to "ticket size" - and tickets are small by design.

## Execution modes

| Mode | Accent | Writes | Verification |
|---|---|---|---|
| Code | `#F5B800` amber | yes | language lens command |
| Web | `#06B6D4` cyan | yes | language lens command |
| Chat | `#3B82F6` blue | no (hard boundary) | none; read-only |
| Debug | `#EF4444` red | yes | compiler/test command |

Accents are carried through the CLI header, TUI panels, and the desktop header,
selectors, context strip, and active panels.

## Language lens

`--lang auto|rust|c|python|ts|go` selects a language lens. `auto` resolves from
manifests (`Cargo.toml`, `go.mod`, `pyproject.toml`, `package.json`,
`CMakeLists.txt`) and a bounded extension census. Each lens injects its
verification command and judge guardrails into the prompt and the judge.

## The `.ai/` contract

`.ai/active_task.md` holds the queue and exactly one Active Ticket pointer, the
sole execution authority ([ADR-007](.ai/decisions.md)). Each ticket carries an
allowlist, verification-gated steps, and append-only `relay_notes`.

A step counts as verified only when a single note references that step and
carries success evidence. The resume point is the first unverified step, so an
interrupted session resumes exactly where it stopped. Durable recovery lessons
live in `.ai/EXPERIENCE.md`; the trajectory DAG lives in `.ai/session.jsonl`.

## Quick start

```bash
# Build both binaries
cargo build --release

# Create a workspace (seeds .ai/model_combo.txt if absent)
./target/release/vibe-assembly init my-project
cd my-project

# Inspect
vibe-assembly status          # ticket queue + resume point
vibe-assembly models          # model allocation for the six roles
vibe-assembly doctor          # workspace health check
vibe-assembly profile         # hardware specs and adaptive storage mode
vibe-assembly tui             # terminal dashboard with selectors (alias: top)

# Execute one task
vibe-assembly prompt "Add a health endpoint and its test" \
  --mode code --lang rust

# Read-only architect session
vibe-assembly chat --lang rust

# Full live loop with router auto-combo
vibe-assembly run --live --auto-combo --tournament

# Deterministic offline smoke test (never calls the router)
vibe-assembly run --offline-mock

# Desktop Mission Control
cargo run -p va-desktop -- --mode web --lang ts
```

## Workspace layout

```text
Vibe Assembly/
  .ai/                    # AI contract and authoritative state
  crates/
    va-core/              # parser, allocator, router client, lock, prompts,
                          # bootstrap, runtime, execution modes
    va-harness/           # session DAG, patcher, repomap, diagnostics, review,
                          # judge, sandbox, tournament, experience, engine,
                          # live planner
    va-cli/               # vibe-assembly binary: CLI, TUI, chat, MCP
    va-desktop/           # vibe-assembly-desktop: native GPUI dashboard
  templates/              # seeded project starters
  references/             # upstream study material (not built)
```

## The execution harness

`va-harness` makes agent edits safe to trust.

| Module | Role |
|---|---|
| `session` | Append-only transactional trajectory DAG with per-node hashes, fork, rollback, replay, JSONL persistence, auto-compaction at 75%, think-trace eviction, observation slicing, and a pinned working-set tail. |
| `patcher` | Edits address per-block hashes, never line numbers. Whole-file drift assertion, 1-hop recovery, and hard rejection of ambiguous or repeated anchors. |
| `repomap` | Token-budgeted workspace symbol scanner binding functions, structs, traits, and classes to deterministic block hashes. |
| `diagnostics` | Dual-format compiler pinpointer (`cargo --message-format=json` and text) mapping error spans to block hashes and snippets. |
| `review` | Git checkpoint per phase, isolating each step's exact delta. |
| `judge` | Deterministic gate: slop comments, placeholders, unsafe primitives in production, import-only churn, excessive churn, trailing whitespace, plus language and mode guardrails. One targeted retry, then abort. |
| `sandbox` | Confined command execution with wall-clock timeouts, directory confinement, and bounded output. |
| `tournament` | Multi-branch speculative runner scoring candidates by warnings and line churn, merging the winner into the DAG. |
| `experience` | Harness^2 c1..c8 surfaces with strict reset-to-base, contrastive recovery, and durable `.ai/EXPERIENCE.md` lessons. |
| `live_planner` | Real `POST /v1/chat/completions` Boss -> Code -> Verify -> Debug -> Judge loop with Vibe-Link headers and telemetry. |
| `engine` | Orchestration with auto-compaction, read-only enforcement, stagnation detection, and judge-verified hashes. |

```bash
# Router readiness and token handshake
vibe-assembly up

# Discover symbols and block hashes
vibe-assembly repomap --max-tokens 2048

# Pinpoint compiler diagnostics to block hashes
vibe-assembly diagnose

# Discover block hashes in one file
vibe-assembly blocks src/main.rs

# Patch by hash (4-hex short form or full 64-bit value)
vibe-assembly patch src/main.rs --target 159A <<< 'pub fn beta(y: i32) -> i32 {
    y * 42
}'

# Session DAG operations
vibe-assembly session status --json
vibe-assembly session rollback --to-verified
vibe-assembly session compact --from 2 --to 4 --summary "Collapsed failed attempts"

# MCP server over stdio
vibe-assembly mcp
```

## Router integration

`va-core::router_client` performs a real OpenAI-compatible
`POST /v1/chat/completions` over raw HTTP/1.1 (no TLS stack; point it at a local
router or a TLS-terminating proxy). Requests carry `X-Vibe-Role`,
`X-Vibe-Node-Id`, `X-Vibe-Profile`, and `X-Vibe-Retry`, plus optional
`Authorization: Bearer` or `X-API-Key`. Cost and latency are read from
`X-Vibe-Router-Cost-Micros` / `X-VR-Cost-Micros` and the matching latency headers
and recorded in the session DAG.

Model resolution order: pinned `--model`, per-mode override
(`.ai/models/<mode>.txt`), router assignment, `.ai/model_combo.txt`, then a
conservative default. `init` and `run --auto-combo` auto-seed the combo file.
The judge is always deterministic and independent; `--judge-model` is reported
for audit and display.

## Context safety

- Auto-compaction triggers at 75% of the active model's context window.
- Older verified steps and failed retries collapse into surface summaries while
  the DAG remains intact for `replay()` and `rollback_to_last_verified()`.
- Reasoning traces are evicted from prompts but retained in DAG nodes.
- Large observations render head 30 + `[... elided N lines ...]` + tail 30.
- The prompt always ends with fresh block hashes and line ranges for modified
  files.
- Two identical failures trigger `StagnationDetected`; policy can pivot or
  escalate to `SpeculativeTournament`.

## Hardware-aware runtime

- **Disk-first VPS mode** (`--profile vps`, or available RAM < 1 GB): bounded
  workers and disk-streamed state to keep resident memory low.
- **RAM-first portable mode** (`--profile portable`, host RAM >= 4 GB): hydrate
  state into RAM, perform intermediate edits in memory, and flush on verified
  checkpoints with crash-safe atomic writes to reduce flash wear.
- **Drive-letter independence:** workspace-relative paths normalize through the
  `{{PORTABLE_ROOT}}` token.
- **Universal gateway fallback:** if the assignment endpoint returns 404, the
  client queries `GET /v1/models` and computes a six-role allocation heuristically.

## Verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```

Current workspace: 188 tests passing, clippy clean, both release binaries built.

## Building on Windows

Install the MSVC toolchain and Visual Studio Build Tools (C++ workload), then
run Cargo from a normal shell. If a machine still hits
`STATUS_STACK_BUFFER_OVERRUN` while compiling GPUI dependencies, build from a
shell with `ulimit -s unlimited`; `RUST_MIN_STACK` is not sufficient.

## License

MIT