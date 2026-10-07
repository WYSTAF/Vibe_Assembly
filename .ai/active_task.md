# Active Task Assignment Blueprint

## Wave
- **Name:** Vibe Assembly Desktop Launch
- **Objective:** Launch the Vibe Assembly Electron Desktop GUI application in development mode, verifying its dependencies and ensuring it runs correctly.
- **Status:** planning_complete
- **User paste:** `Execute active task`

## Status Snapshot
- **current_ticket:** T-401
- **completed_tickets:** T-401
- **remaining_tickets:** none
- **resume_rule:** Open the explicit Active Ticket, read its `relay_notes`, and continue at the first step without successful verification evidence.

## Active Ticket
- **id:** none

## Queue
| order | id | title | status | effort | depends_on |
|------:|----|-------|--------|--------|------------|
| 1 | T-401 | Launch Vibe Assembly Desktop in Dev Mode | done | S | none |

---

## Ticket T-401
### Meta
- **status:** done
- **effort:** S
- **depends_on:** none
- **files_allowed:**
   - .ai/active_task.md
- **files_forbidden:** (default: all other paths)

### context_inline
The desktop app is located in `app/opencode-dev/packages/desktop`. We need to verify its dependencies, run its predev script, and launch the development server.

### Steps
1. Verify and install dependencies inside the desktop package.
   - **Verify:** `cd app/opencode-dev/packages/desktop && bun install` -> expected successful installation of dependencies.
2. Run the predev script to prepare the build assets.
   - **Verify:** `cd app/opencode-dev/packages/desktop && bun run predev` -> expected predev script to complete successfully.
3. Launch the electron-vite development environment.
   - **Verify:** `cd app/opencode-dev/packages/desktop && bun run dev` -> expected electron-vite development server to start and open the Electron window.

### done_when
- The Electron development window opens successfully showing the Vibe Assembly GUI interface.

### stop_when
- Any dependency installation or predev script fails with a fatal error.
- The Electron window fails to open or crashes on startup.

### relay_notes
- **Step 1 (bun install):** PASS. exited 0 (prior session).
- **Step 2 (bun run predev):** PASS. CLI binary copied to `resources/opencode-cli.exe`. Boss ruling: the prior session's offline-fallback edit in `scripts/utils.ts` (try/catch around the CLI download; writes a placeholder binary on failure) is ACCEPTED and retained — it keeps `predev` usable on flaky networks. It was outside this ticket's allowlist when made; recorded here as an approved exception.
- **Step 3 (bun run dev):** PASS after Boss-approved fix (2026-08-25).
  - **Root cause (empirically reproduced):** `ELECTRON_RUN_AS_NODE=1` leaked into the launch environment (editors/agent harnesses running inside Electron — VS Code/Roo Code/Cursor — inject it into spawned shells). electron-vite spawns the Electron binary with inherited env, so electron.exe booted as plain Node; `import { BrowserWindow } from 'electron'` in transitive dep `electron-dl@4.0.0` (via `electron-context-menu`) resolved against the npm stub package, which has no named exports → `SyntaxError: ... does not provide an export named 'BrowserWindow'`. Reproduced byte-for-byte by launching `electron.exe .` with the var set; clean env launches fine.
  - **Fix (Boss-approved, outside original allowlist):** new `app/opencode-dev/packages/desktop/scripts/dev.ts` sanitizes `ELECTRON_RUN_AS_NODE` from a copied env object and passes the explicit `env` option to `spawn("electron-vite", ["dev"])`; `"dev"` script now routes through it (`bun ./scripts/dev.ts`). Explicit-env is required because mutating `process.env` under Bun does not reliably propagate to children.
  - **Verification:** `ELECTRON_RUN_AS_NODE=1 bun run dev` → sanitizer warning printed; vite main/preload built; renderer dev server up on localhost:5173; Electron window opened; crash reporter/auto-updater/sidecar/onboarding all initialized; ran ~78s until test-harness SIGTERM (exit 143 shutdown cascade only). No SyntaxError anywhere in the log.

## Completion Record
- **Closed:** 2026-08-25. Root cause + fix logged in `.ai/known_bugs.md` (KB-001) and `.ai/decisions.md` (ADR-008). `current_state.md` updated.
