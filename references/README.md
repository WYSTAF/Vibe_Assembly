# References

Upstream projects kept for study. **Nothing here is built or linked** — this
directory exists so the architecture decisions in `.ai/decisions.md` can be
traced back to their sources.

Contents are gitignored (`references/upstream/`); only this README is tracked.

## Why this directory exists

Vibe Assembly borrows specific ideas rather than depending on code. Keeping the
sources visible makes it possible to re-check an assumption later without
re-cloning.

| Idea | Source | Where it landed |
|---|---|---|
| GPUI rendering, windowing, and text stack | `zed-industries/zed` | `crates/va-desktop` |
| Multi-agent mode separation | Roo Code mode spec | `crates/va-core/src/prompts.rs` |
| Loop-prevention attempt caps | Rune's loop-prevention rules | `MASTER_CONSTITUTION` in `prompts.rs` |
| Model capability classes | Community free-tier reports | `.ai/model_knowledge.json` |
| OpenAI-compatible transport | 9router | `crates/va-core/src/router.rs` |

## Adding a reference

Clone into `references/upstream/<name>/`. It is ignored by git, so it will never
reach the published workspace:

```bash
cd references
mkdir -p upstream
git clone --depth 1 <url> upstream/<name>
```

## Note on Zed

Zed is referenced for its **UI framework only**. Zed's extension API cannot host
a custom dashboard — extensions are Rust→Wasm with six fixed capabilities and no
custom-UI hook. That is why this project depends on published `gpui` rather than
forking the editor. See ADR-013.
