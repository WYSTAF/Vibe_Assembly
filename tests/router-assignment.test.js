const { describe, it } = require('node:test');
const assert = require('node:assert/strict');
const http = require('node:http');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { fetchRouterAssignment, resolveAssignment } = require('../bin/lib/router');

function createMockServer(handler) {
  return new Promise((resolve) => {
    const server = http.createServer(handler);
    server.listen(0, '127.0.0.1', () => {
      const port = server.address().port;
      resolve({
        url: `http://127.0.0.1:${port}/v1/assignment`,
        close: () => new Promise((res) => server.close(res)),
      });
    });
  });
}

function makeTempWorkspace() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'va-router-test-'));
  const ai = path.join(dir, '.ai');
  fs.mkdirSync(ai);

  // Seed model_knowledge.json
  const kb = {
    mode_requirements: {
      boss: { primary_class: 'flagship', fallback_classes: ['reasoner'] },
      chat: { primary_class: 'fast', fallback_classes: ['workhorse'] },
      code: { primary_class: 'coder', fallback_classes: ['flagship'] },
      debug: { primary_class: 'reasoner', fallback_classes: ['flagship'] },
      hermes: { primary_class: 'workhorse', fallback_classes: ['fast'] },
    },
    models: [
      { name: 'Gemini Flash', aliases: ['gemini-flash'], class: 'fast' },
      { name: 'Qwen Coder', aliases: ['qwen-coder'], class: 'coder' },
      { name: 'DeepSeek Reasoner', aliases: ['deepseek-r1'], class: 'reasoner' },
      { name: 'Claude Flagship', aliases: ['claude-3-5-sonnet'], class: 'flagship' },
    ],
  };
  fs.writeFileSync(path.join(ai, 'model_knowledge.json'), JSON.stringify(kb, null, 2));
  fs.writeFileSync(
    path.join(ai, 'model_combo.txt'),
    'gemini-flash\nqwen-coder\ndeepseek-r1\nclaude-3-5-sonnet\n'
  );

  return dir;
}

describe('Vibe Router assignment integration (/v1/assignment)', () => {
  it('parses live dynamic roles and recommended ticket sizes', async () => {
    const mock = await createMockServer((req, res) => {
      if (req.url === '/v1/assignment') {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(
          JSON.stringify({
            offline_fallback_recommended: false,
            roles: {
              boss: { model: 'live-boss-model', class: 'flagship' },
              code: { model: 'live-code-model', class: 'coder' },
              debug: { model: 'live-debug-model', class: 'reasoner' },
              chat: { model: 'live-chat-model', class: 'fast' },
            },
            recommended_ticket_sizes: {
              boss: 'S',
              code: 'S',
              debug: 'M',
              chat: 'S',
            },
          })
        );
      } else {
        res.writeHead(404);
        res.end();
      }
    });

    try {
      const res = await fetchRouterAssignment(mock.url, 1000);
      assert.equal(res.is_live, true);
      assert.equal(res.offline_fallback_recommended, false);
      assert.equal(res.roles.boss.model, 'live-boss-model');
      assert.equal(res.roles.code.model, 'live-code-model');
      assert.equal(res.roles.debug.model, 'live-debug-model');
      assert.equal(res.roles.chat.model, 'live-chat-model');
      assert.equal(res.recommended_ticket_sizes.debug, 'M');
    } finally {
      await mock.close();
    }
  });

  it('falls back to .ai/model_knowledge.json when offline_fallback_recommended is true', async () => {
    const ws = makeTempWorkspace();
    const mock = await createMockServer((req, res) => {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(
        JSON.stringify({
          offline_fallback_recommended: true,
          roles: {},
        })
      );
    });

    try {
      const res = await resolveAssignment(ws, { routerUrl: mock.url, timeoutMs: 1000 });
      assert.equal(res.is_live, false);
      assert.match(res.source, /offline/);
      assert.ok(res.assignments.code, 'must allocate code mode offline');
      assert.equal(res.assignments.code.model, 'qwen-coder');
    } finally {
      await mock.close();
      fs.rmSync(ws, { recursive: true, force: true });
    }
  });

  it('falls back gracefully when router returns 404 or connection error', async () => {
    const ws = makeTempWorkspace();

    // Query non-existent port (127.0.0.1:19999)
    const res = await resolveAssignment(ws, {
      routerUrl: 'http://127.0.0.1:19999/v1/assignment',
      timeoutMs: 200,
    });

    assert.equal(res.is_live, false);
    assert.match(res.source, /offline/);
    assert.ok(res.assignments.boss);
    assert.ok(res.assignments.code);
    assert.equal(res.recommended_ticket_sizes.code, 'S');

    fs.rmSync(ws, { recursive: true, force: true });
  });

  it('handles string model formats and uniform ticket sizes', async () => {
    const mock = await createMockServer((req, res) => {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(
        JSON.stringify({
          offline_fallback_recommended: false,
          roles: {
            boss: 'gpt-4o',
            code: 'claude-3-5-sonnet',
            debug: 'o1-preview',
            chat: 'gpt-4o-mini',
          },
          recommended_ticket_sizes: 'S',
        })
      );
    });

    try {
      const res = await fetchRouterAssignment(mock.url, 1000);
      assert.equal(res.is_live, true);
      assert.equal(res.roles.boss.model, 'gpt-4o');
      assert.equal(res.roles.code.model, 'claude-3-5-sonnet');
      assert.equal(res.recommended_ticket_sizes.code, 'S');
      assert.equal(res.recommended_ticket_sizes.boss, 'S');
    } finally {
      await mock.close();
    }
  });
});
