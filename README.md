<p align="center">
  <img src="assets/vibe-assembly.svg" alt="Vibe Assembly" width="720">
</p>

# Vibe Assembly

**v2.0.0 - a production-hardened, pure-Rust multi-agent coding workspace.**

Vibe Assembly lets small, focused agents build and maintain large software
projects without exhausting a single chat context. Roles communicate through a
regulated `.ai/` contract, and every edit passes through a deterministic,
hash-anchored execution harness.

**Current status:** 200 tests passing, clippy clean with `-D warnings`, both
release binaries build. There is no browser runtime, webview, or Node/npm
toolchain.

## The problem

Keeping an entire project history in one chat exhausts token limits, loses
architectural detail, and invites stale edits. The failure is contextual, not a
model-quality problem.

## The solution

Use focused roles that communicate through regulated markdown state in `.ai/`,
and execute every edit through a verifiable harness. Because each task starts
fresh and tickets are small, the limit moves from "chat length" to "ticket
size" - and tickets are small by design.

### Execution modes

| Mode | Accent | Writes | Purpose |
|---|---|---|---|
| Code | `#F5B800` amber | yes | Implement the active ticket with verification and judge gates |
| Web | `#06B6D4` cyan | yes | HTML5, CSS, Tailwind, TypeScript, React, and `templates/` |
| Chat | `#3B82F6` blue | **no** | Read-only architecture, explanation, and planning |
| Debug | `#EF4444` red | yes | Compiler diagnostics, root-cause analysis, security audit |

The native desktop workbench uses Zed One Dark surfaces: `#181A1F` sunken
chrome, `#21252B` editor, `#282C34` / `#2C313A` elevated rows and borders, and
`#DCDFE4` / `#ABB2BF` / `#5C6370` text. The active mode accent is carried
through the titlebar, activity strip, editor, composer, and status bar. The TUI
and desktop also expose a live context meter.

### Planner roles

| Role | Responsibility |
|---|---|
| Boss | Chooses files, exact block hashes, and a verification command |
| Code | Authors complete block replacements |
| Debug | Repairs a failed verification from pinned diagnostics |
| Judge | Deterministic anti-slop gate, independent of the coding model |
| Chat | Answers questions without writing files |
| Hermes | Documentation and release notes, written from disk evidence |

The live loop is:

```text
Observe -> Think (Boss) -> Patch (Code) -> Verify
        -> Debug on failure -> Judge -> Commit
```

## Requirements

- Rust stable (edition 2021).
- Windows: the MSVC toolchain and Visual Studio Build Tools with the C++ workload.
- A local OpenAI-compatible router for live runs. Offline mock runs need no router.
- No Node.js, npm, Electron, or browser runtime.

## Install

```bash
git clone https://github.com/WYSTAF/Vibe_Assembly.git
cd Vibe_Assembly

# Build both binaries
cargo build --release

# Optional: put `vibe-assembly` on PATH
cargo install --path crates/va-cli
```

The release binaries are:

- `target/release/vibe-assembly`
- `target/release/vibe-assembly-desktop`

## Quick start

```bash
# From the repository root, so templates/ is discoverable
./target/release/vibe-assembly init my-project
cd my-project

# Inspect the workspace
vibe-assembly status          # ticket queue + resume point
vibe-assembly models          # model allocation for the six roles
vibe-assembly doctor          # workspace health check
vibe-assembly profile         # hardware specs and adaptive storage mode
vibe-assembly tui             # terminal dashboard with selectors (alias: top)

# Execute one task in a chosen mode and language lens
vibe-assembly prompt "Add a health endpoint and its test" \
  --mode code --lang rust

# One-shot read-only answer
vibe-assembly prompt "How is the session DAG persisted?" \
  --mode chat --lang rust --json

# Interactive read-only architect session
vibe-assembly chat --lang rust

# Full live loop with router auto-combo
vibe-assembly run --live --auto-combo --tournament

# Deterministic offline smoke run (never calls the router)
vibe-assembly run --offline-mock

# Native desktop Zed-style workbench
cargo run -p va-desktop -- --root . --mode web --lang ts
```

`init` and `run --auto-combo` auto-seed `.ai/model_combo.txt` when it is absent.
The file is gitignored because it is machine-local configuration.

### Desktop workbench

`va-desktop` is a native GPUI 0.2.2 workbench, not a card dashboard. Its six
zones are:

- a 32px titlebar with workspace breadcrumbs, symbol, block hash, router state,
  dock toggles, command-palette button, and native window controls;
- a 36px activity strip plus a 260px project / outline / session dock;
- a flexible editor with tabs, an 8-character block-hash gutter, Code and Diff
  views, and an inline patch preview;
- a 180px bottom drawer for pinned compiler Problems, Verify Output, and the
  Harness² c1..c8 overlays;
- a 390px assistant panel with collapsible Observe / Think / Patch / Verify /
  Judge rows and an interactive composer;
- a 24px status bar for mode, language lens, `.ai` lock, branch, DAG spend and
  latency, and the context watermark.

The composer accepts typed text, Space, Backspace, Enter to submit, and
Shift+Enter for a newline. Mode, language, and model pills cycle real state;
the judge badge toggles independently. Chat / Architect is enforced read-only:
it produces an answer but never stages a patch. Keybindings: `Ctrl+P` command
palette, `Ctrl+B` left dock, `Ctrl+J` bottom drawer, `Ctrl+Shift+A` assistant,
`Ctrl+W` close tab, `Ctrl+Q` quit.

The workbench is backed by real workspace data: the file tree, RepoMap symbols,
64-bit FNV-1a `segment_blocks` hashes, the immutable session DAG,
`ExperienceStore` lessons, pinned diagnostics, and the eight harness surfaces.
Shared presentation primitives live in `crates/va-desktop/src/ui_kit`; the
supplied brand mark is embedded as a GPUI asset, and the Windows executable
carries the matching multi-size application icon.
Inline diffs come from `git diff` or `git show HEAD` and fall back to a session
patch summary. The desktop surface never fabricates a diff and never writes
files directly; applying patches remains the live CLI's job.

## Command reference

| Command | Purpose |
|---|---|
| `status` | Show the ticket queue and resume point (default command) |
| `models` | Resolve model allocations for the six roles |
| `doctor` | Run a workspace health check |
| `init <name>` | Create a workspace, optionally with `--template <name>` |
| `reset` | Reset volatile `.ai/` state to pristine template defaults |
| `up` | Ensure Vibe Router and workspace readiness with a token handshake |
| `repomap` | Print the token-budgeted, hash-anchored repository symbol map |
| `diagnose` | Run compiler checks and pinpoint errors to block hashes |
| `run` | Run the harness execution loop |
| `prompt <text>` | Run one task in a chosen mode and print the result |
| `chat` | Start an interactive read-only architect chat |
| `tui` / `top` | Launch the terminal dashboard with mode, language, and model selectors |
| `profile` | Display detected hardware capabilities and storage strategy |
| `session` | Inspect or mutate the transactional session DAG |
| `mcp` | Start the Model Context Protocol server over stdio |
| `patch` | Apply a hash-anchored patch by block hash |
| `blocks` | List block hashes in a file |

`--root <path>` is a global option. Commands that support machine-readable
output list `--json` in their `--help` (for example `up`, `repomap`, `diagnose`,
`prompt`, `profile`, and the `session` subcommands).

## Language lens

`--lang auto|rust|c|python|ts|go` selects a language lens. `auto` resolves from
manifests (`Cargo.toml`, `go.mod`, `pyproject.toml`, `package.json`,
`CMakeLists.txt`) and a bounded extension census. Each lens supplies a
verification command and judge guardrails:

| Lens | Verification example |
|---|---|
| Rust | `cargo check --all-targets` |
| C / C++ | `cmake --build build`, `make`, or `cc -fsyntax-only *.c` |
| Python | `python -m compileall -q .` |
| TS / JS | `npx tsc --noEmit` or `node --check .` |
| Go | `go build ./...` |

## The `.ai/` contract

`.ai/active_task.md` holds the queue and exactly one Active Ticket pointer, the
sole execution authority ([ADR-007](.ai/decisions.md)). Each ticket carries an
allowlist, verification-gated steps, and append-only `relay_notes`.

A step counts as verified only when a single note references that step and
carries success evidence. The resume point is the first unverified step, so an
interrupted session resumes exactly where it stopped. Durable recovery lessons
live in `.ai/EXPERIENCE.md`; the trajectory DAG lives in `.ai/session.jsonl`.

## The execution harness

`va-harness` makes agent edits safe to trust.

| Module | Role |
|---|---|
| `session` | Append-only transactional trajectory DAG with per-node hashes, fork, rollback, replay, JSONL persistence, auto-compaction at 75%, think-trace eviction, observation slicing, and a pinned working-set tail |
| `patcher` | Edits address per-block hashes, never line numbers. Whole-file drift assertion, 1-hop recovery, and hard rejection of ambiguous or repeated anchors |
| `repomap` | Token-budgeted workspace symbol scanner binding functions, structs, traits, and classes to deterministic block hashes |
| `diagnostics` | Dual-format compiler pinpointer (`cargo --message-format=json` and text) mapping error spans to block hashes and snippets |
| `review` | Git checkpoint per phase, isolating each step's exact delta |
| `judge` | Deterministic gate: slop comments, placeholders, unsafe primitives in production, import-only churn, excessive churn, trailing whitespace, plus language and mode guardrails. One targeted retry, then abort |
| `sandbox` | Confined command execution with wall-clock timeouts, directory confinement, and bounded output |
| `tournament` | Multi-branch speculative runner scoring candidates by warnings and line churn, merging the winner into the DAG |
| `experience` | Harness^2 c1..c8 surfaces with strict reset-to-base, contrastive recovery, and durable `.ai/EXPERIENCE.md` lessons |
| `live_planner` | Real `POST /v1/chat/completions` Boss -> Code -> Verify -> Debug -> Judge loop with Vibe-Link headers and telemetry |
| `engine` | Orchestration with auto-compaction, read-only enforcement, stagnation detection, and judge-verified hashes |

Useful harness commands:

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
vibe-assembly patch src/main.rs --target 159A "pub fn beta(y: i32) -> i32 { y * 42 }"

# Or pipe a multi-line replacement on stdin
vibe-assembly patch src/main.rs --target 159A < replacement.txt

# Session DAG operations
vibe-assembly session status --json
vibe-assembly session rollback --to-verified
vibe-assembly session compact --from 2 --to 4 --summary "Collapsed failed attempts"

# MCP server over stdio
vibe-assembly mcp
```

## Router integration

`va-core::router_client` performs a real OpenAI-compatible
`POST /v1/chat/completions` over raw HTTP/1.1. It does not implement TLS; point
it at a local router or a TLS-terminating proxy.

Requests carry `X-Vibe-Role`, `X-Vibe-Node-Id`, `X-Vibe-Profile`, and
`X-Vibe-Retry`, plus optional `Authorization: Bearer` or `X-API-Key`. Cost and
latency are read from `X-Vibe-Router-Cost-Micros` / `X-VR-Cost-Micros` and the
matching latency headers, then recorded in the session DAG.

Common environment variables:

| Variable | Purpose |
|---|---|
| `VIBE_ROUTER_HOST` / `VIBE_ROUTER_PORT` | Router address (default `127.0.0.1:20128`) |
| `VIBE_ROUTER_PATH` | Assignment endpoint path (default `/api/assignment`) |
| `VIBE_ROUTER_CHAT_PATH` | Chat completions path (default `/v1/chat/completions`) |
| `VIBE_ROUTER_TOKEN` / `VIBE_ROUTER_API_KEY` | Router credentials; a workspace `.vibe_router_token` file also works |
| `VIBE_MODE` / `VIBE_LANG` / `VIBE_MODEL` | Initial desktop selectors (CLI and TUI use flags) |
| `VIBE_ASSEMBLY_PROFILE` | Force `auto`, `vps`, `portable`, or `workstation` |

Model resolution order: pinned `--model`, per-mode override
(`.ai/models/<mode>.txt` or `.ai/model_<mode>.txt`), live router assignment,
`.ai/model_combo.txt`, then a conservative default. The judge is always
deterministic and independent; `--judge-model` is reported for audit and display.

## Context safety

- Auto-compaction triggers at 75% of the active model's context window.
- Older verified steps and failed retries collapse into surface summaries while
  the DAG remains intact for `replay()` and `rollback_to_last_verified()`.
- Reasoning traces are evicted from prompts but retained in DAG nodes.
- Large observations render head 30 + `[... elided N lines ...]` + tail 30.
- In the live loop, the prompt ends with fresh block hashes and line ranges for
  modified files.
- Two identical failures trigger `StagnationDetected`; policy can pivot or
  escalate to `SpeculativeTournament`.

## Safety model

- Chat mode is a hard read-only boundary enforced by the engine, not a prompt hint.
- The active ticket, including its `files_allowed` list, is injected into the
  planner context; the sandbox confines command execution to the workspace root.
- Ambiguous or repeated block anchors are rejected rather than guessed.
- A normal live run never fabricates edits. Deterministic synthesis is available
  only through tests or the explicit `--offline-mock` flag.
- Live-run cost and latency telemetry is recorded in the local session DAG.

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

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```

Current workspace: **200 tests passing**, clippy clean, both release binaries
built. The live HTTP planner is tested against a local mock OpenAI-compatible
TCP server; unit tests cover compaction, think-trace eviction, observation
slicing, repeated anchors, read-only enforcement, stagnation escalation,
language detection, `EXPERIENCE.md` round-tripping, and the desktop workbench
(real block hashes, mode/language/model cycling, read-only Chat, dock toggles,
and composer submission).

## Troubleshooting

- **Router offline:** run `vibe-assembly up`, then check the `VIBE_ROUTER_*`
  variables. Offline work is available with `--offline-mock`.
- **Workspace not found:** run from a directory containing `.ai/`, pass
  `--root <path>`, or create a workspace with `vibe-assembly init`.
- **GPUI stack overflow on Windows:** build from Git Bash or WSL with
  `ulimit -s unlimited`; `RUST_MIN_STACK` is not sufficient.
- **Unexpected patch rejection:** the block hash or whole-file hash changed.
  Re-run `vibe-assembly blocks <file>` and retry with the fresh hash.

## License

MIT
