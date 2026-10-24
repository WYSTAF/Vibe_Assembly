# Code Style & Standards Guide

- Prefer explicit, readable Rust over clever shorthand.
- Every parser, router, allocator, patcher, engine, or session behavior needs an
  isolated test; integration tests belong in `crates/*/tests/`.
- Address edits by block hash and include a regression test for every ambiguity,
  drift, or recovery bug.
- Keep modules dependency-light and document any upstream-inspired behavior.
- Public APIs carry doc comments; internal helpers stay private.
- Never swallow an error that changes execution semantics. Fail closed on
  ambiguity, authorization failure, read-only violations, and repeated failures.
- Run `cargo fmt --all`, clippy with `-D warnings`, and the workspace tests before
  claiming a task is done.
