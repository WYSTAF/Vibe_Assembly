#!/usr/bin/env node

const { execFileSync } = require('child_process');
const path = require('path');
const fs = require('fs');
const readline = require('readline');

const { resetTemplateState } = require('./reset-template-state');

// Standard project templates: a working starter in app/ + pre-filled .ai/ context.
function loadTemplateCatalog() {
  try {
    const catalogPath = path.join(__dirname, '..', 'templates', 'catalog.json');
    return JSON.parse(fs.readFileSync(catalogPath, 'utf8')).templates;
  } catch {
    return [];
  }
}

function applyProjectTemplate(name, projectPath) {
  // Precondition: name already validated against the catalog before cloning.
  const src = path.join(__dirname, '..', 'templates', name);
  fs.cpSync(path.join(src, 'app'), path.join(projectPath, 'app'), { recursive: true });
  // Seed (overwrite) the .ai context files the template pre-fills. The clone
  // already has generic versions; template ones describe THIS starting point.
  const seedDir = path.join(src, '.ai');
  if (fs.existsSync(seedDir)) {
    for (const file of fs.readdirSync(seedDir)) {
      const target = path.join(projectPath, '.ai', file);
      if (fs.existsSync(target)) fs.copyFileSync(path.join(seedDir, file), target);
    }
  }
  console.log(`\n🧩 Applied "${name}" project template — a working starter is in app/.`);
}

const ORANGE = '\x1b[38;5;208m';
const BOLD = '\x1b[1m';
const RESET = '\x1b[0m';

const banner = `

${ORANGE}${BOLD} █░█ ░█░ █▄▄ █▀▀   ▄▀█ █▀▀ █▀▀ █▀▀ █▀▄▀█ █▄▄ █░░ █▄█ ${RESET}
${ORANGE}${BOLD} ▀▄▀ ░█░ █▄█ ██▄   █▀█ ▄█░ ▄█░ ██▄ █░▀░█ █▄█ █▄▄ ░█░ ${RESET}
`;

const SAFE_NAME = /^[A-Za-z0-9._-]+$/;

function isSafeProjectName(name) {
  if (!name || typeof name !== 'string') return false;
  if (name === '.' || name === '..') return false;
  if (name.includes('/') || name.includes('\\')) return false;
  if (path.isAbsolute(name)) return false;
  return SAFE_NAME.test(name);
}

function askGraphify() {
  // Non-interactive stdin (CI, piped npx): default to No instead of hanging forever.
  if (!process.stdin.isTTY) {
    console.log('\n⏭️ Non-interactive session detected. Skipping optional installs (pass a TTY to be asked).');
    return Promise.resolve(false);
  }
  return new Promise((resolve) => {
    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout,
    });
    rl.question('Would you like to install Graphify for visual codebase analysis? (y/N): ', (answer) => {
      rl.close();
      const trimmed = answer.trim().toLowerCase();
      resolve(trimmed === 'y' || trimmed === 'yes');
    });
  });
}

function askMissionControl() {
  if (!process.stdin.isTTY) return Promise.resolve(false);
  return new Promise((resolve) => {
    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout,
    });
    rl.question('Would you like to set up Mission Control, the Vibe Assembly desktop dashboard? (Y/n): ', (answer) => {
      rl.close();
      const trimmed = answer.trim().toLowerCase();
      resolve(trimmed === '' || trimmed === 'y' || trimmed === 'yes');
    });
  });
}

function setupMissionControl(projectPath) {
  // On Windows, npm is npm.cmd and cannot be spawned without a shell.
  const npmCmd = process.platform === 'win32' ? 'npm.cmd' : 'npm';
  console.log('\n📦 Mission Control (Native GPUI Desktop Dashboard):');
  console.log(`   Run with: cd ${path.join(projectPath, 'app', 'desktop-rust')} && cargo run`);
}

function initMetadata(projectName, projectPath) {
  try {
    const configPath = path.join(projectPath, '.ai', 'project_config.md');
    if (fs.existsSync(configPath)) {
      let content = fs.readFileSync(configPath, 'utf8');
      content = content.replace(/CHANGE_ME_PROJECT_NAME/g, projectName);
      fs.writeFileSync(configPath, content, 'utf8');
    }
  } catch (_) { /* skip silently */ }

  try {
    const pkgPath = path.join(projectPath, 'package.json');
    if (fs.existsSync(pkgPath)) {
      const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
      // Turn the template's CLI manifest into a neutral project manifest:
      // no bin (prevents accidental publish under the template's name),
      // no template-specific keywords/repository.
      delete pkg.bin;
      delete pkg.repository;
      delete pkg.keywords;
      pkg.name = projectName;
      pkg.version = '0.1.0';
      pkg.private = true;
      pkg.description = 'TODO: describe your project';
      fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n', 'utf8');
    }
  } catch (_) { /* skip silently */ }
}

async function main() {
  // Flag-aware argv walk: --template/-t consumes its value; everything else
  // is positional. Flags stay in positional[] so `projectName === '--help'`
  // guards below keep working.
  let templateName;
  let templateFlagSeen = false;
  const positional = [];
  const args = process.argv.slice(2);
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--template' || args[i] === '-t') {
      templateFlagSeen = true;
      const val = args[++i];
      // Missing value (end of argv) or another flag: loud usage error, not a
      // silent no-template bootstrap.
      if (!val || val.startsWith('-')) {
        console.error('\n❌ Error: --template requires a value.');
        const names = loadTemplateCatalog().map((t) => t.name).join(', ');
        console.error(`   Available templates: ${names || '(none bundled)'}\n`);
        process.exit(1);
      }
      templateName = val;
    } else {
      positional.push(args[i]);
    }
  }
  const projectName = positional[0];

  console.log(banner);

  if (projectName === '--help' || projectName === '-h') {
    const names = loadTemplateCatalog().map((t) => t.name);
    console.log('Usage: npx create-vibe-assembly <project-name> [--template <name>]');
    console.log('');
    console.log('Bootstraps a fresh Vibe Assembly workspace (5-Mode Roo Code template).');
    console.log('');
    console.log('Options:');
    console.log('  -h, --help            Show this help.');
    console.log('  -v, --version         Print the CLI version.');
    console.log(`  -t, --template <name> Start from a standard project template: ${names.join(', ') || '(none bundled)'}`);
    console.log('');
    return;
  }

  if (projectName === '--version' || projectName === '-v') {
    const pkg = require('../package.json');
    console.log(`create-vibe-assembly v${pkg.version}`);
    return;
  }

  if (!isSafeProjectName(projectName)) {
    console.error('\n❌ Error: Please specify a safe project directory name:');
    console.error('   npx create-vibe-assembly <project-name>');
    console.error('   Allowed: letters, numbers, dots, underscores, hyphens. No path separators.\n');
    process.exit(1);
  }

  const currentPath = process.cwd();
  const projectPath = path.join(currentPath, projectName);
  const pkg = require('../package.json');
  const gitRepo = process.env.VIBE_TEMPLATE_REPO || pkg.repository.url;

  // https(s) for the published repo; file:// also allowed so developers can
  // test template changes locally via VIBE_TEMPLATE_REPO.
  if (!/^(https?|file):\/\//i.test(gitRepo) || /shadcn\/improve/i.test(gitRepo)) {
    console.error('\n❌ Error: Invalid template repository URL.');
    console.error('   Set package.json repository.url or VIBE_TEMPLATE_REPO to an https (or local file://) git URL.\n');
    process.exit(1);
  }

  if (fs.existsSync(projectPath)) {
    console.error(`\n❌ Error: "${projectName}" already exists in this directory.`);
    console.error('   Choose another name or remove/rename the existing folder.\n');
    process.exit(1);
  }

  // Validate the template BEFORE any side effects (clone/rename) so a typo
  // can never leave a half-created project behind.
  if (templateFlagSeen && templateName) {
    const known = loadTemplateCatalog().some((t) => t.name === templateName);
    if (!known) {
      console.error(`\n❌ Unknown template "${templateName}". Available: ${loadTemplateCatalog().map((t) => t.name).join(', ') || '(none)'}`);
      process.exit(1);
    }
  }

  let cloned = false;
  try {
    console.log(`\n🚀 Bootstrapping Vibe Assembly in ${projectPath}...`);

    console.log(`\n📥 Cloning template...`);
    execFileSync('git', ['clone', '--depth', '1', gitRepo, projectName], { stdio: 'inherit' });
    cloned = true;

    process.chdir(projectPath);

    // Order matters: apply the project template FIRST (it overwrites generic
    // .ai context with template-specific seeds that still contain
    // CHANGE_ME_PROJECT_NAME), THEN initMetadata fills those placeholders.
    if (templateName) applyProjectTemplate(templateName, projectPath);

    console.log(`\n📝 Initializing project metadata...`);
    initMetadata(projectName, projectPath);
    resetTemplateState(projectPath);

    console.log(`\n🧹 Cleaning template history...`);
    fs.rmSync(path.join(projectPath, '.git'), { recursive: true, force: true });

    console.log(`\n🌱 Initializing fresh Git repository...`);
    execFileSync('git', ['init'], { stdio: 'inherit' });

    console.log(`\n🛡️ Activating AI architecture guardrails...`);
    execFileSync('git', ['config', 'core.hooksPath', '.githooks'], { stdio: 'inherit' });

    const wantGraphify = await askGraphify();

    if (wantGraphify) {
      console.log(`\n📦 Cloning Graphify...`);
      try {
        execFileSync('git', ['clone', '--depth', '1', 'https://github.com/Graphify-Labs/graphify', path.join(projectPath, 'app', 'graphify')], { stdio: 'inherit' });
        console.log(`\n🔧 Setting up Graphify...`);
        try {
          execFileSync('python', ['--version'], { stdio: 'ignore' });
          try {
            execFileSync('python', ['-m', 'graphify', 'install', '--project'], { cwd: path.join(projectPath, 'app', 'graphify'), stdio: 'inherit' });
          } catch (_) {
            console.log('\n⚠️ Graphify install command failed. You can run it manually later.');
          }
        } catch (_) {
          console.log('\n⚠️ Graphify cloned but Python not found. Run \'python -m graphify install --project\' inside app/graphify/ later.');
        }
      } catch (_) {
        console.log('\n⚠️ Failed to clone Graphify. You can install it later from https://github.com/Graphify-Labs/graphify');
      }
    } else {
      console.log('\n⏭️ Skipping Graphify. You can install it later with:\n   cd app/\n   git clone --depth 1 https://github.com/Graphify-Labs/graphify\n   cd graphify\n   python -m graphify install --project');
    }

    const wantMissionControl = await askMissionControl();
    if (wantMissionControl) {
      setupMissionControl(projectPath);
    } else {
      console.log('\n⏭️ Skipping Mission Control. Launch it later with:\n   cd app/desktop-rust/\n   cargo run');
    }

    console.log(`\n✅ Setup complete! Welcome to Vibe Assembly.`);
    console.log(`\n👉 Next steps:`);
    console.log(`   cd ${projectName}`);
    console.log(`   Open the folder in your IDE, start Roo Code, and let the Boss take over!\n`);
  } catch (error) {
    // Never leave a broken half-created project behind after a failed clone.
    if (cloned && fs.existsSync(projectPath)) {
      console.log('\n🧹 Removing partially created project directory...');
      try { fs.rmSync(projectPath, { recursive: true, force: true }); } catch (_) { /* best effort */ }
    }
    console.error('\n🛑 Installation failed:', error.message);
    process.exit(1);
  }
}

if (require.main === module) {
  main();
} else {
  module.exports = { isSafeProjectName };
}
