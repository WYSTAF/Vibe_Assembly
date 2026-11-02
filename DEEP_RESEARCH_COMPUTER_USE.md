# Deep Research: Computer Use and Thinking Modes

Date: 2026-11-02
Scope: a native Windows desktop coding agent (Vibe Assembly, GPUI 0.2.2) planning a
computer-use mode with a large yellow agent cursor and trail, plus a Codex-style
thinking selector for models.
Status: standard depth (two parallel retrieval agents, official docs first).

## Assumptions

The session brief fixed the product context, so Phase 0 clarification was answered
from context rather than by pausing:

- Platform is Windows 10+ first, with macOS as a later target.
- The agent drives the real desktop, like Qoder on Windows, not a hosted VM.
- The yellow cursor and trail are a deliberate differentiator, not a copy of a
  shipped competitor visual.
- Thinking modes ship as a UI contract now; provider request wiring comes when the
  composer actually sends requests.

## Methodology and confidence

Each claim below carries a confidence tag:

- [High]: at least one Tier 1 source (official docs, platform reference) and
  independent corroboration.
- [Medium]: one official source, or several credible sources without a first-party
  contract.
- [Low]: single secondary source or inference.

Two retrieval agents covered the topics in parallel: one on computer-use products
and Windows overlay mechanics, one on reasoning-effort controls in Codex, Claude
Code, Cursor and the OpenAI-compatible API. Sources were fetched in November 2026.
Vendor UIs change quickly; labels in particular should be re-checked before copying.

## Part 1: What computer-use products actually do

### Qoder

[High] Qoder documents four capabilities under Settings > Computer use: AppShot
(active-window screenshot plus accessibility text in chat), a Chromium browser
connector, desktop Computer Use, and Record & Replay. AppShot and Computer Use
support macOS 14+ and Windows 10+.

[High] Qoder's Computer Use Agent takes screenshots throughout a run to confirm the
page loaded and the previous action took effect, and streams screenshots and
progress into the session in real time. On macOS operations run in the background;
on Windows they run in the foreground, "so you can see the cursor move and each
action take place." Esc interrupts at any time.

[High] Approval is per application with three modes: Ask every time (default),
Auto-run, and Disabled. No action-chip or takeover overlay is documented; the
visible feedback is the real cursor plus the live screenshot stream.

Design lesson: the strongest precedent is not a drawn overlay at all. It is a live
screenshot and progress stream in the transcript, paired with an honest
foreground/background distinction and a global interrupt.

### OpenAI Operator and Computer-Using Agent

[High] CUA operates over raw pixel screenshots with a virtual mouse and keyboard.
Operator shows that virtual cursor inside a hosted browser. OpenAI seeks user
confirmation for sensitive actions, requires supervision on sensitive sites, and
pauses when a monitor detects suspicious screen content.

[High] Operator's takeover mode asks the user to type credentials or payment data
directly, and during takeover it does not collect or screenshot what the user
enters. Before significant actions such as submitting an order or sending an
email, it asks for approval.

[High] The API guide models actions as an ordered array. A click carries button,
x and y; the action set includes click, double_click, drag, move, scroll, keypress,
type, wait and screenshot. The guide recommends an isolated browser or VM, an
allowlist of sites and actions, confirmation for consequential actions, step,
time and cost limits, and cancellation. The demos use before_action and confirm
callbacks that only permit specific controls, with UI to watch, take control,
return control, or stop.

[Medium] No official OpenAI source documents a drawn cursor trail. The visible
cursor in Operator is the real one inside its hosted browser.

### Anthropic computer use tool

[High] One computer toolset entry exposes member tools including screenshot,
left_click, type, zoom and cursor_position. Coordinates are in the pixel space of
full-display screenshots, origin top-left. Downscaled screenshots must be scaled
back up. Batched actions run in order and stop at the first failure.

[High] Safety guidance: isolated VM or container, internet restricted to an
allowlist, human confirmation for consequential actions, and because a batch can
complete a multistep action in one turn, any human check must happen before each
block runs. The docs warn that Claude follows commands found in content even when
they conflict with instructions, and that classifiers flag injections.

### Open-source desktop agents

[High] ByteDance UI-TARS Desktop is a native GUI agent with local and remote
computer plus browser operators, cross-platform Windows/macOS/browser support,
screenshot and visual recognition, and real-time feedback and status display.
Its README does not document a drawn cursor, a trail, human-in-the-loop approval,
or a kill switch. The visible motion is the real OS cursor being driven.

[Medium] A Tauri feature request for transparent click-through windows and a Wails
issue about per-pixel alpha with click-through confirm that overlays are built
explicitly, not inherited from the UI framework. Electron's setIgnoreMouseEvents
is the de-facto recipe other agents use: transparent, frameless, always-on-top,
and ignoring input.

[Medium] Zed's GPUI has an open request for window background transparency
(issue #5040). Do not assume per-pixel-alpha overlay support exists in GPUI's
Windows backend.

Bottom line: shipped products show real cursors plus screenshot streams. A drawn
yellow cursor with a trail is our own layer, and the research supports it as a
differentiator, not a copied feature.

## Part 2: Yellow cursor and trail specification

The following recipe combines Qoder's foreground visibility, Operator's takeover
safety, and Windows overlay mechanics. Confidence is [Medium] for the visual
values (design inference) and [High] for the platform mechanics.

### Surfaces

1. A per-monitor full-screen overlay window: topmost, click-through,
   non-activating, hidden from Alt-Tab. It never takes focus.
2. The in-app transcript: action chips and screenshot thumbnails, matching
   Qoder's live stream.
3. Optional action pill anchored above the cursor: "Click Save", "Type 4 chars".

### Motion

- Draw the agent cursor at 48 to 64 DIP with the hotspot at the tip, scaled by the
  monitor DPI factor. Keep the native cursor visible so users can tell agent from
  human.
- Never teleport. Animate along an eased path for roughly 250 to 350 ms between
  consecutive coordinates, which masks model latency and reads as intent.
- Before a click, pulse a soft yellow ring at the target for about 200 ms, then
  squash briefly on click. Park the cursor on the field during typing.
- Trail: keep the last 12 to 20 sampled positions as a fading ribbon, alpha decay
  around 180 ms with width taper, GPU-instanced, capped at 60 fps. Disable it
  under reduced motion and hide it after about 1 second idle.

### Coordinates and verification

- Treat model coordinates as untrusted. Convert once using the same scale factor
  used to capture the screenshot.
- Re-screenshot after each action and draw a correction if the target moved.

## Part 3: Windows implementation constraints

[High] A transparent, always-on-top, click-through, non-activating layered window
is the standard Windows approach:

- WS_POPUP base window, plus WS_EX_LAYERED (0x00080000).
- WS_EX_TRANSPARENT (0x00000020) makes the layered overlay ignored by the mouse.
- WS_EX_TOPMOST (0x00000008) with SetWindowPos(HWND_TOPMOST, SWP_NOACTIVATE).
- WS_EX_NOACTIVATE (0x08000000) prevents foreground activation and taskbar entry.
- WS_EX_TOOLWINDOW hides it from Alt-Tab.
- Alpha via UpdateLayeredWindow, or a premultiplied-alpha DComp/wgpu surface.

[High] Per Monitor v2 DPI awareness adjusts scale whenever DPI changes and is not
auto-scaled by the system. Practical consequence: one overlay per monitor, or
reposition on WM_DPICHANGED, and convert model coordinates with the same scale as
the screenshot.

[Medium] GPUI on Windows is not a safe bet for a transparent overlay today (open
issue). Budget a separate Win32 layered-window overlay, driven by wgpu or
DirectComposition, and keep GPUI for the app and transcript UI.

[Low] Cost of the overlay is dominated by per-frame alpha composition; the trail
should be a single instanced draw and idle at zero frames when hidden.

## Part 4: Safety checklist

[High] Patterns supported by Qoder, OpenAI and Anthropic docs:

1. Pre-flight plan preview: which apps and windows are in scope, what "done" means.
2. Default mode "Ask every time" per application. Offer Auto-run only per app.
3. Policy tiers: read-only actions can auto-run; mutating actions (submit, send,
   purchase, delete, install, permission change, terminal) require confirmation
   before the action fires. Confirmation must gate each batch block.
4. Allowlist apps, sites and domains; deny unknown windows.
5. Block credential and payment fields entirely; hand over control instead.
6. Global kill switch: Esc and a hotkey that aborts before the next action and
   releases input hooks. Pause on suspicious screen content.
7. No screenshots while the user types in takeover; scrub tokens from context.
8. Bounded execution: step, time and cost limits, with cancellation.
9. Verify by re-screenshot rather than trusting the model's claim.
10. Audit log: timestamp, coordinates, target window, before/after thumbnail,
    approver.
11. Prompt-injection posture: screen content can never grant permission or widen
    scope.

## Part 5: Thinking modes

### What competitors ship

[High] OpenAI's reasoning guide lists none, minimal, low, medium, high, xhigh and
max, with medium as the default on GPT-5.5/5.6/6.x. Azure's table maps none to
latency-critical work, low to tool use and planning, medium to a balanced start,
high to hard reasoning, xhigh to deep research and long agent runs, and max to the
most complex tasks. Azure also separates reasoning mode (for example "pro") from
effort: mode and effort are independent controls.

[High] Codex exposes a compact reasoning control beneath the composer. The CLI uses
/model to switch model or adjust effort. The changelog describes "a compact
composer gauge for viewing and adjusting reasoning effort." Desktop and IDE
surfaces have used labels Light, Medium, High and Extra High, plus Max and Ultra;
Ultra coordinates subagents and is not another effort value. Effort is persisted
as model_reasoning_effort in config.toml.

[High] Claude Code now uses an effort enum: low, medium, high, xhigh, plus max
under a separate cap. It exposes /effort, --effort, CLAUDE_CODE_EFFORT_LEVEL, and
an effort slider inside the model picker. Alt+T toggles thinking for the session.
On always-thinking models the row reads "Thinking can't be turned off." Only
ultrathink remains a mechanical keyword; plain "think" passes through as text.
Per-model settings win over the global level, and maxEffortLevel caps them; the
lowest cap applies.

[Medium] Cursor moved from flat variant rows (Opus 4.7 Low, Medium, High, Max) to
one model row with a sub-panel for reasoning level and context size. The flat list
exploded; nesting variants under the model was the fix.

[High] OpenRouter's auto router is a single model id that classifies the prompt.
Established UX: one pinned Auto row at the top, visually distinct, no per-token
price, "routes per request" subtitle, then a divider and direct models.

[High] In the OpenAI-compatible Chat Completions API the field is reasoning_effort
and the widely supported set is minimal, low, medium and high, default medium.
Newer models add none, xhigh and max. One documented trap: on some models, tools
plus reasoning_effort is rejected on chat completions even when the parameter is
omitted, because medium is the default.

### What Vibe Assembly now ships

Implemented in `crates/va-desktop/src/design/glass.rs`:

- Auto, Fast, Balanced, Deep and Max, mapped to auto, low, medium, high and xhigh.
- The ladder lives inside the model menu, above the gateway status footer, and the
  selected effort is shown on the model pill as "Model · Deep".
- Auto is the honest default because the app does not yet send provider requests.
- Per-model persistence and capability coercion are deliberate follow-ups, not
  hidden behavior: the UI never claims a level is active for a model that rejects
  it.

Recommended next step when requests are wired: store per-model effort, coerce down
on unsupported models, show the coerced value, and add a per-turn override keyword
rather than persisting magic words.

### Model catalogue

Implemented alongside thinking modes: the app fetches GET /v1/models from the
configured gateway, lists models whose ids start with va_ as router combos first,
strips the prefix for display, and keeps direct models after them. When the
gateway is offline the menu says so and shows the built-in list. OpenRouter's
single Auto entry is a useful reference, but the brief explicitly requires combos
to be prioritized over specific models, so combos get their own group.

## Part 6: Suggested implementation phases for computer use

1. Overlay spike: a Win32 layered window that draws one static yellow cursor over
   the app, click-through and invisible to Alt-Tab. Prove mixed-DPI placement
   before any animation work.
2. Motion and trail: eased movement, pre-click ring, fading trail, reduced-motion
   and idle behavior. Keep it a single instanced draw.
3. Action protocol: ordered action array with coordinates in screenshot pixels,
   scale conversion, and post-action screenshots for verification.
4. Approval and safety: per-app Ask every time default, mutating-action
   confirmation, takeover for secrets, Esc kill switch, audit log.
5. Transcript stream: screenshot thumbnails and action chips, following Qoder's
   live progress model.

Open risk to de-risk first: GPUI transparency (Zed issue #5040) and per-pixel
alpha with click-through (Wails #6088). If the GPUI route fails, the Win32 overlay
is the fallback and the recommended default.

## Sources

All accessed November 2026.

1. Qoder, "Computer Use" - https://docs.qoder.com/qoder/computer-use - Tier 1.
2. Qoder, "Computer Use Agent" - https://docs.qoder.com/user-guide/chat/computer-use-agent - Tier 1.
3. OpenAI, "Computer-Using Agent" - https://openai.com/index/computer-using-agent/ - Tier 1.
4. OpenAI, "Introducing Operator" - https://openai.com/index/introducing-operator/ - Tier 1.
5. OpenAI, "Computer use" API guide - https://developers.openai.com/api/docs/guides/tools-computer-use - Tier 1.
6. Anthropic, "Computer use tool" - https://platform.claude.com/docs/en/agents-and-tools/tool-use/computer-use-tool - Tier 1.
7. ByteDance, "UI-TARS Desktop" - https://github.com/bytedance/ui-tars-desktop - Tier 1.
8. Tauri, "Transparent Window Support Click-Through (#13070)" - https://github.com/tauri-apps/tauri/issues/13070 - Tier 2.
9. Wails, "per-pixel transparency + click-through (#6088)" - https://github.com/wailsapp/wails/issues/6088 - Tier 2.
10. Electron, "BrowserWindow" - https://electronjs.org/docs/latest/api/browser-window - Tier 1.
11. Zed, "GPUI window transparency (#5040)" - https://github.com/zed-industries/zed/issues/5040 - Tier 2.
12. Microsoft Learn, "Extended Window Styles" - https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles - Tier 1.
13. Microsoft Learn, "DPI Awareness Context" - https://learn.microsoft.com/en-us/windows/win32/hidpi/dpi-awareness-context - Tier 1.
14. OpenAI, "Reasoning" API guide - https://developers.openai.com/api/docs/guides/reasoning - Tier 1.
15. Azure OpenAI, "Reasoning models" - https://learn.microsoft.com/en-us/azure/foundry/openai/how-to/reasoning - Tier 1.
16. OpenAI, "Codex models" - https://learn.chatgpt.com/docs/models - Tier 1.
17. OpenAI, "ChatGPT and Codex changelog" - https://learn.chatgpt.com/docs/changelog - Tier 1.
18. openai/codex issue #2760 - https://github.com/openai/codex/issues/2760 - Tier 2.
19. Anthropic, "Claude Code model config" - https://code.claude.com/docs/en/model-config - Tier 1.
20. Anthropic, "Claude Code settings reference" - https://code.claude.com/docs/en/settings-reference - Tier 1.
21. Cursor forum, "thinking-effort variants gone from model picker" - https://forum.cursor.com/t/opus-4-7-thinking-effort-variants-low-medium-high-max-gone-from-the-chat-agent-model-picker-on-cursor-stable-v3-3-27-windows-ui-regression-vs-v3-0-12-linux/160112 - Tier 3.
22. OpenRouter release notes - https://releasebot.io/updates/openrouter - Tier 3.
23. Onyx changelog - https://docs.onyx.app/changelog - Tier 3.
24. Zencoder changelog - https://docs.zencoder.ai/changelog/home - Tier 3.
25. kingy.ai, "Codex reasoning levels" - https://kingy.ai/news/openai-codex-reasoning-levels-low-medium-high-extra-high/ - Tier 3.

## Caveats

- Vendor labels drift between surfaces (Codex desktop says Light while the CLI
  says Low). The implementation uses neutral names that do not collide with any
  one vendor.
- The trail is a design inference; no shipped product documents one.
- Windows overlay mechanics are well documented, but GPUI's transparency support
  on Windows must be spiked before committing to an in-app overlay.
- This report is standard depth. A deep run would add hands-on prototypes and
  latency measurements.
