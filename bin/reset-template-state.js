#!/usr/bin/env node
// Resets volatile .ai/ workflow state to pristine template defaults.
// Used by bin/cli.js after cloning so every new project starts with clean,
// self-consistent agent memory instead of the authoring workspace's history.
// Also runnable manually: node bin/reset-template-state.js [project-dir]
'use strict';

const fs = require('fs');
const path = require('path');
const { withTaskLock } = require('./lib/lock');

const FILES = {
  'active_task.md': `# Active Task Assignment Blueprint

## Wave
- **Name:** First Feature
- **Objective:** Define and deliver the first feature of this project. Replace this blueprint with your real task via 👑 Boss Mode.
- **Status:** awaiting_boss
- **User paste:** n/a — Boss has not scoped any tickets yet.

## Status Snapshot
- **current_ticket:** none
- **completed_tickets:** none
- **remaining_tickets:** none
- **resume_rule:** Open the explicit Active Ticket, read its \`relay_notes\`, and continue at the first step without successful verification evidence.

## Active Ticket
- **id:** none

## Queue
| order | id | title | status | effort | depends_on |
|------:|----|-------|--------|--------|------------|

(No tickets yet. 👑 Boss fills this table per \`.ai/ticket_contract.md\`.)
`,
  'current_state.md': `# Current Operational State
- **Active Milestone:** Project Initialization
- **Current Assignment:** none — awaiting first Boss blueprint.
- **Planning Status:** No tickets defined yet.
- **Implementation Status:** Not started.
- **Verification Status:** Nothing to verify yet.
- **Resume Anchor:** n/a — no active tickets.
- **Blockers:** None.
`,
  'progress.md': `# Project Progress

## Active Wave
- (none yet — 👑 Boss records the first wave here)

## Scope Guard
- (👑 Boss narrows this when the first wave is scoped)

## Historical Waves
- (empty)
`,
  'session_context.md': `# ⏳ Active Session Context Handoff
Last Session Status: Fresh template — no sessions yet.
Active Blockers: None
Next Immediate Action Step: Switch to 💬 Chat to shape your idea, or 👑 Boss to scope the first feature.
`,
  'known_bugs.md': `# 🐞 Forensic Anomaly Log
No active anomalies detected. Code and Debug modes can append records directly here.
`,
  'changelog.md': `# Changelog (agent OS)
- Template initialized.
`,
  'verification.md': `# Verification Record & Proven Baselines
Current Feature Proven: Base AI Operating System Initialized
Test Suites & Checks Executed:
[x] Template cloned and metadata initialized.

Runtime Environment Specs:
Engine Host: CHANGE_ME_ENGINE_HOST (e.g., Node 24)
Persistence: CHANGE_ME_PERSISTENCE (e.g., PostgreSQL 17)

Last Date Proven Working: never — nothing executed yet.
`,
};

function resetTemplateState(projectDir) {
  const aiDir = path.join(projectDir, '.ai');
  if (!fs.existsSync(aiDir)) return false;

  return withTaskLock(projectDir, 'bootstrap-reset', () => {
    for (const [name, content] of Object.entries(FILES)) {
      const target = path.join(aiDir, name);
      try {
        if (fs.existsSync(target)) fs.writeFileSync(target, content, 'utf8');
      } catch (_) { /* best effort */ }
    }
    return true;
  });
}

module.exports = { resetTemplateState };

if (require.main === module) {
  const dir = process.argv[2] || process.cwd();
  console.log(resetTemplateState(dir) ? `✅ Reset .ai/ state in ${dir}` : `❌ No .ai/ directory found in ${dir}`);
}
