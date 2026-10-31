//! Center conversation: the agent thread, the prompt composer, and the quiet
//! session chrome around them. This is the primary surface of the workbench —
//! code only appears as inline diffs until you open a file on the right.
use crate::design::icon::icon;
use crate::state::{LeftView, PaletteScope, ThreadItem, Workbench};
use crate::{theme, ui_kit, widgets};
use gpui::prelude::*;
use gpui::{div, px, Context, Div, FontWeight, Rgba, Stateful};
use ui_kit::{CollapsibleStepCard, ContextChip, CursorStyleComposer, TintedBadge};

pub fn render(
    workbench: &Workbench,
    accent: Rgba,
    _accent_strong: Rgba,
    focused: bool,
    cx: &mut Context<Workbench>,
) -> Div {
    let state = &workbench.state;
    let workspace = state
        .root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workspace")
        .to_string();
    let header = div()
        .h(px(56.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(20.0))
        .border_b_1()
        .border_color(theme::border())
        .child(avatar(30.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(1.0))
                .child(
                    div()
                        .text_size(px(theme::HEADING))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme::text_primary())
                        .child(workspace),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .text_size(px(theme::MICRO))
                        .text_color(theme::text_muted())
                        .child(widgets::dot(if state.router_live {
                            theme::success()
                        } else {
                            theme::text_muted()
                        }))
                        .child(format!(
                            "Vibe · {} mode · this session",
                            state.mode.short_label()
                        )),
                ),
        )
        .child(div().flex_1())
        .when(!state.open_buffers.is_empty(), |this| {
            this.child(TintedBadge::render(
                if state.open_buffers.len() == 1 {
                    "1 file open".to_string()
                } else {
                    format!("{} files open", state.open_buffers.len())
                },
                theme::text_muted(),
            ))
        })
        .child(
            ui_kit::icon_button("assistant-dag", "history", false).on_click(cx.listener(
                |this, _, _, cx| {
                    this.state.show_left = true;
                    this.state.set_left_view(LeftView::Dag);
                    cx.notify();
                },
            )),
        )
        .child(
            div()
                .id("assistant-new-thread")
                .flex()
                .items_center()
                .gap(px(6.0))
                .h(px(28.0))
                .px(px(10.0))
                .rounded(px(8.0))
                .bg(theme::brand_soft())
                .text_size(px(theme::BODY))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme::brand_ink())
                .cursor_pointer()
                .hover(|style| style.bg(theme::brand()))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.state.clear_thread();
                    window.focus(&this.composer_focus);
                    cx.notify();
                }))
                .child(icon("plus", 14.0, theme::brand_ink()))
                .child("New"),
        );

    let mut stream = div()
        .id("assistant-scroll")
        .track_scroll(&workbench.thread_scroll)
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .items_center();
    let mut column = div()
        .w_full()
        .max_w(px(760.0))
        .flex()
        .flex_col()
        .gap(px(18.0))
        .px(px(24.0))
        .pt(px(24.0))
        .pb(px(20.0));
    if state.thread.is_empty() {
        column = column.child(welcome(cx));
    }
    for (index, item) in state.thread.iter().enumerate() {
        match item {
            ThreadItem::User { text } => {
                column = column.child(
                    div().w_full().flex().justify_end().child(
                        div()
                            .max_w(px(560.0))
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .px(px(14.0))
                            .py(px(10.0))
                            .rounded(px(14.0))
                            .bg(theme::brand_soft())
                            .border_1()
                            .border_color(theme::wash(theme::brand(), 0.35))
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme::brand_ink())
                                    .child("You"),
                            )
                            .child(message(text, theme::text_primary())),
                    ),
                )
            }
            ThreadItem::Assistant { text, read_only } => {
                column = column.child(
                    div()
                        .w_full()
                        .flex()
                        .gap(px(10.0))
                        .child(avatar(26.0))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .text_size(px(theme::MICRO))
                                        .text_color(theme::text_muted())
                                        .child(
                                            div()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(theme::text_primary())
                                                .child("Vibe"),
                                        )
                                        .child("this session")
                                        .when(*read_only, |this| {
                                            this.child(TintedBadge::render(
                                                "Read only",
                                                theme::info(),
                                            ))
                                        }),
                                )
                                .child(message(text, theme::text_secondary())),
                        ),
                )
            }
            ThreadItem::System { text } => {
                let expanded = state.expanded_messages.contains(&index);
                column = column.child(
                    div()
                        .id(("system-message", index))
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .px(px(12.0))
                        .py(px(9.0))
                        .rounded(px(10.0))
                        .border_1()
                        .border_color(theme::border())
                        .bg(theme::wash(theme::elevated(), 0.7))
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::elevated()))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !this.state.expanded_messages.remove(&index) {
                                this.state.expanded_messages.insert(index);
                            }
                            cx.notify();
                        }))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(7.0))
                                .text_size(px(theme::MICRO))
                                .text_color(theme::text_muted())
                                .child(icon(
                                    if expanded {
                                        "chevron-down"
                                    } else {
                                        "chevron-right"
                                    },
                                    13.0,
                                    theme::text_muted(),
                                ))
                                .child(widgets::first_line(text)),
                        )
                        .when(expanded, |this| {
                            this.child(message(text, theme::text_secondary()))
                        }),
                )
            }
            ThreadItem::Step(step) => {
                let mut card = div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .rounded(px(12.0))
                    .border_1()
                    .border_color(theme::border())
                    .bg(theme::elevated())
                    .shadow(widgets::card_shadow())
                    .overflow_hidden()
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
                                        .text_size(px(theme::MICRO))
                                        .text_color(theme::text_muted())
                                        .child("Thought text compacted after patch"),
                                )
                            }),
                    );
                }
                column = column.child(card);
            }
        }
    }
    stream = stream.child(column);

    let mut context = div().flex().flex_wrap().items_center().gap(px(6.0));
    if state.context_file {
        if let Some(path) = state.active_rel_path() {
            context = context.child(
                ContextChip::render(
                    format!("@{}", path.rsplit('/').next().unwrap_or(&path)),
                    theme::text_secondary(),
                )
                .child(
                    ui_kit::icon_button("remove-file-context", "x", false)
                        .size(px(16.0))
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
                    ui_kit::icon_button("remove-block-context", "x", false)
                        .size(px(16.0))
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
        context = context.child(chip_button("add-active-context", "Attach file").on_click(
            cx.listener(|this, _, _, cx| {
                this.state.context_file = true;
                cx.notify();
            }),
        ));
    }
    let input = div()
        .id("assistant-input")
        .min_h(px(64.0))
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
                .text_size(px(13.0))
                .line_height(px(21.0))
                .child(workbench.prompt_selection.display(
                    &state.prompt_input,
                    focused,
                    "Ask Vibe to change something…",
                    theme::brand_ink(),
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
            div()
                .id("composer-mode")
                .flex()
                .items_center()
                .gap(px(6.0))
                .h(px(28.0))
                .px(px(10.0))
                .rounded_full()
                .bg(theme::brand_soft())
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme::brand_ink())
                .cursor_pointer()
                .hover(|style| style.bg(theme::brand()))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_palette(PaletteScope::Mode, window, cx);
                }))
                .child(widgets::dot(theme::brand_ink()))
                .child(state.mode.short_label())
                .child(icon("chevron-down", 12.0, theme::brand_ink())),
        )
        .child(
            chip_button("composer-language", state.resolved_language().short_label()).on_click(
                cx.listener(|this, _, window, cx| {
                    this.open_palette(PaletteScope::Language, window, cx);
                }),
            ),
        )
        .child(
            chip_button(
                "composer-model",
                widgets::truncate(&state.model_label(), 14),
            )
            .on_click(cx.listener(|this, _, window, cx| {
                this.open_palette(PaletteScope::Model, window, cx);
            })),
        )
        .child(div().flex_1())
        .child(
            div()
                .id("composer-judge")
                .flex()
                .items_center()
                .gap(px(5.0))
                .h(px(28.0))
                .px(px(8.0))
                .rounded(px(7.0))
                .text_size(px(theme::MICRO))
                .text_color(if state.judge_on {
                    theme::text_muted()
                } else {
                    theme::danger()
                })
                .cursor_pointer()
                .hover(|style| style.bg(theme::elevated_alt()))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.state.toggle_judge();
                    cx.notify();
                }))
                .child(icon(
                    "scale",
                    13.0,
                    if state.judge_on {
                        theme::text_muted()
                    } else {
                        theme::danger()
                    },
                ))
                .child(if state.judge_on {
                    "Judge on"
                } else {
                    "Judge off"
                }),
        )
        .child(
            div()
                .id("assistant-send")
                .flex()
                .items_center()
                .gap(px(6.0))
                .h(px(30.0))
                .px(px(12.0))
                .rounded(px(8.0))
                .bg(theme::brand())
                .text_size(px(theme::BODY))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme::text_primary())
                .cursor_pointer()
                .hover(|style| style.bg(theme::brand_hover()))
                .active(|style| style.bg(theme::brand_press()))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.state.submit_prompt();
                    this.thread_scroll.scroll_to_bottom();
                    window.focus(&this.composer_focus);
                    cx.notify();
                }))
                .child(icon("arrow-up", 14.0, theme::text_primary()))
                .child(if state.mode.allows_writes() {
                    "Send"
                } else {
                    "Ask"
                }),
        );
    let note = div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .text_size(px(theme::MICRO))
        .text_color(theme::text_muted())
        .child(
            div()
                .id("assistant-context")
                .cursor_pointer()
                .hover(|style| style.text_color(theme::text_secondary()))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.state.compact_context();
                    cx.notify();
                }))
                .child(format!("{}% context", state.context_pct)),
        )
        .child(div().flex_1())
        .child("Vibe can make mistakes — review every change before you apply it.");

    div()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .h_full()
        .flex()
        .flex_col()
        .bg(theme::editor())
        .child(header)
        .child(stream)
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .items_center()
                .flex_shrink_0()
                .child(
                    CursorStyleComposer::surface(focused, accent)
                        .w_full()
                        .max_w(px(760.0))
                        .child(context)
                        .child(input)
                        .child(widgets::meter(
                            state.context_pct,
                            if state.context_pct >= 75 {
                                theme::warning()
                            } else {
                                theme::brand()
                            },
                        ))
                        .child(controls)
                        .child(note),
                ),
        )
}

fn welcome(cx: &mut Context<Workbench>) -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .pt(px(18.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(avatar(26.0))
                .child(
                    div()
                        .text_size(px(theme::MICRO))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme::text_primary())
                        .child("Vibe"),
                )
                .child(
                    div()
                        .text_size(px(theme::MICRO))
                        .text_color(theme::text_muted())
                        .child("this session"),
                ),
        )
        .child(
            div()
                .text_size(px(22.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme::text_primary())
                .child("What are we building?"),
        )
        .child(
            div()
                .max_w(px(560.0))
                .text_size(px(13.0))
                .line_height(px(22.0))
                .text_color(theme::text_secondary())
                .child("Tell Vibe what to change. It reads the workspace, proposes a patch, and waits for you to apply it — nothing changes on its own."),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(8.0))
                .pt(px(4.0))
                .child(suggestion("suggest-explain", "Explain this workspace", cx))
                .child(suggestion("suggest-fix", "Find something to fix", cx))
                .child(suggestion("suggest-restyle", "Restyle the sidebar", cx)),
        )
}

fn suggestion(id: &'static str, label: &'static str, cx: &mut Context<Workbench>) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(30.0))
        .flex()
        .items_center()
        .px(px(12.0))
        .rounded_full()
        .bg(theme::elevated())
        .border_1()
        .border_color(theme::border())
        .text_size(px(11.5))
        .text_color(theme::text_secondary())
        .cursor_pointer()
        .hover(|style| style.bg(theme::brand_soft()).text_color(theme::brand_ink()))
        .on_click(cx.listener(move |this, _, window, cx| {
            this.state.prompt_input = label.to_string();
            this.prompt_selection = Default::default();
            window.focus(&this.composer_focus);
            cx.notify();
        }))
        .child(label)
}

fn chip_button(id: &'static str, label: impl Into<String>) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(6.0))
        .h(px(28.0))
        .px(px(9.0))
        .rounded_full()
        .text_size(px(11.5))
        .text_color(theme::text_secondary())
        .cursor_pointer()
        .hover(|style| style.bg(theme::elevated_alt()))
        .child(label.into())
        .child(icon("chevron-down", 12.0, theme::text_muted()))
}

fn avatar(size: f32) -> Div {
    div()
        .size(px(size))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(theme::brand_soft())
        .child(icon("message-circle", size * 0.52, theme::brand_ink()))
}

fn message(text: &str, color: Rgba) -> Div {
    div()
        .text_size(px(13.0))
        .line_height(px(22.0))
        .text_color(color)
        .child(text.to_string())
}
