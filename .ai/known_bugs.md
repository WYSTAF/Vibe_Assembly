# 🐞 Forensic Anomaly Log

## KB-001 — Electron dev launch dies with `SyntaxError: ... does not provide an export named 'BrowserWindow'`
- **Date:** 2026-08-25 (T-401)
- **Symptom:** `bun run dev` in `app/opencode-dev/packages/desktop` crashed at startup: `SyntaxError: The requested module 'electron' does not provide an export named 'BrowserWindow'` from `electron-dl@4.0.0/index.js:5`.
- **Root cause:** `ELECTRON_RUN_AS_NODE=1` present in the launch environment. Editors and agent harnesses that themselves run inside Electron (VS Code, Roo Code, Cursor) inject this var into spawned shells/terminals. electron-vite spawns the Electron binary with inherited env, so electron.exe booted as plain Node instead of Electron. Named ESM imports from `'electron'` then resolve against the npm stub package (default export only, no named exports). `electron-dl@4.0.0` — a transitive dependency of `electron-context-menu` — uses `import { BrowserWindow } from 'electron'`, so module load failed before any app code ran.
- **Evidence:** reproduced byte-for-byte via `ELECTRON_RUN_AS_NODE=1 node_modules/electron/dist/electron.exe .`; clean env launches the app normally; probe confirmed the stub exposes no named exports under `ELECTRON_RUN_AS_NODE=1`.
- **Fix:** `scripts/dev.ts` sanitizes `ELECTRON_RUN_AS_NODE` out of a copied env object and spawns `electron-vite dev` with the explicit sanitized `env`. `"dev"` script routes through it. Explicit-env is mandatory: mutating `process.env` under Bun does not reliably propagate to child processes.
- **Lesson:** when an Electron toolchain crashes with "module 'electron' does not provide an export named X", check for `ELECTRON_RUN_AS_NODE` leakage first — it is an environment defect, not a code defect.
