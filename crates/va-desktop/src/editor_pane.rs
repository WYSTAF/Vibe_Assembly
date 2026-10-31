//! Right-hand Changes panel: open tabs, the block-hash code view, and the
//! inline patch preview. The conversation owns the center; this panel is where
//! code and diffs appear when you want to inspect them.

use gpui::prelude::*;
use gpui::{div, px, Context, Div, FontWeight, Rgba};

use crate::design::icon::icon;
use crate::state::{EditorView, Workbench};
use crate::widgets;
use crate::{theme, ui_kit};
use ui_kit::SegmentedPillBar;

pub fn render(
    workbench: &Workbench,
    accent: Rgba,
    _accent_strong: Rgba,
    cx: &mut Context<Workbench>,
) -> Div {
    let state = &workbench.state;

    let tabs = if state.open_buffers.is_empty() {
        div()
            .flex()
            .items_center()
            .px(px(12.0))
            .text_size(px(theme::BODY))
            .text_color(theme::text_muted())
            .child("No open files")
    } else {
        let mut strip = div().flex().items_center().h_full();
        for (index, buffer) in state.open_buffers.iter().enumerate() {
            let active = state.active_tab == Some(index);
            let name = buffer
                .rel_path
                .rsplit('/')
                .next()
                .unwrap_or(&buffer.rel_path)
                .to_string();
            let glyph = widgets::file_glyph(&name);
            strip = strip.child(
                div()
                    .id(("editor-tab", index))
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .px(px(10.0))
                    .bg(if active {
                        theme::editor()
                    } else {
                        theme::sunken()
                    })
                    .border_t_2()
                    .when(active, |this| this.border_color(theme::brand()))
                    .border_r_1()
                    .text_size(px(theme::MICRO))
                    .text_color(if active {
                        theme::text_primary()
                    } else {
                        theme::text_muted()
                    })
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.state.active_tab = Some(index);
                        this.state.active_block = None;
                        this.state.cursor_line = 1;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .font_family(theme::code_font())
                            .text_color(theme::file_color(&name))
                            .child(glyph),
                    )
                    .child(div().max_w(px(130.0)).truncate().child(name))
                    .child(
                        div()
                            .id(("editor-tab-close", index))
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(16.0))
                            .rounded(px(4.0))
                            .cursor_pointer()
                            .hover(|style| {
                                style.bg(theme::danger_soft()).text_color(theme::danger())
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.state.close_tab(index);
                                cx.stop_propagation();
                                cx.notify();
                            }))
                            .child(icon(
                                "x",
                                12.0,
                                if active {
                                    theme::text_secondary()
                                } else {
                                    theme::text_muted()
                                },
                            )),
                    ),
            );
        }
        strip
    };

    let view_toggle = SegmentedPillBar::new()
        .item(
            SegmentedPillBar::segment(
                "view-code",
                "code",
                "Code",
                state.editor_view == EditorView::Code,
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.state.set_editor_view(EditorView::Code);
                cx.notify();
            })),
        )
        .item(
            SegmentedPillBar::segment(
                "view-diff",
                "scale",
                "Diff",
                state.editor_view == EditorView::Diff,
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.state.set_editor_view(EditorView::Diff);
                cx.notify();
            })),
        )
        .build();

    let header = div()
        .h(px(46.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(14.0))
        .border_b_1()
        .border_color(theme::border())
        .child(icon("file-code", 15.0, theme::text_muted()))
        .child(
            div()
                .text_size(px(theme::HEADING))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme::text_primary())
                .child("Changes"),
        )
        .when(!state.open_buffers.is_empty(), |this| {
            this.child(ui_kit::TintedBadge::render(
                format!("{}", state.open_buffers.len()),
                theme::text_muted(),
            ))
        })
        .child(div().flex_1())
        .child(view_toggle);

    let tab_bar = div()
        .h(px(34.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .bg(theme::sunken())
        .border_b_1()
        .border_color(theme::border())
        .child(
            div()
                .flex()
                .items_center()
                .h_full()
                .min_w_0()
                .overflow_hidden()
                .child(tabs),
        );

    let body = if state.active_buffer().is_none() {
        no_file_state()
    } else if state.editor_view == EditorView::Diff {
        render_diff(workbench, accent, cx)
    } else {
        render_code(workbench, accent, cx)
    };

    div()
        .w(px(theme::ASSISTANT))
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .bg(theme::editor())
        .border_l_1()
        .border_color(theme::border())
        .child(header)
        .child(tab_bar)
        .child(body)
}

fn render_code(workbench: &Workbench, accent: Rgba, cx: &mut Context<Workbench>) -> Div {
    let state = &workbench.state;
    let Some(buffer) = state.active_buffer() else {
        return no_file_state();
    };

    let line_count = buffer.lines.len();
    let mut block_at = vec![None; line_count + 1];
    for (block_index, block) in buffer.blocks.iter().enumerate() {
        let start = block.start_line.max(1);
        let end = block.end_line.min(line_count);
        for slot in block_at.iter_mut().take(end + 1).skip(start) {
            *slot = Some(block_index);
        }
    }
    let selected_block = state.current_block_index();
    let cursor_line = state.cursor_line;

    let info = div()
        .h(px(34.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(14.0))
        .text_size(px(theme::MICRO))
        .text_color(theme::text_muted())
        .child(
            div()
                .min_w_0()
                .truncate()
                .font_family(theme::code_font())
                .child(buffer.rel_path.clone()),
        )
        .when_some(state.active_symbol_name(), |this, symbol| {
            this.child(format!("› {symbol}"))
        })
        .child(div().flex_1())
        .child(
            div()
                .text_color(theme::text_muted())
                .child("Pin a block for context"),
        );

    let width = buffer
        .lines
        .iter()
        .map(|line| line.replace('\t', "    ").chars().count())
        .max()
        .unwrap_or(0) as f32
        * 7.6
        + 150.0;
    let mut rows = div().flex().flex_col().py(px(8.0)).min_w(px(width));
    for (line_index, text) in buffer.lines.iter().enumerate().take(1_400) {
        let line_number = line_index + 1;
        let block_index = block_at.get(line_number).copied().flatten();
        let is_block_start = block_index
            .and_then(|index| buffer.blocks.get(index))
            .map(|block| block.start_line == line_number)
            .unwrap_or(false);
        let is_selected_block = block_index.is_some() && block_index == selected_block;
        let is_cursor = line_number == cursor_line;
        let trimmed = text.trim_start();
        let text_color = if trimmed.starts_with("//") || trimmed.starts_with("<!--") {
            theme::text_muted()
        } else {
            theme::text_primary()
        };
        let display = text.replace('\t', "    ");
        let hash = if is_block_start {
            block_index.and_then(|index| buffer.blocks.get(index).map(|block| block.hash))
        } else {
            None
        };

        rows =
            rows.child(
                div()
                    .id(("code-line", line_number))
                    .group("code-row")
                    .hover(|style| style.bg(theme::elevated()))
                    .h(px(22.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .bg(if is_selected_block {
                        theme::wash(theme::brand(), 0.14)
                    } else if is_cursor {
                        theme::wash(theme::text_muted(), 0.06)
                    } else {
                        theme::wash(theme::editor(), 1.0)
                    })
                    .border_l_2()
                    .when(is_selected_block, |this| this.border_color(theme::brand()))
                    .cursor_text()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.state.cursor_line = line_number;
                        this.state.active_block = block_index;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .w(px(130.0))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .pr(px(10.0))
                            .child(
                                div()
                                    .w(px(32.0))
                                    .flex_shrink_0()
                                    .text_right()
                                    .font_family(theme::code_font())
                                    .text_size(px(theme::MICRO))
                                    .text_color(if is_cursor {
                                        theme::text_secondary()
                                    } else {
                                        theme::text_muted()
                                    })
                                    .child(line_number.to_string()),
                            )
                            .child(div().w(px(84.0)).flex_shrink_0().when_some(
                                hash,
                                |this, hash| {
                                    this.child(
                                        div()
                                            .id(("gutter-hash", line_number))
                                            .opacity(
                                                if state
                                                    .pinned_block
                                                    .as_ref()
                                                    .is_some_and(|(_, pinned)| *pinned == hash)
                                                {
                                                    1.0
                                                } else {
                                                    0.0
                                                },
                                            )
                                            .group_hover("code-row", |style| style.opacity(1.0))
                                            .cursor_pointer()
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                if let Some(index) = block_index {
                                                    this.state.select_block(index);
                                                }
                                                this.restore_focus(window);
                                                cx.stop_propagation();
                                                cx.notify();
                                            }))
                                            .child(widgets::hash_badge(hash, accent)),
                                    )
                                },
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .whitespace_nowrap()
                            .text_size(px(theme::BODY))
                            .font_family(theme::code_font())
                            .text_color(text_color)
                            .child(display),
                    ),
            );
    }

    if line_count > 1_400 {
        rows = rows.child(
            div()
                .px(px(14.0))
                .py(px(8.0))
                .text_size(px(theme::MICRO))
                .text_color(theme::text_muted())
                .child(format!(
                    "… editor slice capped at 1,400 of {line_count} lines …"
                )),
        );
    }

    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(info)
        .child(
            div()
                .id("editor-scroll")
                .flex_1()
                .min_h_0()
                .overflow_scroll()
                .bg(theme::editor())
                .child(rows),
        )
}

fn render_diff(workbench: &Workbench, accent: Rgba, cx: &mut Context<Workbench>) -> Div {
    let state = &workbench.state;
    let Some(patch) = state.patch_preview.as_ref() else {
        return div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(10.0))
            .bg(theme::editor())
            .child(
                div()
                    .text_size(px(theme::HEADING))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme::text_primary())
                    .child("No inline patch staged"),
            )
            .child(
                div()
                    .w(px(440.0))
                    .text_size(px(theme::MICRO))
                    .line_height(px(18.0))
                    .text_center()
                    .text_color(theme::text_muted())
                    .child("Open a file with a working-tree or HEAD diff, or select a block in Code view. The desktop surface never fabricates a patch."),
            )
            .child(
                div()
                    .id("diff-back-to-code")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.set_editor_view(EditorView::Code);
                        cx.notify();
                    }))
                    .child(widgets::ghost_button("Back to code")),
            );
    };

    let source_color = match patch.source {
        crate::state::PatchSource::WorkingTree => theme::warning(),
        crate::state::PatchSource::RecentCommit => theme::info(),
        crate::state::PatchSource::Session => theme::text_muted(),
    };

    let header = div()
        .h(px(46.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(14.0))
        .border_b_1()
        .border_color(theme::border())
        .child(ui_kit::TintedBadge::render(
            patch.source.label(),
            source_color,
        ))
        .child(
            div()
                .min_w_0()
                .truncate()
                .font_family(theme::code_font())
                .text_size(px(theme::MICRO))
                .text_color(theme::text_secondary())
                .child(patch.rel_path.clone()),
        );
    let header = header.when_some(patch.block_hash, |this, hash| {
        this.child(widgets::hash_badge(hash, accent))
    });

    let toolbar = div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .px(px(14.0))
        .py(px(8.0))
        .flex_shrink_0()
        .border_b_1()
        .border_color(theme::border())
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(theme::MICRO))
                .text_color(theme::text_muted())
                .child(patch.summary.clone()),
        )
        .child(
            div()
                .id("diff-judge")
                .flex()
                .items_center()
                .gap(px(5.0))
                .h(px(26.0))
                .px(px(8.0))
                .rounded(px(7.0))
                .text_size(px(theme::MICRO))
                .text_color(theme::text_secondary())
                .cursor_pointer()
                .hover(|style| style.bg(theme::elevated_alt()))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.state.run_judge_preview();
                    cx.notify();
                }))
                .child(icon("scale", 13.0, theme::text_muted()))
                .child("Judge"),
        )
        .child(
            div()
                .id("diff-rollback")
                .flex()
                .items_center()
                .gap(px(5.0))
                .h(px(26.0))
                .px(px(9.0))
                .rounded(px(7.0))
                .text_size(px(theme::MICRO))
                .text_color(theme::text_secondary())
                .cursor_pointer()
                .hover(|style| style.bg(theme::elevated_alt()))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.state.rollback();
                    cx.notify();
                }))
                .child(icon("history", 13.0, theme::text_muted()))
                .child("Revert"),
        )
        .child(
            div()
                .id("diff-accept")
                .flex()
                .items_center()
                .gap(px(5.0))
                .h(px(28.0))
                .px(px(10.0))
                .rounded(px(8.0))
                .bg(theme::brand())
                .text_size(px(theme::MICRO))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme::text_primary())
                .cursor_pointer()
                .hover(|style| style.bg(theme::brand_hover()))
                .active(|style| style.bg(theme::brand_press()))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.state.accept_patch();
                    cx.notify();
                }))
                .child(icon("check", 13.0, theme::text_primary()))
                .child("Accept changes"),
        );

    let body = if patch.lines.is_empty() {
        div()
            .id("diff-summary")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(10.0))
            .child(
                div()
                    .text_size(px(theme::HEADING))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme::text_primary())
                    .child("Session DAG patch summary"),
            )
            .child(
                div()
                    .w(px(440.0))
                    .text_size(px(theme::MICRO))
                    .line_height(px(18.0))
                    .text_center()
                    .text_color(theme::text_muted())
                    .child("The immutable DAG retains the patch summary and cost, not line-level replacement text. Open the file in Code view to inspect its live block hashes."),
            )
    } else {
        let mut lines = div().flex().flex_col().py(px(8.0));
        for (index, line) in patch.lines.iter().enumerate() {
            let (bg, fg) = match line.marker {
                '+' => (theme::diff_add_bg(), theme::diff_add_text()),
                '-' => (theme::diff_delete_bg(), theme::diff_delete_text()),
                _ => (theme::editor(), theme::text_secondary()),
            };
            let prefix = if line.marker == ' ' {
                " ".to_string()
            } else {
                line.marker.to_string()
            };
            lines = lines.child(
                div()
                    .id(("diff-line", index))
                    .h(px(20.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .bg(bg)
                    .text_size(px(11.0))
                    .font_family(theme::code_font())
                    .text_color(fg)
                    .child(
                        div()
                            .w(px(18.0))
                            .flex_shrink_0()
                            .text_color(fg)
                            .child(prefix),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .whitespace_nowrap()
                            .child(line.text.replace('\t', "    ")),
                    ),
            );
        }
        div()
            .id("diff-scroll")
            .flex_1()
            .min_h_0()
            .overflow_scroll()
            .bg(theme::editor())
            .child(lines)
    };

    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .bg(theme::editor())
        .child(header)
        .child(toolbar)
        .child(body)
}

fn no_file_state() -> Div {
    ui_kit::empty_state(
        icon("file-code", 22.0, theme::brand_ink()),
        "No file open",
        "Pick a file on the left, or press Ctrl+P to find a file, symbol, or command.",
    )
    .flex_1()
    .min_h_0()
}
