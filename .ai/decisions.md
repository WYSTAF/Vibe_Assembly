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
