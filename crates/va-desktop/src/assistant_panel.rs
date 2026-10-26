//! Quiet agent timeline and a context-aware, docked prompt composer.
use crate::state::{PaletteScope, ThreadItem, Workbench};
use crate::{theme, ui_kit, widgets};
use gpui::prelude::*;
use gpui::{div, px, Context, Div, FontWeight, Rgba};
use ui_kit::{CollapsibleStepCard, ContextChip, CursorStyleComposer, TintedBadge};

pub fn render(
    workbench: &Workbench,
    accent: Rgba,
    _accent_strong: Rgba,
    focused: bool,
    cx: &mut Context<Workbench>,
) -> Div {
    let state = &workbench.state;
    let header = div()
        .h(px(46.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(16.0))
        .child(
            div()
                .text_size(px(theme::HEADING))
                .font_weight(FontWeight::SEMIBOLD)
                .child("✧  Assistant"),
        )
        .child(div().flex_1())
        .child(
            ui_kit::button("assistant-new-thread", "＋ New")
                .text_size(px(theme::MICRO))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.state.clear_thread();
                    window.focus(&this.composer_focus);
                    cx.notify();
                })),
        );
    let mut stream = div()
        .id("assistant-scroll")
        .track_scroll(&workbench.thread_scroll)
        .flex_1()
        .min_h_0()
        .overflow_scroll()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .p(px(16.0));
    for (index, item) in state.thread.iter().enumerate() {
        match item {
            ThreadItem::User { text } => {
                stream = stream.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .p(px(12.0))
                        .rounded(px(10.0))
                        .bg(theme::elevated())
                        .child(widgets::section_label("You"))
                        .child(message(text, theme::text_primary())),
                )
            }
            ThreadItem::Assistant { text, read_only } => {
                stream = stream.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .text_size(px(theme::MICRO))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme::text_secondary())
                                .child("✧ Vibe Assembly")
                                .when(*read_only, |this| {
                                    this.child(TintedBadge::render("Read only", theme::info()))
                                }),
                        )
                        .child(message(text, theme::text_secondary())),
                )
            }
            ThreadItem::System { text } => {
                stream = stream.child(
                    div()
                        .id(("system-message", index))
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .p(px(10.0))
                        .rounded(px(8.0))
                        .bg(theme::elevated())
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !this.state.expanded_messages.remove(&index) {
                                this.state.expanded_messages.insert(index);
                            }
                            cx.notify();
                        }))
                        .child(
                            div()
                                .text_size(px(theme::MICRO))
                                .text_color(theme::text_muted())
                                .child(format!(
                                    "{}  {}",
                                    if state.expanded_messages.contains(&index) {
                                        "⌄"
                                    } else {
                                        "›"
                                    },
                                    widgets::first_line(text)
                                )),
                        )
                        .when(state.expanded_messages.contains(&index), |this| {
                            this.child(message(text, theme::text_secondary()))
                        }),
                )
            }
            ThreadItem::Step(step) => {
                let mut card = div()
                    .flex()
                    .flex_col()
                    .rounded(px(8.0))
                    .bg(theme::elevated())
                    .child(
                        CollapsibleStepCard::header(("assistant-step", index), step).on_click(
                            cx.listener(move |this, _, _, cx| {
                                if let Some(ThreadItem::Step(step)) =
                                    this.state.thread.get_mut(index)
                                {
                                    step.expanded = !step.expanded;
                                }
                                cx.notify();
                            }),
                        ),
                    );
                if step.expanded {
                    card = card.child(
                        div()
                            .px(px(12.0))
                            .pb(px(12.0))
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(px(theme::BODY))
                                    .text_color(theme::text_secondary())
                                    .child(step.title.clone()),
                            )
                            .child(message(&step.detail, theme::text_muted()))
                            .child(
                                div()
                                    .text_size(px(theme::MICRO))
                                    .text_color(theme::text_muted())
                                    .child(format!("{} tokens · {}", step.tokens, step.status)),
                            )
                            .when(step.think_evicted, |this| {
                                this.child(
                                    div()
                                        .px(px(12.0))
                                        .pb(px(10.0))
                                        .text_size(px(theme::MICRO))
                                        .text_color(theme::text_muted())
                                        .child("Thought text compacted after patch"),
                                )
                            }),
                    );
                }
                stream = stream.child(card);
            }
        }
    }
    let mut context = div().flex().flex_wrap().items_center().gap(px(6.0));
    if state.context_file {
        if let Some(path) = state.active_rel_path() {
            context = context.child(
                ContextChip::render(
                    format!("@{}", path.rsplit('/').next().unwrap_or(&path)),
                    theme::text_secondary(),
                )
                .child(
                    ui_kit::icon_button("remove-file-context", "×", false)
                        .size(px(14.0))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.state.context_file = false;
                            cx.notify();
                        })),
                ),
            );
        }
    }
    if let Some((path, hash)) = &state.pinned_block {
        context = context
            .child(
                ContextChip::render(format!("#{}", widgets::hash8(*hash)), accent).child(
                    ui_kit::icon_button("remove-block-context", "×", false)
                        .size(px(14.0))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.state.pinned_block = None;
                            cx.notify();
                        })),
                ),
            )
            .child(
                div()
                    .max_w(px(150.0))
                    .truncate()
                    .text_size(px(theme::MICRO))
                    .text_color(theme::text_muted())
                    .child(state.rel_path(path)),
            );
    }
    if !state.context_file && state.pinned_block.is_none() {
        context = context.child(
            ui_kit::button("add-active-context", "＋ Add active file")
                .text_size(px(theme::MICRO))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.state.context_file = true;
                    cx.notify();
                })),
        );
    }
    let input = div()
        .id("assistant-input")
        .min_h(px(88.0))
        .max_h(px(180.0))
        .w_full()
        .cursor_text()
        .track_focus(&workbench.composer_focus)
        .on_key_down(cx.listener(Workbench::on_composer_key))
        .on_click(cx.listener(|this, _, window, cx| {
            window.focus(&this.composer_focus);
            cx.notify();
        }))
        .relative()
        .child(
            div()
                .id("prompt-text-scroll")
                .max_h(px(180.0))
                .overflow_y_scroll()
                .text_size(px(theme::BODY))
                .line_height(px(21.0))
                .child(workbench.prompt_selection.display(
                    &state.prompt_input,
                    focused,
                    "Ask, plan, or describe a change…",
                    accent,
                )),
        )
        .child(ui_kit::capture(
            cx.entity(),
            workbench.composer_focus.clone(),
        ));
    let controls = div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .child(
            ui_kit::button("composer-mode", format!("{} ⌄", state.mode.short_label()))
                .rounded_full()
                .text_size(px(11.5))
                .bg(theme::wash(accent, 0.12))
                .text_color(accent)
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_palette(PaletteScope::Mode, window, cx);
                })),
        )
        .child(
            ui_kit::button(
                "composer-language",
                format!("{} ⌄", state.resolved_language().short_label()),
            )
            .rounded_full()
            .px(px(6.0))
            .text_size(px(11.5))
            .on_click(cx.listener(|this, _, window, cx| {
                this.open_palette(PaletteScope::Language, window, cx);
            })),
        )
        .child(
            ui_kit::button(
                "composer-model",
                format!("{} ⌄", widgets::truncate(&state.model_label(), 14)),
            )
            .rounded_full()
            .px(px(6.0))
            .text_size(px(11.5))
            .on_click(cx.listener(|this, _, window, cx| {
                this.open_palette(PaletteScope::Model, window, cx);
            })),
        )
        .child(div().flex_1())
        .child(
            ui_kit::button(
                "assistant-send",
                if state.mode.allows_writes() {
                    "Run ↵"
                } else {
                    "Ask ↵"
                },
            )
            .bg(theme::wash(accent, 0.18))
            .text_color(accent)
            .border_1()
            .border_color(theme::wash(accent, 0.28))
            .on_click(cx.listener(|this, _, window, cx| {
                this.state.submit_prompt();
                this.thread_scroll.scroll_to_bottom();
                window.focus(&this.composer_focus);
                cx.notify();
            })),
        );
    let footer = div()
        .flex()
        .items_center()
        .justify_between()
        .text_size(px(theme::MICRO))
        .text_color(theme::text_muted())
        .child(
            ui_kit::button(
                "assistant-context",
                format!("{}% context", state.context_pct),
            )
            .text_size(px(theme::MICRO))
            .px_0()
            .on_click(cx.listener(|this, _, _, cx| {
                this.state.compact_context();
                cx.notify();
            })),
        )
        .child(
            ui_kit::button(
                "composer-judge",
                if state.judge_on {
                    "◇ Judge on"
                } else {
                    "◇ Judge off"
                },
            )
            .text_size(px(theme::MICRO))
            .px_0()
            .text_color(if state.judge_on {
                theme::text_muted()
            } else {
                theme::danger()
            })
            .on_click(cx.listener(|this, _, _, cx| {
                this.state.toggle_judge();
                cx.notify();
            })),
        );
    div()
        .w(px(theme::ASSISTANT))
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .bg(theme::sunken())
        .border_l_1()
        .border_color(theme::border())
        .child(header)
        .child(
            div()
                .px(px(16.0))
                .pb(px(10.0))
                .flex()
                .items_center()
                .justify_between()
                .child(widgets::section_label("Current thread"))
                .child(TintedBadge::render(
                    "Auto-Compact @ 75%",
                    theme::text_muted(),
                )),
        )
        .child(stream)
        .child(
            CursorStyleComposer::surface(focused, accent)
                .child(context)
                .child(input)
                .child(controls)
                .child(widgets::meter(
                    state.context_pct,
                    if state.context_pct >= 75 {
                        theme::warning()
                    } else {
                        accent
                    },
                ))
                .child(footer),
        )
        .child(
            div()
                .px(px(16.0))
                .pb(px(10.0))
                .text_size(px(theme::MICRO))
                .text_color(theme::text_muted())
                .child(if state.mode.allows_writes() {
                    "Preview session · Shift+Enter for a new line"
                } else {
                    "Read-only session · Shift+Enter for a new line"
                }),
        )
}

fn message(text: &str, color: Rgba) -> Div {
    div()
        .text_size(px(theme::BODY))
        .line_height(px(21.0))
        .text_color(color)
        .child(text.to_string())
}
