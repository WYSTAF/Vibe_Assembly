//! Studio — an editorial take on the chat-first workbench.
//!
//! Warm ivory paper, a serif voice for headings, and a session-history rail
//! instead of a file tree. The conversation stays the workspace; craft shows up
//! in the details — generous margins, hairlines, and numbered next steps.

use gpui::prelude::*;
use gpui::{
    div, point, px, rgb, rgba, BoxShadow, Context, Div, FontWeight, IntoElement, Rgba, Stateful,
    Window, WindowControlArea,
};
use va_core::modes::ExecutionMode;

use crate::branding;
use crate::design::icon::icon;
use crate::theme;

// ------------------------------------------------------------------ palette

fn canvas() -> Rgba {
    rgb(0xEFE5D1)
}
fn paper() -> Rgba {
    rgb(0xF8F2E4)
}
fn raised() -> Rgba {
    rgb(0xFFFDF7)
}
fn sunken() -> Rgba {
    rgb(0xE9DEC8)
}
fn border() -> Rgba {
    rgb(0xE0D3BB)
}
fn border_strong() -> Rgba {
    rgb(0xC8B99D)
}
fn ink() -> Rgba {
    rgb(0x241F19)
}
fn ink_soft() -> Rgba {
    rgb(0x57503F)
}
fn ink_muted() -> Rgba {
    rgb(0x8A7E6A)
}
fn brand() -> Rgba {
    rgb(0xFDCC02)
}
fn brand_hover() -> Rgba {
    rgb(0xFFD83D)
}
fn brand_press() -> Rgba {
    rgb(0xE0B400)
}
fn brand_ink() -> Rgba {
    rgb(0x6F5510)
}
fn brand_soft() -> Rgba {
    rgb(0xFBEFC2)
}
fn success() -> Rgba {
    rgb(0x3E7A4E)
}
fn success_soft() -> Rgba {
    rgb(0xE5F0E4)
}
fn warning() -> Rgba {
    rgb(0xA65F00)
}
fn danger() -> Rgba {
    rgb(0xA33B2E)
}
fn danger_soft() -> Rgba {
    rgb(0xF6E4E0)
}
fn hover_wash() -> Rgba {
    rgba(0x241F190D)
}

// ------------------------------------------------------------------- syntax

fn syn_keyword() -> Rgba {
    rgb(0x8E3D67)
}
fn syn_call() -> Rgba {
    rgb(0x2C5F94)
}
fn syn_type() -> Rgba {
    rgb(0x0E6A5E)
}
fn syn_number() -> Rgba {
    rgb(0x95521A)
}
fn syn_comment() -> Rgba {
    rgb(0x8A7E6A)
}
fn punct() -> Rgba {
    rgb(0x6B6353)
}

fn kw(text: &'static str) -> (&'static str, Rgba) {
    (text, syn_keyword())
}
fn fx(text: &'static str) -> (&'static str, Rgba) {
    (text, syn_call())
}
fn ty(text: &'static str) -> (&'static str, Rgba) {
    (text, syn_type())
}
fn nu(text: &'static str) -> (&'static str, Rgba) {
    (text, syn_number())
}
fn cm(text: &'static str) -> (&'static str, Rgba) {
    (text, syn_comment())
}
fn p(label: &'static str) -> (&'static str, Rgba) {
    (label, punct())
}

// ------------------------------------------------------------------- metric

const R_SM: f32 = 8.0;
const R_LG: f32 = 18.0;
const TITLEBAR: f32 = 52.0;
const STATUS: f32 = 28.0;
const ROW: f32 = 34.0;

fn serif() -> &'static str {
    "Georgia"
}

fn soft_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: rgba(0x241F1914).into(),
        offset: point(px(0.0), px(3.0)),
        blur_radius: px(12.0),
        spread_radius: px(0.0),
    }]
}

fn lifted_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: rgba(0x241F191A).into(),
        offset: point(px(0.0), px(12.0)),
        blur_radius: px(34.0),
        spread_radius: px(0.0),
    }]
}

// ------------------------------------------------------------------- screen

pub struct StudioPreview {
    mode: ExecutionMode,
}

impl StudioPreview {
    pub fn new(mode: ExecutionMode) -> Self {
        Self { mode }
    }
}

impl Render for StudioPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mode = self.mode;
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(canvas())
            .font_family(theme::ui_font())
            .text_color(ink())
            .child(titlebar())
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(history_rail())
                    .child(conversation())
                    .child(inspector()),
            )
            .child(status_bar(mode))
    }
}

// ----------------------------------------------------------------- titlebar

fn titlebar() -> Div {
    div()
        .h(px(TITLEBAR))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(16.0))
        .bg(paper())
        .border_b_1()
        .border_color(border())
        .child(
            div()
                .window_control_area(WindowControlArea::Drag)
                .child(brand_tile()),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(0.0))
                .child(
                    div()
                        .font_family(serif())
                        .text_size(px(15.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink())
                        .child("Vibe Assembly"),
                )
                .child(
                    div()
                        .text_size(px(9.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink_muted())
                        .child("STUDIO"),
                ),
        )
        .child(div().flex_1())
        .child(search())
        .child(div().flex_1())
        .child(bare_icon("studio-help", "info"))
        .child(account_avatar())
        .child(separator())
        .child(window_control(
            "win-min",
            "minus",
            WindowControlArea::Min,
            false,
        ))
        .child(window_control(
            "win-max",
            "square",
            WindowControlArea::Max,
            false,
        ))
        .child(window_control(
            "win-close",
            "x",
            WindowControlArea::Close,
            true,
        ))
}

fn brand_tile() -> Div {
    div()
        .size(px(32.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(10.0))
        .bg(raised())
        .border_1()
        .border_color(border())
        .shadow(soft_shadow())
        .child(branding::logo(21.0))
}

fn search() -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .w(px(380.0))
        .h(px(34.0))
        .px(px(12.0))
        .rounded(px(10.0))
        .bg(raised())
        .border_1()
        .border_color(border())
        .child(icon("search", 14.0, ink_muted()))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(12.5))
                .text_color(ink_muted())
                .child("Search sessions, files, or ask anything…"),
        )
        .child(keycap("Ctrl K"))
}

fn account_avatar() -> Div {
    div()
        .size(px(30.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(brand_soft())
        .border_1()
        .border_color(rgba(0xFDCC0259))
        .text_size(px(10.5))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(brand_ink())
        .child("VA")
}

// ---------------------------------------------------------------- left rail

fn history_rail() -> Div {
    div()
        .w(px(286.0))
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .px(px(12.0))
        .pt(px(16.0))
        .pb(px(10.0))
        .bg(paper())
        .border_r_1()
        .border_color(border())
        .child(new_session_button())
        .child(div().h(px(16.0)))
        .child(div().px(px(6.0)).pb(px(6.0)).child(overline("SESSIONS")))
        .child(session_item(
            "session-restyle",
            "Restyle the workbench",
            "Code · just now",
            true,
        ))
        .child(session_item(
            "session-flaky",
            "Fix flaky test in state.rs",
            "Debug · 1h ago",
            false,
        ))
        .child(session_item(
            "session-notes",
            "Draft release notes",
            "Chat · 3h ago",
            false,
        ))
        .child(
            div()
                .px(px(6.0))
                .pt(px(14.0))
                .pb(px(6.0))
                .child(overline("THIS WEEK")),
        )
        .child(session_item(
            "session-tokens",
            "Audit dark-mode tokens",
            "Code · Tuesday",
            false,
        ))
        .child(session_item(
            "session-start",
            "Faster cold start",
            "Web · Monday",
            false,
        ))
        .child(div().flex_1())
        .child(div().px(px(6.0)).pb(px(6.0)).child(overline("PINNED")))
        .child(pinned_row("pinned-theme", "theme.rs"))
        .child(pinned_row("pinned-state", "state.rs"))
        .child(pinned_row("pinned-cargo", "Cargo.toml"))
        .child(
            div()
                .mt(px(8.0))
                .pt(px(8.0))
                .border_t_1()
                .border_color(border())
                .flex()
                .items_center()
                .gap(px(6.0))
                .h(px(26.0))
                .px(px(6.0))
                .child(icon("git-branch", 12.0, ink_muted()))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(ink_muted())
                        .child("main · up to date"),
                ),
        )
}

fn new_session_button() -> Stateful<Div> {
    div()
        .id("studio-new-session")
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .h(px(34.0))
        .rounded(px(10.0))
        .bg(brand())
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(ink())
        .cursor_pointer()
        .hover(|style| style.bg(brand_hover()))
        .active(|style| style.bg(brand_press()))
        .child(icon("plus", 13.0, ink()))
        .child("New session")
}

fn overline(label: &'static str) -> Div {
    div()
        .text_size(px(9.5))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(ink_muted())
        .child(label)
}

fn session_item(
    id: &'static str,
    title: &'static str,
    meta: &'static str,
    active: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(2.0))
        .h(px(46.0))
        .px(px(10.0))
        .rounded(px(10.0))
        .cursor_pointer()
        .when(active, |this| {
            this.bg(brand_soft()).border_l_2().border_color(brand())
        })
        .when(!active, |this| this.hover(|style| style.bg(hover_wash())))
        .child(
            div()
                .text_size(px(12.5))
                .when(active, |this| {
                    this.font_weight(FontWeight::SEMIBOLD).text_color(ink())
                })
                .when(!active, |this| this.text_color(ink_soft()))
                .truncate()
                .child(title),
        )
        .child(
            div()
                .text_size(px(10.5))
                .text_color(ink_muted())
                .child(meta),
        )
}

fn pinned_row(id: &'static str, name: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(7.0))
        .h(px(ROW))
        .px(px(6.0))
        .rounded(px(R_SM))
        .cursor_pointer()
        .hover(|style| style.bg(hover_wash()))
        .child(icon("file-code", 12.0, ink_muted()))
        .child(div().text_size(px(11.5)).text_color(ink_soft()).child(name))
}

// -------------------------------------------------------------- conversation

fn conversation() -> Div {
    div()
        .flex_1()
        .min_w_0()
        .h_full()
        .flex()
        .flex_col()
        .bg(canvas())
        .child(conversation_header())
        .child(
            div()
                .id("studio-thread")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_center()
                        .px(px(26.0))
                        .pt(px(16.0))
                        .pb(px(10.0))
                        .child(
                            div()
                                .w_full()
                                .max_w(px(720.0))
                                .flex()
                                .flex_col()
                                .gap(px(16.0))
                                .child(intro_turn())
                                .child(user_turn())
                                .child(work_turn())
                                .child(closing_turn())
                                .child(suggestions()),
                        ),
                ),
        )
        .child(composer())
        .child(div().w_full().flex().justify_center().pb(px(12.0)).child(
            div().text_size(px(10.0)).text_color(ink_muted()).child(
                "Vibe can make mistakes — review every change before you apply. · 62% context used",
            ),
        ))
}

fn conversation_header() -> Div {
    div()
        .h(px(60.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(26.0))
        .bg(canvas())
        .border_b_1()
        .border_color(border())
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(overline("SESSION · CODE MODE"))
                .child(
                    div()
                        .font_family(serif())
                        .text_size(px(17.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink())
                        .child("Restyle the workbench"),
                ),
        )
        .child(div().flex_1())
        .child(soft_chip("6 files changed"))
        .child(bare_icon("studio-panel", "panel-right"))
}

fn vibe_avatar(size: f32) -> Div {
    div()
        .size(px(size))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(brand_soft())
        .border_1()
        .border_color(rgba(0xFDCC0259))
        .child(icon("sparkles", size * 0.48, brand_ink()))
}

fn vibe_voice(content: impl IntoElement) -> Div {
    div()
        .flex()
        .items_start()
        .gap(px(10.0))
        .child(vibe_avatar(28.0))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .text_size(px(11.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(ink_soft())
                                .child("Vibe"),
                        )
                        .child(
                            div()
                                .text_size(px(10.5))
                                .text_color(ink_muted())
                                .child("just now"),
                        ),
                )
                .child(content),
        )
}

fn intro_turn() -> Div {
    vibe_voice(
        div()
            .max_w(px(590.0))
            .text_size(px(13.5))
            .line_height(px(21.0))
            .text_color(ink_soft())
            .child(
                "I read through the workbench and kept its bones — same panels, same habits. \
                 What changed is the voice: warmer paper, a serif for headings, and fewer boxes \
                 competing for your attention.",
            ),
    )
}

fn user_turn() -> Div {
    div().w_full().flex().justify_end().child(
        div()
            .max_w(px(470.0))
            .p(px(14.0))
            .rounded(px(16.0))
            .bg(raised())
            .border_1()
            .border_color(border())
            .shadow(soft_shadow())
            .text_size(px(13.0))
            .line_height(px(20.0))
            .text_color(ink())
            .child("It feels like a tool I'd leave open all day. Make it a little more editorial."),
    )
}

fn work_turn() -> Div {
    vibe_voice(work_card())
}

fn work_card() -> Div {
    div()
        .w_full()
        .p(px(16.0))
        .rounded(px(R_LG))
        .bg(raised())
        .border_1()
        .border_color(border())
        .shadow(lifted_shadow())
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_end()
                .child(
                    div()
                        .font_family(serif())
                        .text_size(px(18.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink())
                        .child("What I changed"),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(ink_muted())
                        .child("6 files · 86 lines"),
                ),
        )
        .child(
            div()
                .text_size(px(12.5))
                .line_height(px(19.0))
                .text_color(ink_muted())
                .child("Warmer surfaces, quieter rules, and a serif voice for the headings."),
        )
        .child(check_line("Warmed the paper, kept the contrast"))
        .child(check_line("Set headings in a serif — UI copy stays sans"))
        .child(check_line("Left the brand yellow on one action only"))
        .child(diff_preview())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .pt(px(4.0))
                .child(primary_button("Apply changes"))
                .child(secondary_button("Review diff"))
                .child(div().flex_1())
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(ink_muted())
                        .child("You can undo anytime"),
                ),
        )
}

fn check_line(label: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .size(px(16.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(success_soft())
                .child(icon("check", 10.0, success())),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(ink_soft())
                .child(label),
        )
}

fn diff_preview() -> Div {
    div()
        .mt(px(2.0))
        .rounded(px(12.0))
        .bg(sunken())
        .border_1()
        .border_color(border())
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(32.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(px(7.0))
                .px(px(10.0))
                .bg(paper())
                .border_b_1()
                .border_color(border())
                .child(icon("file-code", 12.0, brand_ink()))
                .child(
                    div()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(ink())
                        .child("theme.rs"),
                )
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(ink_muted())
                        .child("crates/va-desktop/src"),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .font_family(theme::code_font())
                        .text_size(px(10.5))
                        .text_color(success())
                        .child("+42"),
                )
                .child(
                    div()
                        .font_family(theme::code_font())
                        .text_size(px(10.5))
                        .text_color(danger())
                        .child("−12"),
                ),
        )
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .py(px(4.0))
                .child(diff_row(" ", None, &[cm("@@ paper @@")]))
                .child(diff_row(
                    " ",
                    None,
                    &[kw("pub fn "), fx("paper"), p("() -> "), ty("Rgba"), p(" {")],
                ))
                .child(diff_row(
                    "-",
                    Some(danger_soft()),
                    &[p("    "), fx("rgb"), p("("), nu("0xFFFFFF"), p(")")],
                ))
                .child(diff_row(
                    "+",
                    Some(success_soft()),
                    &[p("    "), fx("rgb"), p("("), nu("0xF8F2E4"), p(")")],
                ))
                .child(diff_row(" ", None, &[cm("@@ headings @@")]))
                .child(diff_row(
                    "-",
                    Some(danger_soft()),
                    &[
                        p("    "),
                        fx("font_family"),
                        p("("),
                        fx("ui_font"),
                        p("())"),
                    ],
                ))
                .child(diff_row(
                    "+",
                    Some(success_soft()),
                    &[p("    "), fx("font_family"), p("("), fx("serif"), p("())")],
                )),
        )
}

fn diff_row(sign: &'static str, wash: Option<Rgba>, spans: &[(&'static str, Rgba)]) -> Div {
    let sign_color = match sign {
        "+" => success(),
        "-" => danger(),
        _ => ink_muted(),
    };
    div()
        .flex()
        .items_center()
        .h(px(18.0))
        .px(px(8.0))
        .when_some(wash, |this, color| this.bg(color))
        .child(
            div()
                .w(px(12.0))
                .flex_shrink_0()
                .font_family(theme::code_font())
                .text_size(px(10.5))
                .text_color(sign_color)
                .child(sign),
        )
        .children(spans.iter().map(|(text, color)| {
            div()
                .font_family(theme::code_font())
                .text_size(px(11.5))
                .text_color(*color)
                .child(*text)
        }))
}

fn closing_turn() -> Div {
    vibe_voice(
        div()
            .max_w(px(590.0))
            .text_size(px(13.0))
            .line_height(px(20.0))
            .text_color(ink_soft())
            .child(
                "The layout is untouched — you can move between the session rail, the thread, \
                 and the inspector exactly as before. Want a darker paper or a denser rhythm?",
            ),
    )
}

fn suggestions() -> Div {
    div()
        .pl(px(38.0))
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(suggestion("Try a darker paper"))
        .child(suggestion("Show the full diff"))
        .child(suggestion("Tighten the spacing"))
}

fn suggestion(label: &'static str) -> Stateful<Div> {
    div()
        .id(label)
        .flex()
        .items_center()
        .h(px(26.0))
        .px(px(10.0))
        .rounded_full()
        .bg(raised())
        .border_1()
        .border_color(border())
        .text_size(px(11.5))
        .text_color(ink_soft())
        .cursor_pointer()
        .hover(|style| style.bg(sunken()))
        .child(label)
}

fn composer() -> Div {
    div()
        .w_full()
        .flex()
        .justify_center()
        .px(px(26.0))
        .pb(px(8.0))
        .child(
            div()
                .w_full()
                .max_w(px(720.0))
                .flex()
                .flex_col()
                .gap(px(10.0))
                .p(px(12.0))
                .rounded(px(R_LG))
                .bg(raised())
                .border_1()
                .border_color(border_strong())
                .shadow(soft_shadow())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(2.0))
                        .h(px(30.0))
                        .px(px(4.0))
                        .text_size(px(13.0))
                        .text_color(ink_muted())
                        .child("Ask Vibe to change something…")
                        .child(div().w(px(1.5)).h(px(15.0)).bg(brand())),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(bare_icon("studio-add-context", "plus"))
                        .child(context_chip("folder", "src/"))
                        .child(context_chip("file-code", "theme.rs"))
                        .child(div().flex_1())
                        .child(soft_chip("Auto"))
                        .child(soft_chip("Rust"))
                        .child(keycap("Enter"))
                        .child(send_button()),
                ),
        )
}

fn context_chip(icon_name: &'static str, label: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(5.0))
        .h(px(26.0))
        .px(px(8.0))
        .rounded(px(R_SM))
        .bg(sunken())
        .child(icon(icon_name, 11.0, ink_muted()))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(ink_soft())
                .child(label),
        )
}

fn send_button() -> Stateful<Div> {
    div()
        .id("studio-send")
        .size(px(34.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(brand())
        .cursor_pointer()
        .hover(|style| style.bg(brand_hover()))
        .active(|style| style.bg(brand_press()))
        .child(icon("arrow-up", 15.0, ink()))
}

// ---------------------------------------------------------------- inspector

fn inspector() -> Div {
    div()
        .w(px(300.0))
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .bg(paper())
        .border_l_1()
        .border_color(border())
        .child(
            div()
                .h(px(60.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(16.0))
                .border_b_1()
                .border_color(border())
                .child(
                    div()
                        .font_family(serif())
                        .text_size(px(15.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink())
                        .child("Session"),
                )
                .child(soft_chip("in review"))
                .child(div().flex_1())
                .child(bare_icon("studio-files", "files")),
        )
        .child(
            div()
                .id("studio-inspector")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(px(14.0))
                .p(px(14.0))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .child(overline("NEXT STEPS"))
                        .child(step_row(1, "Review the diff", "6 files · 86 lines", true))
                        .child(step_row(2, "Apply changes", "nothing applied yet", false))
                        .child(step_row(
                            3,
                            "Run the test suite",
                            "200 tests · 0 failures",
                            false,
                        )),
                )
                .child(div().h(px(1.0)).w_full().bg(border()))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(overline("CHANGED FILES"))
                        .child(file_line("ins-theme", "theme.rs", 42, 12, true))
                        .child(file_line("ins-widgets", "widgets.rs", 12, 3, false))
                        .child(file_line("ins-state", "state.rs", 9, 2, false))
                        .child(file_line("ins-editor", "editor_pane.rs", 4, 1, false))
                        .child(file_line("ins-rail", "left_dock.rs", 4, 2, false))
                        .child(file_line("ins-icon", "icon.rs", 2, 1, false)),
                )
                .child(div().h(px(1.0)).w_full().bg(border()))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .child(overline("CONTEXT"))
                                .child(div().flex_1())
                                .child(
                                    div()
                                        .text_size(px(10.5))
                                        .text_color(ink_muted())
                                        .child("62% used"),
                                ),
                        )
                        .child(
                            div().h(px(4.0)).w_full().rounded_full().bg(sunken()).child(
                                div().h(px(4.0)).w(px(168.0)).rounded_full().bg(ink_muted()),
                            ),
                        )
                        .child(inspector_fact("folder-open", "src/ attached", "24 files"))
                        .child(inspector_fact("code", "Auto · Rust", "execution mode"))
                        .child(inspector_fact("lock", "Local only", "nothing uploaded")),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap(px(8.0))
                        .p(px(10.0))
                        .rounded(px(12.0))
                        .bg(brand_soft())
                        .child(icon("sparkles", 13.0, brand_ink()))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(11.0))
                                .line_height(px(16.0))
                                .text_color(brand_ink())
                                .child("Vibe works inside this repo. Nothing leaves your machine."),
                        ),
                ),
        )
}

fn step_row(number: u32, title: &'static str, meta: &'static str, active: bool) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(9.0))
        .child(
            div()
                .size(px(20.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .when(active, |this| this.bg(brand()))
                .when(!active, |this| {
                    this.bg(raised()).border_1().border_color(border())
                })
                .text_size(px(9.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(if active { ink() } else { ink_muted() })
                .child(format!("{number}")),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(1.0))
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .text_size(px(12.0))
                        .when(active, |this| {
                            this.font_weight(FontWeight::SEMIBOLD).text_color(ink())
                        })
                        .when(!active, |this| this.text_color(ink_soft()))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(ink_muted())
                        .child(meta),
                ),
        )
        .when(active, |this| {
            this.child(icon("chevron-right", 12.0, brand_ink()))
        })
}

fn file_line(
    id: &'static str,
    name: &'static str,
    added: u32,
    removed: u32,
    active: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(7.0))
        .h(px(28.0))
        .px(px(6.0))
        .rounded(px(7.0))
        .cursor_pointer()
        .when(active, |this| this.bg(brand_soft()))
        .when(!active, |this| this.hover(|style| style.bg(hover_wash())))
        .child(icon(
            "file-code",
            12.0,
            if active { brand_ink() } else { ink_muted() },
        ))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(11.5))
                .when(active, |this| this.text_color(ink()))
                .when(!active, |this| this.text_color(ink_soft()))
                .child(name),
        )
        .child(
            div()
                .font_family(theme::code_font())
                .text_size(px(10.0))
                .text_color(success())
                .child(format!("+{added}")),
        )
        .child(
            div()
                .font_family(theme::code_font())
                .text_size(px(10.0))
                .text_color(danger())
                .child(format!("−{removed}")),
        )
}

fn inspector_fact(icon_name: &'static str, label: &'static str, meta: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(7.0))
        .h(px(22.0))
        .child(icon(icon_name, 11.0, ink_muted()))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(ink_soft())
                .child(label),
        )
        .child(div().flex_1())
        .child(div().text_size(px(9.5)).text_color(ink_muted()).child(meta))
}

// ------------------------------------------------------------------- status

fn status_bar(mode: ExecutionMode) -> Div {
    div()
        .h(px(STATUS))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(16.0))
        .bg(paper())
        .border_t_1()
        .border_color(border())
        .text_size(px(11.0))
        .text_color(ink_muted())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(5.0))
                .h(px(18.0))
                .px(px(7.0))
                .rounded(px(6.0))
                .bg(raised())
                .border_1()
                .border_color(border())
                .child(div().size(px(5.0)).rounded_full().bg(brand()))
                .child(mode.label()),
        )
        .child(separator())
        .child(icon("git-branch", 11.0, ink_muted()))
        .child(div().child("main"))
        .child(separator())
        .child(div().size(px(5.0)).rounded_full().bg(success()))
        .child(div().text_color(ink_soft()).child("Everything builds"))
        .child(separator())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(5.0))
                .child(icon("triangle-alert", 10.0, warning()))
                .child(div().child("3 minor warnings")),
        )
        .child(div().flex_1())
        .child(div().child("6 files changed"))
        .child(separator())
        .child(div().child("62% context"))
        .child(separator())
        .child(icon("lock", 11.0, ink_muted()))
        .child(div().child("Local only"))
}

// ------------------------------------------------------------------- pieces

fn bare_icon(id: &'static str, name: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .size(px(30.0))
        .rounded(px(R_SM))
        .cursor_pointer()
        .hover(|style| style.bg(sunken()))
        .child(icon(name, 14.0, ink_muted()))
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
        .w(px(32.0))
        .h(px(28.0))
        .rounded(px(R_SM))
        .cursor_pointer()
        .hover(move |style| {
            if destructive {
                style.bg(danger_soft())
            } else {
                style.bg(sunken())
            }
        })
        .child(icon(name, 12.0, ink_muted()))
}

fn keycap(label: &'static str) -> Div {
    div()
        .h(px(20.0))
        .px(px(6.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .bg(sunken())
        .text_size(px(10.0))
        .text_color(ink_muted())
        .child(label)
}

fn soft_chip(label: &'static str) -> Div {
    div()
        .h(px(22.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .rounded(px(6.0))
        .bg(sunken())
        .text_size(px(11.0))
        .text_color(ink_soft())
        .child(label)
}

fn separator() -> Div {
    div().w(px(1.0)).h(px(12.0)).flex_shrink_0().bg(border())
}

fn primary_button(label: &'static str) -> Stateful<Div> {
    div()
        .id(label)
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .h(px(32.0))
        .px(px(14.0))
        .rounded_full()
        .bg(brand())
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(ink())
        .cursor_pointer()
        .hover(|style| style.bg(brand_hover()))
        .active(|style| style.bg(brand_press()))
        .child(icon("check", 12.0, ink()))
        .child(label)
}

fn secondary_button(label: &'static str) -> Stateful<Div> {
    div()
        .id(label)
        .flex()
        .items_center()
        .justify_center()
        .h(px(32.0))
        .px(px(14.0))
        .rounded_full()
        .bg(raised())
        .border_1()
        .border_color(border_strong())
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(ink_soft())
        .cursor_pointer()
        .hover(|style| style.bg(sunken()))
        .child(label)
}
