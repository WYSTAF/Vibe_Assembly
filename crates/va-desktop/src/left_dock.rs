//! Project navigation with shared controls and quiet tree rows.
use crate::design::icon::icon;
use crate::state::{LeftView, Workbench};
use crate::{theme, ui_kit, widgets};
use gpui::prelude::*;
use gpui::{div, px, Context, Div, Rgba};
use ui_kit::{FileTreeRow, SegmentedPillBar, SymbolRow};

pub fn render(workbench: &Workbench, accent: Rgba, cx: &mut Context<Workbench>) -> Div {
    let state = &workbench.state;
    let mut segments = SegmentedPillBar::new();
    for (index, view) in LeftView::ALL.into_iter().enumerate() {
        segments = segments.item(
            SegmentedPillBar::segment(
                ("left-view", index),
                view.icon(),
                view.label(),
                state.left_view == view,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.state.set_left_view(view);
                cx.notify();
            })),
        );
    }
    let body = match state.left_view {
        LeftView::Files => files(workbench, cx),
        LeftView::Outline => symbols(workbench, cx),
        LeftView::Dag => timeline(workbench, accent, cx),
    };
    div()
        .w(px(theme::SIDEBAR))
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .bg(theme::sunken())
        .border_r_1()
        .border_color(theme::border())
        .child(
            div()
                .h(px(46.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .px(px(10.0))
                .child(segments.build()),
        )
        .child(
            div()
                .h(px(30.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_between()
                .px(px(16.0))
                .child(widgets::section_label(match state.left_view {
                    LeftView::Files => "Workspace",
                    LeftView::Outline => "Symbols",
                    LeftView::Dag => "Session timeline",
                }))
                .child(
                    ui_kit::icon_button("navigation-refresh", "refresh-cw", false).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.state.refresh();
                            cx.notify();
                        }),
                    ),
                ),
        )
        .child(
            div()
                .id("left-dock-scroll")
                .flex_1()
                .min_h_0()
                .overflow_scroll()
                .px(px(6.0))
                .child(body),
        )
        .child(
            div()
                .h(px(32.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(px(6.0))
                .px(px(16.0))
                .border_t_1()
                .border_color(theme::border())
                .text_size(px(theme::MICRO))
                .text_color(theme::text_muted())
                .child(widgets::dot(theme::success()))
                .child(format!("{} files indexed", state.file_count())),
        )
}

fn files(workbench: &Workbench, cx: &mut Context<Workbench>) -> Div {
    let state = &workbench.state;
    let rows = state.visible_rows();
    if rows.is_empty() {
        return ui_kit::empty_state(
            icon("files", 22.0, theme::brand_ink()),
            "Your workspace",
            "Open a project or refresh to see its files here.",
        );
    }
    let mut list = div().flex().flex_col().pb(px(12.0));
    for (index, row) in rows.into_iter().enumerate() {
        let selected = state.active_file_path().as_ref() == Some(&row.path);
        list = list.child(
            FileTreeRow::render(
                ("file-row", index),
                &row.name,
                row.depth,
                row.is_dir,
                row.expanded,
                selected,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if row.is_dir {
                    this.state.toggle_dir(&row.path);
                } else if let Err(error) = this.state.open_file(&row.path) {
                    this.state.status_message = error;
                }
                cx.notify();
            })),
        );
    }
    list
}

fn symbols(workbench: &Workbench, cx: &mut Context<Workbench>) -> Div {
    let rows = workbench.state.outline_rows();
    if rows.is_empty() {
        return ui_kit::empty_state(
            icon("braces", 22.0, theme::brand_ink()),
            "An outline of your code",
            "Open a source file to explore its functions and types.",
        );
    }
    let mut list = div().flex().flex_col();
    for (index, row) in rows.into_iter().enumerate() {
        list = list.child(
            SymbolRow::render(
                ("symbol-row", index),
                &row.kind,
                &row.name,
                row.line,
                row.is_active,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Err(error) = this.state.open_at_block(&row.file_path, row.block_hash) {
                    this.state.status_message = error;
                }
                cx.notify();
            })),
        );
    }
    list
}

fn timeline(workbench: &Workbench, accent: Rgba, cx: &mut Context<Workbench>) -> Div {
    let state = &workbench.state;
    let mut list = div().flex().flex_col().gap(px(8.0)).p(px(8.0));
    if state.dag.is_empty() {
        list = list.child(ui_kit::empty_state(
            icon("git-branch", 22.0, theme::brand_ink()),
            "A fresh timeline",
            "Prompts and verified steps will appear here as you work.",
        ));
    }
    for (index, node) in state.dag.iter().enumerate() {
        let expanded = state.expanded_dag.contains(&node.id);
        let id = node.id;
        let mut card = div()
            .flex()
            .flex_col()
            .rounded(px(12.0))
            .bg(theme::elevated())
            .border_1()
            .border_color(theme::border())
            .shadow(widgets::card_shadow())
            .child(
                div()
                    .id(("dag-node", index))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .p(px(10.0))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme::elevated_alt()))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.state.expanded_dag.remove(&id) {
                            this.state.expanded_dag.insert(id);
                        }
                        cx.notify();
                    }))
                    .child(widgets::dot(if node.verified {
                        theme::success()
                    } else if node.active {
                        accent
                    } else {
                        theme::text_muted()
                    }))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(theme::BODY))
                            .text_color(theme::text_secondary())
                            .child(node.label.clone()),
                    )
                    .child(icon(
                        if expanded {
                            "chevron-down"
                        } else {
                            "chevron-right"
                        },
                        14.0,
                        theme::text_muted(),
                    )),
            );
        if expanded {
            card = card.child(
                div()
                    .px(px(12.0))
                    .pb(px(12.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .text_size(px(theme::MICRO))
                    .line_height(px(18.0))
                    .text_color(theme::text_muted())
                    .child(node.detail.clone())
                    .child(format!(
                        "Node {}{} · {}ms · ${:.4}",
                        node.id,
                        if node.active { " · active" } else { "" },
                        node.latency_ms.unwrap_or(0),
                        node.cost_micros.unwrap_or(0) as f64 / 1_000_000.0
                    )),
            );
        }
        list = list.child(card);
    }
    list.child(
        div()
            .flex()
            .flex_wrap()
            .gap(px(4.0))
            .child(
                ui_kit::button("dag-rollback", "Revert to verified").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.state.rollback();
                        cx.notify();
                    },
                )),
            )
            .child(
                ui_kit::button("dag-tournament", "Tournament").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.state.run_tournament();
                        cx.notify();
                    },
                )),
            ),
    )
}
