# Project Progress

## Active Wave

- **v2.0.0 Production Hardening and Feature Completion (ADR-018)**
- Status: implementation complete; final verification gates green.
- **v2.0.1 (2026-10-27):** rounded yellow brand icon adopted for the Windows
  executable and the in-app logo; desktop binary rebuilt and release refreshed.
- Scope: real HTTP live planner, four execution modes with dynamic accents,
  language lens, auto-compaction at 75%, think-trace eviction, pinned working
  set, repeated-anchor rejection, spin-loop breaker, durable EXPERIENCE.md,
  per-mode model overrides, CLI `prompt`/`chat`, TUI and desktop selectors.

## Scope Guard

- Keep the pure-Rust four-crate workspace and the binary names
  `vibe-assembly` and `vibe-assembly-desktop`.
- No new runtime services, no browser process, no non-Rust package manager.
- Preserve the `.ai/` ticket contract and the session DAG semantics.

## Historical Waves

- **ADR-014 (2026-10-08):** migrated to a standalone pure-Rust Cargo workspace
  with `va-core`, `va-cli`, and `va-desktop`.
- **ADR-015 (2026-10-10):** added `va-harness` with the session DAG,
  hash-anchored patcher, review, judge, and engine.
- **ADR-016 (2026-10-18):** adaptive runtime, context compaction, Vibe-Link
  telemetry, universal gateway fallback, TUI, and MCP server.
- **ADR-017 (2026-10-19):** hash-anchored RepoMap, diagnostic pinpointer, live
  router planner, and speculative tournament.
- **ADR-018 (2026-10-23):** production hardening wave: real chat-completions
  planner, execution modes and language lens, automatic context compaction,
  reasoning-trace eviction, ambiguity/stagnation guards, EXPERIENCE.md,
  `prompt`/`chat`, and interactive desktop/TUI selectors.

## Verification

- `cargo fmt --all -- --check` - clean
- `cargo clippy --workspace --all-targets -- -D warnings` - clean
- `cargo test --workspace` - 188 passed, 0 failed
- `cargo build --release` - both binaries produced
