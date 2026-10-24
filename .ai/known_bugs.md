# Forensic Anomaly Log

## KB-002 - Live planner parsed 4-hex block hashes as decimal
- **Date:** 2026-10-23
- **Symptom:** A live Code response referencing a short hash such as `C8C7`
  failed with `code referenced unknown block hash`, even though the RepoMap
  printed that exact hash.
- **Root cause:** `lookup_block` used `str::parse::<u16>()` (decimal) for
  short hashes, while `patcher::short_hash` emits hexadecimal.
- **Fix:** Parse with `u16::from_str_radix(&cleaned, 16)`. Covered by the
  `live_planner_boss_code_verify_against_real_http` integration test.
- **Lesson:** A hash display format and its parser are one contract; test them
  against a real HTTP response, not only synthetic strings.

## KB-003 - Repeated-body ambiguity guard missed indented duplicates
- **Date:** 2026-10-23
- **Symptom:** A 1-hop patch recovery could still target a block whose body
  appeared again nested at a deeper indentation.
- **Root cause:** The repeat guard normalized trailing whitespace but preserved
  leading indentation, so the nested copy did not compare equal.
- **Fix:** Repeat detection trims each line before counting occurrences and
  rejects with `PatchError::AmbiguousRepeatedAnchor`. Regression test:
  `repeated_body_rejects_even_a_correctly_shifted_anchor`.
- **Lesson:** Ambiguity checks must be stricter than hash matching; otherwise a
  correct-looking recovery can silently edit the wrong scope.

## KB-004 - EXPERIENCE.md round-trip lost language/domain
- **Date:** 2026-10-23
- **Symptom:** Lessons reloaded from `EXPERIENCE.md` were never rendered for
  `rust`/`workspace` because the parsed language field was
  `"rust / workspace"`.
- **Root cause:** The parser split the date header with `splitn(3, " - ")`
  while the rendered header packs language and domain after a single separator.
- **Fix:** Split the scope on ` / ` after the date separator. Covered by
  `experience_round_trips_through_markdown`.
- **Lesson:** Markdown persistence needs a round-trip test for every field, not
  just the primary payload.

## Open Environment Caveats

- **GPUI stack on Windows:** the workspace builds with the MSVC toolchain and VS
  Build Tools. If a machine still hits `STATUS_STACK_BUFFER_OVERRUN`, build from
  a shell with `ulimit -s unlimited`; `RUST_MIN_STACK` is not sufficient.
- **Raw HTTP client scope:** `va-core::router_client` speaks HTTP/1.1 without
  TLS by design. Point it at a local router or a TLS-terminating proxy; do not
  send credentials to a remote plaintext endpoint.
- **No remote telemetry:** cost and latency are read only from router response
  headers and stored in the local session DAG.
