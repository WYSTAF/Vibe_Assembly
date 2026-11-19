# Current Operational State

- **Active Milestone:** v2.5.1 deep-dive hardening of the Vibe Local ecosystem wiring
- **Release:** v2.5.1 - bug-fix wave on top of v2.5.0: assignment requests default
  to the public `/v1/assignment` with a 401/404 fallback ladder (the `/api`
  alias is admin-guarded on the current router); LAN discovery parses Vibe
  Local's real `http_port` announcement shape and enriches workers via
  `/v1/models`; the desktop re-probes router liveness at submit time; `up`
  honors env overrides and spawns loopback-only with the right port; the
  engine confines patch paths (drive prefixes/ADS), writes atomically, and
  reports step-cap exhaustion as failure; the sandbox drains pipes
  concurrently, keeps Windows resolution vars, and allows 300s verifications;
  MCP tools are workspace-confined, reject ambiguous short hashes, and persist
  compactions.
- **Current Assignment:** Complete - v2.5.1 shipped.
- **Planning Status:** Complete.
- **Implementation Status:**
  - `va-core`: parser, single-writer task lock, allocator, model knowledge,
    real OpenAI-compatible chat-completions client with Vibe-Link headers,
    telemetry (cost, latency, cache, tok/s, backend), per-model
    `reasoning_effort` coercion, `local_node` (Vibe Local probe, LAN UDP
    discovery, token auth headers), hardware-aware runtime, bootstrap, and
    `modes.rs` (four modes, six language lenses, `ReasoningEffort` ladder,
    accents, contracts, guardrails, verification commands, manifest/extension
    detection).
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
    `harness inspect` shows the c1..c8 surfaces and lessons. `up` probes and
    spawns Vibe Router and Vibe Local and reports LAN Pocket Workers (human
    and `--json`); `doctor` checks the local inference node; `models` and
    auto-combo merge zero-cost local models when the router is offline.
  - `va-desktop`: native GPUI 0.2.2 Liquid Glass default surface (persisted
    night/day palettes, hover nav with pinned Session/Changes/Editor/History
    drawer, fit-to-height sections, nav-clear card text, real command search,
    circular context ring, attachment cards, merged router/Vibe Local/LAN
    model catalogue with backend badges and $0/local blurbs, direct local
    chat routing when the router is offline, and tok/s + backend + $0.00
    telemetry in the Session drawer and status strip) plus the retokenized
    Paper & Ink
    workbench (ADR-019 layout) with real
    RepoMap/session-DAG/diagnostics/harness data, an 8-character block-hash
    editor and git diff view, an interactive assistant composer, a floating
    command palette, a real context-window meter, a shared native `ui_kit`,
    embedded brand/Windows icon assets, and preview-only design samples.
- **Verification Status:** 283 tests passing, 0 failed; clippy clean with
  `-D warnings`; formatting clean; release build produced both binaries.
- **Resume Anchor:** n/a - wave verified and complete.
- **Blockers:** None.
