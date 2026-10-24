# Architectural Decision Log

## [ADR-005] Four-Mode Product Foundation
- **Date:** 2026-08-04
- **Status:** Implemented
- **Decision:** Use native `1-boss`, `2-chat`, `3-code`, and `4-debug` modes with Vibe Assembly Studio branding and the existing selector/onboarding architecture.
- **Outcome:** Phase 1 is complete; no additional mode architecture is authorized.

## [ADR-006] Phase 2 Core Polish Boundaries
- **Date:** 2026-08-05
- **Status:** Implemented; developer-confirmed
- **Decision:** Stabilize mode permissions, active-ticket allowlists, the existing selector, and first-launch project memory without new dependencies or parallel UI architecture.
- **Outcome:** Phase 2 is complete; Phase 3 may correct only remaining workflow inconsistencies.

## [ADR-007] Explicit Active Ticket as the Resume Anchor
- **Date:** 2026-08-06
- **Status:** Approved; implementation pending
- **Context:** Inferring the first executable Queue row conflates the full backlog with current work and makes state conflicts or interrupted sessions ambiguous.
- **Decision:** Keep the full Queue and ticket bodies in `.ai/active_task.md`, add exactly one explicit Active Ticket pointer, and make it the only Code execution and permission source. Queue, Meta, pointer, Status Snapshot, and relay evidence must remain synchronized; conflicts fail closed.
- **Boundary:** Use existing files, standard APIs, and focused tests. Add no panel, service, dependency, advanced feature, or broad refactor.
- **Consequence:** Resume starts from the pointed ticket's durable step evidence; completion advances the pointer atomically or sets it to `none`.

## [ADR-008] Sanitize ELECTRON_RUN_AS_NODE in Desktop Dev Launcher
- **Date:** 2026-08-25
- **Status:** Implemented; verified
- **Context:** T-401 launch crashed because editors/harnesses running inside Electron leak `ELECTRON_RUN_AS_NODE=1` into spawned shells; electron-vite inherits env, forcing electron.exe to run as plain Node, which breaks named ESM imports from 'electron' (electron-dl via electron-context-menu).
- **Decision:** Route the `dev` script through `scripts/dev.ts`, which strips `ELECTRON_RUN_AS_NODE` and spawns electron-vite with an explicit sanitized env object. Also accept the prior session's offline-fallback fallback-binary edit in `scripts/utils.ts` as a permanent exception to its original ticket allowlist.
- **Boundary:** Launcher-level env hygiene only; no changes to electron-vite, dependency graph, or main-process code.
- **Consequence:** Desktop dev mode is resilient to editor-injected environments; new Electron env hazards can be added to `SANITIZED_KEYS` in one place.

## [ADR-009] Mission Control — First-Party Desktop App
- **Date:** 2026-08-26
- **Status:** Implemented; parser tested; launch verified pending Electron install
- **Context:** The workspace had no product UI of its own; the only desktop tree (app/opencode-dev) is an upstream study fork. Users need at-a-glance awareness of ticket state and the exact paste phrase, especially after rate-limit interruptions.
- **Decision:** Build `app/desktop/` (Mission Control): zero-runtime-dependency Electron dashboard over `.ai/` state. Main process parses active_task/current_state/known_bugs/progress, enforces fail-closed consistency checks mirroring ticket_contract hard rules, watches `.ai/` via fs.watch, pushes state to a sandboxed renderer (contextIsolation, strict CSP). Launcher (`start.js`) reuses the T-401 env sanitizer. Parser is pure CJS shared with node:test unit tests. CLI bootstrap offers setup.
- **Boundary:** Read-only over workspace state; no mode may treat Mission Control as an executor. It suggests paste phrases; it never writes .ai files.
- **Consequence:** New workspaces get their own UI by default; parser changes must keep lib/parser.js dependency-free so tests run on stock Node.

## [ADR-010] Fifth Mode — Hermes, Docs & Release Herald
- **Date:** 2026-08-26
- **Status:** Implemented
- **Context:** The assembly line owned planning (Boss), conversation (Chat), execution (Code), and repair (Debug) — but nothing owned telling the world what was built. Docs drifted from code and release notes were ad hoc.
- **Decision:** Add slug `5-hermes`: restricted-edit mode limited to markdown/docs/plans/README/CHANGELOG via fileRegex, no command group. Evidence-first rules: document only what exists on disk; publishing is always human-approved. Developer-selected role from three candidates (herald / orchestrator / scout-liaison); herald filled the real downstream gap.
- **Boundary:** Documentation only. Never touches production code or .ai/active_task.md.
- **Consequence:** Waves can end with an in-house documentation pass; contract pinned by tests so future edits cannot silently grant Hermes broader rights.

## [ADR-011] Standard Project Templates
- **Date:** 2026-08-26
- **Status:** Implemented; E2E verified
- **Context:** New users face an empty app/ directory; free-tier sessions get burned on scaffolding before any real feature work.
- **Decision:** Ship a curated catalog of dependency-free starter projects under templates/ (planner, todo, website), each with a working app/ and pre-filled .ai context files. Bootstrap applies them via --template <name>; manual import from GitHub remains supported. Catalog is data (templates/catalog.json) — new templates require no CLI changes.
- **Boundary:** Starters stay minimal and framework-free; they are baselines to rebrand, not products.
- **Consequence:** First ticket in a templated workspace is real product work (rebrand/features), not scaffolding; catalog growth is additive and hook-safe.

## [ADR-012] Model Combo & Automatic Mode Allocation
- **Date:** 2026-08-26
- **Status:** Implemented
- **Context:** The workspace targets API-router users with heterogeneous free-tier models. Manual per-mode model picking requires benchmark knowledge most beginners lack.
- **Decision:** Users declare their combo in .ai/model_combo.txt (+ optional per-mode locks). A curated knowledge base (.ai/model_knowledge.json) classifies models into stable capability CLASSES rather than volatile benchmark scores; a two-pass allocator assigns classes to modes honoring primary needs before fallbacks. Surfaced via `va --models`. Knowledge file is data — refreshable without code changes; sources listed inside.
- **Boundary:** Allocator recommends; it never mutates router configs or Roo settings itself. Publishing is human-applied.
- **Consequence:** Beginners get expert-level model routing; allocation logic pinned by behavioral tests including the scarcity rule and lock precedence.

## [ADR-013] Native Mission Control on GPUI
- **Date:** 2026-09-27
- **Status:** Implemented; replaces the Electron dashboard for day-to-day use
- **Context:** The Electron Mission Control UI was dense, low-signal, and in two places simply wrong: `renderBugs` rendered one bug's field labels as if they were six separate bugs, and `decisions_head` was fetched but never rendered. It also never surfaced the resume point — the single most decision-relevant value in the dataset.
- **Decision:** Rebuild the dashboard as a native Rust app on `gpui` 0.2.2 from crates.io. Forking Zed was rejected: its extension API has no custom-UI hook and cannot register a plain command, and a fork would inherit ~250 crates of editor, LSP, collab, and telemetry to render a read-only status view. The parser is ported literally rather than rewritten, so the native app and the `va` CLI can never disagree about workspace state; `tests/parser-parity.test.js` enforces that from the JS side.
- **Boundary:** Dashboard only. The `va` CLI, bootstrap, and model allocator stay JavaScript. The `.ai/` contract and mode architecture are untouched — this app observes them, it does not define them.
- **Consequence:** Render priority is explicit: answer "what do I do next" within five seconds, make contract violations unmissable, and let every other panel earn its space. Two bugs in the port were caught by tests before first run (Verify-command extraction, files_allowed scope). On Windows, cargo needs `ulimit -s unlimited` before building or rustc aborts with STATUS_STACK_BUFFER_OVERRUN; see app/desktop-rust/README.md.

## [ADR-014] Standalone Pure-Rust Workspace
- **Date:** 2026-10-08
- **Status:** Implemented
- **Context:** The workspace had evolved into a JS/Roo/Cline hybrid: the state engine and CLI were JavaScript, while the dashboard was already native Rust. This split meant two parsers to keep in sync, an npm/Node toolchain requirement for a Rust-first product, and duplicated contract logic across languages.
- **Decision:** Graduate to a single Cargo workspace with zero Node.js/npm dependencies. Three crates: `va-core` (state engine: parser, allocator, router, lock, prompts, bootstrap), `va-cli` (binary `vibe-assembly`), `va-desktop` (binary `vibe-assembly-desktop`). The JS parser was replaced by `va-core/src/parser.rs`; the `.ai/` contract was copied verbatim and remains authoritative. Legacy JS tooling is preserved read-only at `Q:\Vibe Coding\_Vibe_Assembly` for reference.
- **Naming:** The abbreviation `va` was retired from all user-facing surfaces at the user's request. The command is the full name `vibe-assembly`. Crate directory names retain the `va-` prefix (internal, not user-facing).
- **Boundary:** The `.ai/` contract and the five-mode architecture are unchanged. This ADR moves implementation language, not product semantics. Ticket ids were generalised from the hardcoded `T-NNN` prefix to `PREFIX-NNN`, so the new `VA-RUST-001` namespace parses without a contract rewrite.
- **Consequence:** One parser, one language, one toolchain. Offline builds work from a seeded `Cargo.lock`. On Windows, builds require `ulimit -s unlimited` before invoking cargo (see README) — `RUST_MIN_STACK` does not fix the codegen stack overflow.

## [ADR-015] Execution Harness (va-harness)
- **Date:** 2026-10-10
- **Status:** Implemented
- **Context:** Vibe Assembly had a contract and a dashboard but no execution engine: no trajectory record, no deterministic way to apply an edit, no gate between 'the agent produced a diff' and 'the diff is committed'. Agent edits were line-addressed, so any drift silently corrupted the target file.
- **Decision:** Add `va-harness` with five modules. `session` is an append-only transactional trajectory DAG: nodes carry one or more parents (true merges), per-node content and ancestry state hashes, and are only visible after commit. `fork`, `rollback`, `rollback_to_last_verified`, and `replay` are distinct operations; replay reuses stored results and never re-executes. `patcher` addresses edits by per-block hash rather than line number, recovers from a uniform line shift by re-identifying the block's opening signature, and hard-rejects an ambiguous hash instead of guessing. `review` checkpoints with git and isolates each step's delta. `judge` is a deterministic gate (slop comments, placeholders, unsafe primitives outside tests, import-only churn, excessive churn, trailing whitespace) granting exactly one targeted retry. `engine` runs Observe → Think → Patch → Verify → Judge and is model-agnostic through a `Planner` trait.
- **Relationship to upstream:** The brief cited `deepseek-harness` for a trajectory DAG and `oh-my-pi` for hash-anchored patching. Measured against the actual sources: `deepseek-harness` is TypeScript with a flat, seq-addressed append-only event log and no DAG, state hashing, rollback, or re-execution; `oh-my-pi` anchors a 4-hex XXH32 tag to the whole file and addresses blocks by line plus tree-sitter AST. The behaviors the wave required were therefore reimplemented rather than ported, and each module carries a design note stating exactly what was and was not taken from upstream.
- **Substitutions:** FNV-1a (64-bit) replaces XXH32 (16-bit) because `xxhash-rust` is absent from the local cargo cache and the network is heavily throttled; the spec permits FNV-1a. Indentation-scoped blocks replace tree-sitter because ~55 grammar crates cannot be fetched offline. Both choices keep the dependency graph offline-buildable and the collisions strictly detectable.
- **Consequence:** A patch to a stale file either applies through a verified single-hop rebase or fails with a specific diagnostic. Closing braces belong to the scope they terminate, so a function is one block; this was a real defect caught by end-to-end testing, not by unit tests.

## [ADR-016] Adaptive Runtime, Context Compaction, Vibe-Link Synergy & MCP Server
- **Date:** 2026-10-18
- **Status:** Implemented; 150 tests green; 0 clippy warnings
- **Context:** Vibe Assembly required operational adaptation to execute uniformly on both ultra-cheap Linux VPS instances (256MB RAM, low core count) and portable USB flash drives plugged into high-spec workstations, while preventing USB flash wear and memory bloat. Additionally, agent token exhaustion necessitated context compaction without DAG truncation, universal gateway fallback was needed for third-party OpenAI-compatible proxies (9router, LiteLLM, Ollama), and external agents (Hermes Agent, Claude Code) required native stdio orchestration.
- **Decision:**
  1. **Hardware-Aware Adaptive Runtime (`va-core::runtime`)**: Added system profiler detecting total/available RAM and CPU cores. Mode 1 (`Disk-First VPS`): stream session DAG and states directly to disk with capped thread pool, keeping resident memory < 35MB. Mode 2 (`RAM-First Portable Zero-Wear`): hydrate workspace and DAG into host RAM, execute intermediate steps in RAM to eliminate flash write amplification, and flush via crash-safe atomic write-to-temp + `fsync` + rename. Added `{{PORTABLE_ROOT}}` path normalization for drive-letter independence and host temp directory redirection. Added host proxy and local VPN tunnel auto-discovery.
  2. **ContextSurface Compaction (`va-harness::session`)**: Built a `ContextSurface` view layer over active DAG branches supporting `compact_range(start, end, summary)`. Collapses intermediate steps/failed retries into a single summary for prompt generation while preserving the immutable underlying multi-parent DAG, state hashes, and deterministic `replay()`.
  3. **Durable `onYield` & Sandboxed Execution (`va-harness::engine`, `va-harness::sandbox`)**: When an agent yields completion (`DONE`), automatically intercept and execute the compiler check, test suite, incremental git diff, and judge anti-slop review. Wrap verification in a sandboxed runner with configurable timeouts, working-directory confinement, and bounded stdout/stderr capture. Record a deterministic `judge_verified_hash` on clean passes.
  4. **Universal Gateway Fallback & Vibe-Link Synergy (`va-core::router_client`)**: Automatically query `GET /v1/models` on `404 Not Found` (supporting 9router / standard gateways) and compute optimal 6-role allocations via heuristic auto-combo. Added zero-401 auto-handshake checking local `.vibe_router_token` files. Attached outbound headers (`X-Vibe-Role`, `X-Vibe-Node-Id`, `X-Vibe-Profile`, `X-Vibe-Retry`) and ingested micro-dollar telemetry (`X-VR-Cost-Micros`, `X-VR-Latency-Ms`).
  5. **Terminal UI, Hermes Automation & Built-in MCP Server (`va-cli`)**: Added `vibe-assembly tui` (alias `top`), `vibe-assembly profile [--json]`, `vibe-assembly session` (status, fork, rollback, compact), `vibe-assembly run --stream-jsonl`, and a zero-bloat JSON-RPC 2.0 MCP server (`vibe-assembly mcp`) over stdio exposing blocks, patching, judging, session DAG, and auto-combo tools.
- **Brand Safety & Anti-Framing Guard:** Prohibited mandatory promotional commit trailers during unverified local executions with user tokens. Cryptographically anchored verification evidence directly into local DAG nodes via `judge_verified_hash`.
- **Consequence:** Full headless CLI and MCP control on cheap VPS servers, zero wear on portable flash drives, universal proxy compatibility, and zero-compromise agent auditability across 150 passing tests.

## [ADR-017] Hash-Anchored RepoMap, Diagnostic Pinpointer, Live Planner & Speculative Tournament
- **Date:** 2026-10-19
- **Status:** Implemented; 164 tests green; 0 clippy warnings
- **Context:** Coding models needed fast, token-budgeted symbol discovery to immediately target deterministic block hashes without wasting context reading entire source trees. Furthermore, compiler errors required pinpointing to exact block hashes instead of dumping raw terminal noise, `LiveRouterPlanner` was needed to drive the full loop through Vibe Router, and complex ambiguous tasks required speculative multi-branch tournaments to evaluate competing candidate patches competitively.
- **Decision:**
  1. **Hash-Anchored Repository Map (`va-harness::repomap`)**: Built a fast workspace scanner extracting structural symbols (functions, structs, enums, traits, classes, interfaces) across Rust, TypeScript, Python, and Go, binding each directly to its deterministic 64-bit FNV-1a block hash from `patcher`. Implemented `render_within_budget(max_tokens, priority_files)` providing compact, token-capped symbol outlines.
  2. **Compiler Diagnostic Pinpointer (`va-harness::diagnostics`)**: Implemented dual-format diagnostic parser (`cargo --message-format=json` and terminal text), mapping compiler line spans to exact code block hashes and generating focused failure snippets. Wired directly into the durable `onYield` verification loop in `engine`.
  3. **Live Router-Backed Planner (`va-harness::live_planner`)**: Implemented production-ready `LiveRouterPlanner` driving Observe -> Think (`boss`) -> Patch (`code`) -> Verify (`debug`) -> Judge (`judge`) with contextual outbound headers, cache-bypass retry signals (`X-Vibe-Retry: 1`), and automatic micro-dollar spend ingestion.
  4. **Multi-Branch Speculative Tournament (`va-harness::tournament`)**: Created `SpeculativeTournament` and `TournamentScorer` running parallel candidate branches forked from `last_verified`, evaluating candidates through sandboxed verification and judge rules, scoring survivors by zero warnings and minimal line churn, applying the winner, and appending a multi-parent merge node in the trajectory DAG.
  5. **Companion Launcher (`vibe-assembly up`), CLI & MCP Expansion**: Added `vibe-assembly up` (router health probe, auto-spawn, token handshake), `vibe-assembly repomap`, and `vibe-assembly diagnose`. Registered `vibe_assembly_repomap`, `vibe_assembly_diagnose_blocks`, and `vibe_assembly_tournament_step` on the stdio MCP server. Added live RepoMap and tournament status to desktop and terminal dashboards.
- **Consequence:** 164 passing tests across the workspace, 0 warnings under `cargo clippy --all-targets -- -D warnings`, full offline buildability, and complete agent tool automation.

## [ADR-018] Production Hardening, Execution Modes & Real Live Planner
- **Date:** 2026-10-23
- **Status:** Implemented; 188 tests green; clippy clean with `-D warnings`; both release binaries built
- **Context:** The v2.0.0 harness had the right architecture but several production gaps: the live planner still relied on offline synthesis, prompts had no execution-mode or language-lens contract, context growth was not automatically bounded, reasoning traces leaked into later prompts, repeated block bodies could defeat recovery, identical retries could spin forever, and durable lessons were not persisted. The CLI had no one-shot `prompt` or interactive `chat`, and the TUI/desktop exposed no mode, language, model, or context controls.
- **Decision:**
  1. **Real HTTP planner (`va-core::router_client`, `va-harness::live_planner`)**: Replace heuristic/synthesized live output with real `POST /v1/chat/completions` calls. Parse the standard `choices[].message.content` shape, send the full Vibe-Link header set (`X-Vibe-Role`, `X-Vibe-Node-Id`, `X-Vibe-Profile`, `X-Vibe-Retry`) plus `Authorization: Bearer` / `X-API-Key`, retry transport/5xx faults once with `X-Vibe-Retry: 1`, and ingest cost/latency from both `X-Vibe-Router-*` and `X-VR-*` headers. Boss returns `files`/`targets`/`verify`/`done`; Code/Debug return complete block replacements addressed by 4-hex hash. Offline synthesis is confined to tests and `--offline-mock`; placeholder `// Step {n} implementation` generation is removed.
  2. **Execution modes and language lens (`va-core::modes`)**: Add Code (`#F5B800`), Web (`#06B6D4`), Chat read-only (`#3B82F6`), and Debug (`#EF4444`), each with a contract, guardrails, verification behavior, and UI accent. Add Auto/Rust/C/C++/Python/TS/JS/Go lenses with manifest and extension detection, verification commands, and judge guardrails. Chat is enforced as a hard engine boundary, not a prompt suggestion.
  3. **Context safety (`va-harness::session`, `engine`)**: Auto-compact at 75% of the active model window, collapsing older verified steps and failed retries into surface summaries while preserving the DAG for `replay()` and `rollback_to_last_verified()`. Evict `think`/reasoning traces from rendered prompts while retaining raw nodes. Slice observations over 80 lines or 4,000 bytes to head 30 + elision marker + tail 30, keeping the full log in the DAG. Always append a pinned working-set tail with fresh block hashes and line ranges. Expose a context meter in the CLI/TUI/desktop.
  4. **Recovery and stagnation guards (`patcher`, `engine`, `experience`)**: Reject a target whose normalized block body appears more than once with `PatchError::AmbiguousRepeatedAnchor`, even during 1-hop recovery. Trip `StagnationDetected` on the second identical `(file, block hash, failure signature)` failure and allow pivot or `SpeculativeTournament` escalation. Add Harness^2 c1..c8 surfaces with strict reset-to-base, contrastive pFailure recovery, and durable `.ai/EXPERIENCE.md` lessons.
  5. **Interfaces (`va-cli`, `va-desktop`)**: Add `vibe-assembly prompt "<text>" [--mode ...] [--lang ...] [--model ...] [--json]` and interactive read-only `vibe-assembly chat`. Add mode/language/model flags to `run`, `tui`, and desktop startup, auto-seed `.ai/model_combo.txt` on `init` and `run --auto-combo`, and resolve per-mode overrides from `.ai/models/<mode>.txt`. Add TUI selectors and a context meter. Make desktop mode/language/model chips clickable with dynamic mode accents on the header, selectors, context strip, and active panels.
- **Boundary:** The `.ai/` ticket contract, binary names (`vibe-assembly`, `vibe-assembly-desktop`), and crate package names (`va-core`, `va-harness`, `va-cli`, `va-desktop`) are unchanged. The router client remains a localhost-oriented raw HTTP/1.1 client; TLS is delegated to a local proxy.
- **Consequence:** A normal live run is model-authored end to end and fails closed on ambiguity or repeated failure. Context growth is bounded without losing auditability. The same mode/language/model contract is visible in the CLI, TUI, and desktop. New integration tests drive a local mock TCP server through a real Boss -> Code -> Verify trajectory and assert telemetry ingestion; unit tests cover compaction, think eviction, observation slicing, repeated anchors, read-only enforcement, stagnation escalation, language detection, and EXPERIENCE.md round-tripping.
