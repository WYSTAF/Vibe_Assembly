# Current Operational State

- **Active Milestone:** v2.1.0 Paper & Ink workbench
- **Release:** v2.1.0 - warm Paper & Ink redesign of the native desktop
  workbench, plus design sample surfaces behind `--design-preview*` flags.
- **Current Assignment:** Complete - Paper & Ink tokens and layout applied to
  the shipped workbench; conversation-first canvas, restyled `ui_kit`, Lucide
  icon set, and the Liquid Glass sample.
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
  - `va-desktop`: native GPUI 0.2.2 Paper & Ink workbench (ADR-019 layout,
    retokenized) with a conversation-first canvas, real
    RepoMap/session-DAG/diagnostics/harness data, an 8-character block-hash
    editor and git diff view, an interactive assistant composer, a floating
    command palette, a real context-window meter, a shared native `ui_kit`,
    embedded brand/Windows icon assets, and preview-only design samples.
- **Verification Status:** 200 tests passing, 0 failed; clippy clean with
  `-D warnings`; formatting clean; release build produced both binaries.
- **Resume Anchor:** n/a - wave verified and complete.
- **Blockers:** None.
