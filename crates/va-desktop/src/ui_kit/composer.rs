use crate::theme;
use gpui::prelude::*;
use gpui::{div, point, px, BoxShadow, Div, Rgba};

pub struct CursorStyleComposer;

impl CursorStyleComposer {
    pub fn surface(focused: bool, accent: Rgba) -> Div {
        div()
            .m(px(12.0))
            .mt_0()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .p(px(12.0))
            .rounded(px(12.0))
            .bg(theme::elevated())
            .border_1()
            .border_color(if focused {
                theme::wash(accent, 0.55)
            } else {
                theme::wash(theme::border_strong(), 1.0)
            })
            .when(focused, |this| {
                this.shadow(vec![BoxShadow {
                    color: theme::wash(accent, 0.08),
                    offset: point(px(0.0), px(0.0)),
                    blur_radius: px(16.0),
                    spread_radius: px(2.0),
                }])
            })
    }
}
