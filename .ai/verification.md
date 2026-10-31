# Verification Record & Proven Baselines

**Current feature proven:** v2.3.0 Liquid Glass night/day themes, full-bleed
drawer, circular context ring, manual pinning, and attachment cards

**Test suites and checks executed:**

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo test --workspace` - 200 passed, 0 failed
- [x] `cargo build --release` - `vibe-assembly` and `vibe-assembly-desktop`
- [x] Desktop smoke test - a bare launch of the release binary installed at
      `target/release` opens the Liquid Glass workbench; `--workbench` opens the
      Paper & Ink workspace workbench
- [x] Liquid Glass default surface - chat rail, model pill, glass ask bar,
      auto-hide nav with hover-opened Session/Changes/Editor/History panels,
      context ring, command sheet, and compact 1120x720 layout verified with
      pixel captures
- [x] Windows icon embedded at 16, 20, 24, 32, 40, 48, 64, 96, 128, and
      256 px; rounded transparent corners verified
- [x] Live HTTP integration test against a local mock OpenAI-compatible server
- [x] Compaction, think eviction, observation slicing, repeated-anchor,
      read-only, stagnation, language-lens, and EXPERIENCE.md tests

**Runtime environment:**

- Rust stable MSVC toolchain with Visual Studio Build Tools (C++ workload)
- Windows 10/11 host; Linux builds use the same Cargo workspace
- State: `.ai/*.md` and `.ai/session.jsonl`
- Router: local OpenAI-compatible HTTP gateway

**Last date proven working:** 2026-10-31
