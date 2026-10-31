# Current Operational State

- **Active Milestone:** v2.4.0 Liquid Glass search, persisted themes, and drawer fit
- **Release:** v2.4.0 - the Liquid Glass workbench gains instant command search
  from the titlebar or `Ctrl+K`, persists the night/day choice across launches,
  and fills every drawer section to the window height with card text clear of
  the floating nav capsule; the Paper & Ink editor workbench opens with
  `--workbench` or a repo path, and the remaining design samples stay behind
  `--design-preview*` flags.
- **Current Assignment:** Complete - v2.4.0 shipped.
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
  - `va-desktop`: native GPUI 0.2.2 Liquid Glass default surface (persisted
    night/day palettes, hover nav with pinned Session/Changes/Editor/History
    drawer, fit-to-height sections, nav-clear card text, real command search,
    circular context ring, attachment cards) plus the retokenized Paper & Ink
    workbench (ADR-019 layout) with real
    RepoMap/session-DAG/diagnostics/harness data, an 8-character block-hash
    editor and git diff view, an interactive assistant composer, a floating
    command palette, a real context-window meter, a shared native `ui_kit`,
    embedded brand/Windows icon assets, and preview-only design samples.
- **Verification Status:** 204 tests passing, 0 failed; clippy clean with
  `-D warnings`; formatting clean; release build produced both binaries.
- **Resume Anchor:** n/a - wave verified and complete.
- **Blockers:** None.
