use crate::theme;
use gpui::prelude::*;
use gpui::{div, px, Div, Rgba};

pub struct CursorStyleComposer;

impl CursorStyleComposer {
    pub fn surface(focused: bool, accent: Rgba) -> Div {
        div()
            .m(px(16.0))
            .mt_0()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .p(px(12.0))
            .rounded(px(16.0))
            .bg(theme::elevated())
            .border_1()
            .border_color(if focused {
                theme::wash(accent, 0.55)
            } else {
                theme::wash(theme::border_strong(), 1.0)
            })
            .shadow(crate::widgets::lifted_shadow())
    }
}
