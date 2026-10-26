//! Zed activity bar plus the left project / outline / session dock.

use gpui::prelude::*;
use gpui::{div, px, Context, Div, Rgba};

use crate::state::{LeftView, Workbench};
use crate::theme;
use crate::widgets;

pub fn activity_bar(workbench: &Workbench, accent: Rgba, cx: &mut Context<Workbench>) -> Div {
    let state = &workbench.state;
    let views = LeftView::ALL
        .iter()
        .enumerate()
        .map(|(index, view)| {
            let view = *view;
            let active = state.show_left && state.left_view == view;
            div()
                .id(("activity-view", index))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.state.set_left_view(view);
                    cx.notify();
                }))
                .child(widgets::icon_button(view.glyph(), active, accent))
        })
        .collect::<Vec<_>>();

    div()
        .w(px(36.0))
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .items_center()
        .gap_2()
        .py_2()
        .bg(theme::sunken())
        .border_r_1()
        .border_color(theme::border())
        .child(
            div()
                .size(px(24.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .bg(accent)
                .text_size(px(9.0))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme::sunken())
                .child("VA"),
        )
        .child(div().flex().flex_col().gap_1().children(views))
        .child(div().flex_1())
        .child(
            div()
                .id("activity-refresh")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.state.refresh();
                    cx.notify();
                }))
                .child(widgets::icon_button("⟳", false, accent)),
        )
}

pub fn render(workbench: &Workbench, accent: Rgba, cx: &mut Context<Workbench>) -> Div {
    let state = &workbench.state;
    let meta = match state.left_view {
        LeftView::Files => format!("{} files", state.file_count()),
        LeftView::Outline => format!("{} symbols", state.symbol_count()),
        LeftView::Dag => format!("{} nodes", state.session.len()),
    };

    let header = div()
        .h(px(28.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .px_2()
        .border_b_1()
        .border_color(theme::border())
        .child(widgets::section_label(state.left_view.label()))
        .child(
            div()
                .text_size(px(9.0))
                .text_color(theme::text_muted())
                .child(meta),
        );

    let body = match state.left_view {
        LeftView::Files => files_view(workbench, accent, cx),
        LeftView::Outline => outline_view(workbench, accent, cx),
        LeftView::Dag => dag_view(workbench, accent, cx),
    };

    div()
        .w(px(260.0))
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .bg(theme::sunken())
        .border_r_1()
        .border_color(theme::border())
        .child(header)
        .child(
            div()
                .id("left-dock-scroll")
                .flex_1()
                .min_h_0()
                .overflow_scroll()
                .flex()
                .flex_col()
                .child(body),
        )
}

fn files_view(workbench: &Workbench, accent: Rgba, cx: &mut Context<Workbench>) -> Div {
    let state = &workbench.state;
    let active_rel = state.active_rel_path();
    let rows = state.visible_rows();

    if rows.is_empty() {
        return empty_state("No files indexed. Press ⟳ to refresh.");
    }

    let mut list = div().flex().flex_col().py_1();
    for (index, row) in rows.into_iter().enumerate() {
        let row_rel = state.rel_path(&row.path);
        let active = !row.is_dir && active_rel.as_deref() == Some(row_rel.as_str());
        let indent = 8.0 + (row.depth as f32) * 12.0;
        let path = row.path.clone();
        let is_dir = row.is_dir;
        let expanded = row.expanded;
        let glyph = if is_dir {
            if expanded {
                "▾"
            } else {
                "▸"
            }
        } else {
            widgets::file_glyph(&row.name)
        };
        let glyph_color = if is_dir {
            theme::text_muted()
        } else {
            theme::text_secondary()
        };

        list = list.child(
            div()
                .id(("file-row", index))
                .h(px(22.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap_1()
                .pl(px(indent))
                .pr_2()
                .bg(if active {
                    theme::wash(accent, 0.11)
                } else {
                    theme::wash(theme::sunken(), 1.0)
                })
                .border_l_2()
                .border_color(if active {
                    theme::wash(accent, 1.0)
                } else {
                    theme::wash(theme::text_muted(), 0.0)
                })
                .text_size(px(11.0))
                .text_color(if active {
                    theme::text_primary()
                } else {
                    theme::text_secondary()
                })
                .cursor_pointer()
                .hover(|style| style.bg(theme::elevated()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if is_dir {
                        this.state.toggle_dir(&path);
                    } else if let Err(err) = this.state.open_file(&path) {
                        this.state.status_message = err;
                    }
                    cx.notify();
                }))
                .child(
                    div()
                        .w(px(18.0))
                        .text_size(px(8.0))
                        .font_family("Consolas")
                        .text_color(glyph_color)
                        .child(glyph),
                )
                .child(div().flex_1().min_w_0().truncate().child(row.name))
                .when(active, |this| {
                    this.child(div().text_size(px(8.0)).text_color(accent).child("●"))
                }),
        );
    }

    div().flex().flex_col().child(list).child(
        div()
            .px_2()
            .py_2()
            .border_t_1()
            .border_color(theme::border())
            .text_size(px(9.0))
            .text_color(theme::text_muted())
            .child(format!(
                "RepoMap · {} symbols · {} blocks",
                state.symbol_count(),
                state.block_count()
            )),
    )
}

fn outline_view(workbench: &Workbench, accent: Rgba, cx: &mut Context<Workbench>) -> Div {
    let state = &workbench.state;
    let rows = state.outline_rows();
    if rows.is_empty() {
        return empty_state("No indexed symbols. Open a source file or refresh the RepoMap.");
    }

    let mut list = div().flex().flex_col().py_1();
    for (index, row) in rows.into_iter().enumerate() {
        let file_path = row.file_path.clone();
        let block_hash = row.block_hash;
        let color = theme::symbol_color(&row.kind);
        list = list.child(
            div()
                .id(("outline-row", index))
                .h(px(24.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap_1()
                .px_2()
                .bg(if row.is_active {
                    theme::wash(accent, 0.10)
                } else {
                    theme::wash(theme::sunken(), 1.0)
                })
                .border_l_2()
                .border_color(if row.is_active {
                    theme::wash(accent, 1.0)
                } else {
                    theme::wash(theme::text_muted(), 0.0)
                })
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
                        .w(px(28.0))
                        .text_size(px(8.0))
                        .font_family("Consolas")
                        .text_color(color)
                        .child(row.kind),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(11.0))
                        .text_color(if row.is_active {
                            theme::text_primary()
                        } else {
                            theme::text_secondary()
                        })
                        .child(row.name),
                )
                .child(
                    div()
                        .text_size(px(8.0))
                        .font_family("Consolas")
                        .text_color(theme::text_muted())
                        .child(format!(":{}", row.line)),
                )
                .child(widgets::hash_badge(row.block_hash, color)),
        );
    }

    list.child(
        div()
            .px_2()
            .py_2()
            .border_t_1()
            .border_color(theme::border())
            .text_size(px(9.0))
            .text_color(theme::text_muted())
            .child(format!(
                "{} · block hashes are 8-char FNV-1a",
                state
                    .active_rel_path()
                    .unwrap_or_else(|| "workspace outline".to_string())
            )),
    )
}

fn dag_view(workbench: &Workbench, accent: Rgba, cx: &mut Context<Workbench>) -> Div {
    let state = &workbench.state;
    let mut content = div().flex().flex_col().p_2().gap_2();

    content = content.child(
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(widgets::section_label("Session DAG"))
            .child(
                div()
                    .text_size(px(9.0))
                    .font_family("Consolas")
                    .text_color(theme::text_muted())
                    .child(match state.head {
                        Some(head) => format!("HEAD {head}"),
                        None => "EMPTY".to_string(),
                    }),
            ),
    );

    if state.dag.is_empty() {
        content = content.child(empty_state("No committed session nodes."));
    } else {
        let mut nodes = div().flex().flex_col().gap_1();
        for node in &state.dag {
            let verified_color = if node.verified {
                theme::success()
            } else {
                theme::text_muted()
            };
            nodes = nodes.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_2()
                    .rounded(px(3.0))
                    .bg(if node.active {
                        theme::wash(accent, 0.10)
                    } else {
                        theme::wash(theme::elevated(), 1.0)
                    })
                    .border_l_2()
                    .border_color(if node.active { accent } else { theme::border() })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(9.0))
                                    .font_family("Consolas")
                                    .text_color(theme::text_muted())
                                    .child(format!("#{}", node.id)),
                            )
                            .child(
                                div()
                                    .text_size(px(9.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(if node.active {
                                        accent
                                    } else {
                                        theme::text_secondary()
                                    })
                                    .child(node.label.to_uppercase()),
                            )
                            .child(div().flex_1())
                            .child(widgets::dot(verified_color)),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme::text_secondary())
                            .child(node.detail.clone()),
                    )
                    .when(
                        node.cost_micros.is_some() || node.latency_ms.is_some(),
                        |this| {
                            this.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .text_size(px(8.0))
                                    .font_family("Consolas")
                                    .text_color(theme::text_muted())
                                    .child(format!(
                                        "${:.6}",
                                        node.cost_micros.unwrap_or(0) as f64 / 1_000_000.0
                                    ))
                                    .child(format!("{}ms", node.latency_ms.unwrap_or(0))),
                            )
                        },
                    ),
            );
        }
        content = content.child(nodes);
    }

    content = content.child(
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(widgets::section_label("Harness² c1..c8"))
            .child(
                div()
                    .id("reset-harness")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.reset_harness();
                        cx.notify();
                    }))
                    .child(widgets::ghost_button("Reset")),
            ),
    );

    let pristine = state.harness.is_pristine();
    let surfaces = state
        .harness
        .surfaces()
        .into_iter()
        .map(|surface| {
            div()
                .w(px(114.0))
                .flex()
                .items_center()
                .justify_between()
                .px_1()
                .py(px(2.0))
                .rounded(px(2.0))
                .bg(theme::elevated())
                .border_1()
                .border_color(theme::border())
                .text_size(px(8.0))
                .font_family("Consolas")
                .text_color(theme::text_secondary())
                .child(surface.id.chars().take(2).collect::<String>())
                .child(
                    div()
                        .text_color(if pristine {
                            theme::success()
                        } else {
                            theme::warning()
                        })
                        .child(if pristine { "BASE" } else { "OVR" }),
                )
        })
        .collect::<Vec<_>>();
    content = content.child(div().flex().flex_wrap().gap_1().children(surfaces));

    content = content.child(
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(widgets::section_label("EXPERIENCE"))
            .child(
                div()
                    .text_size(px(9.0))
                    .text_color(theme::text_muted())
                    .child(format!("{} lessons", state.experience.entries().len())),
            ),
    );

    let entries = state.experience.entries();
    if entries.is_empty() {
        content = content.child(empty_state(
            "No durable lessons yet. Contrastive recovery writes .ai/EXPERIENCE.md.",
        ));
    } else {
        let mut lessons = div().flex().flex_col().gap_1();
        for entry in entries.iter().take(4) {
            lessons = lessons.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_2()
                    .rounded(px(3.0))
                    .bg(theme::elevated())
                    .border_1()
                    .border_color(theme::border())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_size(px(8.0))
                            .font_family("Consolas")
                            .text_color(theme::cyan())
                            .child(format!("{} / {}", entry.language, entry.domain))
                            .child(div().flex_1())
                            .child(entry.kind.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme::text_secondary())
                            .child(widgets::truncate(&entry.lesson, 150)),
                    ),
            );
        }
        content = content.child(lessons);
    }

    content
}

fn empty_state(text: impl Into<String>) -> Div {
    div()
        .m_2()
        .p_2()
        .rounded(px(3.0))
        .border_1()
        .border_color(theme::border())
        .bg(theme::elevated())
        .text_size(px(10.0))
        .line_height(px(15.0))
        .text_color(theme::text_muted())
        .child(text.into())
}
