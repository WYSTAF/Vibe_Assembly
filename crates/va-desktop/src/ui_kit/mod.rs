//! Native GPUI primitives. Callers own behavior; the kit owns presentation.

mod composer;
mod input;
mod navigation;
mod step_card;

pub use composer::CursorStyleComposer;
pub use input::{capture, InputSelection};
pub use navigation::{FileTreeRow, SegmentedPillBar, SymbolRow};
pub use step_card::CollapsibleStepCard;

use crate::theme;
use gpui::prelude::*;
use gpui::{div, px, Div, ElementId, FontWeight, Rgba, Stateful};

pub struct TintedBadge;

impl TintedBadge {
    pub fn render(label: impl Into<String>, color: Rgba) -> Div {
        div()
            .flex()
            .items_center()
            .gap(px(5.0))
            .px(px(6.0))
            .h(px(20.0))
            .flex_shrink_0()
            .rounded(px(4.0))
            .border_1()
            .border_color(theme::wash(color, 0.28))
            .bg(theme::wash(color, 0.12))
            .text_size(px(theme::MICRO))
            .font_weight(FontWeight::MEDIUM)
            .text_color(color)
            .child(label.into())
    }
}

pub struct ContextChip;

impl ContextChip {
    pub fn render(label: impl Into<String>, color: Rgba) -> Div {
        TintedBadge::render(label, color)
            .max_w(px(235.0))
            .overflow_hidden()
            .font_family(theme::code_font())
    }
}

pub fn button(id: impl Into<ElementId>, label: impl Into<String>) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .h(px(26.0))
        .px(px(10.0))
        .flex_shrink_0()
        .rounded(px(6.0))
        .text_size(px(theme::BODY))
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme::text_secondary())
        .cursor_pointer()
        .hover(|style| {
            style
                .bg(theme::elevated_alt())
                .text_color(theme::text_primary())
        })
        .active(|style| style.bg(theme::border()).text_color(theme::text_primary()))
        .child(label.into())
}

pub fn icon_button(id: impl Into<ElementId>, glyph: &str, selected: bool) -> Stateful<Div> {
    button(id, glyph).w(px(28.0)).px_0().when(selected, |this| {
        this.bg(theme::elevated_alt())
            .text_color(theme::text_primary())
    })
}

pub fn empty_state(glyph: &str, title: &str, detail: &str) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(10.0))
        .p(px(24.0))
        .text_center()
        .child(
            div()
                .size(px(44.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(12.0))
                .bg(theme::elevated())
                .text_size(px(22.0))
                .text_color(theme::text_muted())
                .child(glyph.to_string()),
        )
        .child(
            div()
                .text_size(px(theme::HEADING))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme::text_primary())
                .child(title.to_string()),
        )
        .child(
            div()
                .max_w(px(340.0))
                .text_size(px(theme::BODY))
                .line_height(px(20.0))
                .text_color(theme::text_muted())
                .child(detail.to_string()),
        )
}
