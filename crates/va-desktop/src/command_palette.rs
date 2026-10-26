//! Floating Zed command palette. Every row is a real state transition.

use gpui::prelude::*;
use gpui::{div, px, rgba, Context, Rgba};
use va_core::modes::{ExecutionMode, LanguageLens};

use crate::state::{PaletteCommand, Workbench};
use crate::theme;
use crate::widgets;

pub fn render(
    workbench: &Workbench,
    accent: Rgba,
    accent_strong: Rgba,
    cx: &mut Context<Workbench>,
) -> impl gpui::IntoElement {
    let state = &workbench.state;
    let commands = all_commands();
    let mut list = div().flex().flex_col().py_1();
    let mut current_group = "";

    for (index, command) in commands.into_iter().enumerate() {
        if command.group() != current_group {
            current_group = command.group();
            list = list.child(
                div()
                    .px_2()
                    .pt_1()
                    .pb(px(2.0))
                    .text_size(px(8.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme::text_muted())
                    .child(current_group.to_uppercase()),
            );
        }

        let active = command.is_active(state);
        let shortcut = command.shortcut();
        list = list.child(
            div()
                .id(("palette-row", index))
                .h(px(28.0))
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .bg(if active {
                    theme::wash(accent, 0.09)
                } else {
                    theme::wash(theme::elevated(), 1.0)
                })
                .cursor_pointer()
                .hover(|style| style.bg(theme::elevated_alt()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.state.execute_palette(command);
                    cx.notify();
                }))
                .child(
                    div()
                        .w(px(12.0))
                        .text_size(px(10.0))
                        .text_color(if active { accent } else { theme::text_muted() })
                        .child(if active { "✓" } else { "·" }),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(10.5))
                        .text_color(if active {
                            theme::text_primary()
                        } else {
                            theme::text_secondary()
                        })
                        .child(command.label()),
                )
                .when(!shortcut.is_empty(), |this| {
                    this.child(widgets::keycap(shortcut))
                }),
        );
    }

    let header = div()
        .h(px(40.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .border_b_1()
        .border_color(theme::border())
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(accent)
                .child(">"),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(1.0))
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme::text_primary())
                        .child("Command Palette"),
                )
                .child(
                    div()
                        .text_size(px(8.0))
                        .text_color(theme::text_muted())
                        .child("Quick actions · one click to execute"),
                ),
        )
        .child(div().flex_1())
        .child(widgets::keycap("Ctrl+P"))
        .child(
            div()
                .id("palette-close")
                .flex()
                .items_center()
                .justify_center()
                .size(px(20.0))
                .rounded(px(2.0))
                .text_size(px(12.0))
                .text_color(theme::text_muted())
                .cursor_pointer()
                .hover(|style| {
                    style
                        .bg(theme::wash(theme::danger(), 0.18))
                        .text_color(theme::danger())
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.state.show_palette = false;
                    cx.notify();
                }))
                .child("×"),
        );

    div()
        .id("palette-overlay")
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .flex()
        .justify_center()
        .pt(px(80.0))
        .bg(rgba(0x000000B0))
        .on_click(cx.listener(|this, _, _, cx| {
            this.state.show_palette = false;
            cx.notify();
        }))
        .child(
            div()
                .id("palette-panel")
                .w(px(600.0))
                .max_h(px(680.0))
                .flex()
                .flex_col()
                .rounded(px(6.0))
                .bg(theme::elevated())
                .border_1()
                .border_color(accent_strong)
                .on_click(cx.listener(|_this, _, _, cx| {
                    cx.stop_propagation();
                }))
                .child(header)
                .child(
                    div()
                        .id("palette-scroll")
                        .flex_1()
                        .min_h_0()
                        .overflow_scroll()
                        .child(list),
                ),
        )
}

fn all_commands() -> Vec<PaletteCommand> {
    let mut commands = Vec::new();
    for mode in ExecutionMode::ALL {
        commands.push(PaletteCommand::Mode(mode));
    }
    for language in LanguageLens::ALL {
        commands.push(PaletteCommand::Language(language));
    }
    commands.extend([
        PaletteCommand::Compact,
        PaletteCommand::Tournament,
        PaletteCommand::Rollback,
        PaletteCommand::ToggleLeft,
        PaletteCommand::ToggleBottom,
        PaletteCommand::ToggleAssistant,
        PaletteCommand::RefreshRepoMap,
        PaletteCommand::CloseTab,
    ]);
    commands
}
