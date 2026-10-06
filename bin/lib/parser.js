// Pure CommonJS markdown parser for the Vibe Assembly .ai/ workspace state.
// No dependencies — runs identically in Electron main, tests, and Node.
'use strict';

function between(text, startMarker, endMarker) {
  const i = text.indexOf(startMarker);
  if (i === -1) return undefined;
  const rest = text.slice(i + startMarker.length);
  if (endMarker == null) return rest.trim();
  const j = rest.indexOf(endMarker);
  return (j === -1 ? rest : rest.slice(0, j)).trim();
}

/** Parse `- **field:** value` lines anywhere in a section (case-insensitive). */
function field(text, name) {
  const m = text.match(new RegExp(`^- \\*\\*${name}:\\*\\*\\s*(.*)$`, 'mi'));
  return m ? m[1].trim() : undefined;
}

// Normalize CRLF (Windows checkouts via git autocrlf) so field captures
// never carry a trailing \r — otherwise status comparisons fail silently.
function normalize(raw) {
  return String(raw ?? '').replace(/\r\n?/g, '\n');
}

/**
 * Parse `.ai/active_task.md` per the ticket contract:
 * Wave, Status Snapshot, Active Ticket pointer, Queue table, per-ticket sections.
 */
function parseActiveTask(raw) {
  const text = normalize(raw);
  const queueSection = between(text, '## Queue', '\n## ') || '';

  const rows = [];
  for (const line of queueSection.split('\n')) {
    const m = line.match(/^\|\s*(\d+)\s*\|\s*(T-\d+)\s*\|(.*?)\|(.*?)\|(.*?)\|(.*?)\|/);
    if (!m) continue;
    const cells = m.slice(3).map((c) => c.trim());
    rows.push({
      order: Number(m[1]),
      id: m[2],
      title: cells[0],
      status: cells[1] || '',
      effort: cells[2] || '',
      depends_on: cells[3] || '',
    });
  }

  const pointerMatch = text.match(/^##\s*Active Ticket\s*[\r\n]+- \*\*id:\*\*\s*(.+)$/m);
  const activeId = pointerMatch ? pointerMatch[1].trim() : undefined;

  // Per-ticket detail section: "## Ticket T-NNN" up to the next "## " heading.
  let active = null;
  if (activeId && activeId !== 'none') {
    const sec = between(text, `## Ticket ${activeId}`, '\n## ');
    if (sec) {
      const relayIdx = sec.indexOf('### relay_notes');
      // Allowlist lives under "### Meta" only — never absorb Steps bullets.
      const metaSec = between(sec, '### Meta', '\n### ') || '';
      active = {
        id: activeId,
        title: queueRows(rows, activeId).title,
        status: field(metaSec, 'status'),
        effort: field(metaSec, 'effort'),
        files_allowed: [...metaSec.matchAll(/^\s+- (.+)$/gm)]
          .map((m) => m[1].trim())
          .filter((p) => p !== 'none' && !p.startsWith('(')), // drop "(default: ...)"
        context_inline: between(sec, '### context_inline', '\n### '),
        steps: parseSteps(between(sec, '### Steps', '\n### ')),
        done_when: bullets(between(sec, '### done_when', '\n### ')),
        stop_when: bullets(between(sec, '### stop_when', '\n### ')),
        relay_notes: bullets(relayIdx === -1 ? '' : sec.slice(relayIdx + '### relay_notes'.length)),
      };
    }
  }

  const snapshot = {
    current_ticket: field(between(text, '## Status Snapshot', '\n## ') || '', 'current_ticket'),
    completed_tickets: field(between(text, '## Status Snapshot', '\n## ') || '', 'completed_tickets'),
    remaining_tickets: field(between(text, '## Status Snapshot', '\n## ') || '', 'remaining_tickets'),
  };

  const waveName = field(between(text, '## Wave', '\n## ') || '', 'name');

  return { wave_name: waveName, snapshot, active_id: activeId || null, queue: rows, active };
}

function queueRows(rows, id) {
  return rows.find((r) => r.id === id) || {};
}

function parseSteps(section) {
  if (!section) return [];
  const steps = [];
  let cur = null;
  for (const line of section.split('\n')) {
    const step = line.match(/^\s*(\d+)\.\s+(.*)$/);
    if (step) {
      if (cur) steps.push(cur);
      cur = { n: Number(step[1]), text: step[2].trim(), verify: null };
      continue;
    }
    const verify = line.match(/\*\*Verify:\*\*\s*`([^`]+)`\s*(?:→|->)?\s*(.*)/);
    if (verify && cur) cur.verify = { command: verify[1].trim(), expected: (verify[2] || '').trim() };
  }
  if (cur) steps.push(cur);
  return steps;
}

function bullets(section) {
  if (!section) return [];
  return section
    .split('\n')
    .map((l) => l.trim())
    .filter((l) => l.startsWith('- '))
    .map((l) => l.replace(/^- /, '').replace(/\s*\(empty until Code runs\)\s*/i, '').trim())
    .filter((l) => l.length > 0);
}

/**
 * Parse the simple bullet-dash state file (`# Title` + `- **k:** v`).
 * Duplicate keys (e.g. several KB entries each with "Date:") are collected
 * into an array instead of silently overwriting earlier values.
 */
function parseStateFile(raw) {
  const text = normalize(raw);
  const title = (text.match(/^#\s+(.+)$/m) || [, ''])[1].trim();
  const fields = {};
  for (const line of text.split('\n')) {
    const m = line.match(/^- \*\*(.+?):\*\*\s*(.*)$/);
    if (!m) continue;
    const key = m[1].trim();
    const value = m[2].trim();
    if (key in fields) {
      if (Array.isArray(fields[key])) fields[key].push(value);
      else fields[key] = [fields[key], value];
    } else {
      fields[key] = value;
    }
  }
  return { title, fields };
}

/**
 * Fail-closed consistency checks mirroring .ai/ticket_contract.md hard rules.
 * Returns a human-readable violation string, or null when state is coherent.
 */
function validateContract(parsed) {
  const q = parsed.queue || [];
  const id = parsed.active_id;

  // Duplicate ids make the pointer ambiguous even before other checks.
  const seen = new Set();
  for (const r of q) {
    if (seen.has(r.id)) return `Queue contains duplicate id ${r.id}.`;
    seen.add(r.id);
  }

  const workRemains = q.some((r) => r.status !== 'done');

  if (!id) return 'Active Ticket pointer section is missing.';
  if (id === 'none') {
    // Contract: "none" is invalid while any Queue row is not done.
    if (workRemains) {
      const open = q.filter((r) => r.status !== 'done').map((r) => r.id).join(', ');
      return `Active Ticket is "none" but unfinished Queue rows remain: ${open}.`;
    }
    return null;
  }

  const pointed = q.filter((r) => r.id === id);
  if (pointed.length !== 1) {
    return `Queue must contain exactly one row for Active Ticket ${id}, found ${pointed.length}.`;
  }
  const row = pointed[0];
  if (!['pending', 'in_progress'].includes(row.status)) {
    return `Active Ticket ${id} has Queue status "${row.status}" — expected pending/in_progress.`;
  }
  if (parsed.active?.status && parsed.active.status !== row.status) {
    return `Status mismatch: Meta "${parsed.active.status}" vs Queue "${row.status}" for ${id}.`;
  }

  // Every dependency must exist and be done.
  const deps = String(row.depends_on || '')
    .split(',')
    .map((d) => d.trim())
    .filter((d) => d && d.toLowerCase() !== 'none');
  for (const dep of deps) {
    const depRow = q.find((r) => r.id === dep);
    if (!depRow) return `${id} depends_on ${dep}, which is not in the Queue.`;
    if (depRow.status !== 'done') {
      return `${id} depends_on ${dep}, whose status is "${depRow.status}" — must be done.`;
    }
  }

  return null;
}

/**
 * Relay-evidence semantics shared by Mission Control and `va`.
 * A step counts as verified ONLY when a SINGLE relay note both:
 *   - references that exact step number (word-boundary matched, so
 *     "Step 12" never satisfies "Step 1"), and
 *   - carries success evidence (PASS / exit code 0) within that same note.
 * Cross-note leakage (one PASS verifying every step) is impossible here.
 */
function verifiedSteps(steps, relayNotes) {
  const notes = (relayNotes || []).map((n) => String(n).toLowerCase());
  const verified = new Set();
  (steps || []).forEach((_, idx) => {
    const n = idx + 1;
    // \b keeps "step 12" from matching "step 1" (prefix trap).
    const mentionsThisStep = new RegExp(`\\bstep\\s*${n}\\b`);
    const successEvidence = /\bpass\b|\bexit(\s*code)?\s*[:=]?\s*0\b/;
    if (notes.some((note) => mentionsThisStep.test(note) && successEvidence.test(note))) {
      verified.add(n);
    }
  });
  return [...verified];
}

module.exports = { parseActiveTask, parseStateFile, parseSteps, bullets, validateContract, verifiedSteps };
