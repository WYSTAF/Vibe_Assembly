# Current Operational State

- **Active Milestone:** v2.4.1 thinking-effort wiring and router/CLI consistency
- **Release:** v2.4.1 - the Liquid Glass thinking ladder (Auto/Fast/Balanced/
  Deep/Max) now drives real `reasoning_effort` on router chat completions,
  persists per model in `%APPDATA%\Vibe Assembly\preferences.conf`, and shows
  the coerced effective level on the model pill; `run`/`prompt` gain
  `--effort auto|low|medium|high|xhigh`; the legacy router client, `models`,
  `doctor`, and `up` all agree on `/api/assignment` at 127.0.0.1:20128; the MCP
  server version tracks the workspace version.
- **Current Assignment:** Complete - v2.4.1 shipped.
- **Planning Status:** Complete.
- **Implementation Status:**
  - `va-core`: parser, single-writer task lock, allocator, model knowledge,
    real OpenAI-compatible chat-completions client with Vibe-Link headers,
    telemetry, and per-model `reasoning_effort` coercion, hardware-aware
    runtime, bootstrap, and `modes.rs` (four modes, six language lenses,
    `ReasoningEffort` ladder, accents, contracts, guardrails, verification
    commands, manifest/extension detection).
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
    `run` and `prompt` accept `--effort auto|low|medium|high|xhigh` and
    `harness inspect` shows the c1..c8 surfaces and lessons.
  - `va-desktop`: native GPUI 0.2.2 Liquid Glass default surface (persisted
    night/day palettes, hover nav with pinned Session/Changes/Editor/History
    drawer, fit-to-height sections, nav-clear card text, real command search,
    circular context ring, attachment cards) plus the retokenized Paper & Ink
    workbench (ADR-019 layout) with real
    RepoMap/session-DAG/diagnostics/harness data, an 8-character block-hash
    editor and git diff view, an interactive assistant composer, a floating
    command palette, a real context-window meter, a shared native `ui_kit`,
    embedded brand/Windows icon assets, and preview-only design samples.
- **Verification Status:** 254 tests passing, 0 failed; clippy clean with
  `-D warnings`; formatting clean; release build produced both binaries.
- **Resume Anchor:** n/a - wave verified and complete.
- **Blockers:** None.
