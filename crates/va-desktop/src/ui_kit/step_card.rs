use crate::{state::ExecutionStep, theme, widgets};
use gpui::prelude::*;
use gpui::{div, px, Div, ElementId, FontWeight, Stateful};

pub struct CollapsibleStepCard;

impl CollapsibleStepCard {
    pub fn header(id: impl Into<ElementId>, step: &ExecutionStep) -> Stateful<Div> {
        let color = match step.status.as_str() {
            "off" => theme::danger(),
            "gated" => theme::warning(),
            "armed" | "ready" => theme::success(),
            _ => theme::phase_color(step.phase.as_str()),
        };
        let timing = if step.latency_ms > 0 || step.cost_micros > 0 {
            format!(
                "{}ms · ${:.4}",
                step.latency_ms,
                step.cost_micros as f64 / 1_000_000.0
            )
        } else {
            step.status.clone()
        };
        div()
            .id(id)
            .h(px(34.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(10.0))
            .rounded(px(6.0))
            .cursor_pointer()
            .hover(|style| style.bg(theme::elevated_alt()))
            .active(|style| style.bg(theme::border()))
            .child(widgets::dot(color))
            .child(
                div()
                    .text_size(px(theme::BODY))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme::text_primary())
                    .child(step.phase.label()),
            )
            .child(div().flex_1())
            .child(
                div()
                    .text_size(px(theme::MICRO))
                    .text_color(theme::text_muted())
                    .child(timing),
            )
            .child(
                div()
                    .text_color(theme::text_muted())
                    .child(if step.expanded { "⌄" } else { "›" }),
            )
    }
}
