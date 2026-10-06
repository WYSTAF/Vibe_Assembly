const { describe, it, before, after } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { execFileSync } = require('node:child_process');

// Integration test: runs the real .githooks/pre-commit inside a temp git repo.
// Regression guard for the Windows path-separator bug: git reports staged
// paths with forward slashes on every OS, so the hook must split on '/',
// never path.sep (backslash on Windows), or every nested file gets blocked.
const root = path.join(__dirname, '..');
const HOOK = path.join(root, '.githooks', 'pre-commit');

describe('hook install integrity', () => {
  it('is stored executable in the git index (100755)', () => {
    // Hooks without the executable bit are silently IGNORED by git on
    // macOS/Linux — the guardrail would vanish there entirely.
    const mode = execFileSync('git', ['ls-files', '-s', '.githooks/pre-commit'], {
      cwd: root,
      encoding: 'utf8',
    }).split(' ')[0];
    assert.equal(mode, '100755', 'run: git update-index --chmod=+x .githooks/pre-commit');
  });

  it('has a node shebang', () => {
    const src = fs.readFileSync(HOOK, 'utf8');
    assert.match(src, /^#!\/usr\/bin\/env node/);
  });
});

function runHook(cwd) {
  try {
    execFileSync(process.execPath, [HOOK], { cwd, encoding: 'utf8' });
    return { code: 0, output: '' };
  } catch (err) {
    return { code: err.status ?? 1, output: `${err.stdout ?? ''}${err.stderr ?? ''}` };
  }
}

function git(cwd, ...args) {
  return execFileSync('git', args, { cwd, encoding: 'utf8' });
}

describe('.githooks/pre-commit', () => {
  let repo;

  before(() => {
    repo = fs.mkdtempSync(path.join(os.tmpdir(), 'va-hook-'));
    git(repo, 'init', '-q');
    // The deletion test below runs a real `git commit`, which needs an identity.
    // CI runners have no global user.name/user.email, so without this the suite
    // fails there with "Please tell me who you are" while passing locally.
    // Set it repo-locally so the test never depends on ambient git config.
    git(repo, 'config', 'user.email', 'test@example.invalid');
    git(repo, 'config', 'user.name', 'Vibe Assembly Test');
  });

  after(() => {
    fs.rmSync(repo, { recursive: true, force: true });
  });

  function stage(relPath) {
    const abs = path.join(repo, relPath);
    fs.mkdirSync(path.dirname(abs), { recursive: true });
    fs.writeFileSync(abs, 'x\n');
    git(repo, 'add', relPath);
  }

  it('allows production files under app/ (regression: forward-slash paths)', () => {
    stage('app/src/main.js');
    const r = runHook(repo);
    assert.equal(r.code, 0, `expected pass, got: ${r.output}`);
  });

  it('allows scout plans/ and graphify-out/ directories', () => {
    stage('plans/001-fix.md');
    stage('graphify-out/graph.json');
    const r = runHook(repo);
    assert.equal(r.code, 0, `expected pass, got: ${r.output}`);
  });

  it('allows CI workflows under .github/', () => {
    stage('.github/workflows/ci.yml');
    const r = runHook(repo);
    assert.equal(r.code, 0, `expected pass, got: ${r.output}`);
  });

  it('allows canonical root configuration files (.gitattributes, .gitignore, package.json)', () => {
    stage('.gitattributes');
    const r = runHook(repo);
    assert.equal(r.code, 0, `expected pass, got: ${r.output}`);
  });

  it('allows nested paths with spaces in the workspace name', () => {
    stage('app/my dir/file.js');
    const r = runHook(repo);
    assert.equal(r.code, 0, `expected pass, got: ${r.output}`);
  });

  it('allows non-ASCII (Unicode) filenames under allowed dirs', () => {
    // Git C-quotes non-ASCII paths unless core.quotepath=false; the hook must
    // handle both forms. This exercises the quotepath fix end-to-end.
    stage('app/日本語-файл.js');
    const r = runHook(repo);
    assert.equal(r.code, 0, `expected pass, got: ${r.output}`);
  });

  it('permits staged deletions of stray root files (cleanup is not a violation)', () => {
    const abs = path.join(repo, 'legacy-junk.txt');
    fs.writeFileSync(abs, 'old\n');
    git(repo, 'add', 'legacy-junk.txt');
    git(repo, 'commit', '-q', '-m', 'seed'); // commit so the file exists in HEAD
    git(repo, 'rm', '-q', 'legacy-junk.txt'); // stage deletion — must pass hook
    const r = runHook(repo);
    assert.equal(r.code, 0, `deletion should be allowed, got: ${r.output}`);
    git(repo, 'reset', '-q', '--hard');
  });

  it('blocks unexpected root files', () => {
    stage('stray.sh');
    const r = runHook(repo);
    assert.notEqual(r.code, 0);
    assert.match(r.output, /not allowed in the root directory/);
    git(repo, 'reset', '-q', '--', 'stray.sh');
    fs.rmSync(path.join(repo, 'stray.sh'), { force: true });
  });

  it('blocks unknown top-level directories', () => {
    stage('somewhere/file.txt');
    const r = runHook(repo);
    assert.notEqual(r.code, 0);
    assert.match(r.output, /outside the project architecture blueprints/);
    git(repo, 'reset', '-q', '--', 'somewhere/file.txt');
    fs.rmSync(path.join(repo, 'somewhere'), { recursive: true, force: true });
  });
});
