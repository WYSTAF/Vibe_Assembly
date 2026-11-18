# Verification Record & Proven Baselines

**Current feature proven:** v2.5.0 Vibe Local ecosystem integration - local
node probe, LAN UDP discovery, zero-cost telemetry through the live planner,
CLI readiness integration, and Liquid Glass local-model catalog/routing/UI

**Test suites and checks executed:**

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo test --workspace` - 271 passed, 0 failed
- [x] `cargo build --release` - `vibe-assembly` and `vibe-assembly-desktop`
- [x] Desktop smoke test - a bare launch of the release binary installed at
      `target/release` opens the Liquid Glass workbench in the persisted theme;
      `--workbench` opens the Paper & Ink workspace workbench
- [x] Liquid Glass default surface - chat rail, model pill, glass ask bar,
      auto-hide nav with hover-opened Session/Changes/Editor/History panels,
      circular context ring, command sheet, and compact 1120x720 layout verified
      with pixel captures in both themes
- [x] Command sheet (Ctrl+K and titlebar click) - live input bridge, grouped
      filtering and ranking, arrow-key navigation, Enter to run, Esc to clear
      then close, click-outside dismissal, clear button, and no-results state
- [x] Theme preference persists to `%APPDATA%\Vibe Assembly\preferences.conf`
      and is restored on the next launch
- [x] Per-model thinking preference (`thinking.<model_id>=<effort>`) round-trips
      in `preferences.conf` without disturbing the theme or other models;
      malformed values fail closed to Auto
- [x] `ReasoningEffort` parsing, labels, serde round-trip, and per-model
      coercion (XHigh -> High, omission on fast-tier models and tool conflicts)
- [x] `reasoning_effort` JSON payload emission verified against the local mock
      HTTP server, including omission for Auto/unsupported models and coercion
      on every Boss/Code call through `LivePlannerConfig`
- [x] Model pill displays the coerced effective level (`Model · Deep (coerced)`)
      instead of claiming an unsupported level
- [x] Router endpoint consistency: `DEFAULT_ROUTER_PATH=/api/assignment` used by
      `models`, `doctor`, and `up`; MCP `initialize` version equals
      `CARGO_PKG_VERSION`
- [x] `probe_vibe_local` against a local mock TCP server (`GET /v1/models`):
      models, backend/device headers, endpoint URL; HTTP errors and garbage
      bodies fail closed to `None`
- [x] `discover_lan_workers` against a local mock UDP socket on an ephemeral
      port: `VIBE_DISCOVER_V1` probe answered with a JSON worker announcement;
      no listeners returns an empty list without error
- [x] Worker announcement parsing: JSON and key-value forms, sender-IP host
      fallback, backend normalization (`vulkan|cuda|cpu`), fail-closed on
      missing host/port and unknown backends
- [x] `X-Vibe-Local-Tok-Per-Sec`, `X-Vibe-Local-Backend`, and
      `X-Vibe-Router-Cost-Micros: 0` ingestion verified end-to-end through
      `LiveRouterPlanner` against the local mock HTTP server
      (`last_tok_per_sec`, `last_backend`, `last_cost_micros`)
- [x] CLI `doctor` reports the Vibe Local inference node (live/offline) and
      reachable LAN Pocket Workers; `up` probes/spawns both nodes and reports
      workers in human and `--json` output; `models`/auto-combo merge
      zero-cost local models when the router is offline
- [x] Desktop catalogue merging: router/local/LAN deduplicated by id (router
      wins collisions), local models tagged with Vulkan/CUDA/CPU/LAN badges
      and `$0 / local` blurbs; local-endpoint routing config verified
      (direct host/port, `/v1/chat/completions`, offline-mock disabled;
      malformed endpoints fail closed to the router)
- [x] Drawer sections fill the panel to the bottom edge and card text, stats,
      and row links stay clear of the floating nav capsule at 1440x900 and
      1120x720
- [x] Windows icon embedded at 16, 20, 24, 32, 40, 48, 64, 96, 128, and
      256 px; rounded transparent corners verified
- [x] Live HTTP integration test against a local mock OpenAI-compatible server
- [x] Compaction, think eviction, observation slicing, repeated-anchor,
      read-only, stagnation, language-lens, and EXPERIENCE.md tests
- [x] Theme preference round-trip, search filtering/ranking, and drawer reveal
      regression tests

**Runtime environment:**

- Rust stable MSVC toolchain with Visual Studio Build Tools (C++ workload)
- Windows 10/11 host; Linux builds use the same Cargo workspace
- State: `.ai/*.md` and `.ai/session.jsonl`
- Router: local OpenAI-compatible HTTP gateway

**Last date proven working:** 2026-11-18
