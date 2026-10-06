// Single-writer lockfile guard (.ai/.task.lock) around .ai/active_task.md
// Preserves ADR-007 integrity by preventing concurrent mode writes and race conditions.
'use strict';

const fs = require('fs');
const path = require('path');

function getLockPath(root) {
  return path.join(root, '.ai', '.task.lock');
}

/** Check if a PID is still actively running on the host system. */
function isPidAlive(pid) {
  if (!pid || typeof pid !== 'number' || pid <= 0) return false;
  try {
    // Signal 0 tests process existence without sending a terminating signal.
    process.kill(pid, 0);
    return true;
  } catch (err) {
    // ESRCH means process does not exist. EPERM means exists but different user (still alive).
    return err.code === 'EPERM';
  }
}

/**
 * Inspect the current lock status without mutating anything.
 * @returns {{ locked: boolean, stale: boolean, info: object|null, lockPath: string }}
 */
function checkLock(root) {
  const lockPath = getLockPath(root);
  if (!fs.existsSync(lockPath)) {
    return { locked: false, stale: false, info: null, lockPath };
  }

  try {
    const raw = fs.readFileSync(lockPath, 'utf8');
    const info = JSON.parse(raw);
    const alive = isPidAlive(info.pid);
    return {
      locked: alive,
      stale: !alive,
      info,
      lockPath,
    };
  } catch (_) {
    // Malformed lock file or read error
    return { locked: true, stale: false, info: { raw: 'unparseable' }, lockPath };
  }
}

/**
 * Acquire the .ai/.task.lock exclusively.
 * Fails closed if another active process holds the lock.
 *
 * @param {string} root - Workspace root containing .ai/
 * @param {string} holder - Mode or agent identity (e.g. '3-code', '1-boss', 'cli')
 * @returns {{ lockPath: string, release: () => void }}
 */
function acquireLock(root, holder = 'unknown') {
  const lockPath = getLockPath(root);
  const aiDir = path.dirname(lockPath);
  if (!fs.existsSync(aiDir)) {
    fs.mkdirSync(aiDir, { recursive: true });
  }

  const payload = {
    pid: process.pid,
    holder: String(holder),
    acquired_at: new Date().toISOString(),
    cwd: process.cwd(),
  };

  const writePayload = () => JSON.stringify(payload, null, 2) + '\n';

  try {
    // Exclusive file creation flag 'wx' atomically creates or fails if already exists
    const fd = fs.openSync(lockPath, 'wx');
    fs.writeFileSync(fd, writePayload(), 'utf8');
    fs.closeSync(fd);

    return {
      lockPath,
      release: () => releaseLock(root),
    };
  } catch (err) {
    if (err.code !== 'EEXIST') {
      throw new Error(`Failed to create lockfile ${lockPath}: ${err.message}`);
    }

    // Lock file already exists. Check if holder process is still alive.
    const status = checkLock(root);
    if (status.stale) {
      // Stale lock from a dead process: safely reclaim it
      try {
        fs.unlinkSync(lockPath);
        const fd = fs.openSync(lockPath, 'wx');
        fs.writeFileSync(fd, writePayload(), 'utf8');
        fs.closeSync(fd);
        return {
          lockPath,
          release: () => releaseLock(root),
        };
      } catch (retryErr) {
        throw new Error(
          `🛑 Lock Acquisition Failed: Stale lock recovery collided on ${lockPath}: ${retryErr.message}`
        );
      }
    }

    // Active lock held by another process: FAIL CLOSED to preserve ADR-007
    const info = status.info || {};
    const holderMsg = info.holder ? ` (${info.holder})` : '';
    const pidMsg = info.pid ? `PID ${info.pid}` : 'another process';
    const timeMsg = info.acquired_at ? ` at ${info.acquired_at}` : '';

    const diagnostic = [
      `🛑 Lock Acquisition Failed: .ai/.task.lock is held by ${pidMsg}${holderMsg}${timeMsg}.`,
      `Execution halted to preserve ADR-007 single-writer integrity for active_task.md.`,
      `If this is a stale lock from a terminated task, remove "${lockPath}".`,
    ].join('\n');

    const lockError = new Error(diagnostic);
    lockError.code = 'ELOCKED';
    lockError.lockInfo = info;
    throw lockError;
  }
}

/**
 * Release the lockfile if it exists and belongs to the current process (or force).
 */
function releaseLock(root, force = false) {
  const lockPath = getLockPath(root);
  if (!fs.existsSync(lockPath)) return true;

  try {
    if (!force) {
      const raw = fs.readFileSync(lockPath, 'utf8');
      const info = JSON.parse(raw);
      if (info.pid && info.pid !== process.pid) {
        return false; // Do not release another process's lock unless forced
      }
    }
    fs.unlinkSync(lockPath);
    return true;
  } catch (_) {
    return false;
  }
}

/**
 * Execute an operation while holding the task lock, automatically releasing on exit.
 */
function withTaskLock(root, holder, fn) {
  const lock = acquireLock(root, holder);
  try {
    return fn();
  } finally {
    lock.release();
  }
}

/**
 * Async variant of withTaskLock.
 */
async function withTaskLockAsync(root, holder, fn) {
  const lock = acquireLock(root, holder);
  try {
    return await fn();
  } finally {
    lock.release();
  }
}

// CLI handler for manual inspection or scripting
if (require.main === module) {
  const args = process.argv.slice(2);
  const cmd = args[0] || 'status';
  const root = args[1] || process.cwd();

  if (cmd === 'status') {
    const status = checkLock(root);
    console.log(JSON.stringify(status, null, 2));
  } else if (cmd === 'acquire') {
    const holder = args[2] || 'cli';
    try {
      const lock = acquireLock(root, holder);
      console.log(`✅ Lock acquired by PID ${process.pid} (${holder}) at ${lock.lockPath}`);
      // Keep process alive if invoked directly to hold lock until terminated
      process.stdin.resume();
    } catch (e) {
      console.error(e.message);
      process.exit(1);
    }
  } else if (cmd === 'release') {
    const ok = releaseLock(root, true);
    console.log(ok ? '✅ Lock released.' : '⚠️ No lock found or release failed.');
  }
}

module.exports = {
  getLockPath,
  isPidAlive,
  checkLock,
  acquireLock,
  releaseLock,
  withTaskLock,
  withTaskLockAsync,
};
