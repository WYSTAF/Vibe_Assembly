# Dynamic System Architecture

## Topology

Vibe Assembly is a single native Rust workspace. The CLI and desktop are two
front ends over the same `.ai/` contract, parser, and harness. There is no
sidecar service and no browser/webview layer.

```text
                    +---------------------------+
                    |  Vibe Router / 9router    |
                    |  POST /v1/chat/completions|
                    +-------------+-------------+
                                  |
                    raw HTTP/1.1  |  Vibe-Link headers + telemetry
                                  |
+------------------+   +----------v-----------+   +---------------------+
| va-cli           |   | va-harness           |   | va-desktop          |
| prompt / chat    |-->| live_planner         |<--| GPUI Zed workbench  |
| run / tui / mcp  |   | engine / session     |   | block hashes + DAG  |
+--------+---------+   | patcher / judge      |   | editor + assistant  |
         |             | repomap / diagnostics|   +---------------------+
         |             | experience/tournament|
         |             +----------+-----------+
         |                        |
         +-----------+------------+
                     |
             +-------v--------+
             | va-core        |
             | parser/state   |
             | allocator      |
             | router client  |
             | runtime/lock   |
             | modes/lens     |
             +-------+--------+
                     |
                 .ai/ contract
          active_task.md, session.jsonl,
          EXPERIENCE.md, decisions.md
```

## Execution Modes

| Mode | Accent | Writes | Purpose |
|---|---|---|---|
| Code | `#F5B800` | yes | Ticket implementation through Boss -> Code -> Verify -> Judge |
| Web | `#06B6D4` | yes | HTML/CSS/Tailwind/TypeScript/React and `templates/` |
| Chat | `#3B82F6` | no | Read-only architecture, explanation, and planning |
| Debug | `#EF4444` | yes | Compiler diagnostics, root-cause analysis, security audit |

The active accent is carried through the CLI header, TUI panels, and the
desktop header, selectors, context strip, and active panels.

## Language Lens

`LanguageLens` is `Auto`, `Rust`, `C/C++`, `Python`, `TS/JS`, or `Go`.
`Auto` resolves from manifests (`Cargo.toml`, `go.mod`, `pyproject.toml`,
`package.json`, `CMakeLists.txt`, ...) and then a bounded extension census.
Each lens supplies a verification command and language-specific judge
guardrails; the engine also enforces the mode guardrails.

## Execution Loop

1. **Observe** - scan the repository into a token-budgeted `RepoMap`.
2. **Think (Boss)** - choose files and exact block hashes, plus a verify command.
3. **Patch (Code)** - receive complete block replacements addressed by short
   block hash; apply through the patcher with whole-file drift assertions.
4. **Verify** - run the language-lens command in the sandbox.
5. **Debug** - on failure, pin compiler diagnostics to block hashes, re-resolve
   the edited block's fresh hash by signature, and send evidence to the Debug role.
6. **Judge** - deterministic anti-slop gate, independent of the coding model.
7. **Commit** - record the verified trajectory node and refresh the working set.

## Context Management

- Auto-compaction triggers at 75% of the active model's context window.
- Older verified steps and failed retries collapse into surface summaries while
  the immutable DAG is preserved for `replay()` and `rollback_to_last_verified()`.
- `think`/reasoning traces are evicted from rendered prompts but retained raw in
  DAG nodes.
- Observations over 80 lines or 4,000 bytes render as head 30 + elision marker +
  tail 30; the full log stays in the DAG.
- The prompt always ends with a pinned working-set tail carrying fresh block
  hashes and line ranges for every modified file.

## Safety Invariants

- Chat mode is a hard read-only boundary enforced by the engine, not a prompt hint.
- Two identical `(file, block hash, failure signature)` failures trigger
  `StagnationDetected`; the policy can pivot or escalate to `SpeculativeTournament`.
- A block body that appears more than once after normalization is rejected with
  `PatchError::AmbiguousRepeatedAnchor`, even during 1-hop recovery.
- Live execution never fabricates edits; deterministic synthesis is available
  only through tests or the explicit `--offline-mock` flag.

## `.ai/` Contract

`active_task.md` is the execution authority: one active ticket, an allowlist,
verification-gated steps, and append-only relay notes. `ticket_contract.md` is
the schema. `EXPERIENCE.md` stores durable, evidence-backed recovery lessons.
`session.jsonl` is the transactional trajectory DAG.
