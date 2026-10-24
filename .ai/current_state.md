# Current Operational State

- **Active Milestone:** v2.0.0 Production Hardening and Feature Completion (ADR-018)
- **Current Assignment:** Complete - real live planner, execution modes,
  language lens, context compaction, safety guards, durable experience, and
  interactive CLI/TUI/desktop selectors.
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
  - `va-desktop`: native GPUI Mission Control with clickable mode and language
    selectors, a model picker, dynamic mode accents, and a real context-window
    meter computed from `.ai/session.jsonl`.
- **Verification Status:** 188 tests passing, 0 failed; clippy clean with
  `-D warnings`; formatting clean; release build produced both binaries.
- **Resume Anchor:** n/a - wave verified and complete.
- **Blockers:** None.
