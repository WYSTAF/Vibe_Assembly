// Vibe Router assignment client & resolver with offline fallback.
// Interacts with http://127.0.0.1:20128/v1/assignment or falls back to .ai/model_knowledge.json.
'use strict';

const http = require('http');
const path = require('path');
const fs = require('fs');

const DEFAULT_ROUTER_URL = process.env.VIBE_ROUTER_URL || 'http://127.0.0.1:20128/v1/assignment';

/**
 * Fetch dynamic model allocations from Vibe Router.
 *
 * @param {string} [url] - Router URL
 * @param {number} [timeoutMs=1500] - Request timeout
 * @returns {Promise<{
 *   is_live: boolean,
 *   offline_fallback_recommended: boolean,
 *   source: string,
 *   roles?: Record<string, { model: string, class?: string } | string>,
 *   recommended_ticket_sizes?: Record<string, string>,
 *   error?: string
 * }>}
 */
async function fetchRouterAssignment(url = DEFAULT_ROUTER_URL, timeoutMs = 1500) {
  if (!url) {
    return { is_live: false, offline_fallback_recommended: true, source: 'offline' };
  }

  return new Promise((resolve) => {
    let settled = false;
    const finish = (val) => {
      if (!settled) {
        settled = true;
        resolve(val);
      }
    };

    try {
      const parsed = new URL(url);
      const req = http.request(
        {
          hostname: parsed.hostname,
          port: parsed.port || 80,
          path: parsed.pathname + parsed.search,
          method: 'GET',
          headers: {
            Accept: 'application/json',
            'User-Agent': 'vibe-assembly/1.2.0',
          },
          timeout: timeoutMs,
        },
        (res) => {
          let body = '';
          res.on('data', (c) => {
            body += c;
          });
          res.on('end', () => {
            if (res.statusCode < 200 || res.statusCode >= 300) {
              return finish({
                is_live: false,
                offline_fallback_recommended: true,
                source: 'offline',
                error: `HTTP ${res.statusCode}`,
              });
            }

            try {
              const data = JSON.parse(body);
              if (data && data.offline_fallback_recommended === true) {
                return finish({
                  is_live: false,
                  offline_fallback_recommended: true,
                  source: 'offline (router-advised)',
                  data,
                });
              }

              const rawRoles = data.roles || data.assignments || {};
              const roles = {};
              for (const [mode, val] of Object.entries(rawRoles)) {
                if (typeof val === 'string') {
                  roles[mode.toLowerCase()] = { model: val, class: 'router' };
                } else if (val && typeof val === 'object') {
                  roles[mode.toLowerCase()] = {
                    model: val.model || val.name || String(val),
                    class: val.class || 'router',
                    ...val,
                  };
                }
              }

              const rawSizes = data.recommended_ticket_sizes || data.ticket_sizes || {};
              const sizes = {};
              if (typeof rawSizes === 'string') {
                for (const m of ['boss', 'code', 'debug', 'chat', 'hermes']) {
                  sizes[m] = rawSizes;
                }
              } else if (typeof rawSizes === 'object') {
                for (const [m, s] of Object.entries(rawSizes)) {
                  sizes[m.toLowerCase()] = String(s);
                }
              }

              if (Object.keys(roles).length === 0) {
                return finish({
                  is_live: false,
                  offline_fallback_recommended: true,
                  source: 'offline (empty roles)',
                });
              }

              return finish({
                is_live: true,
                offline_fallback_recommended: false,
                source: `Vibe Router (${parsed.host})`,
                roles,
                recommended_ticket_sizes: sizes,
                raw: data,
              });
            } catch (err) {
              return finish({
                is_live: false,
                offline_fallback_recommended: true,
                source: 'offline',
                error: `JSON parse error: ${err.message}`,
              });
            }
          });
        }
      );

      req.on('timeout', () => {
        req.destroy();
        finish({
          is_live: false,
          offline_fallback_recommended: true,
          source: 'offline',
          error: 'timeout',
        });
      });

      req.on('error', (err) => {
        finish({
          is_live: false,
          offline_fallback_recommended: true,
          source: 'offline',
          error: err.message,
        });
      });

      req.end();
    } catch (e) {
      finish({
        is_live: false,
        offline_fallback_recommended: true,
        source: 'offline',
        error: e.message,
      });
    }
  });
}

/**
 * Resolve role allocations, attempting live Vibe Router first, then
 * falling back to .ai/model_knowledge.json and .ai/model_combo.txt.
 *
 * @param {string} root - Workspace root directory
 * @param {object} [opts] - Options: { routerUrl, timeoutMs, locked, forceOffline }
 */
async function resolveAssignment(root, opts = {}) {
  const routerUrl = opts.routerUrl || DEFAULT_ROUTER_URL;
  const timeoutMs = opts.timeoutMs || 1500;

  if (!opts.forceOffline) {
    const live = await fetchRouterAssignment(routerUrl, timeoutMs);
    if (live.is_live && !live.offline_fallback_recommended && live.roles) {
      const defaultSizes = { boss: 'S', code: 'S', debug: 'S', chat: 'S', hermes: 'S' };
      const sizes = { ...defaultSizes, ...(live.recommended_ticket_sizes || {}) };
      return {
        is_live: true,
        source: live.source,
        roles: live.roles,
        assignments: live.roles,
        recommended_ticket_sizes: sizes,
        unassigned: [],
        warnings: [],
      };
    }
  }

  // Graceful offline fallback via .ai/model_knowledge.json
  const allocator = require(path.join(__dirname, '..', 'allocate-models'));
  const comboPath = path.join(root, '.ai', 'model_combo.txt');
  let combo = [];
  const locked = { ...(opts.locked || {}) };

  if (fs.existsSync(comboPath)) {
    try {
      const comboText = fs.readFileSync(comboPath, 'utf8');
      for (const rawLine of comboText.split('\n')) {
        let line = rawLine.trim();
        if (!line || line.startsWith('#')) continue;
        const hash = line.indexOf('#');
        if (hash !== -1) line = line.slice(0, hash).trim();
        if (!line) continue;
        const lockMatch = line.match(/^(boss|chat|code|debug|hermes)\s*=\s*(.+)$/i);
        if (lockMatch) {
          locked[lockMatch[1].toLowerCase()] = lockMatch[2].trim();
          continue;
        }
        combo.push(line);
      }
    } catch (_) { /* ignore */ }
  }

  let knowledge;
  try {
    knowledge = allocator.loadKnowledge(root);
  } catch (_) {
    // If knowledge base file is missing, synthesize a minimal baseline
    knowledge = {
      mode_requirements: {
        boss: { primary_class: 'flagship', fallback_classes: ['reasoner', 'workhorse'] },
        chat: { primary_class: 'fast', fallback_classes: ['workhorse', 'flagship'] },
        code: { primary_class: 'coder', fallback_classes: ['flagship', 'reasoner'] },
        debug: { primary_class: 'reasoner', fallback_classes: ['flagship', 'coder'] },
        hermes: { primary_class: 'workhorse', fallback_classes: ['fast', 'flagship'] },
      },
      models: [],
    };
  }

  // If combo is empty and no locks, provide fallback defaults from knowledge models
  if (combo.length === 0 && Object.keys(locked).length === 0 && knowledge.models?.length) {
    combo = knowledge.models.slice(0, 5).map((m) => m.name);
  }

  const res = allocator.allocate(combo, knowledge, { locked });
  const sizes = { boss: 'S', code: 'S', debug: 'S', chat: 'S', hermes: 'S' };

  return {
    is_live: false,
    source: 'offline (.ai/model_knowledge.json)',
    roles: res.assignments,
    assignments: res.assignments,
    recommended_ticket_sizes: sizes,
    unassigned: res.unassigned,
    warnings: res.warnings,
  };
}

module.exports = {
  DEFAULT_ROUTER_URL,
  fetchRouterAssignment,
  resolveAssignment,
};
