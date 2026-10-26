//! Bottom drawer: pinned compiler problems, verification output, and the
//! c1..c8 harness overlay inspector.

use gpui::prelude::*;
use gpui::{div, px, Context, Div, Rgba};

use crate::state::{BottomTab, Workbench};
use crate::widgets;
use crate::{theme, ui_kit};

pub fn render(workbench: &Workbench, accent: Rgba, cx: &mut Context<Workbench>) -> Div {
    let state = &workbench.state;
    let tabs = [
        (
            BottomTab::Problems,
            format!("Problems · {}", state.diagnostics.len()),
        ),
        (BottomTab::Verify, "Verification Output".to_string()),
        (BottomTab::Harness, "Harness² Rules".to_string()),
    ];

    let mut tab_strip = div().flex().items_center().h_full();
    for (index, (tab, label)) in tabs.into_iter().enumerate() {
        let active = state.bottom_tab == tab;
        tab_strip = tab_strip.child(
            div()
                .id(("bottom-tab", index))
                .h_full()
                .flex()
                .items_center()
                .px_2()
                .border_b_2()
                .border_color(if active {
                    theme::wash(accent, 1.0)
                } else {
                    theme::wash(theme::text_muted(), 0.0)
                })
                .text_size(px(theme::BODY))
                .font_weight(if active {
                    gpui::FontWeight::SEMIBOLD
                } else {
                    gpui::FontWeight::NORMAL
                })
                .text_color(if active {
                    theme::text_primary()
                } else {
                    theme::text_muted()
                })
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.state.set_bottom_tab(tab);
                    cx.notify();
                }))
                .child(label),
        );
    }

    let header = div()
        .h(px(34.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .px_2()
        .bg(theme::sunken())
        .border_b_1()
        .border_color(theme::border())
        .child(tab_strip)
        .child(
            div().flex().items_center().gap_2().child(
                div()
                    .id("bottom-close")
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(18.0))
                    .rounded(px(6.0))
                    .text_size(px(12.0))
                    .text_color(theme::text_muted())
                    .cursor_pointer()
                    .hover(|style| {
                        style
                            .bg(theme::wash(theme::danger(), 0.18))
                            .text_color(theme::danger())
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.toggle_bottom();
                        cx.notify();
                    }))
                    .child("×"),
            ),
        );

    let body = match state.bottom_tab {
        BottomTab::Problems => problems(workbench, accent, cx),
        BottomTab::Verify => verify(workbench, accent, cx),
        BottomTab::Harness => harness(workbench, accent, cx),
    };

    div()
        .h(px(theme::DRAWER))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .bg(theme::sunken())
        .border_t_1()
        .border_color(theme::border())
        .child(header)
        .child(
            div()
                .id("bottom-drawer-scroll")
                .flex_1()
                .min_h_0()
                .overflow_scroll()
                .child(body),
        )
}

fn problems(workbench: &Workbench, accent: Rgba, cx: &mut Context<Workbench>) -> Div {
    let state = &workbench.state;
    if state.diagnostics.is_empty() {
        return div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(16.0))
            .p(px(24.0))
            .child(
                div()
                    .text_size(px(theme::BODY))
                    .text_color(theme::text_secondary())
                    .child("No problems to show"),
            )
            .child(
                div()
                    .text_size(px(theme::MICRO))
                    .text_color(theme::text_muted())
                    .child("Run a workspace check when you’re ready."),
            )
            .child(
                div()
                    .id("run-diagnostics-problems")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.run_diagnostics();
                        cx.notify();
                    }))
                    .child(ui_kit::TintedBadge::render("Run Diagnostics", accent)),
            );
    }

    let mut list = div().flex().flex_col().py_1();
    for (index, diagnostic) in state.diagnostics.iter().enumerate() {
        let level = diagnostic.level.to_ascii_lowercase();
        let color = if level.contains("error") {
            theme::danger()
        } else if level.contains("warn") {
            theme::warning()
        } else {
            theme::info()
        };
        let file_path = diagnostic.file_path.clone();
        let block_hash = diagnostic.block_hash;
        list = list.child(
            div()
                .id(("diagnostic-row", index))
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .py_1()
                .border_b_1()
                .border_color(theme::border())
                .cursor_pointer()
                .hover(|style| style.bg(theme::elevated()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Err(err) = this.state.open_at_block(&file_path, block_hash) {
                        this.state.status_message = err;
                    }
                    cx.notify();
                }))
                .child(
                    div()
                        .w(px(46.0))
                        .text_size(px(theme::MICRO))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(color)
                        .child(diagnostic.level.to_uppercase()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(1.0))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(theme::MICRO))
                                .text_color(theme::text_secondary())
                                .child(diagnostic.message.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(theme::MICRO))
                                .font_family(theme::code_font())
                                .text_color(theme::text_muted())
                                .child(format!(
                                    "{}:{}-{} · span {}-{}",
                                    state.rel_path(&diagnostic.file_path),
                                    diagnostic.line_range.0,
                                    diagnostic.line_range.1,
                                    diagnostic.span_lines.0,
                                    diagnostic.span_lines.1
                                )),
                        ),
                )
                .when_some(diagnostic.error_code.as_ref(), |this, code| {
                    this.child(
                        div()
                            .px_1()
                            .rounded(px(2.0))
                            .bg(theme::wash(color, 0.10))
                            .text_size(px(theme::MICRO))
                            .font_family(theme::code_font())
                            .text_color(color)
                            .child(code.clone()),
                    )
                })
                .child(widgets::hash_badge(diagnostic.block_hash, color)),
        );
    }

    list
}

fn verify(workbench: &Workbench, _accent: Rgba, cx: &mut Context<Workbench>) -> Div {
    let state = &workbench.state;
    let mut body = div().flex().flex_col().px_2().py_1().gap_1().child(
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_size(px(theme::MICRO))
                    .text_color(theme::text_muted())
                    .child("Latest verification"),
            )
            .child(
                div()
                    .id("run-diagnostics-verify")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.run_diagnostics();
                        cx.notify();
                    }))
                    .child(widgets::ghost_button("Run cargo check")),
            ),
    );

    if state.verify_output.trim().is_empty() {
        body = body.child(
            div()
                .py_3()
                .text_size(px(theme::MICRO))
                .text_color(theme::text_muted())
                .child("No verification output yet. Run cargo check or add .ai/verification.md."),
        );
        return body;
    }

    for line in widgets::head_tail(&state.verify_output, 8, 8) {
        let lower = line.to_ascii_lowercase();
        let color = if lower.contains("error") {
            theme::danger()
        } else if lower.contains("warning") {
            theme::warning()
        } else if lower.contains("test result: ok") || lower.contains("finished") {
            theme::success()
        } else {
            theme::text_secondary()
        };
        body = body.child(
            div()
                .font_family(theme::code_font())
                .text_size(px(theme::MICRO))
                .line_height(px(19.0))
                .text_color(color)
                .child(line),
        );
    }

    body.child(
        div()
            .mt_1()
            .text_size(px(theme::MICRO))
            .text_color(theme::text_muted())
            .child(format!(
                "{} · {} lines · independent judge {}",
                state.mode.label(),
                state.verify_output.lines().count(),
                if state.judge_on { "ON" } else { "OFF" }
            )),
    )
}

fn harness(workbench: &Workbench, accent: Rgba, cx: &mut Context<Workbench>) -> Div {
    let state = &workbench.state;
    let pristine = state.harness.is_pristine();
    let mut body = div().flex().flex_col().px_2().py_1().gap_1().child(
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(theme::MICRO))
                            .text_color(theme::text_muted())
                            .child("Workspace rules"),
                    )
                    .child(widgets::pill(
                        if pristine { "BASE" } else { "OVERLAY" },
                        true,
                        if pristine {
                            theme::success()
                        } else {
                            theme::warning()
                        },
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id("harness-reset")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.reset_harness();
                                cx.notify();
                            }))
                            .child(widgets::ghost_button("Reset to Base")),
                    )
                    .child(widgets::pill(
                        format!("{} runs", state.tournament_runs),
                        state.tournament_runs > 0,
                        accent,
                    )),
            ),
    );

    for (index, surface) in state.harness.surfaces().into_iter().enumerate() {
        let expanded = state.expanded_rules.contains(&surface.id);
        let id = surface.id.to_string();
        body = body.child(
            div()
                .flex()
                .flex_col()
                .rounded(px(6.0))
                .child(
                    ui_kit::button(("harness-rule", index), "")
                        .justify_start()
                        .w_full()
                        .h(px(30.0))
                        .child(
                            div()
                                .w(px(12.0))
                                .text_color(theme::text_muted())
                                .child(if expanded { "⌄" } else { "›" }),
                        )
                        .child(
                            div()
                                .w(px(120.0))
                                .text_size(px(theme::MICRO))
                                .text_color(accent)
                                .child(surface.id),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(theme::MICRO))
                                .text_color(theme::text_muted())
                                .child(widgets::first_line(&surface.content)),
                        )
                        .child(ui_kit::TintedBadge::render(
                            if pristine { "Base" } else { "Overlay" },
                            if pristine {
                                theme::success()
                            } else {
                                theme::warning()
                            },
                        ))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !this.state.expanded_rules.remove(&id) {
                                this.state.expanded_rules.insert(id.clone());
                            }
                            cx.notify();
                        })),
                )
                .when(expanded, |this| {
                    this.child(
                        div()
                            .p(px(12.0))
                            .bg(theme::elevated())
                            .text_size(px(theme::BODY))
                            .line_height(px(20.0))
                            .text_color(theme::text_secondary())
                            .child(surface.content.to_string()),
                    )
                }),
        );
    }
    let expanded = state.expanded_rules.contains("experience");
    body = body.child(
        ui_kit::button(
            "durable-lessons",
            format!(
                "{} Durable lessons · {}",
                if expanded { "⌄" } else { "›" },
                state.experience.entries().len()
            ),
        )
        .justify_start()
        .on_click(cx.listener(|this, _, _, cx| {
            if !this.state.expanded_rules.remove("experience") {
                this.state.expanded_rules.insert("experience".into());
            }
            cx.notify();
        })),
    );
    if expanded {
        for entry in state.experience.entries().iter().rev().take(4) {
            body = body.child(
                div()
                    .p(px(12.0))
                    .rounded(px(6.0))
                    .bg(theme::elevated())
                    .text_size(px(theme::BODY))
                    .line_height(px(20.0))
                    .text_color(theme::text_secondary())
                    .child(format!(
                        "{} · {}\n{}",
                        entry.language, entry.domain, entry.lesson
                    )),
            );
        }
        if state.experience.entries().is_empty() {
            body = body.child(
                div()
                    .p(px(12.0))
                    .text_size(px(theme::BODY))
                    .text_color(theme::text_muted())
                    .child("Lessons from verified recovery will appear here."),
            );
        }
    }

    body
}
