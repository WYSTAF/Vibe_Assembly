const { describe, it } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const root = path.join(__dirname, '..');
const pkg = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'));
const cliSrc = fs.readFileSync(path.join(root, 'bin', 'cli.js'), 'utf8');
const { isSafeProjectName } = require(path.join(root, 'bin', 'cli.js'));

describe('package identity', () => {
  it('name and bin are create-vibe-assembly', () => {
    assert.equal(pkg.name, 'create-vibe-assembly');
    assert.ok(pkg.bin && pkg.bin['create-vibe-assembly']);
  });

  it('repository.url is not shadcn/improve', () => {
    assert.ok(pkg.repository && pkg.repository.url);
    assert.equal(/shadcn\/improve/i.test(pkg.repository.url), false);
  });
});

describe('cli bootstrap safety', () => {
  it('agent-facing README_AI references only files that exist', () => {
    // The .ai/README_AI.md tooling section must stay in sync with reality.
    const aiReadme = fs.readFileSync(path.join(root, '.ai', 'README_AI.md'), 'utf8');
    for (const ref of [
      'bin/lib/parser.js',
      'bin/va.js',
      'bin/reset-template-state.js',
      '.githooks/pre-commit',
      '.ai/ticket_contract.md',
    ]) {
      assert.ok(aiReadme.includes(ref), `README_AI must mention ${ref}`);
      assert.ok(fs.existsSync(path.join(root, ...ref.split('/'))), `${ref} must exist`);
    }
  });

  it('roomodes does not manually set the source field (Roo manages it)', () => {
    const modes = fs.readFileSync(path.join(root, '.roomodes'), 'utf8');
    assert.equal(/^\s*source:\s*project\s*$/m.test(modes), false,
      'remove manual "source:" entries — Roo adds them automatically');
  });

  it('ships the five-mode assembly line with valid slugs', () => {
    const modes = fs.readFileSync(path.join(root, '.roomodes'), 'utf8');
    const slugs = [...modes.matchAll(/^\s*-\s*slug:\s*(\S+)/gm)].map((m) => m[1]);
    assert.deepEqual(slugs, ['1-boss', '2-chat', '3-code', '4-debug', '5-hermes']);
    // Roo spec: slugs are letters, numbers, hyphens only.
    for (const s of slugs) assert.match(s, /^[a-zA-Z0-9-]+$/);
  });

  it('hermes is restricted to documentation files only', () => {
    const modes = fs.readFileSync(path.join(root, '.roomodes'), 'utf8');
    const hermesBlock = modes.split('slug: 5-hermes')[1] || '';
    assert.ok(hermesBlock.length > 0, 'hermes mode must exist in .roomodes');
    assert.match(hermesBlock, /fileRegex/, 'hermes edit group must carry a fileRegex restriction');
    assert.match(hermesBlock, /Documentation files only/);
    // Must NOT have unrestricted edit/command groups.
    assert.ok(!/^    groups:\n      - read\n      - edit$/m.test(hermesBlock), 'hermes must not get unrestricted edit');
    assert.ok(!hermesBlock.includes('- command'), 'hermes must not run terminal commands');
  });

  it('hermes fileRegex cannot reach the guardrail or code files', () => {
    // Behavioral check of the exact regex Roo will enforce.
    const modes = fs.readFileSync(path.join(root, '.roomodes'), 'utf8');
    const line = modes.split('\n').find((l) => l.includes('fileRegex') && l.includes('.ai/changelog'));
    assert.ok(line, 'hermes regex line missing');
    const pattern = line.match(/"fileRegex":\s*"([^"]+)"/)[1];
    const re = new RegExp(pattern.replace(/\\\\/g, '\\')); // YAML single-backslash -> JS literal
    for (const mustDeny of ['.githooks/pre-commit', '.roomodes', '.roorules', '.ai/active_task.md',
                            'src/README-helper.js', 'bin/lib/parser.js']) {
      assert.equal(re.test(mustDeny), false, `regex must DENY ${mustDeny}`);
    }
    for (const mustAllow of ['README.md', 'CHANGELOG.md', 'docs/guide.md', 'plans/001-x.md', '.ai/changelog.md']) {
      assert.equal(re.test(mustAllow), true, `regex must ALLOW ${mustAllow}`);
    }
  });

  it('execution modes carry loop-prevention attempt caps', () => {
    // Adopted from Rune's loop prevention: 2 failed attempts → escalate.
    const modes = fs.readFileSync(path.join(root, '.roomodes'), 'utf8');
    const codeBlock = modes.split('slug: 3-code')[1]?.split('slug:')[0] || '';
    const debugBlock = modes.split('slug: 4-debug')[1]?.split('slug:')[0] || '';
    assert.match(codeBlock, /Attempt cap/, 'Code must have the attempt-cap rule');
    assert.match(debugBlock, /Attempt Cap/, 'Debug must have the attempt-cap rule');
    assert.match(debugBlock, /Safety Net First/, 'Debug must capture failing output before surgery');

    const rules = fs.readFileSync(path.join(root, '.roorules'), 'utf8');
    assert.match(rules, /Loop-Prevention Law/, 'constitution must carry the global law');
  });

  it('source has no shadcn/improve', () => {
    assert.equal(/shadcn\/improve/i.test(cliSrc), false);
  });

  it('uses execFileSync with args, not shell git clone interpolation', () => {
    assert.match(cliSrc, /execFileSync/);
    assert.equal(/execSync\s*\(\s*[`'"]git clone/.test(cliSrc), false);
  });

  it('isSafeProjectName rejects path injection', () => {
    assert.equal(isSafeProjectName('my-app'), true);
    assert.equal(isSafeProjectName('my_app.v2'), true);
    assert.equal(isSafeProjectName(undefined), false);
    assert.equal(isSafeProjectName(''), false);
    assert.equal(isSafeProjectName('.'), false);
    assert.equal(isSafeProjectName('..'), false);
    assert.equal(isSafeProjectName('../evil'), false);
    assert.equal(isSafeProjectName('foo/bar'), false);
    assert.equal(isSafeProjectName('foo\\bar'), false);
  });
});
