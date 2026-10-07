# 🧠 AI Context Operating Guide
Welcome, Agent. This directory is the core memory bank of this repository. Read this first to orient yourself before executing tasks.

Core Architecture Rules
1. 👑 Boss owns and manages: project_overview.md, project_config.md, project_rules.md, current_state.md, session_context.md, active_task.md, progress.md, decisions.md, coding_standards.md, verification.md, changelog.md (structure), model_strategy.md, change_protocol.md.
2. 🐞 Debug directly updates: known_bugs.md.
3. 💻 Code strictly consumes active_task.md and modifies targeted source code files.
4. 📣 Hermes may APPEND dated entries to `.ai/changelog.md` (Boss keeps file structure) and fully owns: root-level `*.md`, `docs/`, `plans/` — written only from disk evidence (changelog, progress, decisions, git log). Never touches production code, `.githooks/`, `.roomodes`, or `.ai/active_task.md`.

Tooling every mode should know about
- **Mission Control** (`app/desktop-rust/`): native GPUI dashboard over this directory. It renders the ticket queue, contract checks, and relay evidence — it never writes `.ai/` state. The shared JavaScript parser lives at `bin/lib/parser.js`; if you change the `active_task.md` layout or `.ai/ticket_contract.md`, update that parser and `app/desktop-rust/src/parser.rs` in the same task.
- **`va` CLI** (`bin/va.js`): terminal mirror of Mission Control (`va`, `va --copy`, `va --summary`). Same rule: schema changes here must keep it working.
- **`bin/reset-template-state.js`**: resets volatile workflow files to pristine template defaults; used by the bootstrap CLI after cloning.
- **Guardrail hook** (`.githooks/pre-commit`): blocks commits outside the architecture allowlist. Staged deletions are always permitted (cleanup). If you add a legitimate top-level directory to this workspace, add it to the hook's allowlist in the same change.

State-file contract
- The canonical layout of `active_task.md` is defined by `.ai/ticket_contract.md`. Treat that file as the spec; parsers are its implementations.
