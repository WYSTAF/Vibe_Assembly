# Ticket Contract — Vibe Assembly Free-Tier Assembly Line

## Purpose

Chat brainstorms → Boss emits S-sized machine-checkable micro-tickets → weak/cheap Code models execute one ticket per session. **Disk is source of truth.** Chat history is never required for correctness. When a model hits rate-limit or context death mid-ticket, the next session resumes from disk `relay_notes` — no replay needed.

## Ticket Schema

| Field | Rules |
|-------|-------|
| `id` | `T-NNN` (e.g. T-001) |
| `title` | Imperative verb phrase; one outcome |
| `status` | `pending` \| `in_progress` \| `blocked` \| `done` \| `failed` |
| `order` | Integer execution order |
| `depends_on` | Ticket id(s) or `none` |
| `effort` | Prefer `S`; if M/L, Boss must split before handoff |
| `files_allowed` | Explicit path list — **hard allowlist**; Code may only edit these |
| `files_forbidden` | Optional; default = everything not in allowlist |
| `context_inline` | Facts + short excerpts; never "as discussed" |
| `steps` | 3–12 steps; each step ends with verification command + expected result |
| `done_when` | Machine-checkable (commands, exit codes, greps) |
| `stop_when` | Escalate to Boss/Debug; no improvisation outside allowlist |
| `relay_notes` | Starts empty; Code appends durable per-step verification evidence |

**Hard rules:**
- The Queue is the complete ordered work list; it does not select work for Code.
- `## Active Ticket` is the sole execution pointer and contains exactly one `- **id:** T-NNN` entry while work remains.
- The pointed Queue row and ticket Meta must exist exactly once, use the same status and dependencies, have status `pending` or `in_progress`, and have every dependency `done`.
- When all Queue rows are `done`, the Active Ticket id must be `none`; `none` is invalid while work remains.
- Missing, duplicate, stale, blocked, failed, or contradictory state fails closed. Code stops and returns to Boss; it never infers or falls back to another Queue ticket.
- **Completion Gate:** a ticket may be marked `done` ONLY after every step has an exit-code-0 relay entry AND the resulting file still passes these consistency rules (pointer ↔ Queue ↔ Meta agree). If marking done would create inconsistent state, fix the state first — a broken `done` is worse than an honest `in_progress`.
- One ticket ≈ one weak-model session budget.
- No ticket may require whole-repo recon; only allowlisted files + named commands.
- Everything not in `files_allowed` is off-limits for edits.

## `active_task.md` Canonical Layout

Every Boss blueprint for Code uses this skeleton:

```markdown
# 🎯 Active Task Assignment Blueprint

## Wave
- **Name:** …
- **Objective:** … (one paragraph)
- **Status:** planning_complete
- **User paste:** `Execute active task`

## Status Snapshot
- **current_ticket:** T-001
- **completed_tickets:** none
- **remaining_tickets:** T-001
- **resume_rule:** Open the explicit Active Ticket, read its `relay_notes`, and continue at the first step without successful verification evidence.

## Active Ticket
- **id:** T-001

## Queue
| order | id | title | status | effort | depends_on |
|------:|----|-------|--------|--------|------------|
| 1 | T-001 | … | pending | S | none |

## Ticket T-001
### Meta
- **status:** pending
- **effort:** S
- **depends_on:** none
- **files_allowed:**
  - path/one
- **files_forbidden:** (default: all other paths)

### context_inline
…

### Steps
1. …
   - **Verify:** `command` → expected …
2. …

### done_when
- …

### stop_when
- …

### relay_notes
- (empty until Code runs)
```

## Relay Protocol

1. **Source of truth:** `.ai/active_task.md` only (plus files listed in the Active Ticket's allowlist). Optional read of this file if schema is unclear is not a substitute for the ticket body.
2. **On Code session start:**
   - Read `## Active Ticket`; do not scan the Queue to choose a ticket.
   - Validate the pointer, Queue row, ticket Meta, status, and dependencies against the hard rules. On any conflict, stop and return to Boss without editing production files.
   - If its synchronized status is `pending`, change both Queue and Meta status to `in_progress` in one edit.
   - Read `relay_notes`; resume at the first step without successful verification evidence.
3. **After each completed step:** append an entry to `relay_notes` before starting the next step. Every entry contains the step number, changed paths, exact verification command, exit code, and next step. Only exit code 0 advances the resume point.
4. **On ticket completion:** in one edit, append final evidence, mark Queue and Meta status `done`, point `## Active Ticket` to the next dependency-ready `pending` ticket, and refresh every Status Snapshot field. If no work remains, use Active Ticket id `none`.
5. **On STOP / rate-limit / context pressure:** preserve synchronized status, append the latest evidence and exact next step, then tell the user to switch to Code and say `Continue relay`. Use `Execute active task` for a fresh pending Active Ticket.
6. **Never** depend on chat history for correctness.

## Sample Ticket (EXAMPLE ONLY — not for execution)

```
## Status Snapshot
- **current_ticket:** T-999
- **completed_tickets:** none
- **remaining_tickets:** T-999
- **resume_rule:** Open the explicit Active Ticket, read its `relay_notes`, and continue at the first step without successful verification evidence.

## Active Ticket
- **id:** T-999

## Queue
| order | id | title | status | effort | depends_on |
|------:|----|-------|--------|--------|------------|
| 1 | T-999 | Record the Current Relay | pending | S | none |

## Ticket T-999
### Meta
- **status:** pending
- **effort:** S
- **depends_on:** none
- **files_allowed:**
  - .ai/current_state.md
- **files_forbidden:** (default: all other paths)

### context_inline
current_state.md tracks the active milestone and blocked-on field.
The file uses YAML-like frontmatter with a "Blocked On:" line.

### Steps
1. Read `.ai/current_state.md` and find the "Blocked On:" line.
   - **Verify:** `findstr "Blocked On:" .ai\current_state.md` → non-empty output
2. Append a new line after "Blocked On:" that reads: "Relay: T-999 in progress".
   - **Verify:** `findstr "Relay:" .ai\current_state.md` → "Relay: T-999 in progress"
3. Update the Queue row for T-999 to status "done".
   - **Verify:** `findstr "T-999" .ai\active_task.md` → status "done"

### done_when
- `findstr "Relay:" .ai\current_state.md` exits 0
- Queue row for T-999 shows "done"

### stop_when
- current_state.md is not in files_allowed
- Need to modify any file outside .ai/current_state.md

### relay_notes
- (empty until Code runs)
```

## Differentiation from improve / scout

- `plans/` + `scout` = optional advisor/audit path. Not the default executor contract.
- **Primary executor path** = Boss tickets in `active_task.md` + Relay protocol.
- Not improve: no skill-internal execute / worktree execute as product path.
- Not the upstream improve project: no `shadcn` branding, structure, or execute-skill pipeline.