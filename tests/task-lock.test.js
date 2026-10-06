const { describe, it } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {
  acquireLock,
  releaseLock,
  checkLock,
  withTaskLock,
  getLockPath,
} = require('../bin/lib/lock');

function makeTempWorkspace() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'va-lock-test-'));
  fs.mkdirSync(path.join(dir, '.ai'));
  return dir;
}

describe('single-writer task lock (.ai/.task.lock)', () => {
  it('acquires and releases cleanly in a workspace', () => {
    const ws = makeTempWorkspace();
    const lock = acquireLock(ws, 'test-holder');
    assert.ok(fs.existsSync(lock.lockPath));

    const status = checkLock(ws);
    assert.equal(status.locked, true);
    assert.equal(status.info.holder, 'test-holder');
    assert.equal(status.info.pid, process.pid);

    lock.release();
    assert.equal(fs.existsSync(lock.lockPath), false);
    assert.equal(checkLock(ws).locked, false);
    fs.rmSync(ws, { recursive: true, force: true });
  });

  it('fails closed when an active process holds the lock', () => {
    const ws = makeTempWorkspace();
    const lock = acquireLock(ws, '3-code');

    assert.throws(
      () => {
        acquireLock(ws, '1-boss');
      },
      (err) => {
        assert.equal(err.code, 'ELOCKED');
        assert.match(err.message, /Lock Acquisition Failed/);
        assert.match(err.message, /ADR-007/);
        assert.match(err.message, /3-code/);
        return true;
      }
    );

    lock.release();
    fs.rmSync(ws, { recursive: true, force: true });
  });

  it('recovers from a stale lock held by a dead process PID', () => {
    const ws = makeTempWorkspace();
    const lockPath = getLockPath(ws);

    // Write a lockfile with a PID that definitely does not exist (e.g. 999999)
    const deadPid = 999999;
    fs.writeFileSync(
      lockPath,
      JSON.stringify({
        pid: deadPid,
        holder: 'stale-crashed-mode',
        acquired_at: new Date(Date.now() - 100000).toISOString(),
      }),
      'utf8'
    );

    const initialStatus = checkLock(ws);
    assert.equal(initialStatus.stale, true);

    // Should break stale lock and acquire successfully
    const lock = acquireLock(ws, 'recovered-mode');
    assert.equal(fs.existsSync(lockPath), true);
    const newStatus = checkLock(ws);
    assert.equal(newStatus.locked, true);
    assert.equal(newStatus.info.holder, 'recovered-mode');
    assert.equal(newStatus.info.pid, process.pid);

    lock.release();
    fs.rmSync(ws, { recursive: true, force: true });
  });

  it('withTaskLock reliably releases lock even if callback throws', () => {
    const ws = makeTempWorkspace();
    const lockPath = getLockPath(ws);

    assert.throws(() => {
      withTaskLock(ws, 'throwing-mode', () => {
        assert.ok(fs.existsSync(lockPath));
        throw new Error('simulated crash');
      });
    }, /simulated crash/);

    assert.equal(fs.existsSync(lockPath), false);
    fs.rmSync(ws, { recursive: true, force: true });
  });
});
