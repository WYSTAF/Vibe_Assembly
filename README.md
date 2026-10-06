# 🏗️ Vibe Assembly

> **Zero-Dollar, Infinite-Context Vibe Coding Architecture.**
> Build massive software projects using free API tiers without ever hitting context limits or AI hallucinations.

Vibe Assembly is a highly structured, strict multi-agent workspace template designed for [Roo Code](https://roocode.com/). It converts your editor into an automated software factory, utilizing a **5-Mode Assembly Line** to bypass the memory limits of free AI models.

## 🛑 The Problem: Context Bloat
When coding with LLMs, keeping the entire project history in a single chat quickly exhausts the token limits of free APIs (like OpenRouter, Cloudflare Workers AI, or Gemini Free tier). The AI loses track of the architecture, hallucinates, and breaks the codebase.

## 💡 The Solution: Separation of Concerns
Vibe Assembly strictly isolates AI responsibilities. The AI acts as five different employees who never share the same chat history. They communicate entirely through heavily regulated markdown files inside the `.ai/` directory.

---

## ⚡ Quick Start

Bootstrap a fresh Vibe Assembly workspace in one second:

```bash
npx create-vibe-assembly my-new-project
cd my-new-project
```

Open the folder in your IDE, start Roo Code, and switch to **👑 Boss** for first setup.

---

## 📦 Project Templates (standard starters)

Don't start from an empty `app/`. Bootstrap a working baseline in one command:

```bash
npx create-vibe-assembly my-planner --template planner
npx create-vibe-assembly my-todos   --template todo
npx create-vibe-assembly my-site    --template website
```

| Template | What you get |
|----------|--------------|
| 🗓️ `planner` | Personal planner: goals with localStorage persistence + weekly-view skeleton |
| ✅ `todo` | To-do list: add/complete/delete, All/Open/Done filters, persisted |
| 🌐 `website` | One-page site: sticky nav, hero, content sections — deploy anywhere static |

Every template is **standard and dependency-free**: plain HTML/CSS/JS you can open directly in a browser. Each also ships **pre-filled `.ai/` context** (`project_overview.md` describes what already works and lists natural next features), so 👑 Boss starts warm instead of blank — your first ticket can be *rebranding*, not *scaffolding*.

Prefer manual import? Download this repo from GitHub (or `git clone` it) and copy your chosen folder from `templates/<name>/app/` into your workspace's `app/`, then copy `templates/<name>/.ai/*.md` over the matching files in `.ai/`.

Run `npx create-vibe-assembly --help` to see the always-current template list. Templates are deliberately minimal — 💬 Chat helps you shape the vision, 👑 Boss scopes it, 💻 Code builds it.

---

## 🧭 Modes

| Mode | Role |
|------|------|
| 👑 Boss | Architect + PM. Writes `.ai/active_task.md` blueprints. Never production code. |
| 💬 Chat | Product chatbot. Brainstorm in any language; durable handoffs in English. Tough-love partner. |
| 💻 Code | Implements only what Boss authorized in `active_task.md`. |
| 🐞 Debug | Surgical fixes when tests/builds fail. |
| 📣 Hermes | Docs & Release Herald. READMEs, guides, changelogs, release notes — documentation files only, always from disk evidence. |

---

## 🔭 Scout (advisor skill)

First-party advisor skill at `.agents/skills/scout/`. **Read-only** on application source; writes plans under `plans/` only.

| Command | Behavior |
|---------|----------|
| `scout` | Full audit workflow |
| `scout quick` | High-confidence findings only |
| `scout deep` | Full-repo coverage |
| `scout roadmap` | Direction / next features |
| `scout plan <desc>` | One plan, skip full audit |
| `scout branch` | Diff vs default branch |
| `scout polish <file>` | Tighten an existing plan |
| `scout sync` | Reconcile plan backlog status |
| `scout issues` | Optional GitHub issues (with public-repo safety) |

Implementation path: **plans → Boss → Code** via `.ai/active_task.md`. There is no scout `execute` that edits code.

Default product audit root: `app/`. Use `scout template` to audit agent OS (modes, rules, CLI) instead.

---

## 💬 Chat

- Brainstorm in any language (e.g. Persian).
- Durable artifacts (handoffs, plans, `.ai/`) stay **English**.
- Chat challenges weak or free-tier-hostile ideas once, then supports if you insist.
- When ready to build: copy the English handoff block into **👑 Boss**.

---

## 🪙 Token Usage — Where Your Free Budget Goes

Free tiers die by the token. Vibe Assembly is engineered so the *expensive* thing — model attention — is spent on work, not re-reading your project. Here's the accounting:

### The three token sinks (and how this workspace fights each)

| Sink | What burns tokens | Vibe Assembly's defense |
|------|-------------------|--------------------------|
| **Re-orientation** | Every new session re-learning "what is this project?" | `.ai/` state files are small and structured; a Code session reads **one ticket**, not the repo. `?` SOS or `va` answers "where am I?" without asking a model |
| **Chat bloat** | Long conversations where history must be resent every turn | The 4/5-mode split keeps each chat short-lived by design; the Token Guard Notice tells you exactly when to click **New Session** |
| **Retry storms** | A confused model retrying failed tool calls as context degrades | Anti-chatter laws (`no preambles, just tool calls`) + the XML-compliance fallback rule keep degraded sessions from spiraling |

### Practical budget rules

1. **One ticket = one session.** Tickets are S-sized (~one weak-model session) precisely so a session never needs to survive past its context window.
2. **Boss is the expensive mode** — planning needs depth. Use your best model there (run `va --models`); Chat/Hermes can run on the cheapest ones.
3. **Don't paste whole files into chat.** Boss references paths; Code reads them with tools. Chat history should hold *decisions*, never file contents.
4. **Clear after heavy operations.** When you see the Token Guard Notice, state is already safe on disk — clearing loses nothing.
5. **Check status without tokens.** Mission Control and `va` read disk directly — they cost zero API tokens. Prefer them over asking an agent "what's the status?"
6. **Scout audits are one-time investments.** An audit costs tokens once; its plans save many Code sessions from wandering.

### What things roughly cost (free-tier intuition)

| Operation | Relative cost | Why |
|-----------|---------------|-----|
| `va` / Mission Control glance | 🟢 Zero | Reads disk, no model involved |
| `?` SOS reply | 🟢 Tiny | Two small files read |
| One S-ticket execution | 🟡 Moderate | Bounded by ticket allowlist — no repo sweeps allowed |
| Boss blueprint | 🟠 Heavier | Long-context reasoning; the mode worth spending on |
| Full `scout deep` audit | 🔴 Heaviest | Whole-repo analysis — run rarely, on your strongest model |

> 💡 **Rule of thumb:** if you ever feel the need to ask an agent a yes/no question about workspace state, check Mission Control or `va` first — it's free.

---


## ⚠️ Troubleshooting & Common AI Errors

When using AI agents (especially on free or proxy API tiers), you might encounter specific system errors. Because Vibe Assembly saves state dynamically in the `.ai/` folder, **you will never lose your work.** Here is how to handle them:

### 1. `ResourceExhausted: Worker local total request limit reached`
* **What it means:** You hit the rate limit (too many requests per minute) on your current API provider (e.g., Gemini, OpenAI).
* **How to fix:**
  1. Wait 1-2 minutes for the limit to reset.
  2. OR switch to a different AI model/provider in Roo Code.
  3. Just type: *"Continue relay"* and the agent will read the durable `relay_notes` in `.ai/active_task.md` and resume exactly where it left off.

### 2. `The model provided text/reasoning but did not call any of the required tools.`
* **What it means:** The AI's "Context Window" (memory) is getting too full, causing it to forget how to use terminal commands or edit files.
* **How to fix:**
  1. Click **"New Session"** (Clear the chat history).
  2. Ensure you are in the correct Mode (e.g., Code Mode).
  3. Type: *"Execute active task"* (fresh ticket) or *"Continue relay"* (resume an interrupted one). The agent reloads all state from disk and continues perfectly.

### 3. Agent is stuck in an infinite loop (running the same failing command)
* **What it means:** The `Code Mode` agent is trying to brute-force a fix without stepping back to think, usually because of a complex runtime bug.
* **How to fix:**
  1. Stop the agent manually.
  2. Switch to **🐞 Debug Mode**.
  3. Tell it: *"The last test failed with [paste error]. Investigate the root cause."* Debug mode is specifically prompted to trace backwards and analyze deeply rather than just guessing.

---

## ⌨️ `va` — Status CLI

Terminal mirror of Mission Control. Works over SSH, in scripts, anywhere:

```bash
npm exec --package=create-vibe-assembly va   # one-shot via npx-style runner
# or install once:
npm i -g create-vibe-assembly
va                # queue, active ticket, verified-step count, next paste phrase
va --copy         # copy the exact phrase ("Execute active task" / "Continue relay") to clipboard
va --summary      # Boss-ready markdown wave report (progress table + open work)
va --doctor       # workspace health check (state files, contract, config, combo)
va --models       # allocate your model combo to the five modes
va --root <path>  # inspect another workspace
```

Exit codes: `0` healthy · `1` not a workspace · `2` contract violation (script-friendly).

---

## 🧠 Model Combos & Auto-Allocation

Using an API router (OpenRouter, 9router, OmniRoute, …)? Tell Vibe Assembly which models you have, and it assigns the best one to each mode automatically — based on a bundled model-knowledge base.

```bash
# 1. List your router models, one per line, in .ai/model_combo.txt:
#      gemini-2.0-flash
#      deepseek-r1:free
#      qwen3-coder
#    Optional per-mode locks:
#      boss = gemini-2.0-flash

va --models
```

```
 🧠 MODEL ALLOCATION  ·  combo of 5

 💬 chat    → gemini-2.0-flash  [fast]
 💻 code    → qwen3-coder  [coder]
 🐞 debug   → deepseek-r1:free  [reasoner]
 📣 hermes  → llama-4-scout  [workhorse]
 👑 boss    → gpt-5-mini  [fast]
 ⚠ boss: no flagship-class model in your combo — using gpt-5-mini as a fallback.
```

**How it decides:** the knowledge base ([`.ai/model_knowledge.json`](.ai/model_knowledge.json)) classifies every model into capability classes — `reasoner`, `coder`, `flagship`, `workhorse`, `fast` — and each mode declares what it needs *primary* plus fallbacks. Allocation is two-pass: scarce classes go to the mode whose primary need they serve (a lone reasoner lands on **Debug**, not Boss), user locks always win, and degenerate combos degrade gracefully with honest warnings instead of pretending. Unknown model names fall back to name-shape heuristics (`-r1` → reasoner, `-coder` → coder, `flash/mini/nano` → fast).

Refresh guidance for benchmark sources lives inside the knowledge file itself — it's data, so updating it never touches code.

---

## 🖥️ Mission Control (desktop app)

Vibe Assembly ships with its own native desktop dashboard at `app/desktop-rust/`: a live operations-room view of your workspace built with GPUI.

- **Next Action** panel with one-click copy of the exact paste phrase (`Execute active task` / `Continue relay`)
- Live **Ticket Queue** + Active Ticket (allowlist, verification steps, `relay_notes`)
- **Contract Violation** alerts when `.ai/active_task.md` state is inconsistent (fail-closed, same rules Code enforces)
- Operational state, known bugs, ADR decisions, and progress
- Native Rust binary with zero webview/electron overhead

```bash
cd app/desktop-rust
cargo run
```

The bootstrap CLI offers to install it automatically. Point it at any workspace with `node start.js --root <path>`.

---

## 📊 Graphify (Optional)

[Graphify](https://github.com/Graphify-Labs/graphify) is an optional visual codebase analysis tool that maps your project architecture as an interactive graph.

### Install during bootstrap

When you run `npx create-vibe-assembly my-project`, the CLI will ask:

```
Would you like to install Graphify for visual codebase analysis? (y/N):
```

Type `y` to clone Graphify into `app/graphify/` and run its setup automatically.

### Install later

If you declined during bootstrap, you can add Graphify at any time:

```bash
cd app/
git clone --depth 1 https://github.com/Graphify-Labs/graphify.git
cd graphify
python -m graphify install --project
```

### How Vibe Assembly uses Graphify

When Graphify is present in `app/graphify/`, Vibe Assembly's agent modes can leverage it for codebase visualization, dependency mapping, and architectural analysis. The Scout skill can reference Graphify graphs for deeper audits.

---

## License

MIT
