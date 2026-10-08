# 🎯 Active Task Assignment Blueprint

## Wave
- **Name:** Wave 1 — Native Rust Execution Core
- **Objective:** Establish the pure-Rust execution engine for Vibe Assembly. Wave 1 delivers the `va-harness` crate: a hash-anchored editing engine that lets an agent apply precise, verifiable edits to files without whole-file rewrites, plus the relay/verification loop that makes interrupted sessions resumable. This is the foundation every later wave builds on.
- **Status:** planning_complete
- **User paste:** `Execute active task`

## Status Snapshot
- **current_ticket:** VA-RUST-001
- **completed_tickets:** none
- **remaining_tickets:** VA-RUST-001
- **resume_rule:** Open the explicit Active Ticket, read its `relay_notes`, and continue at the first step without successful verification evidence.

## Active Ticket
- **id:** VA-RUST-001

## Queue
| order | id | title | status | effort | depends_on |
|------:|----|-------|--------|--------|------------|
| 1 | VA-RUST-001 | Scaffold va-harness execution engine (hash-anchored editing) | pending | S | none |

## Ticket VA-RUST-001
### Meta
- **status:** pending
- **effort:** S
- **depends_on:** none
- **files_allowed:**
   - Cargo.toml
   - crates/va-harness/Cargo.toml
   - crates/va-harness/src/lib.rs
   - crates/va-harness/src/anchor.rs
   - crates/va-harness/tests/anchor_tests.rs
   - .ai/active_task.md
- **files_forbidden:** (default: all other paths)

### context_inline
Vibe Assembly migrated to a pure-Rust Cargo workspace on 2026-10-08 (ADR-014). Existing crates: `va-core` (parser/allocator/router/lock/prompts/bootstrap), `va-cli` (binary `vibe-assembly`), `va-desktop` (binary `vibe-assembly-desktop`). All three build offline with `ulimit -s unlimited`.

The workspace root `Cargo.toml` declares members explicitly under `[workspace] members = [...]`. A new crate must be added to that list or it will not build.

**What hash-anchored editing means:** instead of rewriting a whole file, an agent names a small anchor (a unique substring plus a line-context hash) and supplies a replacement. The harness locates the anchor, verifies the surrounding content still hashes to the expected value, and applies the edit only if it matches. This prevents the classic failure where a model edits stale content it remembered from an earlier context — the edit is rejected rather than silently corrupting the file.

**Design constraint:** `va-harness` must depend only on `va-core` plus the standard library. No new third-party crates, because the network is throttled on this machine and offline builds are required. A simple FNV-1a or std `DefaultHasher` is sufficient for anchoring — do not add `sha2`.

**Verification style:** every step below has a command whose exit code proves the step worked. Run them in order.

### Steps
1. Create the `va-harness` crate skeleton and register it in the workspace.
   - **Verify:** `cargo metadata --offline --no-deps --format-version 1 | findstr va-harness` → prints a line containing `va-harness`
2. Implement `Anchor` in `crates/va-harness/src/anchor.rs`: a struct holding `before` (context lines), `target` (the exact text to replace), and `after` (context lines), plus a `hash_of(&str) -> u64` helper using `std::collections::hash_map::DefaultHasher`.
   - **Verify:** `cargo build --offline -p va-harness` → exits 0
3. Implement `apply_anchored_edit(content: &str, anchor: &Anchor, replacement: &str) -> Result<String, AnchorError>` in `anchor.rs`. It must locate `anchor.target` inside `content`, confirm the surrounding context matches, and return the edited content. If the target appears zero times or more than once, return a distinct error variant rather than guessing.
   - **Verify:** `cargo build --offline -p va-harness` → exits 0
4. Re-export the public API from `crates/va-harness/src/lib.rs` (`Anchor`, `AnchorError`, `apply_anchored_edit`, `hash_of`).
   - **Verify:** `cargo build --offline -p va-harness` → exits 0
5. Write integration tests in `crates/va-harness/tests/anchor_tests.rs` covering the four required cases: successful edit; target-not-found; target-ambiguous (appears twice); stale-context rejection (context hash mismatch).
   - **Verify:** `cargo test --offline -p va-harness` → all tests pass, exit 0
6. Append one relay_notes entry recording the step number, changed paths, exact command, and exit code; then mark the Queue row and Meta status `done`, set the Status Snapshot to reflect completion, and set `## Active Ticket` to `none`.
   - **Verify:** `cargo test --offline --workspace` → exits 0 with no regressions

### done_when
- `cargo build --offline -p va-harness` exits 0
- `cargo test --offline -p va-harness` exits 0 and reports at least 4 passing tests
- `cargo clippy --offline -p va-harness --all-targets` exits 0 with zero warnings
- `va-harness` appears in `cargo metadata` workspace members

### stop_when
- Any step requires a third-party dependency not already in `Cargo.lock`
- Any step requires editing a file outside `files_allowed`
- `cargo build` fails twice on the same step (attempt cap — escalate to Debug)
- The network is required for any step (offline builds only)

### relay_notes
- (empty until Code runs)
