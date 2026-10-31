//! Liquid Glass — black and yellow, with the glass kept for the ask bar and
//! the auto-hiding edge drawer.
//!
//! Solid graphite panes sit on a near-black canvas lit by one warm yellow
//! glow; glass is reserved for the composer, the model pill, the session
//! drawer, and the nav rail. GPUI 0.2.2 has no in-app backdrop blur, so frost
//! is approximated with translucent fills, specular edges, and long shadows.

use std::f32::consts::{FRAC_PI_2, TAU};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    div, ease_out_quint, linear_color_stop, linear_gradient, point, pulsating_between, px, rgb,
    rgba, Animation, AnimationExt, AnyElement, AnyView, App, Background, BoxShadow, Context, Div,
    FocusHandle, FontWeight, IntoElement, KeyDownEvent, Rgba, ScrollHandle, Stateful,
    StyleRefinement, Window, WindowControlArea,
};
use va_core::modes::ExecutionMode;

use crate::branding;
use crate::design::icon::icon;
use crate::theme;

// ------------------------------------------------------------------ palette

fn ink() -> Rgba {
    rgb(0xF4F1E8)
}
fn ink_soft() -> Rgba {
    rgb(0xC8C3B6)
}
fn ink_muted() -> Rgba {
    rgb(0x9A9588)
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
    rgb(0x2B2200)
}
fn brand_tint() -> Rgba {
    rgba(0xFDCC0229)
}
fn brand_line() -> Rgba {
    rgba(0xFDCC0259)
}
fn success() -> Rgba {
    rgb(0x4ADE80)
}
fn success_tint() -> Rgba {
    rgba(0x4ADE8024)
}
fn warning() -> Rgba {
    rgb(0xF5B301)
}
fn warning_tint() -> Rgba {
    rgba(0xF5B30126)
}
fn danger() -> Rgba {
    rgb(0xFB7185)
}
fn danger_tint() -> Rgba {
    rgba(0xFB718526)
}
fn hover_wash() -> Rgba {
    rgba(0xFFFFFF10)
}
fn press_wash() -> Rgba {
    rgba(0xFFFFFF1F)
}

// Panels are solid; glass is a deliberate accent, not the default surface.
fn panel_fill() -> Background {
    linear_gradient(
        180.0,
        linear_color_stop(rgb(0x141416), 0.0),
        linear_color_stop(rgb(0x0C0C0E), 1.0),
    )
}
fn panel_border() -> Rgba {
    rgba(0xFFFFFF17)
}
fn panel_border_soft() -> Rgba {
    rgba(0xFFFFFF0D)
}
fn card_fill() -> Rgba {
    rgba(0xFFFFFF08)
}
fn well() -> Rgba {
    rgb(0x08080A)
}

fn glass_fill() -> Background {
    linear_gradient(
        180.0,
        linear_color_stop(rgba(0xFFFFFF18), 0.0),
        linear_color_stop(rgba(0xFFFFFF08), 1.0),
    )
}

fn glass_fill_strong() -> Background {
    linear_gradient(
        180.0,
        linear_color_stop(rgba(0xFFFFFF26), 0.0),
        linear_color_stop(rgba(0xFFFFFF0F), 1.0),
    )
}

fn nav_fill() -> Background {
    linear_gradient(
        180.0,
        linear_color_stop(rgba(0xFFFFFF40), 0.0),
        linear_color_stop(rgba(0xFFFFFF14), 1.0),
    )
}
fn glass_border() -> Rgba {
    rgba(0xFFFFFF3D)
}

// The drawer floats over chat content, and GPUI 0.2.2 cannot blur what is
// behind it — so it stays effectively opaque, glass only at its edges.
fn drawer_fill() -> Background {
    linear_gradient(
        180.0,
        linear_color_stop(rgb(0x17171A), 0.0),
        linear_color_stop(rgb(0x0E0E10), 1.0),
    )
}

// Same constraint for the search sheet: mostly frosted, never see-through
// enough to read the chat beneath it.
fn sheet_fill() -> Background {
    linear_gradient(
        180.0,
        linear_color_stop(rgba(0x2A2A2EF0), 0.0),
        linear_color_stop(rgba(0x131316FA), 1.0),
    )
}

// ------------------------------------------------------------------- syntax

fn syn_keyword() -> Rgba {
    rgb(0xD8CBAF)
}
fn syn_call() -> Rgba {
    rgb(0xAFC4CC)
}
fn syn_type() -> Rgba {
    rgb(0xABC2AC)
}
fn syn_number() -> Rgba {
    rgb(0xD8B67C)
}
fn syn_comment() -> Rgba {
    rgb(0x7C786E)
}
fn punct() -> Rgba {
    rgb(0xA5A096)
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
const TITLEBAR: f32 = 56.0;
const STATUS: f32 = 30.0;
const NAV_ITEM: f32 = 40.0;
const NAV_W: f32 = 56.0;
const DRAWER_W: f32 = 340.0;
const DRAWER_GAP: f32 = 10.0;
// The nav floats over the drawer's right edge, so the panel owns that strip
// and its content keeps the old 316px width.
const DRAWER_OUTER: f32 = NAV_W + DRAWER_GAP + DRAWER_W;
const DRAWER_PAD_R: f32 = DRAWER_GAP + NAV_W + 12.0;
const CHAT_ROW: f32 = 44.0;
const CONTEXT_TICKS: usize = 48;
const CONTEXT_RING: f32 = 46.0;
const CONTEXT_CAPSULE_H: f32 = 62.0;

fn soft_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: rgba(0x00000059).into(),
        offset: point(px(0.0), px(8.0)),
        blur_radius: px(22.0),
        spread_radius: px(0.0),
    }]
}

fn panel_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: rgba(0x00000099).into(),
        offset: point(px(0.0), px(18.0)),
        blur_radius: px(44.0),
        spread_radius: px(0.0),
    }]
}

fn glass_shadow() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: rgba(0x000000B3).into(),
            offset: point(px(0.0), px(12.0)),
            blur_radius: px(30.0),
            spread_radius: px(0.0),
        },
        BoxShadow {
            color: rgba(0x0000004D).into(),
            offset: point(px(0.0), px(2.0)),
            blur_radius: px(8.0),
            spread_radius: px(0.0),
        },
    ]
}

fn lifted_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: rgba(0x000000A6).into(),
        offset: point(px(0.0), px(14.0)),
        blur_radius: px(34.0),
        spread_radius: px(0.0),
    }]
}

fn brand_glow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: rgba(0xFDCC0259).into(),
        offset: point(px(0.0), px(6.0)),
        blur_radius: px(20.0),
        spread_radius: px(0.0),
    }]
}

// -------------------------------------------------------------------- motion

/// Windows' "Animation effects" setting, read once at startup. When it is off
/// the sample snaps between states instead of sliding, and pulsing indicators
/// hold still.
static REDUCE_MOTION: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
fn prefers_reduced_motion() -> bool {
    #[link(name = "user32")]
    #[allow(non_snake_case)]
    extern "system" {
        fn SystemParametersInfoW(
            action: u32,
            param: u32,
            out: *mut std::ffi::c_void,
            win_ini: u32,
        ) -> i32;
    }
    const SPI_GETCLIENTAREAANIMATION: u32 = 0x1042;
    let mut enabled: i32 = 1;
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            (&mut enabled as *mut i32).cast(),
            0,
        )
    };
    ok != 0 && enabled == 0
}

#[cfg(not(windows))]
fn prefers_reduced_motion() -> bool {
    false
}

fn calm() -> bool {
    REDUCE_MOTION.load(Ordering::Relaxed)
}

fn motion(ms: u64) -> Duration {
    Duration::from_millis(if calm() { 1 } else { ms })
}

// Toggle chips already use brand as their "on" colour; focus stays neutral so
// a focused-but-off chip never reads as active.
fn focus_ring_neutral(style: StyleRefinement) -> StyleRefinement {
    style.shadow(vec![BoxShadow {
        color: rgba(0xF4F1E899).into(),
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(0.0),
        spread_radius: px(2.0),
    }])
}

struct HintView {
    label: &'static str,
}

impl Render for HintView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .max_w(px(260.0))
            .px(px(9.0))
            .py(px(5.0))
            .rounded(px(7.0))
            .bg(rgb(0x1B1B1E))
            .border_1()
            .border_color(rgba(0xFFFFFF24))
            .shadow(soft_shadow())
            .text_size(px(11.0))
            .line_height(px(15.0))
            .text_color(ink_soft())
            .child(self.label)
    }
}

fn hint(label: &'static str) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    move |_window, cx| cx.new(|_| HintView { label }).into()
}

// ------------------------------------------------------------------- screen

#[derive(Clone, Copy, PartialEq, Eq)]
enum DrawerSection {
    Session,
    Changes,
    Editor,
    History,
}

impl DrawerSection {
    fn title(self) -> &'static str {
        match self {
            Self::Session => "Session",
            Self::Changes => "Changes",
            Self::Editor => "Editor",
            Self::History => "History",
        }
    }

    fn chip(self) -> &'static str {
        match self {
            Self::Session => "6 files",
            Self::Changes => "+66 −20",
            Self::Editor => "theme.rs",
            Self::History => "24 runs",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Session => "message-circle",
            Self::Changes => "files",
            Self::Editor => "code",
            Self::History => "history",
        }
    }

    fn tooltip(self) -> &'static str {
        match self {
            Self::Session => "Session — click to pin",
            Self::Changes => "Changes — click to pin",
            Self::Editor => "Editor — click to pin",
            Self::History => "History — click to pin",
        }
    }

    fn body_id(self) -> &'static str {
        match self {
            Self::Session => "glass-body-session",
            Self::Changes => "glass-body-changes",
            Self::Editor => "glass-body-editor",
            Self::History => "glass-body-history",
        }
    }

    fn cycle(self, forward: bool) -> Self {
        let all = [Self::Session, Self::Changes, Self::Editor, Self::History];
        let index = all.iter().position(|s| *s == self).unwrap_or(0);
        all[if forward {
            (index + 1) % all.len()
        } else {
            (index + all.len() - 1) % all.len()
        }]
    }
}

pub struct GlassPreview {
    mode: ExecutionMode,
    nav_open: bool,
    nav_epoch: usize,
    nav_pinned: bool,
    nav_hovered: bool,
    section: DrawerSection,
    section_epoch: usize,
    context_epoch: usize,
    search_open: bool,
    search_epoch: usize,
    show_tokens: bool,
    show_warnings: bool,
    nav_focus: FocusHandle,
    search_focus: FocusHandle,
    thread_scroll: ScrollHandle,
}

impl GlassPreview {
    pub fn new(mode: ExecutionMode, cx: &mut Context<Self>) -> Self {
        REDUCE_MOTION.store(prefers_reduced_motion(), Ordering::Relaxed);
        // Chats open on the latest turn, not the oldest.
        let thread_scroll = ScrollHandle::new();
        thread_scroll.scroll_to_bottom();
        Self {
            mode,
            nav_open: false,
            nav_epoch: 0,
            nav_pinned: false,
            nav_hovered: false,
            section: DrawerSection::Session,
            section_epoch: 0,
            context_epoch: 0,
            search_open: false,
            search_epoch: 0,
            show_tokens: false,
            show_warnings: false,
            nav_focus: cx.focus_handle(),
            search_focus: cx.focus_handle(),
            thread_scroll,
        }
    }

    fn open_section(&mut self, section: DrawerSection) {
        if self.section != section {
            self.section = section;
            self.section_epoch += 1;
        }
        if !self.nav_open {
            self.nav_open = true;
            self.nav_epoch += 1;
        }
        self.nav_hovered = true;
    }

    fn open_search(&mut self, window: &mut Window) {
        self.search_open = true;
        self.search_epoch += 1;
        window.focus(&self.search_focus);
    }

    fn close_search(&mut self) {
        if !self.search_open {
            return;
        }
        self.search_open = false;
        self.search_epoch += 1;
    }

    fn on_search_key(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.is_held {
            return;
        }
        if event.keystroke.key == "escape" {
            self.close_search();
            cx.notify();
        }
    }

    fn set_pinned(&mut self, pinned: bool) {
        if self.nav_pinned == pinned {
            return;
        }
        self.nav_pinned = pinned;
        let open = pinned || self.nav_hovered;
        if self.nav_open != open {
            self.nav_open = open;
            self.nav_epoch += 1;
        }
    }

    fn close_nav(&mut self) {
        self.nav_pinned = false;
        if self.nav_open {
            self.nav_open = false;
            self.nav_epoch += 1;
        }
    }

    fn toggle_tokens(&mut self) {
        self.show_tokens = !self.show_tokens;
    }

    fn toggle_warnings(&mut self) {
        self.show_warnings = !self.show_warnings;
    }
}

impl Render for GlassPreview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mode = self.mode;
        let nav_open = self.nav_open;
        let nav_epoch = self.nav_epoch;
        let nav_pinned = self.nav_pinned;
        let section = self.section;
        let section_epoch = self.section_epoch;
        let context_epoch = self.context_epoch;
        let search_open = self.search_open;
        let search_epoch = self.search_epoch;
        let show_tokens = self.show_tokens;
        let show_warnings = self.show_warnings;
        let nav_focus = self.nav_focus.clone();
        let search_focus = self.search_focus.clone();
        let thread_scroll = self.thread_scroll.clone();
        // The nav floats over the thread now, so a closed drawer reserves
        // nothing; below ~600 the header drops the redundant file chip and Auto.
        let reserve = if nav_open {
            DRAWER_OUTER + DRAWER_GAP
        } else {
            0.0
        };
        let thread_w = window.viewport_size().width - px(304.0 + reserve);
        let compact = thread_w < px(600.0);
        div()
            .size_full()
            .relative()
            .overflow_hidden()
            .flex()
            .flex_col()
            .font_family(theme::ui_font())
            .text_color(ink())
            .child(backdrop())
            .child(titlebar(cx))
            .child({
                let content = div()
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .child(
                        div()
                            .size_full()
                            .flex()
                            .gap(px(12.0))
                            .px(px(14.0))
                            .pb(px(8.0))
                            .child(rail())
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .h_full()
                                    .child(conversation(
                                        &thread_scroll,
                                        nav_pinned,
                                        show_tokens,
                                        show_warnings,
                                        compact,
                                        cx,
                                    ))
                                    .with_animation(
                                        ("glass-conv", nav_epoch),
                                        Animation::new(motion(320)).with_easing(ease_out_quint()),
                                        move |el, t| {
                                            let k = edge_reveal(nav_open, nav_epoch, t);
                                            // Only an open panel pushes the thread;
                                            // the floating nav never claims a gutter.
                                            el.pr(px((DRAWER_OUTER + DRAWER_GAP) * k))
                                        },
                                    ),
                            ),
                    )
                    .child(edge_overlay(
                        EdgeState {
                            open: nav_open,
                            epoch: nav_epoch,
                            pinned: nav_pinned,
                            section,
                            section_epoch,
                            show_tokens,
                            show_warnings,
                            context_epoch,
                        },
                        &nav_focus,
                        cx,
                    ));
                if search_open {
                    content.child(search_overlay(search_epoch, &search_focus, cx))
                } else {
                    content
                }
            })
            .child(status_bar(mode, nav_pinned))
    }
}

// ----------------------------------------------------------------- backdrop

fn backdrop() -> Div {
    div()
        .size_full()
        .absolute()
        .top_0()
        .left_0()
        .bg(linear_gradient(
            180.0,
            linear_color_stop(rgb(0x0A0A0A), 0.0),
            linear_color_stop(rgb(0x050505), 1.0),
        ))
}

// ----------------------------------------------------------------- titlebar

fn titlebar(cx: &mut Context<GlassPreview>) -> Div {
    div()
        .h(px(TITLEBAR))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(16.0))
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
                        .text_size(px(11.0))
                        .text_color(ink_muted())
                        .child("Synced just now"),
                ),
        )
        .child(div().flex_1())
        .child(search(cx))
        .child(div().flex_1())
        .child(bare_icon("glass-help", "info", "Help"))
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
        .bg(rgb(0x151517))
        .border_1()
        .border_color(panel_border())
        .shadow(soft_shadow())
        .child(branding::logo(21.0))
}

fn search(cx: &mut Context<GlassPreview>) -> Stateful<Div> {
    div()
        .id("glass-search")
        .flex()
        .items_center()
        .gap(px(8.0))
        .w(px(420.0))
        .h(px(34.0))
        .px(px(12.0))
        .rounded_full()
        .bg(glass_fill())
        .border_1()
        .border_color(rgba(0xFFFFFF24))
        .shadow(soft_shadow())
        .cursor_pointer()
        .hover(|style| style.bg(glass_fill_strong()).border_color(glass_border()))
        .active(|style| style.bg(rgba(0xFFFFFF12)))
        .tooltip(hint("Search chats, files and commands"))
        .on_click(cx.listener(|this, _event, window, cx| {
            this.open_search(window);
            cx.notify();
        }))
        .child(icon("search", 14.0, ink_muted()))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(12.5))
                .text_color(ink_muted())
                .child("Search chats, files, commands…"),
        )
        .child(keycap("Ctrl K"))
}

fn search_overlay(
    epoch: usize,
    focus: &FocusHandle,
    cx: &mut Context<GlassPreview>,
) -> impl IntoElement {
    div()
        .id("glass-search-overlay")
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .flex()
        .justify_center()
        .items_start()
        .pt(px(16.0))
        .bg(rgba(0x050505A6))
        .on_click(cx.listener(|this, _event, _window, cx| {
            this.close_search();
            cx.notify();
        }))
        .child(
            div()
                .id("glass-search-panel")
                .w(px(560.0))
                .max_h(px(430.0))
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(px(18.0))
                .bg(sheet_fill())
                .border_1()
                .border_color(glass_border())
                .shadow(glass_shadow())
                .track_focus(focus)
                .on_key_down(cx.listener(GlassPreview::on_search_key))
                .on_click(cx.listener(|_, _event, _window, cx| cx.stop_propagation()))
                .child(
                    div()
                        .h(px(52.0))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .px(px(16.0))
                        .border_b_1()
                        .border_color(rgba(0xFFFFFF14))
                        .child(icon("search", 15.0, brand()))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(13.0))
                                .text_color(ink_muted())
                                .child("Search chats, files and commands…"),
                        )
                        .child(keycap("Esc")),
                )
                .child(search_group_label("RECENT"))
                .child(search_result(
                    "glass-search-chat",
                    "message-circle",
                    "Restyle the workbench",
                    "Chat · now",
                    true,
                ))
                .child(search_result(
                    "glass-search-file",
                    "file-code",
                    "glass.rs",
                    "File · crates/va-desktop",
                    false,
                ))
                .child(search_result(
                    "glass-search-command",
                    "play",
                    "Run tests",
                    "Command · 200 tests",
                    false,
                ))
                .child(search_result(
                    "glass-search-token",
                    "sparkles",
                    "Toggle glass tokens",
                    "Command · Appearance",
                    false,
                ))
                .child(div().h(px(6.0)))
                .child(
                    div()
                        .h(px(34.0))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .px(px(16.0))
                        .border_t_1()
                        .border_color(rgba(0xFFFFFF14))
                        .text_size(px(10.5))
                        .text_color(ink_muted())
                        .child("↑↓ to navigate")
                        .child(div().size(px(2.0)).rounded_full().bg(ink_muted()))
                        .child("↵ to open")
                        .child(div().flex_1())
                        .child("6 results"),
                )
                .with_animation(
                    ("glass-search-panel", epoch),
                    Animation::new(motion(200)).with_easing(ease_out_quint()),
                    move |el, t| el.opacity(t).mt(px(8.0 * (1.0 - t))),
                ),
        )
}

fn search_group_label(label: &'static str) -> Div {
    div()
        .h(px(26.0))
        .flex()
        .items_end()
        .px(px(16.0))
        .pb(px(4.0))
        .text_size(px(9.5))
        .font_weight(FontWeight::MEDIUM)
        .text_color(ink_muted())
        .child(label)
}

fn search_result(
    id: &'static str,
    name: &'static str,
    title: &'static str,
    meta: &'static str,
    active: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(10.0))
        .h(px(42.0))
        .mx(px(8.0))
        .px(px(8.0))
        .rounded(px(10.0))
        .border_1()
        .border_color(if active {
            brand_line()
        } else {
            rgba(0x00000000)
        })
        .when(active, |this| this.bg(brand_tint()))
        .when(!active, |this| {
            this.cursor_pointer()
                .hover(|style| style.bg(hover_wash()))
                .active(|style| style.bg(press_wash()))
        })
        .child(
            div()
                .size(px(26.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(7.0))
                .bg(if active { rgba(0xFDCC0226) } else { well() })
                .child(icon(name, 13.0, if active { brand() } else { ink_muted() })),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(12.5))
                .text_color(if active { ink() } else { ink_soft() })
                .child(title),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(10.5))
                .text_color(ink_muted())
                .child(meta),
        )
        .when(active, |this| this.child(keycap("↵")))
}

fn account_avatar() -> Div {
    div()
        .size(px(30.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(brand_tint())
        .border_1()
        .border_color(brand_line())
        .text_size(px(11.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(brand())
        .child("VA")
}

// --------------------------------------------------------------------- rail

fn rail() -> Div {
    div()
        .w(px(262.0))
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .px(px(10.0))
        .pt(px(12.0))
        .pb(px(10.0))
        .rounded(px(R_LG))
        .bg(panel_fill())
        .border_1()
        .border_color(panel_border())
        .shadow(panel_shadow())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .h(px(28.0))
                .px(px(6.0))
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink())
                        .child("Chats"),
                )
                .child(div().flex_1())
                .child(bare_icon("glass-chat-new", "plus", "New chat")),
        )
        .child(div().h(px(8.0)))
        .child(chat_section_label("TODAY"))
        .child(chat_row("Restyle the workbench", "now · 6 files", true))
        .child(chat_row("Glass dock, round two", "42m · 4 files", false))
        .child(chat_row("Midnight palette pass", "2h · 9 files", false))
        .child(div().h(px(12.0)))
        .child(chat_section_label("YESTERDAY"))
        .child(chat_row("Paper & Ink rollout", "9:40 PM · 12 files", false))
        .child(chat_row("Studio direction", "3:15 PM · 2 files", false))
        .child(chat_row("Obsidian first look", "11:02 AM", false))
        .child(div().flex_1())
        .child(project_chip())
}

fn chat_section_label(label: &'static str) -> Div {
    div()
        .h(px(24.0))
        .flex()
        .items_center()
        .px(px(8.0))
        .text_size(px(10.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(ink_muted())
        .child(label)
}

fn chat_row(title: &'static str, meta: &'static str, active: bool) -> Stateful<Div> {
    div()
        .id(title)
        .flex()
        .items_center()
        .gap(px(9.0))
        .h(px(CHAT_ROW))
        .px(px(8.0))
        .rounded(px(10.0))
        .border_1()
        .border_color(if active {
            brand_line()
        } else {
            rgba(0x00000000)
        })
        .cursor_pointer()
        .when(active, |this| this.bg(brand_tint()))
        .when(!active, |this| {
            this.hover(|style| style.bg(hover_wash()))
                .active(|style| style.bg(press_wash()))
        })
        .child(
            div()
                .size(px(24.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(7.0))
                .bg(if active { rgba(0xFDCC0226) } else { well() })
                .child(icon(
                    "message-circle",
                    12.0,
                    if active { brand() } else { ink_muted() },
                )),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(1.0))
                .child(
                    div()
                        .truncate()
                        .text_size(px(12.0))
                        .when(active, |this| {
                            this.font_weight(FontWeight::SEMIBOLD).text_color(ink())
                        })
                        .when(!active, |this| this.text_color(ink_soft()))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(ink_muted())
                        .child(meta),
                ),
        )
        .when(active, |this| {
            this.child(div().size(px(5.0)).rounded_full().bg(brand()))
        })
}

fn project_chip() -> Stateful<Div> {
    div()
        .id("glass-project")
        .flex()
        .items_center()
        .gap(px(8.0))
        .p(px(9.0))
        .rounded(px(12.0))
        .bg(card_fill())
        .border_1()
        .border_color(panel_border_soft())
        .cursor_pointer()
        .hover(|style| style.bg(hover_wash()))
        .active(|style| style.bg(press_wash()))
        .child(
            div()
                .size(px(26.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(7.0))
                .bg(well())
                .child(icon("folder", 13.0, ink_muted())),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(1.0))
                .child(
                    div()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(ink_soft())
                        .child("vibe-assembly"),
                )
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(ink_muted())
                        .child("src · 24 files · local"),
                ),
        )
        .child(icon("chevron-down", 11.0, ink_muted()))
}

// -------------------------------------------------------------- conversation

fn conversation(
    scroll: &ScrollHandle,
    pinned: bool,
    show_tokens: bool,
    show_warnings: bool,
    compact: bool,
    cx: &mut Context<GlassPreview>,
) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .h_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .rounded(px(R_LG))
        .bg(panel_fill())
        .border_1()
        .border_color(panel_border())
        .shadow(panel_shadow())
        .child(conversation_header(compact))
        .child(
            div()
                .flex_1()
                .min_h_0()
                .relative()
                .overflow_hidden()
                .child(
                    div()
                        .id("glass-thread")
                        .size_full()
                        .overflow_y_scroll()
                        .track_scroll(scroll)
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .justify_center()
                                .px(px(22.0))
                                .pt(px(13.0))
                                .pb(px(7.0))
                                .child(
                                    div()
                                        .w_full()
                                        .max_w(px(720.0))
                                        .flex()
                                        .flex_col()
                                        .gap(px(14.0))
                                        .child(intro_turn())
                                        .child(user_turn())
                                        .child(work_turn(compact)),
                                ),
                        ),
                )
                // Content slides under these fades, so a scrolled thread never
                // shows a bubble sliced by a hard edge.
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .h(px(22.0))
                        .bg(linear_gradient(
                            180.0,
                            linear_color_stop(rgba(0x141416D9), 0.0),
                            linear_color_stop(rgba(0x14141600), 1.0),
                        )),
                )
                .child(
                    div()
                        .absolute()
                        .bottom_0()
                        .left_0()
                        .right_0()
                        .h(px(26.0))
                        .bg(linear_gradient(
                            180.0,
                            linear_color_stop(rgba(0x14141600), 0.0),
                            linear_color_stop(rgba(0x141416E6), 1.0),
                        )),
                ),
        )
        .child(suggestions(pinned, show_tokens, show_warnings, compact, cx))
        .child(composer())
        .child(
            div().w_full().flex().justify_center().pb(px(8.0)).child(
                div()
                    .text_size(px(10.5))
                    .text_color(ink_muted())
                    .child("Vibe can make mistakes — review every change before you apply."),
            ),
        )
}

fn conversation_header(compact: bool) -> Div {
    div()
        .h(px(56.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(if compact { 8.0 } else { 10.0 }))
        .px(px(if compact { 14.0 } else { 18.0 }))
        .border_b_1()
        .border_color(panel_border_soft())
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(10.0))
                .child(
                    div()
                        .size(px(30.0))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .bg(brand_tint())
                        .border_1()
                        .border_color(brand_line())
                        .child(icon("message-circle", 15.0, brand())),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(13.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(ink())
                                .child("Restyle the workbench"),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(5.0))
                                .child(
                                    div()
                                        .size(px(5.0))
                                        .flex_shrink_0()
                                        .rounded_full()
                                        .bg(success()),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(11.5))
                                        .text_color(ink_muted())
                                        .child(if compact {
                                            "Vibe · Code mode"
                                        } else {
                                            "Vibe · Code mode · this session"
                                        }),
                                ),
                        ),
                ),
        )
        .child(model_pill(compact))
        .child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(4.0))
                // At full width both sides grow so the pill stays centred; in
                // compact the title needs the whole remainder instead.
                .when(!compact, |this| {
                    this.flex_1().child(soft_chip("6 files changed"))
                })
                .child(bare_icon("glass-history", "history", "Run history"))
                .child(bare_icon("glass-new", "plus", "New chat")),
        )
}

fn model_pill(compact: bool) -> Stateful<Div> {
    div()
        .id("glass-model")
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(6.0))
        .h(px(30.0))
        .px(px(if compact { 9.0 } else { 11.0 }))
        .rounded_full()
        .bg(glass_fill())
        .border_1()
        .border_color(rgba(0xFFFFFF2E))
        .shadow(soft_shadow())
        .cursor_pointer()
        .hover(|style| style.bg(glass_fill_strong()))
        .active(|style| style.bg(rgba(0xFFFFFF12)))
        .child(icon("sparkles", 12.0, brand()))
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(ink())
                .child("Opus 4.6"),
        )
        .when(!compact, |this| {
            this.child(div().w(px(1.0)).h(px(12.0)).bg(rgba(0xFFFFFF1F)))
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(ink_muted())
                        .child("Auto"),
                )
        })
        .child(icon("chevron-down", 11.0, ink_muted()))
}

fn vibe_avatar(size: f32) -> Div {
    div()
        .size(px(size))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(brand_tint())
        .border_1()
        .border_color(brand_line())
        .child(icon("sparkles", size * 0.48, brand()))
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
                                .text_size(px(11.0))
                                .text_color(ink_muted())
                                .child("just now"),
                        ),
                )
                .child(content),
        )
}

fn intro_turn() -> Div {
    vibe_turn(
        div()
            .w_full()
            .max_w(px(580.0))
            .p(px(11.0))
            .rounded(px(14.0))
            .bg(card_fill())
            .border_1()
            .border_color(panel_border_soft())
            .text_size(px(13.0))
            .line_height(px(20.0))
            .text_color(ink_soft())
            .child(
                "Took the sketch: chats on the left, the model pill up top, and a nav at the \
                 right edge. Hover a nav icon to open its panel — Session, Changes, Editor, \
                 History — and click it to pin. The context button sits beneath the nav.",
            ),
    )
}

fn user_turn() -> Div {
    div().w_full().flex().justify_end().child(
        div()
            .w_full()
            .max_w(px(500.0))
            .p(px(12.0))
            .rounded(px(14.0))
            .bg(rgb(0x1B1B1D))
            .border_1()
            .border_color(panel_border())
            .shadow(soft_shadow())
            .text_size(px(13.0))
            .line_height(px(20.0))
            .text_color(ink())
            .child(
                "Move Session and While it works into the auto-hide bar, animate it, and \
                 give the context window a refilling circle.",
            ),
    )
}

fn work_turn(compact: bool) -> Div {
    vibe_turn(work_card(compact))
}

fn work_card(compact: bool) -> Div {
    div()
        .w_full()
        .p(px(14.0))
        .rounded(px(R_LG))
        .bg(rgb(0x18181A))
        .border_1()
        .border_color(panel_border())
        .shadow(lifted_shadow())
        .flex()
        .flex_col()
        .gap(px(6.0))
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
                        .text_size(px(11.5))
                        .text_color(ink_muted())
                        .child("6 files · 86 lines"),
                ),
        )
        .child(check_line(
            "Nav opens Session, Changes, Editor and History on hover",
        ))
        .child(check_line("Click a nav icon or Pin to keep a panel open"))
        .child(check_line("Refilling ring shows the 62% context window"))
        .child(check_line(
            "Live chat still steers the run without stopping it",
        ))
        .child(diff_preview("chrome.rs", "crates/va-desktop/src", 42, 12))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .pt(px(2.0))
                .child(primary_button("Apply changes"))
                .child(secondary_button("Review diff"))
                .child(div().flex_1())
                // Below ~600px the row has no room for the reassurance copy.
                .when(!compact, |this| {
                    this.child(
                        div()
                            .text_size(px(11.0))
                            .text_color(ink_muted())
                            .child("You can undo anytime"),
                    )
                }),
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
                .bg(success_tint())
                .child(icon("check", 10.0, success())),
        )
        .child(
            div()
                .text_size(px(12.5))
                .text_color(ink_soft())
                .child(label),
        )
}

fn diff_preview(name: &'static str, dir: &'static str, added: u32, removed: u32) -> Div {
    div()
        .mt(px(2.0))
        .rounded(px(12.0))
        .bg(well())
        .border_1()
        .border_color(panel_border_soft())
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
                .border_b_1()
                .border_color(panel_border_soft())
                .child(icon("file-code", 12.0, brand()))
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(ink())
                        .child(name),
                )
                .child(div().text_size(px(11.0)).text_color(ink_muted()).child(dir))
                .child(div().flex_1())
                .child(
                    div()
                        .font_family(theme::code_font())
                        .text_size(px(11.0))
                        .text_color(success())
                        .child(format!("+{added}")),
                )
                .child(
                    div()
                        .font_family(theme::code_font())
                        .text_size(px(11.0))
                        .text_color(danger())
                        .child(format!("−{removed}")),
                ),
        )
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .py(px(5.0))
                .child(diff_row(" ", None, &[cm("@@ edge bar @@")]))
                .child(diff_row(
                    " ",
                    None,
                    &[
                        kw("pub fn "),
                        fx("edge_bar"),
                        p("() -> "),
                        ty("Div"),
                        p(" {"),
                    ],
                ))
                .child(diff_row(
                    "-",
                    Some(danger_tint()),
                    &[p("    "), fx("right_column"), p("()")],
                ))
                .child(diff_row(
                    "+",
                    Some(success_tint()),
                    &[p("    "), fx("edge_drawer"), p("()")],
                ))
                .child(diff_row(" ", None, &[cm("@@ context ring @@")]))
                .child(diff_row(
                    "-",
                    Some(danger_tint()),
                    &[p("    "), fx("context_orb"), p("()")],
                ))
                .child(diff_row(
                    "+",
                    Some(success_tint()),
                    &[p("    "), fx("context_ring"), p("("), nu("0.62"), p(")")],
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
        .h(px(17.0))
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

fn suggestions(
    pinned: bool,
    show_tokens: bool,
    show_warnings: bool,
    compact: bool,
    cx: &mut Context<GlassPreview>,
) -> Div {
    div()
        .w_full()
        .flex()
        .justify_center()
        .px(px(20.0))
        .pb(px(7.0))
        .child(
            div()
                .w_full()
                .max_w(px(720.0))
                .pl(px(if compact { 0.0 } else { 36.0 }))
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(6.0))
                .child(suggestion(
                    "glass-suggest-pin",
                    if pinned {
                        "Unpin the panel"
                    } else {
                        "Pin the panel"
                    },
                    pinned,
                    |this| this.set_pinned(!this.nav_pinned),
                    cx,
                ))
                .child(suggestion(
                    "glass-suggest-tokens",
                    if show_tokens {
                        "Hide token counts"
                    } else {
                        "Show token counts"
                    },
                    show_tokens,
                    GlassPreview::toggle_tokens,
                    cx,
                ))
                .child(suggestion(
                    "glass-suggest-warnings",
                    if show_warnings {
                        "Hide warnings"
                    } else {
                        "Explain warnings"
                    },
                    show_warnings,
                    GlassPreview::toggle_warnings,
                    cx,
                )),
        )
}

fn suggestion(
    id: &'static str,
    label: &'static str,
    active: bool,
    action: impl Fn(&mut GlassPreview) + 'static,
    cx: &mut Context<GlassPreview>,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .h(px(28.0))
        .px(px(11.0))
        .rounded_full()
        .bg(rgba(0xFFFFFF0C))
        .border_1()
        .border_color(if active { brand_line() } else { panel_border() })
        .text_size(px(11.5))
        .text_color(if active { ink() } else { ink_soft() })
        .cursor_pointer()
        .when(active, |this| this.bg(brand_tint()))
        .when(!active, |this| this.hover(|style| style.bg(hover_wash())))
        .active(|style| style.bg(press_wash()))
        .on_click(cx.listener(move |this, _event, _window, cx| {
            action(this);
            cx.notify();
        }))
        .child(label)
}

fn composer() -> Div {
    div()
        .w_full()
        .flex()
        .justify_center()
        .px(px(20.0))
        .pb(px(10.0))
        .child(
            div()
                .relative()
                .overflow_hidden()
                .w_full()
                .max_w(px(720.0))
                .flex()
                .flex_col()
                .gap(px(24.0))
                .p(px(16.0))
                .rounded(px(24.0))
                .bg(glass_fill_strong())
                .border_1()
                .border_color(glass_border())
                .shadow(glass_shadow())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(2.0))
                        .px(px(2.0))
                        .pt(px(2.0))
                        .text_size(px(15.0))
                        .text_color(ink_muted())
                        .child("Ask me anything…"),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(composer_icon("glass-attach", "plus", "Add attachment"))
                        .child(composer_icon("glass-web", "globe", "Browse the web"))
                        .child(div().flex_1())
                        .child(
                            div()
                                .pr(px(4.0))
                                .font_family(theme::code_font())
                                .text_size(px(10.5))
                                .text_color(ink_muted())
                                .child("Rust"),
                        )
                        .child(composer_icon("glass-mic", "mic", "Voice input"))
                        .child(send_button()),
                ),
        )
}

fn composer_icon(id: &'static str, name: &'static str, label: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .size(px(30.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(8.0))
        .cursor_pointer()
        .hover(|style| style.bg(hover_wash()))
        .active(|style| style.bg(press_wash()))
        .tooltip(hint(label))
        .child(icon(name, 15.0, ink_soft()))
}

fn send_button() -> Stateful<Div> {
    div()
        .id("glass-send")
        .size(px(32.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(brand())
        .shadow(brand_glow())
        .cursor_pointer()
        .hover(|style| style.bg(brand_hover()))
        .active(|style| style.bg(brand_press()))
        .tooltip(hint("Send"))
        .child(icon("arrow-up", 14.0, brand_ink()))
}

// ------------------------------------------------------------- session cards

fn drawer_header(
    section: DrawerSection,
    pinned: bool,
    show_tokens: bool,
    focus: &FocusHandle,
    cx: &mut Context<GlassPreview>,
) -> Div {
    div()
        .h(px(30.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(2.0))
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ink())
                .child(section.title()),
        )
        .child(soft_chip(section.chip()))
        .when(show_tokens, |this| this.child(soft_chip("75k / 120k")))
        .child(div().flex_1())
        .child(pin_toggle(pinned, focus, cx))
        .child(drawer_close(cx))
}

fn pin_toggle(pinned: bool, focus: &FocusHandle, cx: &mut Context<GlassPreview>) -> Stateful<Div> {
    div()
        .id("glass-drawer-pin")
        .flex()
        .items_center()
        .gap(px(5.0))
        .h(px(26.0))
        .px(px(8.0))
        .flex_shrink_0()
        .rounded(px(7.0))
        .border_1()
        .border_color(if pinned {
            brand_line()
        } else {
            rgba(0x00000000)
        })
        .bg(if pinned {
            brand_tint()
        } else {
            rgba(0x00000000)
        })
        .text_size(px(11.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(if pinned { brand() } else { ink_muted() })
        .cursor_pointer()
        .track_focus(focus)
        .focus(focus_ring_neutral)
        .hover(move |style| {
            style.bg(if pinned {
                rgba(0xFDCC0238)
            } else {
                hover_wash()
            })
        })
        .active(|style| style.bg(press_wash()))
        .tooltip(hint(if pinned { "Unpin panel" } else { "Pin panel" }))
        .on_click(cx.listener(|this, _event, _window, cx| {
            let next = !this.nav_pinned;
            this.set_pinned(next);
            cx.notify();
        }))
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
            if event.is_held {
                return;
            }
            match event.keystroke.key.as_str() {
                "enter" | "space" => {
                    let next = !this.nav_pinned;
                    this.set_pinned(next);
                    cx.notify();
                }
                "escape" => {
                    this.close_nav();
                    cx.notify();
                }
                _ => {}
            }
        }))
        .child(icon(
            "pin",
            12.0,
            if pinned { brand() } else { ink_muted() },
        ))
        .child(if pinned { "Pinned" } else { "Pin" })
}

fn drawer_close(cx: &mut Context<GlassPreview>) -> Stateful<Div> {
    div()
        .id("glass-drawer-close")
        .size(px(26.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(7.0))
        .cursor_pointer()
        .hover(|style| style.bg(hover_wash()))
        .active(|style| style.bg(press_wash()))
        .tooltip(hint("Hide panel"))
        .on_click(cx.listener(|this, _event, _window, cx| {
            this.close_nav();
            cx.notify();
        }))
        .child(icon("chevron-right", 13.0, ink_muted()))
}

fn panel_card() -> Div {
    div()
        .rounded(px(14.0))
        .bg(card_fill())
        .border_1()
        .border_color(panel_border_soft())
        .overflow_hidden()
}

fn changed_files_card() -> Div {
    panel_card()
        .flex_shrink_0()
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
                        .text_size(px(11.0))
                        .text_color(ink_muted())
                        .child("+66 −20"),
                ),
        )
        .child(file_change_row(
            "glass-change-theme",
            "theme.rs",
            "va-desktop/src",
            42,
            12,
            true,
        ))
        .child(file_change_row(
            "glass-change-widgets",
            "widgets.rs",
            "ui_kit/src",
            12,
            3,
            false,
        ))
        .child(file_change_row(
            "glass-change-dock",
            "left_dock.rs",
            "va-desktop/src",
            4,
            2,
            false,
        ))
        .child(file_change_row(
            "glass-change-assistant",
            "assistant_panel.rs",
            "va-desktop/src",
            4,
            1,
            false,
        ))
        .child(file_change_row(
            "glass-change-editor",
            "editor_pane.rs",
            "va-desktop/src",
            2,
            1,
            false,
        ))
        .child(file_change_row(
            "glass-change-icon",
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
                        .text_size(px(11.5))
                        .text_color(ink_muted())
                        .child("Nothing applied yet"),
                )
                .child(div().flex_1())
                .child(link_button("glass-open-diff", "Open full diff")),
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
        .h(px(36.0))
        .px(px(8.0))
        .rounded(px(10.0))
        .border_1()
        .border_color(if active {
            brand_line()
        } else {
            rgba(0x00000000)
        })
        .cursor_pointer()
        .when(active, |this| this.bg(brand_tint()))
        .when(!active, |this| {
            this.hover(|style| style.bg(hover_wash()))
                .active(|style| style.bg(press_wash()))
        })
        .child(
            div()
                .size(px(26.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(7.0))
                .bg(well())
                .child(icon(
                    "file-code",
                    13.0,
                    if active { brand() } else { ink_muted() },
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
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(ink())
                        .child(name),
                )
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(ink_muted())
                        .truncate()
                        .child(dir),
                ),
        )
        .child(
            div()
                .font_family(theme::code_font())
                .text_size(px(11.0))
                .text_color(success())
                .child(format!("+{added}")),
        )
        .child(
            div()
                .font_family(theme::code_font())
                .text_size(px(11.0))
                .text_color(danger())
                .child(format!("−{removed}")),
        )
}

fn checks_card(show_warnings: bool, cx: &mut Context<GlassPreview>) -> Div {
    panel_card()
        .flex_shrink_0()
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
        .child(check_warning_row(show_warnings, cx))
        .when(show_warnings, |this| this.child(warning_detail()))
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
                .bg(success_tint())
                .child(icon("check", 10.0, success())),
        )
        .child(
            div()
                .text_size(px(12.5))
                .text_color(ink_soft())
                .child(label),
        )
        .child(div().flex_1())
        .child(
            div()
                .font_family(theme::code_font())
                .text_size(px(11.0))
                .text_color(ink_muted())
                .child(meta),
        )
}

fn check_warning_row(show_warnings: bool, cx: &mut Context<GlassPreview>) -> Div {
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
                .bg(warning_tint())
                .child(icon("triangle-alert", 10.0, warning())),
        )
        .child(
            div()
                .text_size(px(12.5))
                .text_color(ink_soft())
                .child("3 minor warnings"),
        )
        .child(div().flex_1())
        .child(
            link_button(
                "glass-review-warnings",
                if show_warnings { "Hide" } else { "Review" },
            )
            .on_click(cx.listener(|this, _event, _window, cx| {
                this.toggle_warnings();
                cx.notify();
            }))
            .tooltip(hint(if show_warnings {
                "Hide the warnings"
            } else {
                "Show the three warnings"
            })),
        )
}

fn warning_detail() -> Div {
    div()
        .pl(px(24.0))
        .flex()
        .flex_col()
        .gap(px(3.0))
        .child(warning_line("unused import — theme.rs"))
        .child(warning_line("dead_code allow — widgets.rs"))
        .child(warning_line("deprecated field — icon.rs"))
}

fn warning_line(label: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(7.0))
        .child(
            div()
                .size(px(3.0))
                .flex_shrink_0()
                .rounded_full()
                .bg(warning()),
        )
        .child(
            div()
                .text_size(px(11.0))
                .line_height(px(16.0))
                .text_color(ink_soft())
                .child(label),
        )
}

// ---------------------------------------------------------------- sections

fn editor_card() -> Div {
    div()
        .flex_shrink_0()
        .rounded(px(14.0))
        .bg(card_fill())
        .border_1()
        .border_color(panel_border_soft())
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(34.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(px(2.0))
                .px(px(6.0))
                .border_b_1()
                .border_color(panel_border_soft())
                .child(editor_tab("theme.rs", true))
                .child(editor_tab("widgets.rs", false))
                .child(div().flex_1())
                .child(
                    div()
                        .pr(px(6.0))
                        .font_family(theme::code_font())
                        .text_size(px(10.0))
                        .text_color(ink_muted())
                        .child("Rust"),
                ),
        )
        .child(
            div()
                .p(px(10.0))
                .flex()
                .flex_col()
                .child(code_line("1", &[cm("//! Warm-black theme tokens.")]))
                .child(code_line("2", &[]))
                .child(code_line(
                    "3",
                    &[
                        kw("use "),
                        ty("gpui"),
                        p("::{"),
                        fx("rgb"),
                        p(", "),
                        ty("Rgba"),
                        p("};"),
                    ],
                ))
                .child(code_line("4", &[]))
                .child(code_line(
                    "5",
                    &[
                        kw("pub const "),
                        ty("CANVAS"),
                        p(": "),
                        ty("Rgba"),
                        p(" = "),
                        fx("rgb"),
                        p("("),
                        nu("0x050505"),
                        p(");"),
                    ],
                ))
                .child(code_line(
                    "6",
                    &[
                        kw("pub const "),
                        ty("PANEL"),
                        p(": "),
                        ty("Rgba"),
                        p(" = "),
                        fx("rgb"),
                        p("("),
                        nu("0x141416"),
                        p(");"),
                    ],
                ))
                .child(code_line(
                    "7",
                    &[
                        kw("pub const "),
                        ty("WELL"),
                        p(": "),
                        ty("Rgba"),
                        p(" = "),
                        fx("rgb"),
                        p("("),
                        nu("0x1A1A1C"),
                        p(");"),
                    ],
                ))
                .child(code_line(
                    "8",
                    &[
                        kw("pub const "),
                        ty("INK"),
                        p(": "),
                        ty("Rgba"),
                        p(" = "),
                        fx("rgb"),
                        p("("),
                        nu("0xF2F0EA"),
                        p(");"),
                    ],
                ))
                .child(code_line(
                    "9",
                    &[
                        kw("pub const "),
                        ty("BRAND"),
                        p(": "),
                        ty("Rgba"),
                        p(" = "),
                        fx("rgb"),
                        p("("),
                        nu("0xFDCC02"),
                        p(");"),
                    ],
                ))
                .child(code_line("10", &[]))
                .child(code_line(
                    "11",
                    &[
                        kw("pub fn "),
                        fx("ui_font"),
                        p("() -> "),
                        ty("&'static str"),
                        p(" {"),
                    ],
                ))
                .child(code_line("12", &[p("    "), p("\"Inter\"")]))
                .child(code_line("13", &[p("}")]))
                .child(code_line("14", &[]))
                .child(code_line(
                    "15",
                    &[
                        kw("pub fn "),
                        fx("code_font"),
                        p("() -> "),
                        ty("&'static str"),
                        p(" {"),
                    ],
                ))
                .child(code_line("16", &[p("    "), p("\"JetBrains Mono\"")]))
                .child(code_line("17", &[p("}")])),
        )
        .child(
            div()
                .h(px(28.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(px(6.0))
                .px(px(10.0))
                .border_t_1()
                .border_color(panel_border_soft())
                .child(icon("file-code", 11.0, ink_muted()))
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(ink_muted())
                        .child("128 lines · read-only preview"),
                ),
        )
}

fn editor_tab(label: &'static str, active: bool) -> Div {
    div()
        .h(px(26.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .gap(px(5.0))
        .rounded(px(6.0))
        .when(active, |this| this.bg(rgba(0xFFFFFF10)))
        .child(icon(
            "file-code",
            11.0,
            if active { brand() } else { ink_muted() },
        ))
        .child(
            div()
                .text_size(px(11.5))
                .text_color(if active { ink() } else { ink_muted() })
                .child(label),
        )
}

fn code_line(no: &'static str, spans: &[(&'static str, Rgba)]) -> Div {
    div()
        .flex()
        .items_center()
        .h(px(17.0))
        .child(
            div()
                .w(px(14.0))
                .flex_shrink_0()
                .mr(px(8.0))
                .font_family(theme::code_font())
                .text_size(px(9.5))
                .text_color(ink_muted())
                .child(no),
        )
        .child(
            div()
                .flex()
                .min_w_0()
                .children(spans.iter().map(|(text, color)| {
                    div()
                        .font_family(theme::code_font())
                        .text_size(px(10.5))
                        .text_color(*color)
                        .child(*text)
                })),
        )
}

fn history_card() -> Div {
    panel_card()
        .flex_shrink_0()
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .flex()
                .items_center()
                .pb(px(2.0))
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink())
                        .child("Recent runs"),
                )
                .child(div().flex_1())
                .child(soft_chip("24 this week")),
        )
        .child(chat_section_label("TODAY"))
        .child(history_row(
            "glass-hist-1",
            "Restyle the workbench",
            "Now · 6 files changed",
            true,
        ))
        .child(history_row(
            "glass-hist-2",
            "Glass dock, round two",
            "42m ago · 4 files changed",
            false,
        ))
        .child(history_row(
            "glass-hist-3",
            "Midnight palette pass",
            "2h ago · 9 files changed",
            false,
        ))
        .child(chat_section_label("YESTERDAY"))
        .child(history_row(
            "glass-hist-4",
            "Paper & Ink rollout",
            "21:40 · 12 files changed",
            false,
        ))
        .child(history_row(
            "glass-hist-5",
            "Studio direction",
            "15:15 · 2 files changed",
            false,
        ))
        .child(history_row(
            "glass-hist-6",
            "Obsidian first look",
            "11:02 · no changes",
            false,
        ))
        .child(
            div()
                .flex()
                .items_center()
                .pt(px(4.0))
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(ink_muted())
                        .child("6 of 24 runs"),
                )
                .child(div().flex_1())
                .child(link_button("glass-open-history", "Open history")),
        )
}

fn history_row(
    id: &'static str,
    title: &'static str,
    meta: &'static str,
    active: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(9.0))
        .h(px(40.0))
        .px(px(8.0))
        .rounded(px(10.0))
        .border_1()
        .border_color(if active {
            brand_line()
        } else {
            rgba(0x00000000)
        })
        .when(active, |this| this.bg(brand_tint()))
        .when(!active, |this| {
            this.hover(|style| style.bg(hover_wash()))
                .active(|style| style.bg(press_wash()))
        })
        .cursor_pointer()
        .child(
            div()
                .size(px(24.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(7.0))
                .bg(if active { brand_tint() } else { well() })
                .child(icon(
                    if active { "history" } else { "check" },
                    12.0,
                    if active { brand() } else { success() },
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
                        .truncate()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(ink())
                        .child(title),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(px(10.5))
                        .text_color(ink_muted())
                        .child(meta),
                ),
        )
        .child(icon("chevron-right", 12.0, ink_muted()))
}

// ---------------------------------------------------------------- live chat

fn live_chat_panel() -> Div {
    div()
        .w_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .p(px(12.0))
        .rounded(px(R_LG))
        .bg(panel_fill())
        .border_1()
        .border_color(rgba(0xFDCC0226))
        .shadow(panel_shadow())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(7.0))
                .child(live_dot())
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink())
                        .child("While it works"),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .font_family(theme::code_font())
                        .text_size(px(10.5))
                        .text_color(ink_muted())
                        .child("0:42"),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(icon("refresh-cw", 11.0, brand()))
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(ink_soft())
                        .child("Editing chrome.rs · 2 of 6 files"),
                ),
        )
        .child(
            div()
                .h(px(2.0))
                .w_full()
                .rounded_full()
                .bg(rgba(0xFFFFFF12))
                .child(div().h(px(2.0)).w(px(112.0)).rounded_full().bg(brand())),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div().w_full().flex().justify_end().child(
                        div()
                            .max_w(px(256.0))
                            .px(px(10.0))
                            .py(px(6.0))
                            .rounded(px(10.0))
                            .bg(rgb(0x1E1E20))
                            .border_1()
                            .border_color(panel_border())
                            .text_size(px(12.0))
                            .line_height(px(17.0))
                            .text_color(ink())
                            .child("Keep this panel with the bar."),
                    ),
                )
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap(px(6.0))
                        .child(
                            div()
                                .size(px(18.0))
                                .flex_shrink_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_full()
                                .bg(brand_tint())
                                .child(icon("sparkles", 9.0, brand())),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(12.0))
                                .line_height(px(17.0))
                                .text_color(ink_soft())
                                .child(
                                    "On it — hover the nav to switch panels; this chat rides \
                                     in Session. Click the icon or pin it to keep the panel \
                                     open.",
                                ),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .h(px(32.0))
                .pl(px(10.0))
                .pr(px(4.0))
                .rounded_full()
                .bg(well())
                .border_1()
                .border_color(panel_border())
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(11.5))
                        .text_color(ink_muted())
                        .child("Send a note while Vibe works…"),
                )
                .child(
                    div()
                        .id("glass-live-send")
                        .size(px(24.0))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .bg(brand())
                        .shadow(brand_glow())
                        .cursor_pointer()
                        .hover(|style| style.bg(brand_hover()))
                        .active(|style| style.bg(brand_press()))
                        .tooltip(hint("Send"))
                        .child(icon("arrow-up", 12.0, brand_ink())),
                ),
        )
}

fn live_dot() -> AnyElement {
    let dot = div().size(px(6.0)).rounded_full().bg(brand());
    let pulsing = if calm() {
        dot.into_any_element()
    } else {
        dot.with_animation(
            "glass-live-pulse",
            Animation::new(Duration::from_millis(1800))
                .repeat()
                .with_easing(pulsating_between(0.35, 1.0)),
            |el, t| el.opacity(t),
        )
        .into_any_element()
    };
    div()
        .size(px(16.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(brand_tint())
        .child(pulsing)
        .into_any_element()
}

// ------------------------------------------------------------------ overlay

fn status_bar(mode: ExecutionMode, pinned: bool) -> Div {
    div()
        .h(px(STATUS))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(16.0))
        .bg(rgba(0x040404D9))
        .border_t_1()
        .border_color(panel_border_soft())
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
                .bg(rgba(0xFFFFFF0C))
                .border_1()
                .border_color(panel_border_soft())
                .child(div().size(px(5.0)).rounded_full().bg(brand()))
                .child(div().text_color(ink_soft()).child(mode.label())),
        )
        .child(separator())
        .child(status_chip(
            "status-local",
            "monitor",
            "Local",
            false,
            "Running on this machine",
        ))
        .child(status_chip(
            "status-branch",
            "git-branch",
            "main",
            true,
            "Switch branch",
        ))
        .child(separator())
        .child(div().size(px(5.0)).rounded_full().bg(success()))
        .child(div().text_color(ink_soft()).child("Everything builds"))
        .child(separator())
        .child(div().child("3 minor warnings"))
        .child(div().flex_1())
        .child(div().child("6 files changed"))
        .child(separator())
        .child(icon(
            if pinned { "pin" } else { "panel-right" },
            11.0,
            ink_muted(),
        ))
        .child(div().child(if pinned {
            "Panel pinned"
        } else {
            "Panel auto-hides"
        }))
}

fn status_chip(
    id: &'static str,
    name: &'static str,
    label: &'static str,
    caret: bool,
    tip: &'static str,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(5.0))
        .h(px(20.0))
        .px(px(7.0))
        .rounded(px(6.0))
        .bg(rgba(0xFFFFFF0A))
        .border_1()
        .border_color(panel_border_soft())
        .cursor_pointer()
        .hover(|style| style.bg(hover_wash()).border_color(panel_border()))
        .active(|style| style.bg(press_wash()))
        .tooltip(hint(tip))
        .child(icon(name, 11.0, ink_muted()))
        .child(div().text_color(ink_soft()).child(label))
        .when(caret, |this| {
            this.child(icon("chevron-down", 9.0, ink_muted()))
        })
}

#[derive(Clone, Copy)]
struct EdgeState {
    open: bool,
    epoch: usize,
    pinned: bool,
    section: DrawerSection,
    section_epoch: usize,
    show_tokens: bool,
    show_warnings: bool,
    context_epoch: usize,
}

fn edge_overlay(
    state: EdgeState,
    focus: &FocusHandle,
    cx: &mut Context<GlassPreview>,
) -> impl IntoElement {
    let EdgeState { open, epoch, .. } = state;
    div()
        .id("glass-edge-zone")
        .absolute()
        .top_0()
        .bottom(px(8.0))
        .right(px(14.0))
        .on_hover(cx.listener(|this, hovered, _window, cx| {
            if *hovered {
                // The nav is always on screen; only its icons open a section.
                this.nav_hovered = true;
            } else {
                this.nav_hovered = false;
                if this.nav_open && !this.nav_pinned {
                    this.nav_open = false;
                    this.nav_epoch += 1;
                    cx.notify();
                }
            }
        }))
        .child(edge_drawer(state, focus, cx))
        .child(nav_column(state, focus, cx))
        .with_animation(
            ("glass-edge", epoch),
            Animation::new(motion(320)).with_easing(ease_out_quint()),
            move |el, t| {
                let k = edge_reveal(open, epoch, t);
                el.w(px(NAV_W + (DRAWER_OUTER - NAV_W) * k))
            },
        )
}

fn edge_reveal(open: bool, epoch: usize, t: f32) -> f32 {
    if epoch == 0 {
        0.0
    } else if open {
        t
    } else {
        1.0 - t
    }
}

fn edge_drawer(
    state: EdgeState,
    focus: &FocusHandle,
    cx: &mut Context<GlassPreview>,
) -> impl IntoElement {
    let EdgeState {
        open,
        epoch,
        pinned,
        section,
        section_epoch,
        show_tokens,
        ..
    } = state;
    div()
        .absolute()
        .top_0()
        .bottom_0()
        .right_0()
        .overflow_hidden()
        .rounded(px(R_LG))
        .bg(drawer_fill())
        .border_1()
        .border_color(if pinned {
            brand_line()
        } else {
            rgba(0xFFFFFF2E)
        })
        .shadow(glass_shadow())
        .child(
            // Content keeps its width while the panel's left edge sweeps out,
            // so labels never reflow mid-animation.
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .right_0()
                .w(px(DRAWER_OUTER))
                .flex()
                .flex_col()
                .gap(px(10.0))
                .p(px(12.0))
                .pr(px(DRAWER_PAD_R))
                .child(drawer_header(section, pinned, show_tokens, focus, cx))
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .relative()
                        .overflow_hidden()
                        .child(
                            // One scroll view per section, so switching tabs
                            // starts at the top instead of inheriting an offset.
                            div()
                                .id(section.body_id())
                                .size_full()
                                .overflow_y_scroll()
                                .child(
                                    div()
                                        .pt(px(12.0))
                                        .flex()
                                        .flex_col()
                                        .gap(px(10.0))
                                        .child(section_body(state, cx))
                                        .with_animation(
                                            ("glass-section", section_epoch),
                                            Animation::new(motion(170))
                                                .with_easing(ease_out_quint()),
                                            move |el, t| {
                                                el.opacity(if section_epoch == 0 {
                                                    1.0
                                                } else {
                                                    0.35 + 0.65 * t
                                                })
                                            },
                                        ),
                                ),
                        )
                        // The inset scrolls away, so this fade only lands on
                        // cards once the list is scrolled upward.
                        .child(div().absolute().top_0().left_0().right_0().h(px(12.0)).bg(
                            linear_gradient(
                                180.0,
                                linear_color_stop(rgba(0x17171AEB), 0.0),
                                linear_color_stop(rgba(0x17171A00), 1.0),
                            ),
                        ))
                        // Cards fade out here instead of ending on a hard cut
                        // when the session list is taller than the drawer.
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .right_0()
                                .bottom_0()
                                .h(px(22.0))
                                .bg(linear_gradient(
                                    180.0,
                                    linear_color_stop(rgba(0x0E0E1000), 0.0),
                                    linear_color_stop(rgba(0x0E0E10F5), 1.0),
                                )),
                        ),
                )
                .when(section == DrawerSection::Session, |this| {
                    this.child(live_chat_panel())
                }),
        )
        .with_animation(
            ("glass-drawer", epoch),
            Animation::new(motion(320)).with_easing(ease_out_quint()),
            move |el, t| {
                let k = edge_reveal(open, epoch, t);
                el.w(px(DRAWER_OUTER * k)).opacity(k)
            },
        )
}

fn section_body(state: EdgeState, cx: &mut Context<GlassPreview>) -> Div {
    match state.section {
        DrawerSection::Session => div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(changed_files_card())
            .child(checks_card(state.show_warnings, cx)),
        DrawerSection::Changes => div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(diff_preview("theme.rs", "va-desktop/src", 42, 12))
            .child(changed_files_card()),
        DrawerSection::Editor => div().flex().flex_col().gap(px(10.0)).child(editor_card()),
        DrawerSection::History => div().flex().flex_col().gap(px(10.0)).child(history_card()),
    }
}

fn nav_column(state: EdgeState, focus: &FocusHandle, cx: &mut Context<GlassPreview>) -> Div {
    let EdgeState {
        section,
        context_epoch,
        ..
    } = state;
    div()
        .absolute()
        .top_0()
        .bottom_0()
        .right(px(4.0))
        .w(px(NAV_W))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(14.0))
        .child(nav_capsule(section, focus, cx))
        .child(context_indicator(context_epoch, cx))
}

fn context_indicator(epoch: usize, cx: &mut Context<GlassPreview>) -> Stateful<Div> {
    div()
        .id("glass-context")
        .w(px(52.0))
        .h(px(CONTEXT_CAPSULE_H))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(nav_fill())
        .border_1()
        .border_color(rgba(0xFFFFFF4D))
        .shadow(glass_shadow())
        .cursor_pointer()
        .tooltip(hint("62% of context used — 75k of 120k tokens"))
        .on_hover(cx.listener(|this, hovered: &bool, _window, cx| {
            if !*hovered {
                return;
            }
            this.context_epoch += 1;
            cx.notify();
        }))
        .child(context_ring(epoch))
}

fn nav_capsule(
    section: DrawerSection,
    focus: &FocusHandle,
    cx: &mut Context<GlassPreview>,
) -> Stateful<Div> {
    div()
        .id("glass-nav")
        .w(px(52.0))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(2.0))
        .p(px(6.0))
        .relative()
        .overflow_hidden()
        .rounded_full()
        .bg(nav_fill())
        .border_1()
        .border_color(rgba(0xFFFFFF4D))
        .shadow(glass_shadow())
        .cursor_pointer()
        .track_focus(focus)
        .focus(focus_ring_neutral)
        .on_click(cx.listener(|this, _event, window, cx| {
            window.focus(&this.nav_focus);
            let section = this.section;
            this.open_section(section);
            cx.notify();
        }))
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
            if event.is_held {
                return;
            }
            match event.keystroke.key.as_str() {
                "escape" => this.close_nav(),
                "enter" | "space" => {
                    let section = this.section;
                    this.open_section(section);
                    this.set_pinned(true);
                    window.focus(&this.nav_focus);
                }
                "up" | "left" => {
                    let next = this.section.cycle(false);
                    this.open_section(next);
                }
                "down" | "right" => {
                    let next = this.section.cycle(true);
                    this.open_section(next);
                }
                _ => return,
            }
            cx.notify();
        }))
        .child(nav_icon(
            DrawerSection::Session,
            section == DrawerSection::Session,
            cx,
        ))
        .child(nav_icon(
            DrawerSection::Changes,
            section == DrawerSection::Changes,
            cx,
        ))
        .child(nav_icon(
            DrawerSection::Editor,
            section == DrawerSection::Editor,
            cx,
        ))
        .child(nav_icon(
            DrawerSection::History,
            section == DrawerSection::History,
            cx,
        ))
}

fn nav_icon(section: DrawerSection, active: bool, cx: &mut Context<GlassPreview>) -> AnyElement {
    let id = match section {
        DrawerSection::Session => "edge-chat",
        DrawerSection::Changes => "edge-files",
        DrawerSection::Editor => "edge-code",
        DrawerSection::History => "edge-history",
    };
    div()
        .id(id)
        .size(px(NAV_ITEM))
        .flex_shrink_0()
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .cursor_pointer()
        .tooltip(hint(section.tooltip()))
        .when(active, |this| {
            this.child(active_halo(id))
                .bg(rgba(0xFDCC0233))
                .border_1()
                .border_color(brand_line())
        })
        .when(!active, |this| this.hover(|style| style.bg(hover_wash())))
        .active(|style| style.bg(press_wash()))
        .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
            if !*hovered {
                return;
            }
            this.open_section(section);
            cx.notify();
        }))
        .on_click(cx.listener(move |this, _event, window, cx| {
            cx.stop_propagation();
            this.open_section(section);
            this.set_pinned(true);
            window.focus(&this.nav_focus);
            cx.notify();
        }))
        .child(icon(
            section.icon(),
            17.0,
            if active { brand() } else { ink_soft() },
        ))
        .into_any_element()
}

fn context_ring(epoch: usize) -> impl IntoElement {
    div()
        .size(px(CONTEXT_RING))
        .flex_shrink_0()
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_size(px(10.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(brand())
                .child("62%"),
        )
        .with_animation(
            ("glass-context", epoch),
            Animation::new(motion(520)).with_easing(ease_out_quint()),
            move |el, t| {
                // Refill on every hover; usage itself never drains.
                let k = if epoch == 0 { 1.0 } else { t };
                let fill = 0.62 * k;
                el.children((0..CONTEXT_TICKS).map(move |i| context_tick(i, fill)))
            },
        )
}

fn context_tick(i: usize, fill: f32) -> Div {
    let span = i as f32 / CONTEXT_TICKS as f32;
    let angle = -FRAC_PI_2 + span * TAU;
    let center = CONTEXT_RING / 2.0;
    let radius = center - 6.0;
    let dot = 2.0;
    let x = center + radius * angle.cos() - dot / 2.0;
    let y = center + radius * angle.sin() - dot / 2.0;
    let lit = ((fill - span) / 0.03).clamp(0.0, 1.0);
    let tick = div()
        .absolute()
        .left(px(x))
        .top(px(y))
        .size(px(dot))
        .rounded_full()
        .bg(rgba(0xFFFFFF30));
    if lit <= 0.0 {
        tick
    } else {
        tick.child(div().size_full().rounded_full().bg(brand()).opacity(lit))
    }
}

fn active_halo(id: &'static str) -> AnyElement {
    let halo = div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .rounded_full()
        .bg(rgba(0xFDCC022E));
    if calm() {
        halo.opacity(0.5).into_any_element()
    } else {
        halo.with_animation(
            (id, 3usize),
            Animation::new(Duration::from_millis(2600))
                .repeat()
                .with_easing(pulsating_between(0.18, 0.85)),
            |el, t| el.opacity(t),
        )
        .into_any_element()
    }
}

// ------------------------------------------------------------------- pieces

fn bare_icon(id: &'static str, name: &'static str, label: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .size(px(30.0))
        .rounded(px(R_SM))
        .cursor_pointer()
        .tooltip(hint(label))
        .hover(|style| style.bg(hover_wash()))
        .active(|style| style.bg(press_wash()))
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
                style.bg(danger_tint())
            } else {
                style.bg(hover_wash())
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
        .bg(rgba(0xFFFFFF12))
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
        .bg(rgba(0xFFFFFF0E))
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
        .hover(|style| style.bg(hover_wash()))
        .active(|style| style.bg(press_wash()))
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(brand())
                .child(label),
        )
        .child(icon("chevron-right", 11.0, brand()))
}

fn separator() -> Div {
    div()
        .w(px(1.0))
        .h(px(12.0))
        .flex_shrink_0()
        .bg(rgba(0xFFFFFF1F))
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
        .shadow(brand_glow())
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(brand_ink())
        .cursor_pointer()
        .hover(|style| style.bg(brand_hover()))
        .active(|style| style.bg(brand_press()))
        .child(icon("check", 12.0, brand_ink()))
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
        .bg(rgb(0x1B1B1D))
        .border_1()
        .border_color(panel_border())
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(ink_soft())
        .cursor_pointer()
        .hover(|style| style.bg(hover_wash()))
        .child(label)
}
