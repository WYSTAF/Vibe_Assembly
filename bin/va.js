#!/usr/bin/env node
// va — Vibe Assembly status CLI.
// Terminal mirror of Mission Control: shows the ticket queue, active ticket,
// and the exact paste phrase for the current phase. Zero dependencies.
//
//   va               show workspace status
//   va --copy        copy the suggested paste phrase to the clipboard
//   va --summary     emit a Boss-ready markdown wave report
//   va --json        machine-readable status (scripts/integrations)
//   va --models      allocate your model combo to the five modes
//   va --doctor      workspace health check
//   va --root P      point at another workspace
//
// Model combo: put one router model per line in .ai/model_combo.txt
// (e.g. gemini-2.0-flash / deepseek-r1:free / qwen3-coder). Optional locks:
// lines like "boss = <model>". Allocation uses .ai/model_knowledge.json.
'use strict';

const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

function resolveRoot(argv) {
  const i = argv.indexOf('--root');
  if (i !== -1 && argv[i + 1]) return path.resolve(argv[i + 1]);
  let root = process.cwd();
  if (!fs.existsSync(path.join(root, '.ai'))) {
    let probe = path.dirname(root);
    for (let n = 0; n < 6 && probe !== path.parse(probe).root; n++) {
      if (fs.existsSync(path.join(probe, '.ai'))) return probe;
      probe = path.dirname(probe);
    }
  }
  return root;
}

function readIfExists(file, fallback) {
  try {
    return fs.readFileSync(file, 'utf8');
  } catch {
    return fallback;
  }
}

const C = process.stdout.isTTY
  ? { g: '\x1b[32m', y: '\x1b[33m', r: '\x1b[31m', b: '\x1b[36m', dim: '\x1b[2m', bold: '\x1b[1m', x: '\x1b[0m' }
  : { g: '', y: '', r: '', b: '', dim: '', bold: '', x: '' };

function statusColor(s) {
  if (s === 'done') return C.g;
  if (s === 'in_progress') return C.y;
  if (['blocked', 'failed', 'blocked_escalated'].includes(s)) return C.r;
  return C.dim;
}

async function main() {
  const argv = process.argv.slice(2);
  const root = resolveRoot(argv);
  const ai = (n) => path.join(root, '.ai', n);

  const lock = require(path.join(__dirname, 'lib', 'lock'));
  const lockStatus = lock.checkLock(root);

  const parser = require(path.join(__dirname, 'lib', 'parser'));
  const atRaw = readIfExists(ai('active_task.md'), null);
  if (atRaw === null) {
    console.error(`❌ No .ai/ directory found at or above ${root}. Not a Vibe Assembly workspace?`);
    process.exit(1);
  }
  let at;
  try {
    at = parser.parseActiveTask(atRaw);
  } catch (e) {
    console.error(`❌ Could not parse active_task.md: ${e.message}`);
    process.exit(2);
  }
  const cs = parser.parseStateFile(readIfExists(ai('current_state.md'), ''));
  const violation = (() => {
    try {
      return parser.validateContract(at);
    } catch (e) {
      return String(e.message);
    }
  })();

  // Suggested paste phrase, identical rules to Mission Control.
  let phrase = null;
  if (!violation && at.active_id && at.active_id !== 'none') {
    const st = at.active?.status ?? at.queue.find((r) => r.id === at.active_id)?.status;
    if (st === 'pending') phrase = 'Execute active task';
    else if (st === 'in_progress') phrase = 'Continue relay';
  }

  if (argv.includes('--copy')) {
    if (!phrase) {
      console.error('❌ No paste phrase available (no active ticket or contract violation).');
      process.exit(1);
    }
    copyToClipboard(phrase);
    console.log(`✅ Copied: ${C.b}${phrase}${C.x}  — paste it into 💻 Code Mode.`);
    return;
  }

  if (argv.includes('--summary')) {
    console.log(buildSummary(root, at, cs));
    return;
  }

  if (argv.includes('--models')) {
    await runModels(root, argv);
    return;
  }

  if (argv.includes('--doctor')) {
    await runDoctor(root);
    return;
  }

  // --json: machine-readable status for scripts and integrations. Composable
  // with the other flags that produce structured output.
  if (argv.includes('--json')) {
    const parser = require(path.join(__dirname, 'lib', 'parser'));
    const at = parser.parseActiveTask(atRaw);
    const violation = (() => {
      try { return parser.validateContract(at); } catch (e) { return String(e.message); }
    })();
    let verified = [];
    if (!violation && at.active) verified = parser.verifiedSteps(at.active.steps, at.active.relay_notes);
    const router = require(path.join(__dirname, 'lib', 'router'));
    const routerRes = await router.fetchRouterAssignment(undefined, 200);
    process.stdout.write(JSON.stringify({
      root,
      wave_name: at.wave_name ?? null,
      active_id: at.active_id ?? null,
      contract_violation: violation,
      queue: at.queue ?? [],
      verified_steps: verified,
      suggested_phrase: phrase,
      task_lock: {
        locked: lockStatus.locked,
        info: lockStatus.info,
      },
      router_assignment: {
        is_live: routerRes.is_live && !routerRes.offline_fallback_recommended,
        source: routerRes.source,
        roles: routerRes.roles || null,
        recommended_ticket_sizes: routerRes.recommended_ticket_sizes || null,
      },
    }, null, 2));
    return;
  }

  console.log('');
  console.log(` ${C.bold}⚡ VIBE ASSEMBLY${C.x}  ${C.dim}·  ${root}${C.x}`);
  if (at.wave_name) console.log(` ${C.dim}Wave:${C.x} ${at.wave_name}`);

  if (lockStatus.locked) {
    console.log(` ${C.y}🔒 TASK LOCK:${C.x} ${C.dim}Held by PID ${lockStatus.info?.pid} (${lockStatus.info?.holder || 'active'})${C.x}`);
  }

  const router = require(path.join(__dirname, 'lib', 'router'));
  const routerRes = await router.fetchRouterAssignment(undefined, 200);
  if (routerRes.is_live && !routerRes.offline_fallback_recommended) {
    console.log(` ${C.g}🟢 Router:${C.x} ${routerRes.source} (live assignment active)`);
  }

  if (violation) {
    console.log('');
    console.log(` ${C.r}⛔ CONTRACT VIOLATION${C.x} — ${violation}`);
    console.log(` ${C.dim}Return to 👑 Boss and fix .ai/active_task.md.${C.x}`);
    console.log('');
    process.exit(2);
  }

  // Queue table
  if (at.queue.length > 0) {
    console.log('');
    for (const r of at.queue) {
      const mark = r.id === at.active_id ? `${C.b}▸${C.x}` : ' ';
      console.log(
        ` ${mark} ${r.id.padEnd(7)} ${statusColor(r.status)}${(r.status || '—').padEnd(18)}${C.x}${r.title}`
      );
    }
  }

  // Active ticket detail
  if (at.active) {
    const a = at.active;
    const verified = new Set(parser.verifiedSteps(a.steps, a.relay_notes));
    console.log('');
    console.log(` ${C.bold}▶ ${a.id}${C.x} — ${a.title || ''}`);
    if (a.files_allowed?.length) {
      console.log(`   ${C.dim}files:${C.x} ${a.files_allowed.join(', ')}`);
    }
    console.log(`   ${C.dim}steps:${C.x} ${verified.size}/${(a.steps || []).length} verified`);
    if ((a.relay_notes || []).length) {
      const last = a.relay_notes[a.relay_notes.length - 1];
      const short = last.length > 100 ? `${last.slice(0, 97)}…` : last;
      console.log(`   ${C.dim}last evidence:${C.x} ${short.replace(/\n/g, ' ')}`);
    }
  }

  // Next action
  console.log('');
  if (phrase) {
    console.log(` ${C.g}⌨ NEXT:${C.x} ${C.b}${phrase}${C.x}   ${C.dim}(paste into 💻 Code Mode · va --copy)${C.x}`);
  } else {
    console.log(` ${C.y}○ All tickets done or none scoped — switch to 👑 Boss to plan the next wave.${C.x}`);
  }
  const blockers = cs.fields['Blockers'];
  if (blockers && blockers.toLowerCase() !== 'none') {
    console.log(` ${C.r}⚠ Blockers:${C.x} ${blockers}`);
  }
  console.log('');
}

// va --doctor: one-shot workspace health check.
async function runDoctor(root) {
  const ai = (n) => path.join(root, '.ai', n);
  const checks = [];
  const add = (ok, label, detail) => checks.push({ ok, label, detail });

  // Core state files exist and parse.
  for (const f of ['active_task.md', 'current_state.md', 'ticket_contract.md']) {
    const exists = fs.existsSync(ai(f));
    add(exists, `.ai/${f} present`, exists ? '' : 'missing — core contract file');
  }

  // Single-writer lockfile check (.ai/.task.lock)
  const lock = require(path.join(__dirname, 'lib', 'lock'));
  const lockStatus = lock.checkLock(root);
  if (lockStatus.locked) {
    add(false, 'single-writer task lock (.ai/.task.lock)', `held by PID ${lockStatus.info?.pid} (${lockStatus.info?.holder || 'active'})`);
  } else {
    add(true, 'single-writer task lock (.ai/.task.lock)', 'unlocked (available)');
  }

  // Vibe Router connectivity check (informational)
  const router = require(path.join(__dirname, 'lib', 'router'));
  const routerRes = await router.fetchRouterAssignment(undefined, 200);
  if (routerRes.is_live && !routerRes.offline_fallback_recommended) {
    add(true, 'Vibe Router connectivity', `live (${routerRes.source})`);
  } else {
    add(true, 'Vibe Router connectivity', 'offline fallback ready (.ai/model_knowledge.json)');
  }

  let at = null;
  try {
    const parser = require(path.join(__dirname, 'lib', 'parser'));
    at = parser.parseActiveTask(readIfExists(ai('active_task.md'), ''));
    const violation = parser.validateContract(at);
    add(!violation, 'ticket contract consistent', violation || `active: ${at.active_id ?? '(none)'}`);
  } catch (e) {
    add(false, 'ticket contract parses', e.message);
  }

  // Fresh-template detection.
  try {
    const config = fs.readFileSync(ai('project_config.md'), 'utf8');
    const fresh = /CHANGE_ME_/.test(config);
    add(!fresh, 'project configured', fresh ? 'CHANGE_ME_* placeholders remain — finish 👑 Boss first setup' : '');
  } catch {
    add(false, '.ai/project_config.md present', 'missing');
  }

  // Model combo sanity (optional feature — warn only).
  if (fs.existsSync(ai('model_combo.txt'))) {
    try {
      const allocator = require(path.join(__dirname, 'allocate-models'));
      const kb = allocator.loadKnowledge(root);
      const comboText = fs.readFileSync(ai('model_combo.txt'), 'utf8');
      const combo = comboText.split('\n')
        .map((l) => l.trim().replace(/#.*$/, '').trim())
        .filter((l) => l && !/^(boss|chat|code|debug|hermes)\s*=/i.test(l));
      const r = allocator.allocate(combo, kb);
      const classes = new Set(Object.values(r.assignments).map((a) => a.class));
      add(r.warnings.length === 0, 'model combo covers all mode classes',
        r.warnings.length ? r.warnings[0] : `${combo.length} models → ${classes.size} distinct classes`);
      add(fs.existsSync(ai('model_knowledge.json')), '.ai/model_knowledge.json present', '');
    } catch (e) {
      add(false, 'model combo allocates', e.message);
    }
  }

  console.log('');
  console.log(` ${C.bold}🩺 VIBE ASSEMBLY DOCTOR${C.x}  ${C.dim}·  ${root}${C.x}`);
  console.log('');
  let failures = 0;
  for (const c of checks) {
    const mark = c.ok ? `${C.g}✔${C.x}` : `${C.r}✘${C.x}`;
    if (!c.ok) failures++;
    console.log(` ${mark} ${c.label}${c.detail ? `${C.dim} — ${c.detail}${C.x}` : ''}`);
  }
  console.log('');
  console.log(failures === 0
    ? ` ${C.g}All checks passed. Workspace is healthy.${C.x}`
    : ` ${C.r}${failures} issue(s) found.${C.x} Fix the ✘ items above; ask 💬 Chat if unsure.`);
  console.log('');
}

// va --models: read .ai/model_combo.txt or dynamic allocations from Vibe Router.
async function runModels(root, argv) {
  const router = require(path.join(__dirname, 'lib', 'router'));
  const allocator = require(path.join(__dirname, 'allocate-models'));
  const comboPath = path.join(root, '.ai', 'model_combo.txt');

  const combo = [];
  const locked = {};
  if (fs.existsSync(comboPath)) {
    try {
      const comboText = fs.readFileSync(comboPath, 'utf8');
      for (const rawLine of comboText.split('\n')) {
        let line = rawLine.trim();
        if (!line || line.startsWith('#')) continue;
        const hash = line.indexOf('#');
        if (hash !== -1) line = line.slice(0, hash).trim();
        if (!line) continue;
        const lock = line.match(/^(boss|chat|code|debug|hermes)\s*=\s*(.+)$/i);
        if (lock) { locked[lock[1].toLowerCase()] = lock[2].trim(); continue; }
        combo.push(line);
      }
    } catch (_) { /* ignore */ }
  }

  // Attempt live allocation from Vibe Router
  const allocation = await router.resolveAssignment(root, { locked, timeoutMs: 200 });

  if (allocation.is_live) {
    console.log('');
    console.log(` ${C.bold}🧠 MODEL ALLOCATION${C.x}  ${C.g}🟢 LIVE${C.x}  ${C.dim}·  ${allocation.source}${C.x}`);
    console.log('');
    for (const [mode, a] of Object.entries(allocation.assignments)) {
      const icon = { boss: '👑', chat: '💬', code: '💻', debug: '🐞', hermes: '📣' }[mode] || '•';
      const tag = a.locked ? `${C.b}(locked)${C.x}` : `[${a.class || 'router'}]`;
      const size = allocation.recommended_ticket_sizes?.[mode] ? `  ${C.dim}(size: ${allocation.recommended_ticket_sizes[mode]})${C.x}` : '';
      console.log(` ${icon} ${mode.padEnd(7)} → ${C.b}${a.model || a.name}${C.x}  ${tag}${size}`);
    }
    console.log('');
    console.log(` ${C.dim}Dynamic roles actively provided by Vibe Router.${C.x}`);
    console.log('');
    return;
  }

  // Fallback to local model_combo.txt
  if (!fs.existsSync(comboPath)) {
    console.error(`❌ No model combo found. Create ${comboPath} with one router model per line, e.g.:`);
    console.error('   gemini-2.0-flash');
    console.error('   deepseek-r1:free');
    console.error('   qwen3-coder');
    console.error('Optional per-mode locks: "boss = <model>"');
    process.exit(1);
  }

  if (combo.length === 0 && Object.keys(locked).length === 0) {
    console.error('❌ model_combo.txt has no models in it.');
    process.exit(1);
  }

  let knowledge;
  try {
    knowledge = allocator.loadKnowledge(root);
  } catch (e) {
    console.error(`❌ Could not load .ai/model_knowledge.json: ${e.message}`);
    process.exit(1);
  }

  const { assignments, unassigned, warnings } = allocator.allocate(combo, knowledge, { locked });

  console.log('');
  console.log(` ${C.bold}🧠 MODEL ALLOCATION${C.x}  ${C.dim}·  combo of ${combo.length}${Object.keys(locked).length ? ` + ${Object.keys(locked).length} lock(s)` : ''}${C.x}`);
  console.log('');
  for (const [mode, a] of Object.entries(assignments)) {
    const icon = { boss: '👑', chat: '💬', code: '💻', debug: '🐞', hermes: '📣' }[mode] || '•';
    const tag = a.locked ? `${C.b}(locked)${C.x}` : `[${a.class}]`;
    console.log(` ${icon} ${mode.padEnd(7)} → ${C.b}${a.model}${C.x}  ${tag}`);
  }
  if (unassigned.length) {
    console.log('');
    console.log(` ${C.dim}unused: ${unassigned.join(', ')}${C.x}`);
  }
  for (const w of warnings) console.log(` ${C.y}⚠ ${w}${C.x}`);
  console.log('');
  console.log(` ${C.dim}Set these in your router profile / Roo mode settings. Re-run after editing model_combo.txt.${C.x}`);
  console.log('');
}

// Boss-ready wave report, derived entirely from disk state.
function buildSummary(root, at, cs) {
  const q = at.queue || [];
  const done = q.filter((r) => r.status === 'done');
  const open = q.filter((r) => r.status !== 'done');
  const lines = [];
  lines.push(`# Wave Report — ${at.wave_name || '(unnamed wave)'}`);
  lines.push('');
  lines.push(`- **Workspace:** \`${root}\``);
  lines.push(`- **Generated:** ${new Date().toISOString()}`);
  lines.push(`- **Progress:** ${done.length}/${q.length} tickets done`);
  lines.push('');
  lines.push('| Ticket | Title | Status |');
  lines.push('|--------|-------|--------|');
  for (const r of q) {
    lines.push(`| ${r.id} | ${r.title || ''} | ${r.status} |`);
  }
  if (open.length > 0) {
    lines.push('');
    lines.push('## Open work');
    for (const r of open) {
      const a = r.id === at.active_id ? at.active : null;
      const detail = a ? ` — resume via relay_notes (next: ${nextHint(a)})` : '';
      lines.push(`- ${r.id}: ${r.title || '(untitled)'} [${r.status}]${detail}`);
    }
  } else {
    lines.push('');
    lines.push('## Outcome');
    lines.push('All queue rows are done. 👑 Boss: log the outcome in `.ai/decisions.md` / `.ai/progress.md` and scope the next wave.');
  }
  const blockers = cs.fields['Blockers'];
  if (blockers && blockers.toLowerCase() !== 'none') {
    lines.push('');
    lines.push(`## Blockers`);
    lines.push(`- ${blockers}`);
  }
  return lines.join('\n');
}

function nextHint(activeTicket) {
  // Same shared semantics as the dashboard: verifiedSteps, not substrings.
  const parser = require(path.join(__dirname, 'lib', 'parser'));
  const steps = activeTicket?.steps || [];
  const verified = new Set(parser.verifiedSteps(steps, activeTicket?.relay_notes));
  for (let i = 0; i < steps.length; i++) {
    if (!verified.has(i + 1)) return `step ${i + 1}`;
  }
  return 'final verification';
}

function copyToClipboard(text) {
  // Ordered fallbacks per platform; Wayland-only systems lack xclip.
  const attempts =
    process.platform === 'win32' ? [['clip']]
    : process.platform === 'darwin' ? [['pbcopy']]
    : [['wl-copy'], ['xclip', '-selection', 'clipboard'], ['xsel', '--clipboard', '--input']];
  for (const [cmd, ...args] of attempts) {
    try {
      execFileSync(cmd, args, { input: text, stdio: ['pipe', 'ignore', 'ignore'] });
      return;
    } catch (_) { /* try next */ }
  }
  console.log('(clipboard unavailable — copy manually):', text);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
