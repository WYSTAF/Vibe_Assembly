# Project Overview

**Core vision:** Vibe Assembly is a native Rust workspace that lets small,
focused agents build and maintain large software projects without exhausting a
single chat context. Roles communicate through regulated `.ai/` state, and every
edit passes through a deterministic, hash-anchored execution harness.

**Target core features:**

- Four execution modes: Code, Web, Chat (read-only), Debug.
- Six language lenses: Auto, Rust, C/C++, Python, TS/JS, Go.
- Real OpenAI-compatible live planner with Boss, Code, Verify, and Debug phases.
- Immutable session DAG with replay, rollback, compaction, and telemetry.
- Hash-anchored patching with drift recovery and ambiguity rejection.
- Deterministic judge with language and mode guardrails.
- Durable `.ai/EXPERIENCE.md` recovery lessons.
- CLI, TUI, MCP server, and the native GPUI Zed-style workbench.

**Strategic non-goals:**

- No webview, browser process, or second package-manager toolchain.
- No cloud service or remote state store; `.ai/` is the source of truth.
- No silent edits: a live run never fabricates a patch.
- No automatic publishing or commit attribution.
