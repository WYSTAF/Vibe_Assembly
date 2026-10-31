//! Paper & Ink — a warmer, human-first take on the workbench.
//!
//! Same product, different temperament: light warm paper, floating cards,
//! sentence-case labels, verbs on buttons, and an assistant that talks like a
//! colleague instead of a machine. The conversation is the workspace here —
//! code shows up as diffs inside the thread, never as the main surface.

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
    rgb(0xF1EADC)
}
fn paper() -> Rgba {
    rgb(0xFAF6EC)
}
fn raised() -> Rgba {
    rgb(0xFFFFFF)
}
fn sunken() -> Rgba {
    rgb(0xEBE3D3)
}
fn border() -> Rgba {
    rgb(0xE4DBCB)
}
fn border_strong() -> Rgba {
    rgb(0xD1C5B0)
}
fn ink() -> Rgba {
    rgb(0x2B2620)
}
fn ink_soft() -> Rgba {
    rgb(0x5F584C)
}
fn ink_muted() -> Rgba {
    rgb(0x746B5C)
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
    rgb(0x7A5F00)
}
fn brand_soft() -> Rgba {
    rgb(0xFFF3C4)
}
fn success() -> Rgba {
    rgb(0x2F7D4F)
}
fn success_soft() -> Rgba {
    rgb(0xE4F2E6)
}
fn warning() -> Rgba {
    rgb(0xA65F00)
}
fn warning_soft() -> Rgba {
    rgb(0xFCEFDC)
}
fn danger() -> Rgba {
    rgb(0xA33B2E)
}
fn danger_soft() -> Rgba {
    rgb(0xF8E5E2)
}
fn hover_wash() -> Rgba {
    rgba(0x2B26200D)
}

// ------------------------------------------------------------------- syntax

fn syn_keyword() -> Rgba {
    rgb(0x9A3D6B)
}
fn syn_call() -> Rgba {
    rgb(0x2C5F9E)
}
fn syn_type() -> Rgba {
    rgb(0x0E6E63)
}
fn syn_number() -> Rgba {
    rgb(0x9A5518)
}
fn syn_comment() -> Rgba {
    rgb(0x746B5C)
}
fn punct() -> Rgba {
    rgb(0x6B6357)
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
const R_LG: f32 = 16.0;
const TITLEBAR: f32 = 46.0;
const STATUS: f32 = 28.0;
const ROW: f32 = 30.0;

fn soft_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: rgba(0x2B262018).into(),
        offset: point(px(0.0), px(4.0)),
        blur_radius: px(14.0),
        spread_radius: px(0.0),
    }]
}

fn lifted_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: rgba(0x2B262014).into(),
        offset: point(px(0.0), px(10.0)),
        blur_radius: px(30.0),
        spread_radius: px(0.0),
    }]
}

// ------------------------------------------------------------------- screen

pub struct FriendlyPreview {
    mode: ExecutionMode,
}

impl FriendlyPreview {
    pub fn new(mode: ExecutionMode) -> Self {
        Self { mode }
    }
}

impl Render for FriendlyPreview {
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
                    .child(dock())
                    .child(conversation())
                    .child(session_panel()),
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
        .px(px(14.0))
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
                .gap(px(1.0))
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink())
                        .child("Vibe Assembly"),
                )
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(ink_muted())
                        .child("Synced just now"),
                ),
        )
        .child(div().flex_1())
        .child(search())
        .child(div().flex_1())
        .child(bare_icon("help", "info"))
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
        .size(px(30.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(9.0))
        .bg(raised())
        .border_1()
        .border_color(border())
        .shadow(soft_shadow())
        .child(branding::logo(20.0))
}

fn search() -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .w(px(400.0))
        .h(px(34.0))
        .px(px(12.0))
        .rounded_full()
        .bg(raised())
        .border_1()
        .border_color(border())
        .shadow(soft_shadow())
        .child(icon("search", 14.0, ink_muted()))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(12.5))
                .text_color(ink_muted())
                .child("Search files or ask anything…"),
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

// --------------------------------------------------------------------- dock

fn dock() -> Div {
    div()
        .w(px(266.0))
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .px(px(12.0))
        .pt(px(14.0))
        .pb(px(10.0))
        .bg(paper())
        .border_r_1()
        .border_color(border())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .h(px(28.0))
                .px(px(4.0))
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink())
                        .child("Project"),
                )
                .child(div().flex_1())
                .child(bare_icon("project-add", "plus")),
        )
        .child(div().h(px(6.0)))
        .child(folder_row("src", true))
        .child(file_row("theme.rs", 16.0, false, false))
        .child(file_row("widgets.rs", 16.0, false, false))
        .child(file_row("state.rs", 16.0, false, true))
        .child(folder_row("ui_kit", false))
        .child(folder_row("assets", false))
        .child(file_row("Cargo.toml", 0.0, false, false))
        .child(div().flex_1())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .h(px(28.0))
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

fn folder_row(label: &'static str, open: bool) -> Stateful<Div> {
    div()
        .id(label)
        .flex()
        .items_center()
        .gap(px(6.0))
        .h(px(ROW))
        .px(px(6.0))
        .rounded(px(R_SM))
        .cursor_pointer()
        .hover(|style| style.bg(hover_wash()))
        .child(icon(
            if open {
                "chevron-down"
            } else {
                "chevron-right"
            },
            12.0,
            ink_muted(),
        ))
        .child(icon(
            if open { "folder-open" } else { "folder" },
            14.0,
            ink_muted(),
        ))
        .child(
            div()
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(ink_soft())
                .child(label),
        )
}

fn file_row(label: &'static str, indent: f32, active: bool, modified: bool) -> Stateful<Div> {
    div()
        .id(label)
        .flex()
        .items_center()
        .gap(px(7.0))
        .h(px(ROW))
        .pl(px(6.0 + indent))
        .pr(px(6.0))
        .rounded(px(R_SM))
        .cursor_pointer()
        .when(active, |this| this.bg(brand_soft()))
        .when(!active, |this| this.hover(|style| style.bg(hover_wash())))
        .child(icon(
            "file-code",
            13.0,
            if active { brand_ink() } else { ink_muted() },
        ))
        .child(
            div()
                .text_size(px(12.5))
                .when(active, |this| {
                    this.font_weight(FontWeight::SEMIBOLD).text_color(ink())
                })
                .when(!active, |this| this.text_color(ink_soft()))
                .child(label),
        )
        .child(div().flex_1())
        .when(modified, |this| {
            this.child(div().size(px(6.0)).rounded_full().bg(warning()))
        })
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
                .id("thread")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_center()
                        .px(px(20.0))
                        .pt(px(18.0))
                        .pb(px(10.0))
                        .child(
                            div()
                                .w_full()
                                .max_w(px(720.0))
                                .flex()
                                .flex_col()
                                .gap(px(16.0))
                                .child(welcome_turn())
                                .child(user_turn())
                                .child(work_turn())
                                .child(suggestions()),
                        ),
                ),
        )
        .child(composer())
        .child(
            div().w_full().flex().justify_center().pb(px(10.0)).child(
                div()
                    .text_size(px(10.0))
                    .text_color(ink_muted())
                    .child("Vibe can make mistakes — review every change before you apply."),
            ),
        )
}

fn conversation_header() -> Div {
    div()
        .h(px(56.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(18.0))
        .bg(paper())
        .border_b_1()
        .border_color(border())
        .child(
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
                .child(icon("message-circle", 15.0, brand_ink())),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink())
                        .child("Restyle the workbench"),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .child(div().size(px(5.0)).rounded_full().bg(success()))
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(ink_muted())
                                .child("Vibe · Code mode · this session"),
                        ),
                ),
        )
        .child(div().flex_1())
        .child(soft_chip("6 files changed"))
        .child(bare_icon("session-history", "history"))
        .child(bare_icon("session-new", "plus"))
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

fn vibe_turn(content: impl IntoElement) -> Div {
    div()
        .flex()
        .items_start()
        .gap(px(10.0))
        .child(vibe_avatar(26.0))
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

fn welcome_turn() -> Div {
    vibe_turn(
        div()
            .max_w(px(560.0))
            .p(px(13.0))
            .rounded(px(14.0))
            .bg(raised())
            .border_1()
            .border_color(border())
            .shadow(soft_shadow())
            .text_size(px(12.5))
            .line_height(px(19.0))
            .text_color(ink_soft())
            .child(
                "Hi — I read through the workbench and restyled it around your brand. \
                 Six files, 86 lines, and nothing changes until you apply it.",
            ),
    )
}

fn user_turn() -> Div {
    div().w_full().flex().justify_end().child(
        div()
            .max_w(px(460.0))
            .p(px(12.0))
            .rounded(px(14.0))
            .bg(brand_soft())
            .text_size(px(12.5))
            .line_height(px(19.0))
            .text_color(ink())
            .child("Make it feel warmer and easier on the eyes."),
    )
}

fn work_turn() -> Div {
    vibe_turn(work_card())
}

fn work_card() -> Div {
    div()
        .w_full()
        .p(px(14.0))
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
                .items_center()
                .child(
                    div()
                        .text_size(px(12.5))
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
        .child(check_line("Warmed every surface — 12 colors"))
        .child(check_line("Kept brand yellow for the main action"))
        .child(check_line("Rewrote the comments in plain English"))
        .child(diff_preview())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .pt(px(2.0))
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
                .h(px(30.0))
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
                .py(px(6.0))
                .child(diff_row(" ", None, &[cm("@@ paper and ink @@")]))
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
                    &[p("    "), fx("rgb"), p("("), nu("0xFAF6EC"), p(")")],
                ))
                .child(diff_row(" ", None, &[cm("@@ brand @@")]))
                .child(diff_row(
                    " ",
                    None,
                    &[kw("pub fn "), fx("brand"), p("() -> "), ty("Rgba"), p(" {")],
                ))
                .child(diff_row(
                    "-",
                    Some(danger_soft()),
                    &[p("    "), fx("rgb"), p("("), nu("0xFFD60A"), p(")")],
                ))
                .child(diff_row(
                    "+",
                    Some(success_soft()),
                    &[p("    "), fx("rgb"), p("("), nu("0xFDCC02"), p(")")],
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

fn suggestions() -> Div {
    div()
        .pl(px(36.0))
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(suggestion("Show me the diff"))
        .child(suggestion("Try a darker variant"))
        .child(suggestion("Use my accent"))
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
        .px(px(20.0))
        .pb(px(8.0))
        .child(
            div()
                .w_full()
                .max_w(px(720.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .p(px(10.0))
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
                        .h(px(26.0))
                        .px(px(4.0))
                        .text_size(px(12.5))
                        .text_color(ink_muted())
                        .child("Ask Vibe to change something…")
                        .child(div().w(px(1.5)).h(px(14.0)).bg(brand())),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(bare_icon("add-context", "plus"))
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
        .id("send")
        .size(px(30.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(brand_soft())
        .border_1()
        .border_color(rgba(0xFDCC0259))
        .cursor_pointer()
        .hover(|style| style.bg(brand()))
        .child(icon("arrow-up", 14.0, brand_ink()))
}

// ------------------------------------------------------------- session panel

fn session_panel() -> Div {
    div()
        .w(px(360.0))
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .bg(paper())
        .border_l_1()
        .border_color(border())
        .child(session_header())
        .child(
            div()
                .id("session-details")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .p(px(14.0))
                .child(changed_files_card())
                .child(checks_card())
                .child(context_card()),
        )
}

fn session_header() -> Div {
    div()
        .h(px(56.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(16.0))
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ink())
                .child("Session"),
        )
        .child(soft_chip("6 files"))
        .child(div().flex_1())
        .child(bare_icon("details-panel", "files"))
}

fn panel_card() -> Div {
    div()
        .rounded(px(R_LG))
        .bg(raised())
        .border_1()
        .border_color(border())
        .shadow(soft_shadow())
        .overflow_hidden()
}

fn changed_files_card() -> Div {
    panel_card()
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink())
                        .child("Changed files"),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .font_family(theme::code_font())
                        .text_size(px(10.5))
                        .text_color(ink_muted())
                        .child("+66 −20"),
                ),
        )
        .child(file_change_row(
            "change-theme",
            "theme.rs",
            "va-desktop/src",
            42,
            12,
            true,
        ))
        .child(file_change_row(
            "change-widgets",
            "widgets.rs",
            "ui_kit/src",
            12,
            3,
            false,
        ))
        .child(file_change_row(
            "change-dock",
            "left_dock.rs",
            "va-desktop/src",
            4,
            2,
            false,
        ))
        .child(file_change_row(
            "change-assistant",
            "assistant_panel.rs",
            "va-desktop/src",
            4,
            1,
            false,
        ))
        .child(file_change_row(
            "change-editor",
            "editor_pane.rs",
            "va-desktop/src",
            2,
            1,
            false,
        ))
        .child(file_change_row(
            "change-icon",
            "icon.rs",
            "va-desktop/src",
            2,
            1,
            false,
        ))
        .child(
            div()
                .flex()
                .items_center()
                .pt(px(2.0))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(ink_muted())
                        .child("Nothing applied yet"),
                )
                .child(div().flex_1())
                .child(link_button("open-diff", "Open full diff")),
        )
}

fn file_change_row(
    id: &'static str,
    name: &'static str,
    dir: &'static str,
    added: u32,
    removed: u32,
    active: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(9.0))
        .h(px(38.0))
        .px(px(8.0))
        .rounded(px(10.0))
        .cursor_pointer()
        .when(active, |this| this.bg(brand_soft()))
        .when(!active, |this| this.hover(|style| style.bg(hover_wash())))
        .child(
            div()
                .size(px(26.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(7.0))
                .bg(sunken())
                .child(icon(
                    "file-code",
                    13.0,
                    if active { brand_ink() } else { ink_muted() },
                )),
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
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(ink())
                        .child(name),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(if active { ink_soft() } else { ink_muted() })
                        .truncate()
                        .child(dir),
                ),
        )
        .child(
            div()
                .font_family(theme::code_font())
                .text_size(px(10.5))
                .text_color(success())
                .child(format!("+{added}")),
        )
        .child(
            div()
                .font_family(theme::code_font())
                .text_size(px(10.5))
                .text_color(danger())
                .child(format!("−{removed}")),
        )
}

fn checks_card() -> Div {
    panel_card()
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink())
                        .child("Checks"),
                )
                .child(div().flex_1())
                .child(soft_chip("just ran")),
        )
        .child(check_stat_row("Builds clean", "2.1s"))
        .child(check_stat_row("200 tests pass", "0.8s"))
        .child(check_warning_row())
}

fn check_stat_row(label: &'static str, meta: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .h(px(26.0))
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
        .child(div().flex_1())
        .child(
            div()
                .font_family(theme::code_font())
                .text_size(px(10.5))
                .text_color(ink_muted())
                .child(meta),
        )
}

fn check_warning_row() -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .h(px(26.0))
        .child(
            div()
                .size(px(16.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(warning_soft())
                .child(icon("triangle-alert", 10.0, warning())),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(ink_soft())
                .child("3 minor warnings"),
        )
        .child(div().flex_1())
        .child(link_button("review-warnings", "Review"))
}

fn context_card() -> Div {
    panel_card()
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink())
                        .child("Context"),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(ink_muted())
                        .child("62% used"),
                ),
        )
        .child(context_row("folder-open", "src/ attached", "24 files"))
        .child(context_row("code", "Auto · Rust", "execution mode"))
        .child(context_row("lock", "Local only", "nothing uploaded"))
        .child(
            div()
                .h(px(4.0))
                .w_full()
                .rounded_full()
                .bg(sunken())
                .child(div().h(px(4.0)).w(px(191.0)).rounded_full().bg(ink_muted())),
        )
}

fn context_row(icon_name: &'static str, label: &'static str, meta: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .h(px(24.0))
        .child(icon(icon_name, 12.0, ink_muted()))
        .child(
            div()
                .text_size(px(11.5))
                .text_color(ink_soft())
                .child(label),
        )
        .child(div().flex_1())
        .child(
            div()
                .text_size(px(10.0))
                .text_color(ink_muted())
                .child(meta),
        )
}

// ------------------------------------------------------------------- status

fn status_bar(mode: ExecutionMode) -> Div {
    div()
        .h(px(STATUS))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(14.0))
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
        .child(div().child("3 minor warnings"))
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

fn link_button(id: &'static str, label: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(2.0))
        .h(px(24.0))
        .px(px(6.0))
        .rounded(px(6.0))
        .cursor_pointer()
        .hover(|style| style.bg(sunken()))
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(brand_ink())
                .child(label),
        )
        .child(icon("chevron-right", 11.0, brand_ink()))
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
