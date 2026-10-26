# Native workbench UI kit

Implemented directly against the locked GPUI 0.2.2, without additional crates,
runtime downloads or webviews. The supplied brand SVG is embedded in the app.

Research completed before implementation:

- [gpui-component v0.5.1](https://github.com/longbridge/gpui-kit/blob/v0.5.1/Cargo.toml)
  uses GPUI 0.2.2. The full component crate also brings editor, localization,
  tree, and macro infrastructure, so it is compatible but not a small helper.
- The old repository now redirects to gpui-kit. Its
  [current workspace](https://github.com/longbridge/gpui-kit/blob/main/Cargo.toml)
  pins gpui-pre 0.3.10, a different API and type universe.
- [Button variants](https://github.com/longbridge/gpui-kit/blob/v0.5.1/crates/ui/src/button/button.rs),
  [inputs](https://github.com/longbridge/gpui-kit/blob/v0.5.1/crates/ui/src/input/input.rs),
  [tree rows](https://github.com/longbridge/gpui-kit/blob/v0.5.1/crates/ui/src/tree.rs),
  [split panes](https://github.com/longbridge/gpui-kit/blob/v0.5.1/crates/ui/src/resizable/mod.rs),
  [tabs](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/tab/tab.rs),
  [badges](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/badge.rs), and
  [theme tokens](https://github.com/longbridge/gpui-kit/blob/main/crates/component/src/theme/mod.rs)
  informed composable controls, explicit hover/press/focus states, and selected pills.
- Zed's [DynamicSpacing](https://github.com/zed-industries/zed/blob/main/crates/ui/src/styles/spacing.rs),
  [elevation](https://github.com/zed-industries/zed/blob/main/crates/ui/src/styles/elevation.rs), and
  [theme colors](https://github.com/zed-industries/zed/blob/main/crates/theme/src/styles/colors.rs)
  informed consistent spacing and separate editor, panel, hover, and focus surfaces.
  This workbench uses one comfortable density without importing Zed's globals or macros.
- `crates/assistant2` no longer exists on Zed main. The current
  [agent conversation view](https://github.com/zed-industries/zed/blob/main/crates/agent_ui/src/conversation_view/thread_view.rs)
  keeps explicit expansion state for tool details. Our phase cards use that
  progressive disclosure pattern: one summary row, expandable detail.

These are independently implemented design patterns, not copied source.
Callers retain state transitions and va-core / va-harness contracts. The kit
owns shared geometry, typography and semantic colors. Badges use 12% fill
and 28% border opacity. Installed proportional and code fonts are resolved
separately at startup.
