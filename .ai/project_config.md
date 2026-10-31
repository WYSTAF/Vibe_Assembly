# Static Project Configuration

## Stack

- **Project:** Vibe Assembly
- **Version:** 2.2.0
- **Primary language:** Rust (edition 2021, MSVC toolchain on Windows)
- **Build system:** Cargo workspace with four member crates
- **Binaries:** `vibe-assembly` (CLI/TUI/MCP) and `vibe-assembly-desktop` (GPUI)
- **Crates:**
  - `va-core` - parser, allocator, knowledge, router client, lock, prompts, bootstrap, runtime, execution modes
  - `va-harness` - session DAG, patcher, repomap, diagnostics, review, judge, sandbox, tournament, experience, engine, live planner
  - `va-cli` - terminal operations room, interactive chat, TUI dashboard, MCP server
  - `va-desktop` - native GPU dashboard (GPUI 0.2.2)
- **UI runtime:** native GPUI and ANSI terminal only; no webview and no browser process
- **State store:** `.ai/*.md` plus the append-only `.ai/session.jsonl` trajectory DAG
- **Router:** OpenAI-compatible HTTP gateway (`POST /v1/chat/completions`)

## Environment

- **Default router endpoint:** `127.0.0.1:20128` (override with `VIBE_ROUTER_HOST` / `VIBE_ROUTER_PORT`)
- **Auth:** `VIBE_ROUTER_TOKEN`, `VIBE_ROUTER_API_KEY`, or a local `.vibe_router_token`
- **Workspace root:** discovered upward from the current directory; override with `--root`
- **Source root:** `crates/`
- **Model fallback:** `.ai/model_combo.txt` (auto-seeded on `init` and `run --auto-combo`)
- **Per-mode overrides:** `.ai/models/<mode>.txt` or `.ai/model_<mode>.txt`
- **Durable lessons:** `.ai/EXPERIENCE.md`

## Verification Gates

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```
