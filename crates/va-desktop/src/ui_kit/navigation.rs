use crate::{theme, widgets};
use gpui::prelude::*;
use gpui::{div, px, Div, ElementId, FontWeight, Stateful};

pub struct SegmentedPillBar {
    element: Div,
}

impl SegmentedPillBar {
    pub fn new() -> Self {
        Self {
            element: div()
                .flex()
                .items_center()
                .gap(px(2.0))
                .p(px(3.0))
                .rounded(px(8.0))
                .bg(theme::editor()),
        }
    }
    pub fn item(mut self, item: impl IntoElement) -> Self {
        self.element = self.element.child(item);
        self
    }
    pub fn build(self) -> Div {
        self.element
    }
    pub fn segment(
        id: impl Into<ElementId>,
        glyph: &str,
        label: &str,
        active: bool,
    ) -> Stateful<Div> {
        super::button(id, "")
            .h(px(24.0))
            .px(px(8.0))
            .text_size(px(11.5))
            .bg(if active {
                theme::border()
            } else {
                theme::editor()
            })
            .text_color(if active {
                theme::text_primary()
            } else {
                theme::text_muted()
            })
            .child(div().text_size(px(12.0)).child(glyph.to_string()))
            .child(label.to_string())
    }
}

pub struct FileTreeRow;

impl FileTreeRow {
    pub fn render(
        id: impl Into<ElementId>,
        name: &str,
        depth: usize,
        directory: bool,
        expanded: bool,
        selected: bool,
    ) -> Stateful<Div> {
        let mut row = div()
            .id(id)
            .relative()
            .h(px(theme::ROW))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(7.0))
            .pr(px(10.0))
            .pl(px(10.0 + depth as f32 * 14.0))
            .rounded(px(5.0))
            .bg(if selected {
                theme::elevated_alt()
            } else {
                theme::sunken()
            })
            .text_size(px(theme::BODY))
            .text_color(if selected {
                theme::text_primary()
            } else {
                theme::text_secondary()
            })
            .cursor_pointer()
            .hover(|style| style.bg(theme::elevated_alt()))
            .active(|style| style.bg(theme::border()));
        for level in 0..depth {
            row = row.child(
                div()
                    .absolute()
                    .left(px(16.0 + level as f32 * 14.0))
                    .top_0()
                    .h_full()
                    .w(px(1.0))
                    .bg(theme::border()),
            );
        }
        row.child(
            div()
                .w(px(22.0))
                .flex_shrink_0()
                .whitespace_nowrap()
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if directory {
                    theme::text_muted()
                } else {
                    theme::file_color(name)
                })
                .child(if directory {
                    if expanded {
                        "⌄"
                    } else {
                        "›"
                    }
                } else {
                    widgets::file_glyph(name)
                }),
        )
        .child(div().flex_1().min_w_0().truncate().child(name.to_string()))
        .when(selected, |this| {
            this.child(
                div()
                    .text_size(px(theme::MICRO))
                    .text_color(theme::text_muted())
                    .child("open"),
            )
        })
    }
}

pub struct SymbolRow;

impl SymbolRow {
    pub fn render(
        id: impl Into<ElementId>,
        kind: &str,
        name: &str,
        line: usize,
        selected: bool,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .h(px(theme::ROW))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(10.0))
            .rounded(px(5.0))
            .bg(if selected {
                theme::elevated_alt()
            } else {
                theme::sunken()
            })
            .cursor_pointer()
            .hover(|style| style.bg(theme::elevated_alt()))
            .active(|style| style.bg(theme::border()))
            .child(
                div()
                    .w(px(30.0))
                    .text_size(px(theme::MICRO))
                    .text_color(theme::symbol_color(kind))
                    .child(kind.to_string()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(theme::BODY))
                    .text_color(if selected {
                        theme::text_primary()
                    } else {
                        theme::text_secondary()
                    })
                    .child(name.to_string()),
            )
            .when(selected, |this| {
                this.child(
                    div()
                        .text_size(px(theme::MICRO))
                        .text_color(theme::text_muted())
                        .child(format!(":{line}")),
                )
            })
    }
}
