//! Center editor: tab strip, block-hash code view, and inline git diff preview.

use gpui::prelude::*;
use gpui::{div, px, Context, Div, Rgba};

use crate::state::{EditorView, Workbench};
use crate::theme;
use crate::widgets;

pub fn render(
    workbench: &Workbench,
    accent: Rgba,
    accent_strong: Rgba,
    cx: &mut Context<Workbench>,
) -> Div {
    let state = &workbench.state;

    let tabs = if state.open_buffers.is_empty() {
        div()
            .flex()
            .items_center()
            .px_2()
            .text_size(px(10.0))
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
                    .gap_1()
                    .px_2()
                    .bg(if active {
                        theme::editor()
                    } else {
                        theme::sunken()
                    })
                    .border_t_2()
                    .border_color(if active { accent } else { theme::sunken() })
                    .text_size(px(10.0))
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
                            .text_size(px(8.0))
                            .font_family("Consolas")
                            .text_color(if active { accent } else { theme::text_muted() })
                            .child(glyph),
                    )
                    .child(div().max_w(px(150.0)).truncate().child(name))
                    .child(
                        div()
                            .id(("editor-tab-close", index))
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(16.0))
                            .rounded(px(2.0))
                            .text_size(px(11.0))
                            .text_color(theme::text_muted())
                            .cursor_pointer()
                            .hover(|style| {
                                style
                                    .bg(theme::wash(theme::danger(), 0.18))
                                    .text_color(theme::danger())
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.state.close_tab(index);
                                cx.stop_propagation();
                                cx.notify();
                            }))
                            .child("×"),
                    ),
            );
        }
        strip
    };

    let view_toggle = div()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .child(
            div()
                .id("view-code")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.state.set_editor_view(EditorView::Code);
                    cx.notify();
                }))
                .child(widgets::pill(
                    "Code",
                    state.editor_view == EditorView::Code,
                    accent,
                )),
        )
        .child(
            div()
                .id("view-diff")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.state.set_editor_view(EditorView::Diff);
                    cx.notify();
                }))
                .child(widgets::pill(
                    "Diff",
                    state.editor_view == EditorView::Diff,
                    accent,
                )),
        );

    let tab_bar = div()
        .h(px(32.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
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
        )
        .child(view_toggle);

    let body = if state.active_buffer().is_none() {
        no_file_state()
    } else if state.editor_view == EditorView::Diff {
        render_diff(workbench, accent, accent_strong, cx)
    } else {
        render_code(workbench, accent, cx)
    };

    div()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .flex()
        .flex_col()
        .bg(theme::editor())
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
        .h(px(24.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .px_2()
        .bg(theme::editor())
        .border_b_1()
        .border_color(theme::border())
        .text_size(px(9.0))
        .font_family("Consolas")
        .text_color(theme::text_muted())
        .child(format!(
            "{} · {} lines · {} bytes · {} blocks",
            buffer.rel_path,
            line_count,
            buffer.content.len(),
            buffer.blocks.len()
        ))
        .child(format!("Ln {} · FNV-1a 64-bit anchors", cursor_line));

    let mut rows = div().flex().flex_col().py_1();
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
        let text_color = if trimmed.starts_with('#') {
            accent
        } else if trimmed.starts_with("//") || trimmed.starts_with("<!--") {
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

        rows = rows.child(
            div()
                .id(("code-line", line_number))
                .h(px(19.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .bg(if is_selected_block {
                    theme::wash(accent, 0.09)
                } else if is_cursor {
                    theme::wash(theme::text_muted(), 0.06)
                } else {
                    theme::wash(theme::editor(), 1.0)
                })
                .border_l_2()
                .border_color(if is_selected_block {
                    theme::wash(accent, 1.0)
                } else {
                    theme::wash(theme::text_muted(), 0.0)
                })
                .cursor_text()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.state.cursor_line = line_number;
                    this.state.active_block = block_index;
                    cx.notify();
                }))
                .child(
                    div()
                        .w(px(82.0))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap_1()
                        .pr_1()
                        .text_size(px(9.0))
                        .font_family("Consolas")
                        .text_color(if is_block_start {
                            accent
                        } else {
                            theme::text_muted()
                        })
                        .child(line_number.to_string())
                        .when_some(hash, |this, hash| {
                            this.child(widgets::hash_badge(hash, accent))
                        }),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .whitespace_nowrap()
                        .text_size(px(11.5))
                        .font_family("Consolas")
                        .text_color(text_color)
                        .child(display),
                ),
        );
    }

    if line_count > 1_400 {
        rows = rows.child(
            div()
                .px_2()
                .py_1()
                .text_size(px(9.0))
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

fn render_diff(
    workbench: &Workbench,
    accent: Rgba,
    _accent_strong: Rgba,
    cx: &mut Context<Workbench>,
) -> Div {
    let state = &workbench.state;
    let Some(patch) = state.patch_preview.as_ref() else {
        return div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .bg(theme::editor())
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(theme::text_secondary())
                    .child("No inline patch staged"),
            )
            .child(
                div()
                    .w(px(440.0))
                    .text_size(px(11.0))
                    .line_height(px(17.0))
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
                    .child(widgets::ghost_button("Back to Code")),
            );
    };

    let source_color = match patch.source {
        crate::state::PatchSource::WorkingTree => theme::warning(),
        crate::state::PatchSource::RecentCommit => theme::info(),
        crate::state::PatchSource::Session => theme::text_muted(),
    };

    let header = div()
        .h(px(32.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .px_2()
        .bg(theme::sunken())
        .border_b_1()
        .border_color(theme::border())
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .min_w_0()
                .child(widgets::pill(patch.source.label(), true, source_color))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(10.0))
                        .font_family("Consolas")
                        .text_color(theme::text_secondary())
                        .child(patch.rel_path.clone()),
                )
                .when_some(patch.block_hash, |this, hash| {
                    this.child(widgets::hash_badge(hash, accent))
                }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .flex_shrink_0()
                .child(
                    div()
                        .id("diff-accept")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.state.accept_patch();
                            cx.notify();
                        }))
                        .child(widgets::ghost_button("Accept Patch")),
                )
                .child(
                    div()
                        .id("diff-rollback")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.state.rollback();
                            cx.notify();
                        }))
                        .child(widgets::ghost_button("Rollback")),
                )
                .child(
                    div()
                        .id("diff-judge")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.state.run_judge_preview();
                            cx.notify();
                        }))
                        .child(widgets::pill("Run Judge", true, accent)),
                ),
        );

    let summary = div()
        .px_2()
        .py_1()
        .flex_shrink_0()
        .border_b_1()
        .border_color(theme::border())
        .text_size(px(9.0))
        .text_color(theme::text_muted())
        .child(patch.summary.clone());

    let body = if patch.lines.is_empty() {
        div()
            .id("diff-summary")
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme::text_secondary())
                    .child("Session DAG patch summary"),
            )
            .child(
                div()
                    .w(px(440.0))
                    .text_size(px(10.0))
                    .line_height(px(16.0))
                    .text_color(theme::text_muted())
                    .child("The immutable DAG retains the patch summary and cost, not line-level replacement text. Open the file in Code view to inspect its live block hashes."),
            )
    } else {
        let mut lines = div().flex().flex_col().py_1();
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
                    .h(px(19.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .bg(bg)
                    .text_size(px(11.0))
                    .font_family("Consolas")
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
        .child(summary)
        .child(body)
}

fn no_file_state() -> Div {
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .bg(theme::editor())
        .child(
            div()
                .size(px(42.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(8.0))
                .border_1()
                .border_color(theme::border())
                .bg(theme::elevated())
                .text_size(px(15.0))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme::text_muted())
                .child("VA"),
        )
        .child(
            div()
                .text_size(px(15.0))
                .text_color(theme::text_secondary())
                .child("Open a file from the project dock"),
        )
        .child(
            div()
                .w(px(420.0))
                .text_size(px(10.0))
                .line_height(px(16.0))
                .text_color(theme::text_muted())
                .child("Tabs keep real workspace buffers. Each code block carries the same 64-bit FNV-1a hash used by va-harness::patcher, shown as eight characters."),
        )
}
