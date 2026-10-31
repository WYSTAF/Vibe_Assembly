//! Native GPUI primitives. Callers own behavior; the kit owns presentation.

mod composer;
mod input;
mod navigation;
mod step_card;

pub use composer::CursorStyleComposer;
pub use input::{capture, InputSelection};
pub(crate) use input::{from_utf16, to_utf16};
pub use navigation::{FileTreeRow, SegmentedPillBar, SymbolRow};
pub use step_card::CollapsibleStepCard;

use crate::design::icon::icon;
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
            .px(px(8.0))
            .h(px(22.0))
            .flex_shrink_0()
            .rounded_full()
            .border_1()
            .border_color(theme::wash(color, 0.24))
            .bg(theme::wash(color, 0.10))
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
        .h(px(28.0))
        .px(px(10.0))
        .flex_shrink_0()
        .rounded(px(8.0))
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

/// Square icon button. Icons are embedded Lucide SVGs painted as alpha masks.
pub fn icon_button(id: impl Into<ElementId>, name: &'static str, selected: bool) -> Stateful<Div> {
    let color = if selected {
        theme::brand_ink()
    } else {
        theme::text_muted()
    };
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .size(px(28.0))
        .flex_shrink_0()
        .rounded(px(7.0))
        .bg(if selected {
            theme::wash(theme::brand_soft(), 1.0)
        } else {
            theme::wash(theme::sunken(), 0.0)
        })
        .cursor_pointer()
        .hover(|style| style.bg(theme::elevated_alt()))
        .child(icon(name, 15.0, color))
}

pub fn empty_state(glyph: impl IntoElement, title: &str, detail: &str) -> Div {
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
                .size(px(48.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(14.0))
                .bg(theme::brand_soft())
                .text_size(px(22.0))
                .text_color(theme::brand_ink())
                .child(glyph),
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
