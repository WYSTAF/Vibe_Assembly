//! Right assistant dock: thread stream, collapsible execution rows, and the
//! interactive prompt composer with mode / language / model pills.

use gpui::prelude::*;
use gpui::{div, px, Context, Div, Rgba};

use crate::state::{ThreadItem, Workbench};
use crate::theme;
use crate::widgets;

pub fn render(
    workbench: &Workbench,
    accent: Rgba,
    accent_strong: Rgba,
    composer_focused: bool,
    cx: &mut Context<Workbench>,
) -> Div {
    let state = &workbench.state;
    let context_color = if state.context_pct >= 75 {
        theme::warning()
    } else {
        accent
    };

    let header = div()
        .h(px(32.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .bg(theme::sunken())
        .border_b_1()
        .border_color(theme::border())
        .child(widgets::section_label("Assistant"))
        .child(div().flex_1())
        .child(
            div()
                .id("assistant-context")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.state.compact_context();
                    cx.notify();
                }))
                .child(widgets::pill(
                    state.context_label(),
                    state.context_pct >= 75,
                    context_color,
                )),
        )
        .child(
            div()
                .id("assistant-new-thread")
                .flex()
                .items_center()
                .justify_center()
                .px_1()
                .py(px(1.0))
                .rounded(px(2.0))
                .text_size(px(12.0))
                .text_color(theme::text_muted())
                .cursor_pointer()
                .hover(|style| {
                    style
                        .bg(theme::elevated())
                        .text_color(theme::text_primary())
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.state.clear_thread();
                    cx.notify();
                }))
                .child("＋"),
        );

    let meter = div()
        .flex_shrink_0()
        .px_2()
        .pb_1()
        .bg(theme::sunken())
        .child(widgets::meter(state.context_pct, context_color));

    let mut stream = div()
        .id("assistant-scroll")
        .flex_1()
        .min_h_0()
        .overflow_scroll()
        .flex()
        .flex_col()
        .gap_2()
        .p_2();

    if state.thread.is_empty() {
        stream = stream.child(
            div()
                .p_2()
                .text_size(px(10.0))
                .text_color(theme::text_muted())
                .child("No thread items yet."),
        );
    }

    for (index, item) in state.thread.iter().enumerate() {
        match item {
            ThreadItem::User { text } => {
                stream = stream.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .p_2()
                        .rounded(px(3.0))
                        .bg(theme::elevated())
                        .border_l_2()
                        .border_color(accent)
                        .child(
                            div()
                                .text_size(px(8.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(accent)
                                .child("YOU"),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .line_height(px(16.0))
                                .text_color(theme::text_primary())
                                .child(text.clone()),
                        ),
                );
            }
            ThreadItem::Assistant { text, read_only } => {
                let color = if *read_only { theme::info() } else { accent };
                let label = if *read_only {
                    "ASSISTANT · READ-ONLY"
                } else {
                    "ASSISTANT"
                };
                stream = stream.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .pl_2()
                        .border_l_2()
                        .border_color(color)
                        .child(
                            div()
                                .text_size(px(8.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(color)
                                .child(label),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .line_height(px(16.0))
                                .text_color(theme::text_secondary())
                                .child(text.clone()),
                        ),
                );
            }
            ThreadItem::System { text } => {
                stream = stream.child(
                    div()
                        .p_2()
                        .rounded(px(3.0))
                        .bg(theme::elevated())
                        .border_1()
                        .border_color(theme::border())
                        .text_size(px(9.0))
                        .line_height(px(14.0))
                        .text_color(theme::text_muted())
                        .child(text.clone()),
                );
            }
            ThreadItem::Step(step) => {
                let phase_color = theme::phase_color(step.phase.as_str());
                let status_color = if step.status == "off" {
                    theme::danger()
                } else if step.status == "gated" {
                    theme::warning()
                } else if step.status == "armed" || step.status == "ready" {
                    theme::success()
                } else {
                    theme::text_muted()
                };
                let chevron = if step.expanded { "▾" } else { "▸" };
                let mut row = div()
                    .flex()
                    .flex_col()
                    .rounded(px(3.0))
                    .border_1()
                    .border_color(theme::border())
                    .bg(theme::elevated())
                    .child(
                        div()
                            .id(("assistant-step", index))
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_1()
                            .py(px(2.0))
                            .cursor_pointer()
                            .hover(|style| style.bg(theme::elevated_alt()))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(ThreadItem::Step(step)) =
                                    this.state.thread.get_mut(index)
                                {
                                    step.expanded = !step.expanded;
                                }
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .w(px(10.0))
                                    .text_size(px(9.0))
                                    .text_color(theme::text_muted())
                                    .child(chevron),
                            )
                            .child(widgets::pill(step.phase.label(), true, phase_color))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(9.0))
                                    .text_color(theme::text_secondary())
                                    .child(step.title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(8.0))
                                    .font_family("Consolas")
                                    .text_color(theme::text_muted())
                                    .child(format!("{}t", step.tokens)),
                            )
                            .child(
                                div()
                                    .text_size(px(8.0))
                                    .font_family("Consolas")
                                    .text_color(theme::text_muted())
                                    .child(format!("{}ms", step.latency_ms)),
                            )
                            .child(
                                div()
                                    .text_size(px(8.0))
                                    .font_family("Consolas")
                                    .text_color(theme::text_muted())
                                    .child(format!(
                                        "${:.6}",
                                        step.cost_micros as f64 / 1_000_000.0
                                    )),
                            )
                            .child(widgets::dot(status_color)),
                    );

                if step.expanded {
                    row = row
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .text_size(px(9.0))
                                .line_height(px(14.0))
                                .text_color(theme::text_muted())
                                .child(step.detail.clone()),
                        )
                        .when(step.think_evicted, |this| {
                            this.child(
                                div()
                                    .px_2()
                                    .pb_1()
                                    .text_size(px(8.0))
                                    .text_color(theme::warning())
                                    .child("think text evicted after the patch step"),
                            )
                        });
                }
                stream = stream.child(row);
            }
        }
    }

    let composer = div()
        .m_2()
        .mt_0()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
        .rounded(px(4.0))
        .bg(theme::elevated())
        .border_1()
        .border_color(if composer_focused {
            accent_strong
        } else {
            theme::border_strong()
        })
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(widgets::section_label("Context"))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(8.0))
                        .font_family("Consolas")
                        .text_color(accent)
                        .child(state.context_reference()),
                )
                .child(div().flex_1())
                .when(!state.mode.allows_writes(), |this| {
                    this.child(widgets::pill("READ-ONLY", true, theme::info()))
                }),
        )
        .child(
            div()
                .id("assistant-input")
                .h(px(54.0))
                .w_full()
                .overflow_hidden()
                .cursor_text()
                .track_focus(&workbench.composer_focus)
                .on_key_down(cx.listener(Workbench::on_composer_key))
                .on_click(cx.listener(|this, _, window, cx| {
                    window.focus(&this.composer_focus);
                    cx.notify();
                }))
                .text_size(px(11.0))
                .line_height(px(16.0))
                .text_color(theme::text_primary())
                .child(if state.prompt_input.is_empty() {
                    div()
                        .flex()
                        .items_center()
                        .text_color(theme::text_muted())
                        .child(if state.mode.allows_writes() {
                            "Ask, plan, or describe a change…"
                        } else {
                            "Ask a read-only architecture question…"
                        })
                        .when(composer_focused, |this| {
                            this.child(div().text_color(accent).child("▍"))
                        })
                } else {
                    div()
                        .flex()
                        .items_center()
                        .child(state.prompt_input.clone())
                        .when(composer_focused, |this| {
                            this.child(div().text_color(accent).child("▍"))
                        })
                }),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .id("composer-mode")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.state.cycle_mode();
                            cx.notify();
                        }))
                        .child(widgets::pill(state.mode.short_label(), true, accent)),
                )
                .child(
                    div()
                        .id("composer-language")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.state.cycle_language();
                            cx.notify();
                        }))
                        .child(widgets::pill(state.language_label(), true, theme::cyan())),
                )
                .child(
                    div()
                        .id("composer-model")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.state.cycle_model();
                            cx.notify();
                        }))
                        .child(widgets::pill(state.model_label(), true, theme::purple())),
                )
                .child(
                    div()
                        .id("composer-judge")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.state.toggle_judge();
                            cx.notify();
                        }))
                        .child(widgets::pill(
                            if state.judge_on {
                                "Judge ON"
                            } else {
                                "Judge OFF"
                            },
                            state.judge_on,
                            if state.judge_on {
                                theme::success()
                            } else {
                                theme::danger()
                            },
                        )),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(widgets::keycap("Enter"))
                        .child(
                            div()
                                .id("assistant-send")
                                .flex()
                                .items_center()
                                .justify_center()
                                .px_3()
                                .py(px(3.0))
                                .rounded(px(3.0))
                                .bg(accent_strong)
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme::sunken())
                                .cursor_pointer()
                                .hover(|style| style.bg(accent))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.state.submit_prompt();
                                    window.focus(&this.composer_focus);
                                    cx.notify();
                                }))
                                .child(if state.mode.allows_writes() {
                                    "Run"
                                } else {
                                    "Ask"
                                }),
                        ),
                ),
        )
        .child(
            div()
                .text_size(px(8.0))
                .text_color(theme::text_muted())
                .child(if state.mode.allows_writes() {
                    "Enter submits · Shift+Enter adds a line · preview only, no file writes"
                } else {
                    "Read-only Chat / Architect · no patches, writes, or mutating commands"
                }),
        );

    div()
        .w(px(390.0))
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .bg(theme::sunken())
        .border_l_1()
        .border_color(theme::border())
        .child(header)
        .child(meter)
        .child(stream)
        .child(composer)
}
