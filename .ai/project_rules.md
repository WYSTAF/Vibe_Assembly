# Absolute Project Conventions & Directives

## Code Architecture

- Prefer explicit, declarative Rust over clever shorthand or macro-heavy abstractions.
- Keep third-party dependencies to a bare minimum; add one only with explicit developer approval and a Cargo.lock update.
- Preserve the four-crate split: `va-core` (state/router), `va-harness` (execution), `va-cli` (terminal), `va-desktop` (GPUI).
- Keep the `.ai/` parser and the ticket contract in sync; the Rust parser is the implementation.

## Stack Constraints

- Build with Cargo on the stable MSVC toolchain. No other package manager or runtime is required.
- UI is native GPUI or ANSI terminal only. Do not introduce a webview or browser process.
- Router access uses the OpenAI-compatible HTTP client in `va-core::router_client`; TLS is terminated by a local proxy.
- Default to workspace-relative paths and the `{{PORTABLE_ROOT}}` token.

## Execution Guardrails

- Never introduce a new architectural library or design pattern without explicit developer approval.
- Code mode must add isolated tests for new utility, parser, router, or engine behavior.
- Address edits by block hash, never by line number.
- Chat mode is read-only; the engine enforces it.
- Run the full gates before claiming completion:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```
