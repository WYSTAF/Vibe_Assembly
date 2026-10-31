//! Static sample of the Obsidian Gold workbench.
//!
//! Rendered by `--design-preview` and nothing else. Every surface here is a
//! candidate for migration once the direction is approved; nothing in the
//! shipped workbench reads this module yet.

use gpui::prelude::*;
use gpui::{
    div, px, rgb, Context, Div, FontWeight, IntoElement, Render, Rgba, Stateful, Window,
    WindowControlArea,
};
use va_core::modes::ExecutionMode;

use crate::branding;
use crate::design::icon::icon;
use crate::design::tokens as t;
use crate::theme;

// ------------------------------------------------------------------ syntax

fn syn_keyword() -> Rgba {
    rgb(0xFB7185)
}

fn syn_function() -> Rgba {
    rgb(0xF2C14E)
}

fn syn_type() -> Rgba {
    rgb(0x7DD3FC)
}

fn syn_comment() -> Rgba {
    rgb(0x928F85)
}

fn syn_punct() -> Rgba {
    t::text_secondary()
}

fn kw(text: &'static str) -> (&'static str, Rgba) {
    (text, syn_keyword())
}

fn fx(text: &'static str) -> (&'static str, Rgba) {
    (text, syn_function())
}

fn ty(text: &'static str) -> (&'static str, Rgba) {
    (text, syn_type())
}

fn st(text: &'static str) -> (&'static str, Rgba) {
    (text, rgb(0x86EFAC))
}

fn nu(text: &'static str) -> (&'static str, Rgba) {
    (text, rgb(0xFDBA74))
}

fn cm(text: &'static str) -> (&'static str, Rgba) {
    (text, syn_comment())
}

fn p(text: &'static str) -> (&'static str, Rgba) {
    (text, syn_punct())
}

// ----------------------------------------------------------------- helpers

fn dot(color: Rgba) -> Div {
    div().size(px(6.0)).rounded_full().flex_shrink_0().bg(color)
}

fn keycap(label: &str) -> Div {
    div()
        .h(px(18.0))
        .px(px(t::SPACE_SM))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(t::RADIUS_SM))
        .border_1()
        .border_color(t::border_strong())
        .bg(t::sunken())
        .text_size(px(10.0))
        .text_color(t::text_muted())
        .child(label.to_string())
}

fn icon_button(id: &'static str, name: &'static str, active: bool) -> Stateful<Div> {
    div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(t::HEIGHT_ICON_BTN))
        .h(px(t::HEIGHT_ICON_BTN))
        .rounded(px(t::RADIUS_MD))
        .cursor_pointer()
        .when(active, |this| this.bg(t::elevated_alt()))
        .id(id)
        .hover(|style| style.bg(t::elevated_alt()))
        .child(icon(
            name,
            14.0,
            if active {
                t::text_primary()
            } else {
                t::text_muted()
            },
        ))
}

fn ghost_button(id: &'static str, name: &'static str) -> Stateful<Div> {
    div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(22.0))
        .h(px(22.0))
        .rounded(px(t::RADIUS_SM))
        .cursor_pointer()
        .id(id)
        .hover(|style| style.bg(t::elevated_alt()))
        .child(icon(name, 13.0, t::text_muted()))
}

fn pill(id: &'static str, label: &str) -> Stateful<Div> {
    div()
        .flex()
        .items_center()
        .gap(px(t::SPACE_XS))
        .h(px(24.0))
        .px(px(t::SPACE_MD))
        .rounded(px(t::RADIUS_MD))
        .border_1()
        .border_color(t::border())
        .bg(t::elevated())
        .text_size(px(11.5))
        .text_color(t::text_secondary())
        .cursor_pointer()
        .id(id)
        .hover(|style| style.bg(t::elevated_alt()).text_color(t::text_primary()))
        .child(label.to_string())
}

// ---------------------------------------------------------------- titlebar

fn titlebar() -> Div {
    let separator = || div().w(px(1.0)).h(px(16.0)).flex_shrink_0().bg(t::border());
    let divider = || {
        div()
            .text_size(px(11.0))
            .text_color(t::wash(t::text_muted(), 0.55))
            .child("/")
    };

    div()
        .h(px(t::HEIGHT_TITLEBAR))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(t::SPACE_LG))
        .px(px(t::SPACE_LG))
        .bg(t::sunken())
        .border_b_1()
        .border_color(t::border())
        .text_color(t::text_secondary())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(t::SPACE_MD))
                .min_w_0()
                .child(
                    div()
                        .window_control_area(WindowControlArea::Drag)
                        .child(branding::logo(22.0)),
                )
                .child(pill("workspace", "Vibe_Assembly").child(icon(
                    "chevron-down",
                    12.0,
                    t::text_muted(),
                )))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(t::SPACE_SM))
                        .min_w_0()
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(t::text_muted())
                                .child("va-desktop"),
                        )
                        .child(divider())
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(t::text_primary())
                                .child("theme.rs"),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(t::SPACE_XS))
                                .h(px(22.0))
                                .px(px(t::SPACE_SM))
                                .rounded(px(t::RADIUS_SM))
                                .bg(t::elevated())
                                .child(icon("braces", 11.0, t::text_muted()))
                                .child(
                                    div()
                                        .font_family(theme::code_font())
                                        .text_size(px(10.5))
                                        .text_color(t::text_secondary())
                                        .child("init_fonts"),
                                ),
                        ),
                ),
        )
        .child(div().flex_1())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(t::SPACE_SM))
                .w(px(300.0))
                .h(px(t::HEIGHT_CONTROL))
                .px(px(t::SPACE_MD))
                .rounded(px(t::RADIUS_MD))
                .border_1()
                .border_color(t::border())
                .bg(t::elevated())
                .text_size(px(11.5))
                .text_color(t::text_muted())
                .child(icon("search", 13.0, t::text_muted()))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .child("Search or run a command…"),
                )
                .child(keycap("Ctrl K")),
        )
        .child(div().flex_1())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(t::SPACE_XS))
                .child(icon_button("toggle-left", "panel-left", true))
                .child(icon_button("toggle-bottom", "panel-bottom", true))
                .child(icon_button("toggle-right", "panel-right", true))
                .child(separator())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(t::SPACE_SM))
                        .h(px(22.0))
                        .px(px(t::SPACE_MD))
                        .rounded(px(t::RADIUS_MD))
                        .border_1()
                        .border_color(t::wash(t::success(), 0.20))
                        .bg(t::wash(t::success(), 0.08))
                        .text_size(px(10.5))
                        .text_color(t::success())
                        .child(dot(t::success()))
                        .child("Live"),
                )
                .child(separator())
                .child(window_control(
                    "window-min",
                    "minus",
                    WindowControlArea::Min,
                    false,
                ))
                .child(window_control(
                    "window-max",
                    "square",
                    WindowControlArea::Max,
                    false,
                ))
                .child(window_control(
                    "window-close",
                    "x",
                    WindowControlArea::Close,
                    true,
                )),
        )
}

fn window_control(
    id: &'static str,
    name: &'static str,
    area: WindowControlArea,
    destructive: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .window_control_area(area)
        .flex()
        .items_center()
        .justify_center()
        .w(px(34.0))
        .h(px(t::HEIGHT_ICON_BTN))
        .rounded(px(t::RADIUS_SM))
        .cursor_pointer()
        .hover(move |style| {
            if destructive {
                style.bg(t::wash(t::danger(), 0.16))
            } else {
                style.bg(t::elevated_alt())
            }
        })
        .child(icon(name, 12.0, t::text_muted()))
}

// -------------------------------------------------------------------- dock

fn dock(accent: Rgba) -> Div {
    div()
        .w(px(t::DOCK_WIDTH))
        .flex_shrink_0()
        .h_full()
        .flex()
        .flex_col()
        .bg(t::sunken())
        .border_r_1()
        .border_color(t::border())
        .child(
            div()
                .h(px(30.0))
                .px(px(t::SPACE_LG))
                .flex()
                .items_center()
                .justify_between()
                .text_size(px(10.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(t::text_muted())
                .child("EXPLORER")
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(2.0))
                        .child(ghost_button("new-file", "plus"))
                        .child(ghost_button("refresh-tree", "refresh-cw")),
                ),
        )
        .child(
            div().px(px(t::SPACE_LG)).pb(px(t::SPACE_MD)).child(
                div()
                    .flex()
                    .gap(px(2.0))
                    .p(px(2.0))
                    .rounded(px(8.0))
                    .bg(t::elevated())
                    .child(segment("Files", "files", true, accent))
                    .child(segment("Symbols", "braces", false, accent))
                    .child(segment("DAG", "git-branch", false, accent)),
            ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .py(px(t::SPACE_XS))
                .child(tree_row(
                    0.0,
                    "folder-open",
                    "src",
                    false,
                    Some("chevron-down"),
                    accent,
                ))
                .child(tree_row(14.0, "file-code", "theme.rs", true, None, accent))
                .child(tree_row(
                    14.0,
                    "file-code",
                    "widgets.rs",
                    false,
                    None,
                    accent,
                ))
                .child(tree_row(
                    14.0,
                    "folder",
                    "ui_kit",
                    false,
                    Some("chevron-right"),
                    accent,
                ))
                .child(tree_row(14.0, "file-code", "state.rs", false, None, accent))
                .child(tree_row(
                    0.0,
                    "folder",
                    "assets",
                    false,
                    Some("chevron-right"),
                    accent,
                ))
                .child(tree_row(
                    0.0,
                    "file-code",
                    "Cargo.toml",
                    false,
                    None,
                    accent,
                )),
        )
        .child(div().flex_1())
        .child(diffstat())
}

fn segment(label: &str, name: &'static str, active: bool, accent: Rgba) -> Stateful<Div> {
    div()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(t::SPACE_XS))
        .flex_1()
        .h(px(26.0))
        .rounded(px(6.0))
        .text_size(px(11.0))
        .when(active, |this| {
            this.bg(t::elevated_alt())
                .text_color(t::text_primary())
                .child(icon(name, 12.0, accent))
        })
        .when(!active, |this| {
            this.text_color(t::text_muted())
                .child(icon(name, 12.0, t::text_muted()))
        })
        .id(match label {
            "Files" => "seg-files",
            "Symbols" => "seg-symbols",
            _ => "seg-dag",
        })
        .hover(|style| style.text_color(t::text_secondary()))
        .child(label.to_string())
}

fn tree_row(
    indent: f32,
    name: &'static str,
    label: &str,
    active: bool,
    chevron: Option<&'static str>,
    accent: Rgba,
) -> Div {
    div()
        .relative()
        .flex()
        .items_center()
        .gap(px(t::SPACE_SM))
        .h(px(t::HEIGHT_ROW))
        .pl(px(t::SPACE_MD + indent))
        .pr(px(t::SPACE_MD))
        .text_size(px(t::TYPE_MICRO + 0.5))
        .when(active, |this| this.bg(t::wash(accent, 0.07)))
        .child(if let Some(chevron) = chevron {
            icon(chevron, 11.0, t::text_muted()).into_any_element()
        } else {
            div().w(px(11.0)).flex_shrink_0().into_any_element()
        })
        .child(icon(
            name,
            13.0,
            if active { accent } else { t::text_muted() },
        ))
        .child(
            div()
                .truncate()
                .text_color(if active {
                    t::text_primary()
                } else {
                    t::text_secondary()
                })
                .child(label.to_string()),
        )
        .when(active, |this| {
            this.child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .w(px(2.0))
                    .bg(accent),
            )
        })
}

fn diffstat() -> Div {
    div()
        .h(px(28.0))
        .px(px(t::SPACE_LG))
        .flex()
        .items_center()
        .gap(px(t::SPACE_SM))
        .border_t_1()
        .border_color(t::border())
        .child(
            div()
                .h(px(18.0))
                .px(px(t::SPACE_SM))
                .flex()
                .items_center()
                .rounded(px(t::RADIUS_SM))
                .bg(t::diff_add_bg())
                .text_size(px(10.5))
                .font_family(theme::code_font())
                .text_color(t::diff_add_text())
                .child("+128"),
        )
        .child(
            div()
                .h(px(18.0))
                .px(px(t::SPACE_SM))
                .flex()
                .items_center()
                .rounded(px(t::RADIUS_SM))
                .bg(t::diff_delete_bg())
                .text_size(px(10.5))
                .font_family(theme::code_font())
                .text_color(t::diff_delete_text())
                .child("−42"),
        )
        .child(div().flex_1())
        .child(
            div()
                .text_size(px(10.5))
                .text_color(t::text_muted())
                .child("3 files"),
        )
}

// ------------------------------------------------------------------ editor

fn center(accent: Rgba) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .flex()
        .flex_col()
        .child(tab_strip(accent))
        .child(editor(accent))
        .child(drawer(accent))
}

fn tab_strip(accent: Rgba) -> Div {
    div()
        .h(px(t::HEIGHT_TAB))
        .flex_shrink_0()
        .flex()
        .items_center()
        .bg(t::sunken())
        .child(tab("tab-theme", "theme.rs", true, accent, false))
        .child(tab("tab-state", "state.rs", false, accent, true))
        .child(tab("tab-widgets", "widgets.rs", false, accent, false))
        .child(div().flex_1())
        .child(icon_button("split-pane", "panel-right", false))
}

fn tab(id: &'static str, label: &str, active: bool, accent: Rgba, dirty: bool) -> Stateful<Div> {
    div()
        .relative()
        .h(px(t::HEIGHT_TAB))
        .px(px(t::SPACE_LG))
        .flex()
        .items_center()
        .gap(px(t::SPACE_SM))
        .border_r_1()
        .border_color(t::border())
        .text_size(px(t::TYPE_BODY))
        .when(active, |this| {
            this.bg(t::editor()).text_color(t::text_primary())
        })
        .when(!active, |this| this.text_color(t::text_muted()))
        .id(id)
        .when(!active, |this| {
            this.hover(|style| style.bg(t::elevated()).text_color(t::text_secondary()))
        })
        .child(icon(
            "file-code",
            13.0,
            if active { accent } else { t::text_muted() },
        ))
        .child(label.to_string())
        .child(if dirty {
            div()
                .size(px(7.0))
                .rounded_full()
                .bg(t::text_muted())
                .into_any_element()
        } else {
            icon("x", 12.0, t::text_muted()).into_any_element()
        })
        .when(active, |this| {
            this.child(
                div()
                    .absolute()
                    .top(px(0.0))
                    .left(px(0.0))
                    .right(px(0.0))
                    .h(px(2.0))
                    .bg(accent),
            )
        })
}

fn editor(accent: Rgba) -> Div {
    #[allow(clippy::type_complexity)]
    let lines: [(u32, bool, &[(&'static str, Rgba)]); 27] = [
        (
            1,
            false,
            &[kw("use "), p("std::"), p("sync::"), ty("OnceLock"), p(";")],
        ),
        (2, false, &[]),
        (
            3,
            false,
            &[
                kw("use "),
                p("gpui::{"),
                ty("hsla"),
                p(", "),
                ty("rgb"),
                p(", "),
                ty("App"),
                p(", "),
                ty("Hsla"),
                p(", "),
                ty("Rgba"),
                p("};"),
            ],
        ),
        (
            4,
            false,
            &[
                kw("use "),
                p("va_core::modes::"),
                ty("ExecutionMode"),
                p(";"),
            ],
        ),
        (5, false, &[]),
        (6, false, &[cm("// Obsidian Gold — warm black surfaces,")]),
        (
            7,
            false,
            &[cm("// brand yellow as the only saturated hue.")],
        ),
        (8, false, &[]),
        (
            9,
            false,
            &[
                kw("pub fn "),
                fx("sunken"),
                p("() -> "),
                ty("Rgba"),
                p(" {"),
            ],
        ),
        (
            10,
            false,
            &[p("    "), fx("rgb"), p("("), nu("0x0B0B0A"), p(")")],
        ),
        (11, false, &[p("}")]),
        (12, false, &[]),
        (
            13,
            false,
            &[kw("pub fn "), fx("brand"), p("() -> "), ty("Rgba"), p(" {")],
        ),
        (
            14,
            false,
            &[p("    "), fx("rgb"), p("("), nu("0xFDCC02"), p(")")],
        ),
        (15, false, &[p("}")]),
        (16, false, &[]),
        (
            17,
            true,
            &[
                kw("pub fn "),
                fx("init_fonts"),
                p("(cx: &"),
                ty("App"),
                p(") {"),
            ],
        ),
        (
            18,
            true,
            &[
                p("    "),
                kw("let "),
                p("fonts = cx."),
                fx("text_system"),
                p("()."),
                fx("all_font_names"),
                p("();"),
            ],
        ),
        (
            19,
            true,
            &[
                p("    "),
                kw("let "),
                p("choose = |choices: &["),
                p("&"),
                st("'static"),
                p(" "),
                ty("str"),
                p("], fallback| {"),
            ],
        ),
        (20, true, &[p("        "), p("choices")]),
        (21, true, &[p("            ."), fx("iter"), p("()")]),
        (22, true, &[p("            ."), fx("copied"), p("()")]),
        (
            23,
            true,
            &[
                p("            ."),
                fx("find"),
                p("(|font| fonts."),
                fx("iter"),
                p("()."),
                fx("any"),
                p("(|f| f == font))"),
            ],
        ),
        (
            24,
            true,
            &[p("            ."), fx("unwrap_or"), p("(fallback)")],
        ),
        (25, true, &[p("    };")]),
        (
            26,
            true,
            &[
                p("    "),
                kw("let "),
                p("_ = "),
                ty("UI_FONT"),
                p("."),
                fx("set"),
                p("(choose(&["),
                st("\"Inter\""),
                p("], "),
                st("\".SystemUIFont\""),
                p("));"),
            ],
        ),
        (27, true, &[p("}")]),
    ];

    div()
        .flex_1()
        .min_h_0()
        .overflow_hidden()
        .flex()
        .flex_col()
        .bg(t::editor())
        .pt(px(t::SPACE_MD))
        .pl(px(t::SPACE_MD))
        .children(
            lines
                .iter()
                .map(|(no, active, spans)| code_row(*no, *active, accent, spans)),
        )
}

fn code_row(no: u32, active: bool, accent: Rgba, spans: &[(&'static str, Rgba)]) -> Div {
    div()
        .relative()
        .flex()
        .items_center()
        .h(px(21.0))
        .when(active, |this| this.bg(t::wash(accent, 0.05)))
        .child(
            div()
                .w(px(54.0))
                .pr(px(t::SPACE_LG))
                .flex_shrink_0()
                .flex()
                .justify_end()
                .text_size(px(11.0))
                .font_family(theme::code_font())
                .text_color(if active { accent } else { t::text_muted() })
                .child(format!("{no}")),
        )
        .children(spans.iter().map(|(text, color)| {
            div()
                .text_size(px(t::TYPE_BODY))
                .font_family(theme::code_font())
                .text_color(*color)
                .child(*text)
        }))
        .when(active, |this| {
            this.child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .w(px(2.0))
                    .bg(accent),
            )
        })
}

// ------------------------------------------------------------------ drawer

fn drawer(accent: Rgba) -> Div {
    div()
        .h(px(t::DRAWER_HEIGHT))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .bg(t::sunken())
        .border_t_1()
        .border_color(t::border())
        .child(
            div()
                .h(px(30.0))
                .px(px(t::SPACE_LG))
                .flex()
                .items_center()
                .gap(px(t::SPACE_LG))
                .border_b_1()
                .border_color(t::border())
                .text_size(px(10.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(t::text_muted())
                .child(drawer_tab("PROBLEMS", "3", true, accent))
                .child(drawer_tab("OUTPUT", "", false, accent))
                .child(drawer_tab("TERMINAL", "", false, accent))
                .child(div().flex_1())
                .child(ghost_button("drawer-collapse", "chevron-down"))
                .child(ghost_button("drawer-close", "x")),
        )
        .child(diag_group("theme.rs", 2))
        .child(problem_row(
            "triangle-alert",
            t::warning(),
            "unused variable: `accent_strong`",
            "214:9",
            "warning",
        ))
        .child(problem_row(
            "triangle-alert",
            t::warning(),
            "this `wash` call could be const",
            "88:17",
            "warning",
        ))
        .child(diag_group("state.rs", 1))
        .child(problem_row(
            "triangle-alert",
            t::warning(),
            "unused import: `clipboard`",
            "2461:5",
            "warning",
        ))
        .child(div().flex_1())
        .child(
            div()
                .h(px(22.0))
                .px(px(t::SPACE_LG))
                .flex()
                .items_center()
                .gap(px(t::SPACE_SM))
                .text_size(px(10.0))
                .text_color(t::text_muted())
                .child(dot(t::success()))
                .child("cargo clippy — finished in 1.24s"),
        )
}

fn drawer_tab(label: &str, count: &str, active: bool, accent: Rgba) -> Div {
    div()
        .relative()
        .h_full()
        .flex()
        .items_center()
        .gap(px(t::SPACE_SM))
        .when(active, |this| this.text_color(t::text_primary()))
        .child(label.to_string())
        .when(!count.is_empty(), |this| {
            this.child(
                div()
                    .h(px(16.0))
                    .px(px(5.0))
                    .flex()
                    .items_center()
                    .rounded(px(t::RADIUS_SM))
                    .bg(t::elevated_alt())
                    .text_size(px(9.5))
                    .text_color(t::text_secondary())
                    .child(count.to_string()),
            )
        })
        .when(active, |this| {
            this.child(
                div()
                    .absolute()
                    .bottom(px(0.0))
                    .left(px(0.0))
                    .right(px(0.0))
                    .h(px(2.0))
                    .bg(accent),
            )
        })
}

fn diag_group(label: &str, count: usize) -> Div {
    div()
        .h(px(21.0))
        .px(px(t::SPACE_LG))
        .flex()
        .items_center()
        .gap(px(t::SPACE_SM))
        .child(icon("file-code", 11.0, t::text_muted()))
        .child(
            div()
                .text_size(px(10.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(t::text_secondary())
                .child(label.to_string()),
        )
        .child(
            div()
                .text_size(px(9.5))
                .text_color(t::text_muted())
                .child(count.to_string()),
        )
}

fn problem_row(
    name: &'static str,
    color: Rgba,
    message: &str,
    location: &str,
    severity: &str,
) -> Div {
    div()
        .h(px(24.0))
        .pl(px(30.0))
        .pr(px(t::SPACE_LG))
        .flex()
        .items_center()
        .gap(px(t::SPACE_MD))
        .text_size(px(11.5))
        .text_color(t::text_secondary())
        .child(icon(name, 12.0, color))
        .child(div().min_w_0().truncate().child(message.to_string()))
        .child(div().flex_1())
        .child(
            div()
                .font_family(theme::code_font())
                .text_size(px(10.5))
                .text_color(t::text_muted())
                .child(location.to_string()),
        )
        .child(
            div()
                .text_size(px(10.0))
                .text_color(t::text_muted())
                .child(severity.to_string()),
        )
}

// --------------------------------------------------------------- assistant

fn assistant(accent: Rgba) -> Div {
    div()
        .w(px(t::ASSISTANT_WIDTH))
        .flex_shrink_0()
        .h_full()
        .flex()
        .flex_col()
        .bg(t::sunken())
        .border_l_1()
        .border_color(t::border())
        .child(
            div()
                .h(px(34.0))
                .flex_shrink_0()
                .px(px(t::SPACE_XL))
                .flex()
                .items_center()
                .gap(px(t::SPACE_MD))
                .border_b_1()
                .border_color(t::border())
                .child(icon("message-circle", 14.0, accent))
                .child(
                    div()
                        .text_size(px(t::TYPE_BODY))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(t::text_primary())
                        .child("Assistant"),
                )
                .child(
                    div()
                        .h(px(18.0))
                        .px(px(t::SPACE_SM))
                        .flex()
                        .items_center()
                        .rounded(px(t::RADIUS_SM))
                        .bg(t::elevated())
                        .text_size(px(10.0))
                        .text_color(t::text_muted())
                        .child("auto-combo"),
                )
                .child(div().flex_1())
                .child(ghost_button("assistant-new", "plus")),
        )
        .child(
            div()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .flex()
                .flex_col()
                .justify_end()
                .gap(px(t::SPACE_MD))
                .px(px(t::SPACE_XL))
                .pb(px(t::SPACE_XXL))
                .child(user_turn(accent))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(step(
                            "search",
                            t::cyan(),
                            "Observe",
                            "Read theme.rs, widgets.rs, state.rs",
                            "1.2s",
                            false,
                            false,
                        ))
                        .child(step(
                            "sparkles",
                            t::warning(),
                            "Think",
                            "Yellow as chrome, never as syntax",
                            "ok",
                            false,
                            false,
                        ))
                        .child(step(
                            "code",
                            t::brand(),
                            "Patch",
                            "+184 −92 across 6 files",
                            "3 edits",
                            false,
                            false,
                        ))
                        .child(step(
                            "check",
                            t::success(),
                            "Verify",
                            "fmt, clippy -D warnings, 200 tests",
                            "clean",
                            false,
                            false,
                        ))
                        .child(step(
                            "scale",
                            t::violet(),
                            "Judge",
                            "13.0:1 on brand, 5.2:1 minimum",
                            "PASS",
                            true,
                            true,
                        )),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .line_height(px(18.0))
                        .text_color(t::text_muted())
                        .child("Applied 12 tokens and 28 icons. Contrast holds on every surface."),
                )
                .child(result_row(accent))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(t::SPACE_XS))
                        .child(context_chip("theme.rs"))
                        .child(context_chip("ui_kit/mod.rs"))
                        .child(context_chip("state.rs"))
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(t::text_muted())
                                .child("+12"),
                        ),
                ),
        )
        .child(composer(accent))
}

fn user_turn(accent: Rgba) -> Div {
    div().flex().justify_end().child(
        div()
            .max_w(px(305.0))
            .p(px(10.0))
            .rounded(px(t::RADIUS_LG))
            .bg(t::wash(accent, 0.10))
            .text_size(px(t::TYPE_HEADING))
            .text_color(t::text_primary())
            .child("Restyle the workbench to Obsidian Gold."),
    )
}

fn step(
    icon_name: &'static str,
    color: Rgba,
    title: &str,
    detail: &str,
    meta: &str,
    active: bool,
    last: bool,
) -> Div {
    div()
        .relative()
        .flex()
        .gap(px(10.0))
        .pb(px(if last { 0.0 } else { 12.0 }))
        .when(!last, |this| {
            this.child(
                div()
                    .absolute()
                    .left(px(8.5))
                    .top(px(18.0))
                    .bottom(px(0.0))
                    .w(px(1.0))
                    .bg(t::border()),
            )
        })
        .child(
            div()
                .size(px(18.0))
                .rounded_full()
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(t::wash(color, 0.14))
                .child(icon(icon_name, 11.0, color)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .flex_1()
                .min_w_0()
                .when(active, |this| {
                    this.p(px(8.0))
                        .rounded(px(t::RADIUS_MD))
                        .bg(t::elevated())
                        .border_1()
                        .border_color(t::wash(color, 0.24))
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(t::SPACE_SM))
                        .child(
                            div()
                                .text_size(px(11.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(t::text_primary())
                                .child(title.to_string()),
                        )
                        .child(div().flex_1())
                        .child(
                            div()
                                .font_family(theme::code_font())
                                .text_size(px(9.5))
                                .text_color(if active { color } else { t::text_muted() })
                                .child(meta.to_string()),
                        ),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(t::text_muted())
                        .child(detail.to_string()),
                ),
        )
}

fn result_row(accent: Rgba) -> Div {
    div()
        .p(px(10.0))
        .rounded(px(8.0))
        .bg(t::elevated())
        .border_1()
        .border_color(t::border())
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(t::text_primary())
                        .child("Result"),
                )
                .child(
                    div()
                        .font_family(theme::code_font())
                        .text_size(px(10.0))
                        .text_color(t::text_muted())
                        .child("+184 −92 · 6 files"),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .h(px(24.0))
                        .px(px(t::SPACE_MD))
                        .flex()
                        .items_center()
                        .rounded(px(t::RADIUS_SM))
                        .text_size(px(11.0))
                        .text_color(t::text_secondary())
                        .cursor_pointer()
                        .id("result-diff")
                        .hover(|style| style.bg(t::elevated_alt()))
                        .child("Diff"),
                )
                .child(
                    div()
                        .h(px(24.0))
                        .px(px(t::SPACE_MD))
                        .flex()
                        .items_center()
                        .rounded(px(t::RADIUS_SM))
                        .border_1()
                        .border_color(t::wash(accent, 0.26))
                        .bg(t::wash(accent, 0.08))
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(accent)
                        .cursor_pointer()
                        .id("result-apply")
                        .hover(|style| style.bg(t::wash(accent, 0.16)))
                        .child("Apply"),
                ),
        )
}

fn context_chip(label: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(t::SPACE_XS))
        .h(px(20.0))
        .px(px(t::SPACE_SM))
        .rounded(px(t::RADIUS_SM))
        .border_1()
        .border_color(t::border())
        .bg(t::elevated())
        .font_family(theme::code_font())
        .text_size(px(10.0))
        .text_color(t::text_secondary())
        .child(icon("file-code", 11.0, t::text_muted()))
        .child(label.to_string())
}

fn composer(accent: Rgba) -> Div {
    div().px(px(t::SPACE_LG)).pb(px(t::SPACE_LG)).child(
        div()
            .flex()
            .flex_col()
            .gap(px(t::SPACE_MD))
            .p(px(10.0))
            .rounded(px(t::RADIUS_LG))
            .bg(t::elevated_alt())
            .border_1()
            .border_color(t::wash(accent, 0.35))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .text_size(px(t::TYPE_BODY))
                    .text_color(t::text_primary())
                    .child("apply the tokens to the editor")
                    .child(div().w(px(1.5)).h(px(14.0)).bg(accent)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(t::SPACE_SM))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(t::SPACE_SM))
                            .h(px(22.0))
                            .px(px(t::SPACE_MD))
                            .rounded(px(t::RADIUS_SM))
                            .border_1()
                            .border_color(t::border())
                            .bg(t::elevated())
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t::text_secondary())
                            .cursor_pointer()
                            .id("mode-pill")
                            .hover(|style| style.border_color(t::border_strong()))
                            .child(dot(accent))
                            .child("Code")
                            .child(icon("chevron-down", 10.0, t::text_muted())),
                    )
                    .child(toolbar_pill("Rust"))
                    .child(toolbar_pill("auto-combo"))
                    .child(div().flex_1())
                    .child(keycap("Ctrl Enter"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(t::SPACE_SM))
                            .h(px(t::HEIGHT_CONTROL))
                            .px(px(t::SPACE_MD))
                            .rounded(px(t::RADIUS_MD))
                            .bg(t::brand())
                            .text_size(px(11.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(t::on_brand())
                            .cursor_pointer()
                            .id("run")
                            .hover(|style| style.bg(t::brand_hover()))
                            .active(|style| style.bg(t::brand_press()))
                            .child(icon("play", 11.0, t::on_brand()))
                            .child("Run"),
                    ),
            ),
    )
}

fn toolbar_pill(label: &str) -> Div {
    div()
        .flex()
        .items_center()
        .h(px(22.0))
        .px(px(t::SPACE_MD))
        .rounded(px(t::RADIUS_SM))
        .border_1()
        .border_color(t::border())
        .bg(t::elevated())
        .text_size(px(10.5))
        .text_color(t::text_muted())
        .child(label.to_string())
}

// -------------------------------------------------------------- status bar

fn status_bar(accent: Rgba) -> Div {
    div()
        .h(px(t::HEIGHT_STATUS))
        .flex_shrink_0()
        .px(px(t::SPACE_LG))
        .flex()
        .items_center()
        .gap(px(t::SPACE_LG))
        .bg(t::sunken())
        .border_t_1()
        .border_color(t::border())
        .text_size(px(11.0))
        .text_color(t::text_muted())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(t::SPACE_XS))
                .h(px(18.0))
                .px(px(t::SPACE_SM))
                .rounded(px(t::RADIUS_SM))
                .bg(t::wash(accent, 0.12))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(accent)
                .child(dot(accent))
                .child("CODE"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(t::SPACE_XS))
                .child(icon("git-branch", 12.0, t::text_muted()))
                .child("main"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(t::SPACE_XS))
                .child(icon("triangle-alert", 12.0, t::warning()))
                .child("3 problems"),
        )
        .child(div().flex_1())
        .child("Ln 21, Col 17")
        .child("Rust")
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(t::SPACE_XS))
                .child(icon("lock", 11.0, t::text_muted()))
                .child("Local"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(t::SPACE_XS))
                .child("62% context")
                .child(icon("info", 11.0, t::text_muted())),
        )
}

// -------------------------------------------------------------------- view

pub struct DesignPreview {
    mode: ExecutionMode,
}

impl DesignPreview {
    pub fn new(mode: ExecutionMode) -> Self {
        Self { mode }
    }
}

impl Render for DesignPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let accent = t::mode_accent(self.mode);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(t::sunken())
            .font_family(theme::ui_font())
            .text_size(px(t::TYPE_BODY))
            .text_color(t::text_primary())
            .child(titlebar())
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(dock(accent))
                    .child(center(accent))
                    .child(assistant(accent)),
            )
            .child(status_bar(accent))
    }
}
