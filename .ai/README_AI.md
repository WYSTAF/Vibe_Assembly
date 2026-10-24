# AI Context Operating Guide

Welcome, Agent. This directory is the core memory bank of the repository. Read
this file before executing a task.

## Ownership

1. **Boss** owns and manages `project_overview.md`, `project_config.md`,
   `project_rules.md`, `current_state.md`, `session_context.md`,
   `active_task.md`, `progress.md`, `decisions.md`, `coding_standards.md`,
   `verification.md`, `changelog.md` structure, `model_strategy.md`, and
   `change_protocol.md`.
2. **Debug** directly updates `known_bugs.md` and consumes pinned compiler
   diagnostics.
3. **Code** consumes `active_task.md` and modifies only files inside the active
   ticket's `files_allowed` allowlist.
4. **Chat / Architect** is read-only. It never patches, writes files, or runs
   mutating commands.
5. **Hermes** may append dated entries to `.ai/changelog.md` (Boss keeps the
   structure) and owns root-level `*.md`, `docs/`, and `plans/`. Hermes writes
   only from disk evidence and never touches production code or
   `.ai/active_task.md`.

## Tooling

- **`vibe-assembly`** (`crates/va-cli`) is the terminal operations room:
  `status`, `models`, `doctor`, `init`, `reset`, `up`, `repomap`, `diagnose`,
  `run`, `prompt`, `chat`, `tui`/`top`, `profile`, `session`, `mcp`, `patch`,
  `blocks`, `fork`, `rollback`, and `compact`.
- **`vibe-assembly-desktop`** (`crates/va-desktop`) is the native GPUI Mission
  Control. It observes `.ai/` state and never writes it. It provides mode and
  language selectors, a model picker, and an auto-compaction context meter.
- **`va-core`** owns the `.ai/` parser, model allocator, router client, task
  lock, prompts, bootstrap, hardware runtime, and execution modes.
- **`va-harness`** owns the execution engine: session DAG, hash-anchored
  patcher, RepoMap, diagnostic pinpointer, review, judge, sandbox, tournament,
  durable experience, and the live router planner.

## State-File Contract

- The canonical layout of `active_task.md` is defined by
  `.ai/ticket_contract.md`. Treat that file as the specification; the Rust
  parsers in `va-core/src/parser.rs` are its implementation.
- A step is verified only when a single relay note references that step with a
  word boundary and carries success evidence. The resume point is the first
  unverified step.
- `session.jsonl` is an append-only DAG. Compaction changes only the rendered
  view, never the stored nodes.
- `EXPERIENCE.md` is durable, evidence-backed, and scoped by language and
  execution mode. It never overrides the active ticket contract.

## Execution Rules

- Work only inside `files_allowed`; forbidden paths are rejected by the patcher
  and the engine.
- Address edits by block hash, never by line number.
- Verify with the active language lens command before claiming success.
- After two identical failures, stop: the engine raises `StagnationDetected`
  and can escalate to `SpeculativeTournament`.
- Chat mode is a hard read-only boundary.
- Never emit placeholder implementations, `todo!()` bodies, or slop comments.
  The deterministic judge rejects them.
- A normal live run never fabricates edits. `--offline-mock` exists only for
  deterministic local testing.

## Verification Gates

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```
