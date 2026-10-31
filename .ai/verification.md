# Verification Record & Proven Baselines

**Current feature proven:** v2.1.0 Paper & Ink workbench and design samples

**Test suites and checks executed:**

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo test --workspace` - 200 passed, 0 failed
- [x] `cargo build --release` - `vibe-assembly` and `vibe-assembly-desktop`
- [x] Desktop smoke test - Paper & Ink workbench window renders from the
      release binary installed at `target/release`
- [x] `--design-preview-glass` sample - always-on nav and context capsule,
      hover-opened Session/Changes/Editor/History panels, pinned state, and
      compact 1120x720 layout verified with pixel captures
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
