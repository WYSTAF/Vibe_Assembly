# Current Operational State

- **Active Milestone:** v2.0.0 Production Hardening and Feature Completion (ADR-018)
- **Patch Release:** v2.0.1 - rounded yellow brand icon for the Windows executable
  and the in-app titlebar/empty-state logo.
- **Current Assignment:** Complete - real live planner, execution modes,
  language lens, context compaction, safety guards, durable experience,
  interactive CLI/TUI selectors, and the modular Zed-style desktop workbench.
- **Planning Status:** Complete.
- **Implementation Status:**
  - `va-core`: parser, single-writer task lock, allocator, model knowledge,
    real OpenAI-compatible chat-completions client with Vibe-Link headers and
    telemetry, hardware-aware runtime, bootstrap, and `modes.rs` (four modes,
    six language lenses, accents, contracts, guardrails, verification commands,
    manifest/extension detection).
  - `va-harness`: immutable session DAG with JSONL persistence, automatic
    compaction at 75% of the model window, think-trace eviction, head/tail
    observation slicing, pinned working-set tail, hash-anchored patcher with
    repeated-anchor rejection, RepoMap, diagnostic pinpointer, review, judge
    with language/mode guardrails, sandbox, tournament, Harness^2 surfaces,
    contrastive recovery, `.ai/EXPERIENCE.md`, the execution engine with
    read-only/stagnation enforcement, and the production live planner
    (Boss -> Code -> Verify -> Debug -> Judge).
  - `va-cli`: `vibe-assembly` with `status`, `models`, `doctor`, `init`,
    `reset`, `up`, `repomap`, `diagnose`, `run`, `prompt`, `chat`, `tui`/`top`,
    `profile`, `session`, `mcp`, `patch`, `blocks`, `fork`, `rollback`, and
    `compact`. `init` and `run --auto-combo` auto-seed `.ai/model_combo.txt`.
  - `va-desktop`: native GPUI 0.2.2 Zed One Dark workbench (ADR-019) with a
    six-zone layout, real RepoMap/session-DAG/diagnostics/harness data, an
    8-character block-hash editor and git diff view, an interactive assistant
    composer, a floating command palette, a real context-window meter, a shared
    native `ui_kit`, and embedded brand/Windows icon assets.
- **Verification Status:** 200 tests passing, 0 failed; clippy clean with
  `-D warnings`; formatting clean; release build produced both binaries.
- **Resume Anchor:** n/a - wave verified and complete.
- **Blockers:** None.
