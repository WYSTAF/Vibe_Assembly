# Verification Record & Proven Baselines

**Current feature proven:** v2.4.0 Liquid Glass real command sheet, persisted
day/night theme, full-height drawer sections with nav-clear card text, and
window-fit drawer geometry

**Test suites and checks executed:**

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo test --workspace` - 204 passed, 0 failed
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

**Last date proven working:** 2026-10-31
