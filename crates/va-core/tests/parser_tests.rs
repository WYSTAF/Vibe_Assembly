//! Contract-semantics tests for the Rust parser.
//!
//! These mirror `bin/lib/parser.js` exactly. The two implementations
//! must never disagree: the `vibe-assembly` CLI reports state in a terminal while this
//! app reports it on screen, and a divergence would make one of them lie.

use va_core::parser::*;

fn step(n: i64) -> Step {
    Step {
        n,
        text: format!("step {n}"),
        verify_command: None,
        verify_expected: None,
    }
}

const QUIESCENT: &str = r#"
# Active Task Assignment Blueprint

## Wave
- **Name:** Vibe Assembly Desktop Launch
- **Status:** planning_complete

## Status Snapshot
- **current_ticket:** T-401
- **completed_tickets:** T-401
- **remaining_tickets:** none

## Active Ticket
- **id:** none

## Queue
| order | id | title | status | effort | depends_on |
|------:|----|-------|--------|--------|------------|
| 1 | T-401 | Launch Vibe Assembly Desktop in Dev Mode | done | S | none |
"#;

#[test]
fn parses_quiescent_workspace() {
    let p = parse_active_task(QUIESCENT);
    assert_eq!(p.wave_name.as_deref(), Some("Vibe Assembly Desktop Launch"));
    // "none" is a real pointer value, not a missing one.
    assert_eq!(p.active_id.as_deref(), Some("none"));
    assert_eq!(p.queue.len(), 1);
    assert!(!p.has_work());
    assert_eq!(
        validate_contract(&p),
        None,
        "a finished workspace is coherent"
    );
}

#[test]
fn detects_none_with_open_work() {
    let raw = QUIESCENT.replace("| done | S | none |", "| pending | S | none |");
    let p = parse_active_task(&raw);
    let v = validate_contract(&p).expect("must fail closed");
    assert!(v.contains("none") && v.contains("T-401"), "got: {v}");
}

#[test]
fn parses_steps_and_verify_commands() {
    let raw = r#"
## Active Ticket
- **id:** T-500

## Queue
| order | id | title | status | effort | depends_on |
|------:|----|-------|--------|--------|------------|
| 1 | T-500 | Wire the relay | in_progress | S | none |

## Ticket T-500
### Meta
- **status:** in_progress
- **effort:** S
- **depends_on:** none
- **files_allowed:**
   - .ai/active_task.md
   - app/desktop-rust/src/parser.rs
- **files_forbidden:** (default: all other paths)

### context_inline
Some durable context.

### Steps
1. Install dependencies.
   - **Verify:** `bun install` -> expected successful installation.
2. Run the build.
   - **Verify:** `cargo build --release` -> expected a binary.
3. Launch.
   - **Verify:** `cargo run` -> expected a window.

### done_when
- The window opens.

### stop_when
- The build fails.

### relay_notes
- **Step 1 (bun install):** PASS. exited 0 (prior session).
- **Step 2 (cargo build):** FAILED, exit 1.
"#;
    let p = parse_active_task(raw);
    let t = p.active.as_ref().expect("active ticket parsed");
    assert_eq!(t.steps.len(), 3);
    assert_eq!(t.steps[0].verify_command.as_deref(), Some("bun install"));
    assert_eq!(
        t.steps[2].verify_expected.as_deref(),
        Some("expected a window.")
    );
    // files_allowed must not absorb the Steps bullets.
    assert_eq!(t.files_allowed.len(), 2);
    assert_eq!(validate_contract(&p), None);
}

#[test]
fn resume_point_is_first_unverified_step() {
    let steps = vec![step(1), step(2), step(3)];
    let notes = vec!["Step 1: PASS exited 0".to_string()];
    assert_eq!(verified_steps(&steps, &notes), vec![1]);
}

#[test]
fn step_prefix_trap_is_not_a_match() {
    // "Step 12" must never verify step 1.
    let steps = vec![step(1), step(12)];
    let notes = vec!["Step 12 (cargo build): PASS exited 0".to_string()];
    assert_eq!(verified_steps(&steps, &notes), vec![12]);
}

#[test]
fn evidence_must_be_in_the_same_note() {
    // One note says step 1, another says PASS. Neither alone qualifies.
    let steps = vec![step(1)];
    let notes = vec![
        "Working on step 1 now.".to_string(),
        "All good. PASS".to_string(),
    ];
    assert!(verified_steps(&steps, &notes).is_empty());
}

#[test]
fn detects_status_mismatch() {
    let raw = r#"
## Active Ticket
- **id:** T-500

## Queue
| order | id | title | status | effort | depends_on |
|------:|----|-------|--------|--------|------------|
| 1 | T-500 | Mismatch | pending | S | none |

## Ticket T-500
### Meta
- **status:** in_progress
### Steps
1. Do it.
"#;
    let v = validate_contract(&parse_active_task(raw)).expect("must fail closed");
    assert!(v.contains("Status mismatch"), "got: {v}");
}

#[test]
fn detects_unfinished_dependency() {
    let raw = r#"
## Active Ticket
- **id:** T-501

## Queue
| order | id | title | status | effort | depends_on |
|------:|----|-------|--------|--------|------------|
| 1 | T-500 | First | pending | S | none |
| 2 | T-501 | Second | in_progress | S | T-500 |

## Ticket T-501
### Meta
- **status:** in_progress
### Steps
1. Do it.
"#;
    let v = validate_contract(&parse_active_task(raw)).expect("must fail closed");
    assert!(v.contains("T-500") && v.contains("done"), "got: {v}");
}

#[test]
fn detects_duplicate_queue_ids() {
    let raw = r#"
## Active Ticket
- **id:** T-500

## Queue
| order | id | title | status | effort | depends_on |
|------:|----|-------|--------|--------|------------|
| 1 | T-500 | A | pending | S | none |
| 2 | T-500 | B | pending | S | none |
"#;
    let v = validate_contract(&parse_active_task(raw)).expect("must fail closed");
    assert!(v.contains("duplicate"), "got: {v}");
}

#[test]
fn crlf_checkouts_do_not_break_parsing() {
    let crlf = QUIESCENT.replace('\n', "\r\n");
    let p = parse_active_task(&crlf);
    assert_eq!(p.active_id.as_deref(), Some("none"));
    assert_eq!(p.queue[0].status, "done");
    assert_eq!(validate_contract(&p), None);
}

#[test]
fn log_entries_split_on_headings() {
    let bugs = "# Log\n\n## KB-001 — Electron crash\n- **Symptom:** boom\n\n## KB-002 — Second\n- **Symptom:** bang\n";
    let entries = parse_log_entries(bugs);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].id, "KB-001");
    assert!(entries[0].body.contains("boom"));
    assert!(!entries[1].body.contains("boom"), "bodies must not bleed");
}

#[test]
fn bracketed_adr_headings_parse() {
    let adrs = "## [ADR-008] Sanitize env\n- **Date:** 2026-08-25\n";
    let e = parse_log_entries(adrs);
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].id, "ADR-008");
    assert_eq!(e[0].title, "Sanitize env");
}

#[test]
fn state_file_keeps_duplicate_keys() {
    let s = parse_state_file("# T\n- **Date:** one\n- **Date:** two\n- **Blockers:** None.\n");
    assert_eq!(s.get("Date"), Some("one"));
    assert_eq!(s.all("Date"), vec!["one", "two"]);
    assert_eq!(s.get("Blockers"), Some("None."));
}

/// The template writes `None.` with a trailing period. Treating that as a real
/// blocker made the dashboard headline read BLOCKED on a healthy workspace.
#[test]
fn none_with_a_period_is_not_a_blocker() {
    use va_core::state::Workspace;
    let dir = std::env::temp_dir().join("va-blockers-test");
    let ai = dir.join(".ai");
    std::fs::create_dir_all(&ai).unwrap();
    std::fs::write(
        ai.join("current_state.md"),
        "# Current Operational State\n- **Blockers:** None.\n",
    )
    .unwrap();
    std::fs::write(ai.join("active_task.md"), QUIESCENT).unwrap();
    let ws = Workspace::load(&dir);
    assert_eq!(ws.blockers(), None, "`None.` must not read as a blocker");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_real_blocker_is_reported() {
    use va_core::state::Workspace;
    let dir = std::env::temp_dir().join("va-blockers-real");
    let ai = dir.join(".ai");
    std::fs::create_dir_all(&ai).unwrap();
    std::fs::write(
        ai.join("current_state.md"),
        "# S\n- **Blockers:** KB-002 unresolved.\n",
    )
    .unwrap();
    std::fs::write(ai.join("active_task.md"), QUIESCENT).unwrap();
    let ws = Workspace::load(&dir);
    assert_eq!(ws.blockers(), Some("KB-002 unresolved."));
    std::fs::remove_dir_all(&dir).ok();
}

/// Namespaced ticket ids (e.g. `VA-RUST-001`) must parse alongside the legacy
/// `T-NNN` form, so a new wave can introduce its own prefix without a parser
/// change. Bare numbers and lowercase junk must still be rejected.
#[test]
fn accepts_namespaced_ticket_ids() {
    let raw = r#"
## Active Ticket
- **id:** VA-RUST-001

## Queue
| order | id | title | status | effort | depends_on |
|------:|----|-------|--------|--------|------------|
| 1 | VA-RUST-001 | Scaffold the harness | pending | S | none |
| 2 | not-a-ticket | lowercase junk | pending | S | none |
| 3 | 42 | bare number | pending | S | none |

## Ticket VA-RUST-001
### Meta
- **status:** pending
- **effort:** S
- **depends_on:** none
- **files_allowed:**
   - crates/va-harness/src/lib.rs

### Steps
1. Create the crate.
"#;
    let p = parse_active_task(raw);
    assert_eq!(p.active_id.as_deref(), Some("VA-RUST-001"));
    // Only the well-formed id is accepted as a queue row.
    assert_eq!(p.queue.len(), 1, "junk ids must be rejected: {:?}", p.queue);
    assert_eq!(p.queue[0].id, "VA-RUST-001");
    assert_eq!(validate_contract(&p), None);
}

#[test]
fn legacy_t_prefix_still_parses() {
    let raw = r#"
## Active Ticket
- **id:** T-001

## Queue
| order | id | title | status | effort | depends_on |
|------:|----|-------|--------|--------|------------|
| 1 | T-001 | Legacy ticket | pending | S | none |

## Ticket T-001
### Meta
- **status:** pending
### Steps
1. Do the thing.
"#;
    let p = parse_active_task(raw);
    assert_eq!(p.active_id.as_deref(), Some("T-001"));
    assert_eq!(p.queue.len(), 1);
    assert_eq!(validate_contract(&p), None);
}
