// Parity guard: the Rust parser (app/desktop-rust/src/parser.rs) and the JS
// parser (bin/lib/parser.js) must agree exactly. The `va` CLI reports
// state in a terminal while Mission Control reports it on screen; if the two
// ever disagree, one of them is lying to the user.
//
// These cases mirror app/desktop-rust/tests/parser.rs. Both sides assert the
// same expected values, so a change to one parser that is not mirrored in the
// other fails here.

const assert = require('node:assert/strict');
const { describe, it } = require('node:test');
const p = require('../bin/lib/parser.js');

const mkSteps = (n) =>
  Array.from({ length: n }, (_, i) => ({ n: i + 1, text: '', verify: null }));

describe('parser parity: verifiedSteps', () => {
  const steps = mkSteps(12);

  // Word-boundary matching: "step 12" must never satisfy "step 1".
  it('does not let a step prefix satisfy a shorter step', () => {
    assert.deepEqual(p.verifiedSteps(steps, ['Step 12 (cargo build): PASS exited 0']), [12]);
  });

  it('accepts a word-boundary match with PASS', () => {
    assert.deepEqual(p.verifiedSteps(steps, ['Step 1: PASS exited 0']), [1]);
  });

  it('accepts an exit-code-zero note', () => {
    assert.deepEqual(p.verifiedSteps(steps, ['Step 2 done, exit code: 0']), [2]);
  });

  it('rejects a note with no step reference', () => {
    assert.deepEqual(p.verifiedSteps(steps, ['All good. PASS']), []);
  });

  it('rejects a failed step', () => {
    assert.deepEqual(p.verifiedSteps(steps, ['Step 2 failed, exit 1']), []);
  });

  it('rejects lowercase "passed" without success evidence', () => {
    assert.deepEqual(p.verifiedSteps(steps, ['Step 2 passed']), []);
  });

  // Evidence must live in the SAME note as the step reference, so one PASS
  // can never verify every step.
  it('never leaks evidence across notes', () => {
    assert.deepEqual(
      p.verifiedSteps(steps, ['Working on step 1 now.', 'All good. PASS']),
      [],
    );
  });
});

describe('parser parity: active ticket pointer', () => {
  it('treats "none" as a real pointer, not a missing one', () => {
    const raw = [
      '## Active Ticket',
      '- **id:** none',
      '',
      '## Queue',
      '| order | id | title | status | effort | depends_on |',
      '|------:|----|-------|--------|--------|------------|',
      '| 1 | T-401 | Done already | done | S | none |',
    ].join('\n');
    const parsed = p.parseActiveTask(raw);
    assert.equal(parsed.active_id, 'none');
    assert.equal(p.validateContract(parsed), null);
  });
});

describe('parser parity: CRLF checkouts', () => {
  it('does not leave a trailing carriage return on captured fields', () => {
    const lf = [
      '## Active Ticket',
      '- **id:** none',
      '',
      '## Queue',
      '| order | id | title | status | effort | depends_on |',
      '|------:|----|-------|--------|--------|------------|',
      '| 1 | T-401 | Done already | done | S | none |',
    ].join('\n');
    const parsed = p.parseActiveTask(lf.replace(/\n/g, '\r\n'));
    assert.equal(parsed.active_id, 'none');
    assert.equal(parsed.queue[0].status, 'done');
  });
});
