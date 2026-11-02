//! Liquid Glass — black and yellow, with the glass kept for the ask bar and
//! the auto-hiding edge drawer.
//!
//! Solid graphite panes sit on a near-black canvas lit by one warm yellow
//! glow; glass is reserved for the composer, the model pill, the session
//! drawer, and the nav rail. GPUI 0.2.2 has no in-app backdrop blur, so frost
//! is approximated with translucent fills, specular edges, and long shadows.

use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, TAU};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    canvas, deferred, div, ease_out_quint, linear_color_stop, linear_gradient, point,
    pulsating_between, px, rgb, rgba, Animation, AnimationExt, AnyElement, AnyView, App,
    Background, BoxShadow, ClipboardItem, Context, Div, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, FocusHandle, FontWeight, HighlightStyle, IntoElement, KeyDownEvent,
    Keystroke, PathBuilder, PathPromptOptions, Point, Rgba, ScrollHandle, Stateful,
    StyleRefinement, StyledText, UTF16Selection, UnderlineStyle, Window, WindowControlArea,
};
use va_core::modes::ExecutionMode;

use crate::branding;
use crate::design::icon::icon;
use crate::spell;
use crate::theme;
use crate::ui_kit::InputSelection;

// ------------------------------------------------------------------ palette

// One window runs one sample, and the palette is read during render, so a
// theme flip only needs a notify. Night is the shipped default.
static DAY_MODE: AtomicBool = AtomicBool::new(false);

fn day() -> bool {
    DAY_MODE.load(Ordering::Relaxed)
}

fn veil(base: u32, alpha: u8) -> Rgba {
    rgba((base << 8) | alpha as u32)
}

/// Hairlines and washes: white on night, warm ink on paper.
fn tint(alpha: u8) -> Rgba {
    if day() {
        veil(0x2B2200, alpha)
    } else {
        veil(0xFFFFFF, alpha)
    }
}

fn ink() -> Rgba {
    if day() {
        rgb(0x241F17)
    } else {
        rgb(0xF4F1E8)
    }
}
fn ink_soft() -> Rgba {
    if day() {
        rgb(0x524B3E)
    } else {
        rgb(0xC8C3B6)
    }
}
fn ink_muted() -> Rgba {
    if day() {
        rgb(0x6F6757)
    } else {
        rgb(0x9A9588)
    }
}
fn brand() -> Rgba {
    rgb(0xFDCC02)
}
// Brand yellow is unreadable on paper; links and small labels deepen to gold.
fn brand_text() -> Rgba {
    if day() {
        rgb(0x7A6200)
    } else {
        brand()
    }
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
    rgba(0xFDCC0224)
}
fn brand_line() -> Rgba {
    rgba(0xFDCC0259)
}
fn success() -> Rgba {
    if day() {
        rgb(0x17713C)
    } else {
        rgb(0x4ADE80)
    }
}
fn success_tint() -> Rgba {
    if day() {
        rgba(0x21854A24)
    } else {
        rgba(0x4ADE8024)
    }
}
fn success_line() -> Rgba {
    if day() {
        rgba(0x17713C59)
    } else {
        rgba(0x4ADE8059)
    }
}
fn warning() -> Rgba {
    if day() {
        rgb(0x8A5E00)
    } else {
        rgb(0xF5B301)
    }
}
fn warning_tint() -> Rgba {
    if day() {
        rgba(0xB07A0026)
    } else {
        rgba(0xF5B30126)
    }
}
fn danger() -> Rgba {
    if day() {
        rgb(0xB22D45)
    } else {
        rgb(0xFB7185)
    }
}
fn danger_tint() -> Rgba {
    if day() {
        rgba(0xC23A5226)
    } else {
        rgba(0xFB718526)
    }
}
fn hover_wash() -> Rgba {
    tint(if day() { 0x12 } else { 0x10 })
}
fn press_wash() -> Rgba {
    tint(0x1F)
}

// Panels are solid; glass is a deliberate accent, not the default surface.
fn panel_fill() -> Background {
    if day() {
        linear_gradient(
            180.0,
            linear_color_stop(rgb(0xFCFAF5), 0.0),
            linear_color_stop(rgb(0xF2EDE3), 1.0),
        )
    } else {
        linear_gradient(
            180.0,
            linear_color_stop(rgb(0x141416), 0.0),
            linear_color_stop(rgb(0x0C0C0E), 1.0),
        )
    }
}
fn panel_border() -> Rgba {
    tint(0x17)
}
fn panel_border_soft() -> Rgba {
    tint(0x0D)
}
fn card_fill() -> Rgba {
    if day() {
        rgba(0xFFFFFFC0)
    } else {
        rgba(0xFFFFFF08)
    }
}
fn well() -> Rgba {
    if day() {
        rgb(0xEDE7DB)
    } else {
        rgb(0x08080A)
    }
}

fn glass_fill() -> Background {
    if day() {
        linear_gradient(
            180.0,
            linear_color_stop(rgba(0xFFFFFFB8), 0.0),
            linear_color_stop(rgba(0xFFFFFF80), 1.0),
        )
    } else {
        linear_gradient(
            180.0,
            linear_color_stop(rgba(0xFFFFFF18), 0.0),
            linear_color_stop(rgba(0xFFFFFF08), 1.0),
        )
    }
}

fn glass_fill_strong() -> Background {
    if day() {
        linear_gradient(
            180.0,
            linear_color_stop(rgba(0xFFFFFFE6), 0.0),
            linear_color_stop(rgba(0xFFFFFFC8), 1.0),
        )
    } else {
        linear_gradient(
            180.0,
            linear_color_stop(rgba(0xFFFFFF26), 0.0),
            linear_color_stop(rgba(0xFFFFFF0F), 1.0),
        )
    }
}

fn nav_fill() -> Background {
    // The nav floats over open drawer cards, so it frosts them almost solid —
    // no diff numbers bleeding through the capsule.
    if day() {
        linear_gradient(
            180.0,
            linear_color_stop(rgba(0xFFFFFFF7), 0.0),
            linear_color_stop(rgba(0xF6F1E8F0), 1.0),
        )
    } else {
        linear_gradient(
            180.0,
            linear_color_stop(rgba(0x2C2C30F2), 0.0),
            linear_color_stop(rgba(0x17171AF7), 1.0),
        )
    }
}
fn glass_border() -> Rgba {
    if day() {
        tint(0x24)
    } else {
        rgba(0xFFFFFF3D)
    }
}
fn nav_border() -> Rgba {
    if day() {
        tint(0x26)
    } else {
        rgba(0xFFFFFF4D)
    }
}

// The drawer floats over chat content, and GPUI 0.2.2 cannot blur what is
// behind it — so it stays effectively opaque, glass only at its edges.
fn drawer_fill() -> Background {
    if day() {
        linear_gradient(
            180.0,
            linear_color_stop(rgb(0xFDFAF4), 0.0),
            linear_color_stop(rgb(0xF2EDE3), 1.0),
        )
    } else {
        linear_gradient(
            180.0,
            linear_color_stop(rgb(0x17171A), 0.0),
            linear_color_stop(rgb(0x0E0E10), 1.0),
        )
    }
}

// Same constraint for the search sheet: mostly frosted, never see-through
// enough to read the chat beneath it.
fn sheet_fill() -> Background {
    if day() {
        linear_gradient(
            180.0,
            linear_color_stop(rgba(0xFDFBF6F7), 0.0),
            linear_color_stop(rgba(0xF0EBE1FA), 1.0),
        )
    } else {
        linear_gradient(
            180.0,
            linear_color_stop(rgba(0x2A2A2EF0), 0.0),
            linear_color_stop(rgba(0x131316FA), 1.0),
        )
    }
}

fn panel_top_fade(alpha: u8) -> Rgba {
    veil(if day() { 0xFCFAF5 } else { 0x141416 }, alpha)
}
fn drawer_top_fade(alpha: u8) -> Rgba {
    veil(if day() { 0xFDFAF4 } else { 0x17171A }, alpha)
}
fn drawer_bottom_fade(alpha: u8) -> Rgba {
    veil(if day() { 0xF2EDE3 } else { 0x0E0E10 }, alpha)
}
fn sheet_scrim() -> Rgba {
    if day() {
        rgba(0x3A332680)
    } else {
        rgba(0x050505A6)
    }
}
fn raised() -> Rgba {
    if day() {
        rgb(0xFFFFFF)
    } else {
        rgb(0x1B1B1D)
    }
}
fn work_surface() -> Rgba {
    if day() {
        rgb(0xFFFDF8)
    } else {
        rgb(0x18181A)
    }
}
fn bubble() -> Rgba {
    if day() {
        rgb(0xFFFFFF)
    } else {
        rgb(0x1E1E20)
    }
}
fn status_fill() -> Rgba {
    if day() {
        rgba(0xF4EFE6E6)
    } else {
        rgba(0x040404D9)
    }
}

// ------------------------------------------------------------------- syntax

fn syn_keyword() -> Rgba {
    if day() {
        rgb(0x8A6414)
    } else {
        rgb(0xD8CBAF)
    }
}
fn syn_call() -> Rgba {
    if day() {
        rgb(0x2F6D86)
    } else {
        rgb(0xAFC4CC)
    }
}
fn syn_type() -> Rgba {
    if day() {
        rgb(0x3F7A55)
    } else {
        rgb(0xABC2AC)
    }
}
fn syn_number() -> Rgba {
    if day() {
        rgb(0x9A6414)
    } else {
        rgb(0xD8B67C)
    }
}
fn syn_comment() -> Rgba {
    if day() {
        rgb(0x8C8577)
    } else {
        rgb(0x7C786E)
    }
}
fn punct() -> Rgba {
    if day() {
        rgb(0x6E685C)
    } else {
        rgb(0xA5A096)
    }
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
// The drawer floats 12px off the window edge and its cards run the full
// width; the nav rides on top of them with its own margin from the edge.
const EDGE_INSET: f32 = 12.0;
const NAV_INSET: f32 = 12.0;
const DRAWER_W: f32 = 408.0;
// Card surfaces stay full-bleed under the floating nav, but their text stops
// this far from the card's right edge so stats and row links are never hidden
// (or click-blocked) by the 52px nav capsule.
const DRAWER_CARD_RIGHT: f32 = 62.0;
const CHAT_ROW: f32 = 44.0;
const CONTEXT_SIZE: f32 = 52.0;
const CONTEXT_RING: f32 = 42.0;

fn soft_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: if day() {
            rgba(0x3A332614)
        } else {
            rgba(0x00000059)
        }
        .into(),
        offset: point(px(0.0), px(8.0)),
        blur_radius: px(22.0),
        spread_radius: px(0.0),
    }]
}

fn panel_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: if day() {
            rgba(0x3A332621)
        } else {
            rgba(0x00000099)
        }
        .into(),
        offset: point(px(0.0), px(18.0)),
        blur_radius: px(44.0),
        spread_radius: px(0.0),
    }]
}

fn glass_shadow() -> Vec<BoxShadow> {
    let (deep, close) = if day() {
        (rgba(0x3A33261F), rgba(0x3A33260F))
    } else {
        (rgba(0x000000B3), rgba(0x0000004D))
    };
    vec![
        BoxShadow {
            color: deep.into(),
            offset: point(px(0.0), px(12.0)),
            blur_radius: px(30.0),
            spread_radius: px(0.0),
        },
        BoxShadow {
            color: close.into(),
            offset: point(px(0.0), px(2.0)),
            blur_radius: px(8.0),
            spread_radius: px(0.0),
        },
    ]
}

fn lifted_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: if day() {
            rgba(0x3A33261F)
        } else {
            rgba(0x000000A6)
        }
        .into(),
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
        color: if day() {
            rgba(0x241F1766)
        } else {
            rgba(0xF4F1E899)
        }
        .into(),
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
            .bg(raised())
            .border_1()
            .border_color(tint(0x24))
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

// ------------------------------------------------------------- preferences

// A bare launch opens Liquid Glass, so the day/night choice outlives the
// window. One small key=value file keeps it honest without new dependencies.
fn preferences_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|base| base.join("Vibe Assembly").join("preferences.conf"))
}

fn parse_theme(raw: &str) -> Option<bool> {
    raw.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        if key.trim() != "theme" {
            return None;
        }
        match value.trim() {
            "day" => Some(true),
            "night" => Some(false),
            _ => None,
        }
    })
}

fn format_theme(day: bool) -> &'static str {
    if day {
        "theme=day\n"
    } else {
        "theme=night\n"
    }
}

fn stored_theme() -> Option<bool> {
    std::fs::read_to_string(preferences_path()?)
        .ok()
        .and_then(|raw| parse_theme(&raw))
}

fn store_theme(day: bool) {
    let Some(path) = preferences_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, format_theme(day));
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
            Self::Session => "Session — Pin keeps it open",
            Self::Changes => "Changes — Pin keeps it open",
            Self::Editor => "Editor — Pin keeps it open",
            Self::History => "History — Pin keeps it open",
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorFile {
    Theme,
    Widgets,
}

impl EditorFile {
    fn title(self) -> &'static str {
        match self {
            Self::Theme => "theme.rs",
            Self::Widgets => "widgets.rs",
        }
    }

    fn outline(self) -> &'static [(&'static str, &'static str, &'static str)] {
        match self {
            Self::Theme => &[
                ("const", "CANVAS", "5"),
                ("const", "PANEL", "6"),
                ("const", "WELL", "7"),
                ("const", "INK", "8"),
                ("const", "BRAND", "9"),
                ("fn", "ui_font()", "11"),
                ("fn", "code_font()", "15"),
            ],
            Self::Widgets => &[
                ("const", "BUTTON_H", "5"),
                ("const", "BUTTON_R", "6"),
                ("fn", "primary_fill()", "8"),
                ("fn", "primary_ink()", "12"),
                ("fn", "quiet_fill()", "16"),
            ],
        }
    }
}

struct ChangeFile {
    name: &'static str,
    dir: &'static str,
    added: u32,
    removed: u32,
}

const CHANGE_FILES: &[ChangeFile] = &[
    ChangeFile {
        name: "theme.rs",
        dir: "va-desktop/src",
        added: 42,
        removed: 12,
    },
    ChangeFile {
        name: "widgets.rs",
        dir: "ui_kit/src",
        added: 12,
        removed: 3,
    },
    ChangeFile {
        name: "left_dock.rs",
        dir: "va-desktop/src",
        added: 4,
        removed: 2,
    },
    ChangeFile {
        name: "assistant_panel.rs",
        dir: "va-desktop/src",
        added: 4,
        removed: 1,
    },
    ChangeFile {
        name: "editor_pane.rs",
        dir: "va-desktop/src",
        added: 2,
        removed: 1,
    },
    ChangeFile {
        name: "icon.rs",
        dir: "va-desktop/src",
        added: 2,
        removed: 1,
    },
    ChangeFile {
        name: "composer.rs",
        dir: "va-desktop/src",
        added: 18,
        removed: 6,
    },
    ChangeFile {
        name: "search.rs",
        dir: "va-desktop/src",
        added: 9,
        removed: 4,
    },
    ChangeFile {
        name: "history.rs",
        dir: "va-desktop/src",
        added: 7,
        removed: 2,
    },
    ChangeFile {
        name: "nav.rs",
        dir: "va-desktop/src",
        added: 6,
        removed: 3,
    },
    ChangeFile {
        name: "tokens.rs",
        dir: "ui_kit/src",
        added: 11,
        removed: 5,
    },
    ChangeFile {
        name: "motion.rs",
        dir: "ui_kit/src",
        added: 5,
        removed: 2,
    },
];

// Each chat owns the files it produced; the drawer shows that chat's slice
// instead of the workbench run's list.
fn changes_for(count: usize) -> &'static [ChangeFile] {
    &CHANGE_FILES[..count.min(CHANGE_FILES.len())]
}

fn change_totals(files: &[ChangeFile]) -> (u32, u32) {
    files.iter().fold((0, 0), |(added, removed), file| {
        (added + file.added, removed + file.removed)
    })
}

fn section_chip(section: DrawerSection, changes: &[ChangeFile]) -> String {
    match section {
        DrawerSection::Session => match changes.len() {
            0 => "No files".to_string(),
            1 => "1 file".to_string(),
            count => format!("{count} files"),
        },
        DrawerSection::Changes => {
            if changes.is_empty() {
                "No diff".to_string()
            } else {
                let (added, removed) = change_totals(changes);
                format!("+{added} −{removed}")
            }
        }
        DrawerSection::Editor => "Rust".to_string(),
        DrawerSection::History => "24 runs".to_string(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SearchAction {
    Chat,
    File(EditorFile),
    Change(&'static str),
    Section(DrawerSection),
    Theme,
    Tokens,
    Warnings,
    Pin,
}

impl SearchAction {
    fn rank(self) -> u8 {
        match self {
            Self::Chat => 0,
            Self::File(_) | Self::Change(_) => 1,
            Self::Section(_) => 2,
            Self::Theme | Self::Tokens | Self::Warnings | Self::Pin => 3,
        }
    }
}

struct SearchEntry {
    id: &'static str,
    icon: &'static str,
    title: &'static str,
    meta: &'static str,
    action: SearchAction,
    recent: bool,
}

const SEARCH_ENTRIES: &[SearchEntry] = &[
    SearchEntry {
        id: "glass-search-chat",
        icon: "message-circle",
        title: "Restyle the workbench",
        meta: "Chat · this session",
        action: SearchAction::Chat,
        recent: true,
    },
    SearchEntry {
        id: "glass-search-theme-file",
        icon: "file-code",
        title: "theme.rs",
        meta: "File · va-desktop/src",
        action: SearchAction::File(EditorFile::Theme),
        recent: true,
    },
    SearchEntry {
        id: "glass-search-widgets-file",
        icon: "file-code",
        title: "widgets.rs",
        meta: "File · ui_kit/src",
        action: SearchAction::File(EditorFile::Widgets),
        recent: false,
    },
    SearchEntry {
        id: "glass-search-dock-file",
        icon: "files",
        title: "left_dock.rs",
        meta: "Change · va-desktop/src",
        action: SearchAction::Change("left_dock.rs"),
        recent: true,
    },
    SearchEntry {
        id: "glass-search-assistant-file",
        icon: "files",
        title: "assistant_panel.rs",
        meta: "Change · va-desktop/src",
        action: SearchAction::Change("assistant_panel.rs"),
        recent: false,
    },
    SearchEntry {
        id: "glass-search-editor-file",
        icon: "files",
        title: "editor_pane.rs",
        meta: "Change · va-desktop/src",
        action: SearchAction::Change("editor_pane.rs"),
        recent: false,
    },
    SearchEntry {
        id: "glass-search-icon-file",
        icon: "files",
        title: "icon.rs",
        meta: "Change · va-desktop/src",
        action: SearchAction::Change("icon.rs"),
        recent: false,
    },
    SearchEntry {
        id: "glass-search-session-panel",
        icon: "message-circle",
        title: "Open Session",
        meta: "Panel · chat, files and checks",
        action: SearchAction::Section(DrawerSection::Session),
        recent: false,
    },
    SearchEntry {
        id: "glass-search-changes-panel",
        icon: "files",
        title: "Open Changes",
        meta: "Panel · diff and file list",
        action: SearchAction::Section(DrawerSection::Changes),
        recent: true,
    },
    SearchEntry {
        id: "glass-search-editor-panel",
        icon: "code",
        title: "Open Editor",
        meta: "Panel · read-only file preview",
        action: SearchAction::Section(DrawerSection::Editor),
        recent: false,
    },
    SearchEntry {
        id: "glass-search-history-panel",
        icon: "history",
        title: "Open History",
        meta: "Panel · recent runs",
        action: SearchAction::Section(DrawerSection::History),
        recent: false,
    },
    SearchEntry {
        id: "glass-search-tokens",
        icon: "sparkles",
        title: "Show or hide token counts",
        meta: "Command · Composer",
        action: SearchAction::Tokens,
        recent: true,
    },
    SearchEntry {
        id: "glass-search-warnings",
        icon: "triangle-alert",
        title: "Explain warnings",
        meta: "Command · Checks",
        action: SearchAction::Warnings,
        recent: false,
    },
    SearchEntry {
        id: "glass-search-pin",
        icon: "pin",
        title: "Pin or unpin the panel",
        meta: "Command · Layout",
        action: SearchAction::Pin,
        recent: false,
    },
    SearchEntry {
        id: "glass-search-theme",
        icon: "sun",
        title: "Switch day / night theme",
        meta: "Command · Appearance",
        action: SearchAction::Theme,
        recent: true,
    },
];

fn filter_entries(entries: &'static [SearchEntry], query: &str) -> Vec<&'static SearchEntry> {
    let tokens: Vec<String> = query
        .split_whitespace()
        .map(|token| token.to_lowercase())
        .collect();
    if tokens.is_empty() {
        return entries.iter().filter(|entry| entry.recent).collect();
    }
    let mut hits: Vec<_> = entries
        .iter()
        .filter(|entry| {
            let haystack = format!("{} {}", entry.title, entry.meta).to_lowercase();
            // Words may be separated by punctuation in the label ("day / night"),
            // so every token must appear rather than one contiguous substring.
            tokens.iter().all(|token| haystack.contains(token))
        })
        .collect();
    // Grouped display: this session, then files, panels, commands.
    hits.sort_by_key(|entry| entry.action.rank());
    hits
}

/// Diff rows only exist for files the active chat produced; offering them
/// elsewhere would open an empty Changes panel.
fn search_results(
    entries: &'static [SearchEntry],
    query: &str,
    files: usize,
) -> Vec<&'static SearchEntry> {
    let changes = changes_for(files);
    filter_entries(entries, query)
        .into_iter()
        .filter(|entry| match entry.action {
            SearchAction::Change(name) => changes.iter().any(|file| file.name == name),
            _ => true,
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChatGroup {
    Today,
    Yesterday,
}

#[derive(Clone, Copy)]
enum ChatKind {
    /// The live demo conversation with the work card and its diff.
    Workbench,
    /// Earlier sessions: a prompt and the reply that closed it.
    Archive {
        prompt: &'static str,
        reply: &'static str,
    },
    /// A fresh chat, empty until the first message is sent.
    New,
}

#[derive(Clone)]
struct ChatThread {
    id: u64,
    title: String,
    meta: String,
    files: usize,
    group: ChatGroup,
    kind: ChatKind,
}

fn seeded_chats() -> Vec<ChatThread> {
    let seeds = [
        (
            "Restyle the workbench",
            "now · 6 files",
            6,
            ChatGroup::Today,
            ChatKind::Workbench,
        ),
        (
            "Glass dock, round two",
            "42m · 4 files",
            4,
            ChatGroup::Today,
            ChatKind::Archive {
                prompt: "Round two on the dock: it should float, not dock. Give the nav a margin \
                         from the edge and keep the panel on top.",
                reply: "Widened the right gutter to 12, floated the bar over the drawer, and \
                        rebuilt the dock as a capsule. Opening a panel pushes the thread instead \
                        of covering it.",
            },
        ),
        (
            "Midnight palette pass",
            "2h · 9 files",
            9,
            ChatGroup::Today,
            ChatKind::Archive {
                prompt: "Take the whole surface to midnight. Keep the yellow only on chrome and \
                         primary actions.",
                reply: "Rebuilt the palette around near-black glass with one warm yellow light. \
                        Body text sits at 7:1, muted labels at 4.8:1, and no syntax color moved.",
            },
        ),
        (
            "Paper & Ink rollout",
            "9:40 PM · 12 files",
            12,
            ChatGroup::Yesterday,
            ChatKind::Archive {
                prompt: "Roll Paper & Ink out to the full app: rails, palette, docks and the \
                         editor. It ships by default.",
                reply: "Swapped the default tokens, restyled the shared kit, and rebuilt every \
                        dock on the new radii. The release ships Paper & Ink as the default look.",
            },
        ),
        (
            "Studio direction",
            "3:15 PM · 2 files",
            2,
            ChatGroup::Yesterday,
            ChatKind::Archive {
                prompt: "Show me a calmer studio direction — more space, fewer borders, one \
                         accent.",
                reply: "Pulled two borders out of every card, opened the gutters to 14, and kept \
                        the single yellow accent. The hierarchy is quieter and the thread reads \
                        first.",
            },
        ),
        (
            "Obsidian first look",
            "11:02 AM · 3 files",
            3,
            ChatGroup::Yesterday,
            ChatKind::Archive {
                prompt: "First look at Obsidian: black glass, gold hairlines, and a dock that \
                         hides.",
                reply: "Obsidian holds up: black glass panels, gold hairlines and an auto-hiding \
                        dock. The look is darker than Liquid Glass, with the same structure \
                        underneath.",
            },
        ),
    ];
    seeds
        .into_iter()
        .enumerate()
        .map(|(id, (title, meta, files, group, kind))| ChatThread {
            id: id as u64,
            title: title.to_string(),
            meta: meta.to_string(),
            files,
            group,
            kind,
        })
        .collect()
}

#[derive(Clone)]
struct Attachment {
    name: String,
    meta: String,
    icon: &'static str,
}

#[derive(Clone)]
struct SentMessage {
    chat: u64,
    text: String,
    attachments: Vec<Attachment>,
}

#[derive(Default)]
struct ComposerDraft {
    text: String,
    selection: InputSelection,
    attachments: Vec<Attachment>,
}

fn seeded_attachments() -> Vec<Attachment> {
    [
        ("design-preview-glass.png", "PNG image · 1.8 MB", "image"),
        ("theme.rs", "Rust source · 4.2 KB", "file-code"),
        ("README.md", "Markdown · 6.1 KB", "file-text"),
    ]
    .into_iter()
    .map(|(name, meta, icon)| Attachment {
        name: name.to_string(),
        meta: meta.to_string(),
        icon,
    })
    .collect()
}

fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let bytes_f = bytes as f64;
    if bytes_f >= MB {
        format!("{:.1} MB", bytes_f / MB)
    } else if bytes_f >= KB {
        format!("{:.1} KB", bytes_f / KB)
    } else {
        format!("{bytes} B")
    }
}

fn attachment_for_path(path: &std::path::Path) -> Attachment {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let (icon, kind) = match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" => ("image", "Image"),
        "rs" => ("file-code", "Rust source"),
        "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "toml" | "json" => ("file-code", "Source"),
        "md" => ("file-text", "Markdown"),
        "pdf" => ("file-text", "PDF"),
        _ => ("file-text", "File"),
    };
    let size = std::fs::metadata(path)
        .map(|meta| human_size(meta.len()))
        .unwrap_or_else(|_| "attached".to_string());
    Attachment {
        name,
        meta: format!("{kind} · {size}"),
        icon,
    }
}

const MODELS: [&str; 3] = ["Opus 4.6", "Sonnet 4.6", "Haiku 4.5"];
const MODEL_BLURBS: [&str; 3] = [
    "Deepest reasoning",
    "Balanced everyday coding",
    "Fastest answers",
];

pub struct GlassPreview {
    mode: ExecutionMode,
    day: bool,
    nav_open: bool,
    nav_epoch: usize,
    nav_pinned: bool,
    nav_hovered: bool,
    section: DrawerSection,
    section_epoch: usize,
    search_open: bool,
    search_epoch: usize,
    search_query: String,
    search_selection: InputSelection,
    search_index: usize,
    show_tokens: bool,
    show_warnings: bool,
    editor_file: EditorFile,
    selected_change: &'static str,
    all_runs: bool,
    applied_chats: Vec<u64>,
    chats: Vec<ChatThread>,
    active_chat: u64,
    next_chat: u64,
    chat_epoch: usize,
    model: usize,
    model_menu: bool,
    web_search: bool,
    attachments: Vec<Attachment>,
    draft: String,
    draft_selection: InputSelection,
    parked_drafts: HashMap<u64, ComposerDraft>,
    typo_notice: Option<usize>,
    typo_notice_epoch: usize,
    sent: Vec<SentMessage>,
    nav_focus: FocusHandle,
    search_focus: FocusHandle,
    composer_focus: FocusHandle,
    composer_scroll: ScrollHandle,
    thread_scroll: ScrollHandle,
    drawer_scroll: ScrollHandle,
    thread_settled: bool,
    focus_settled: bool,
}

const SEARCH_LIMIT: usize = 6;

fn single_line(text: &str) -> String {
    text.replace(['\n', '\r'], " ")
}

fn normalize_input(text: &str, search: bool) -> String {
    if search {
        single_line(text)
    } else {
        text.replace("\r\n", "\n").replace('\r', "\n")
    }
}

/// GPUI clips text without painting an ellipsis, so trim long labels ourselves.
fn ellipsize(text: &str, max: usize) -> String {
    let mut chars = text.chars();
    let head: String = chars.by_ref().take(max).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

fn draft_title(text: &str) -> String {
    ellipsize(text.lines().next().unwrap_or(text).trim(), 32)
}

impl GlassPreview {
    pub fn new(mode: ExecutionMode, cx: &mut Context<Self>) -> Self {
        REDUCE_MOTION.store(prefers_reduced_motion(), Ordering::Relaxed);
        // The day/night choice survives a relaunch; night is the first-run look.
        let day = stored_theme().unwrap_or(false);
        DAY_MODE.store(day, Ordering::Relaxed);
        // Chats open on the latest turn, not the oldest.
        let thread_scroll = ScrollHandle::new();
        let chats = seeded_chats();
        let next_chat = chats.len() as u64;
        Self {
            mode,
            day,
            nav_open: false,
            nav_epoch: 0,
            nav_pinned: false,
            nav_hovered: false,
            section: DrawerSection::Session,
            section_epoch: 0,
            search_open: false,
            search_epoch: 0,
            search_query: String::new(),
            search_selection: InputSelection::default(),
            search_index: 0,
            show_tokens: false,
            show_warnings: false,
            editor_file: EditorFile::Theme,
            selected_change: "theme.rs",
            all_runs: false,
            applied_chats: Vec::new(),
            chats,
            active_chat: 0,
            next_chat,
            chat_epoch: 0,
            model: 0,
            model_menu: false,
            web_search: false,
            attachments: seeded_attachments(),
            draft: String::new(),
            draft_selection: InputSelection::default(),
            parked_drafts: HashMap::new(),
            typo_notice: None,
            typo_notice_epoch: 0,
            sent: Vec::new(),
            nav_focus: cx.focus_handle(),
            search_focus: cx.focus_handle(),
            composer_focus: cx.focus_handle(),
            composer_scroll: ScrollHandle::new(),
            thread_scroll,
            drawer_scroll: ScrollHandle::new(),
            thread_settled: false,
            focus_settled: false,
        }
    }

    fn set_nav_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.nav_open == open {
            return;
        }
        self.nav_open = open;
        self.nav_epoch += 1;
        // The drawer pushes the thread, so re-anchor it to the latest turn —
        // once now, and once when the 320ms reveal finishes reflowing.
        self.thread_settled = false;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(380))
                .await;
            this.update(cx, |this, cx| {
                this.thread_settled = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn open_section(&mut self, section: DrawerSection, cx: &mut Context<Self>) {
        if self.section != section {
            self.section = section;
            self.section_epoch += 1;
            // Each panel starts at its own top, not at the offset the previous
            // panel was scrolled to.
            self.drawer_scroll.set_offset(point(px(0.0), px(0.0)));
        }
        self.model_menu = false;
        self.set_nav_open(true, cx);
    }

    fn reveal_section(&mut self, section: DrawerSection, pin: bool, cx: &mut Context<Self>) {
        // A command asked for a panel by name; land on the content it names.
        self.drawer_scroll.set_offset(point(px(0.0), px(0.0)));
        self.open_section(section, cx);
        if pin && !self.nav_pinned {
            // A command asked for this panel by name, so keep it on screen.
            self.nav_pinned = true;
            self.set_nav_open(true, cx);
        }
    }

    fn open_search(&mut self, window: &mut Window) {
        self.search_open = true;
        self.search_epoch += 1;
        self.search_index = 0;
        self.model_menu = false;
        window.focus(&self.search_focus);
    }

    fn close_search(&mut self, window: &mut Window) {
        if !self.search_open {
            return;
        }
        self.search_open = false;
        self.search_epoch += 1;
        // Reopening starts from the recent list, not the last run's query.
        self.clear_search();
        window.focus(&self.composer_focus);
    }

    fn clear_search(&mut self) {
        self.search_query.clear();
        self.search_selection = InputSelection::default();
        self.search_index = 0;
    }

    fn input_ref(&self) -> (&str, &InputSelection) {
        if self.search_open {
            (&self.search_query, &self.search_selection)
        } else {
            (&self.draft, &self.draft_selection)
        }
    }

    fn edit_input(&mut self, edit: impl FnOnce(&mut String, &mut InputSelection)) {
        if self.search_open {
            let previous = self.search_query.clone();
            edit(&mut self.search_query, &mut self.search_selection);
            // IME bookkeeping also routes through here on Enter; only a real
            // query change should throw away the highlighted row.
            if previous != self.search_query {
                self.search_index = 0;
            }
        } else {
            let before = self.draft.len();
            edit(&mut self.draft, &mut self.draft_selection);
            self.typo_notice = None;
            self.thread_settled = false;
            // The composer caps its height; typing past the last visible line
            // must keep the caret in view.
            if self.draft.len() > before && self.draft_selection.head == self.draft.len() {
                self.composer_scroll.scroll_to_bottom();
            }
        }
    }

    fn text_shortcut(&mut self, key: &Keystroke, cx: &mut Context<Self>) -> bool {
        if key.modifiers.control {
            match key.key.as_str() {
                "a" => self.edit_input(|text, selection| {
                    selection.anchor = 0;
                    selection.head = text.len();
                }),
                "v" => {
                    if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                        let text = normalize_input(&text, self.search_open);
                        self.edit_input(|value, selection| {
                            selection.replace(value, None, &text);
                        });
                    }
                }
                "c" | "x" => {
                    let selected = {
                        let (text, selection) = self.input_ref();
                        text[selection.range(text)].to_string()
                    };
                    if !selected.is_empty() {
                        cx.write_to_clipboard(ClipboardItem::new_string(selected));
                    }
                    if key.key == "x" {
                        self.edit_input(|text, selection| {
                            selection.replace(text, None, "");
                        });
                    }
                }
                _ => return false,
            }
            true
        } else if !key.modifiers.alt && !key.modifiers.platform {
            let mut handled = false;
            self.edit_input(|text, selection| {
                handled = selection.key(text, &key.key, key.modifiers.shift);
            });
            handled
        } else {
            false
        }
    }

    fn on_search_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.is_held {
            return;
        }
        if self.text_shortcut(&event.keystroke, cx) {
            cx.notify();
            return;
        }
        let results = search_results(
            SEARCH_ENTRIES,
            &self.search_query,
            self.active_thread().files,
        );
        let shown = results.len().min(SEARCH_LIMIT);
        match event.keystroke.key.as_str() {
            "escape" => {
                if self.search_query.is_empty() {
                    self.close_search(window);
                } else {
                    self.clear_search();
                }
            }
            "up" => {
                if shown > 0 {
                    self.search_index = (self.search_index + shown - 1) % shown;
                }
            }
            "down" => {
                if shown > 0 {
                    self.search_index = (self.search_index + 1) % shown;
                }
            }
            "enter" => {
                if let Some(entry) = results.get(self.search_index.min(shown.saturating_sub(1))) {
                    self.run_search(entry.action, window, cx);
                    return;
                }
            }
            _ => return,
        }
        cx.notify();
    }

    fn on_composer_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.is_held {
            return;
        }
        if self.text_shortcut(&event.keystroke, cx) {
            cx.notify();
            return;
        }
        let plain_tab = event.keystroke.key == "tab"
            && !event.keystroke.modifiers.shift
            && !event.keystroke.modifiers.control
            && !event.keystroke.modifiers.alt
            && !event.keystroke.modifiers.platform;
        if plain_tab && self.fix_typo(cx) {
            return;
        }
        if event.keystroke.key == "enter" {
            if event.keystroke.modifiers.shift {
                self.edit_input(|text, selection| {
                    selection.replace(text, None, "\n");
                });
                cx.notify();
                return;
            }
            self.send_draft(window, cx);
        }
    }

    fn run_search(&mut self, action: SearchAction, window: &mut Window, cx: &mut Context<Self>) {
        self.close_search(window);
        match action {
            SearchAction::Chat => {
                self.select_chat(0, window, cx);
                self.reveal_section(DrawerSection::Session, true, cx);
            }
            SearchAction::File(file) => {
                self.editor_file = file;
                self.reveal_section(DrawerSection::Editor, true, cx);
            }
            SearchAction::Change(name) => {
                self.selected_change = name;
                self.reveal_section(DrawerSection::Changes, true, cx);
            }
            SearchAction::Section(section) => self.reveal_section(section, true, cx),
            SearchAction::Theme => self.toggle_theme(cx),
            SearchAction::Tokens => {
                self.toggle_tokens();
                self.reveal_section(DrawerSection::Session, true, cx);
            }
            SearchAction::Warnings => {
                self.show_warnings = true;
                self.reveal_section(DrawerSection::Session, true, cx);
                self.scroll_to_warnings(cx);
            }
            SearchAction::Pin => {
                let next = !self.nav_pinned;
                self.set_pinned(next, cx);
                if next {
                    self.reveal_section(self.section, true, cx);
                }
            }
        }
        cx.notify();
    }

    fn send_draft(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (text, fixed) = spell::correct(self.draft.trim());
        if text.is_empty() {
            return;
        }
        if let Some(chat) = self
            .chats
            .iter_mut()
            .find(|chat| chat.id == self.active_chat)
        {
            if matches!(chat.kind, ChatKind::New) && chat.title == "New chat" {
                chat.title = draft_title(&text);
                chat.meta = "now · just started".to_string();
            }
        }
        // Files travel with the message they were attached to.
        let files = std::mem::take(&mut self.attachments);
        self.sent.push(SentMessage {
            chat: self.active_chat,
            text,
            attachments: files,
        });
        self.draft.clear();
        self.draft_selection = InputSelection::default();
        self.thread_settled = false;
        if fixed > 0 {
            self.typo_notice = Some(fixed);
            self.typo_notice_epoch += 1;
            let epoch = self.typo_notice_epoch;
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(4200))
                    .await;
                this.update(cx, |this, cx| {
                    if this.typo_notice_epoch == epoch {
                        this.typo_notice = None;
                        cx.notify();
                    }
                })
                .ok();
            })
            .detach();
        }
        window.focus(&self.composer_focus);
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(80))
                .await;
            this.update(cx, |this, cx| {
                this.thread_settled = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn set_pinned(&mut self, pinned: bool, cx: &mut Context<Self>) {
        if self.nav_pinned == pinned {
            return;
        }
        self.nav_pinned = pinned;
        let open = pinned || self.nav_hovered;
        self.set_nav_open(open, cx);
    }

    fn close_nav(&mut self, cx: &mut Context<Self>) {
        self.nav_pinned = false;
        self.set_nav_open(false, cx);
    }

    fn toggle_tokens(&mut self) {
        self.show_tokens = !self.show_tokens;
    }

    fn toggle_warnings(&mut self) {
        self.show_warnings = !self.show_warnings;
    }

    fn explain_warnings(&mut self, cx: &mut Context<Self>) {
        if self.show_warnings {
            self.show_warnings = false;
            cx.notify();
            return;
        }
        // The explanation lives in the Session panel, so reveal it instead of
        // flipping a flag the user cannot see.
        self.show_warnings = true;
        self.reveal_section(DrawerSection::Session, true, cx);
        self.scroll_to_warnings(cx);
        cx.notify();
    }

    fn scroll_to_warnings(&mut self, cx: &mut Context<Self>) {
        // The detail renders a frame after the section switches, and the
        // drawer is taller than its viewport once the warnings expand.
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(80))
                .await;
            this.update(cx, |this, cx| {
                this.drawer_scroll.scroll_to_bottom();
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        self.day = !self.day;
        self.model_menu = false;
        DAY_MODE.store(self.day, Ordering::Relaxed);
        store_theme(self.day);
        cx.notify();
    }

    fn active_thread(&self) -> &ChatThread {
        self.chats
            .iter()
            .find(|chat| chat.id == self.active_chat)
            .unwrap_or(&self.chats[0])
    }

    fn park_draft(&mut self) {
        let text = std::mem::take(&mut self.draft);
        let selection = std::mem::take(&mut self.draft_selection);
        let attachments = std::mem::take(&mut self.attachments);
        if text.is_empty() && attachments.is_empty() {
            self.parked_drafts.remove(&self.active_chat);
        } else {
            self.parked_drafts.insert(
                self.active_chat,
                ComposerDraft {
                    text,
                    selection,
                    attachments,
                },
            );
        }
    }

    fn restore_draft(&mut self, id: u64) {
        let parked = self.parked_drafts.remove(&id).unwrap_or_default();
        self.draft = parked.text;
        self.draft_selection = parked.selection;
        self.attachments = parked.attachments;
        self.typo_notice = None;
    }

    fn select_chat(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if !self.chats.iter().any(|chat| chat.id == id) {
            return;
        }
        window.focus(&self.composer_focus);
        self.model_menu = false;
        if self.active_chat == id {
            cx.notify();
            return;
        }
        // Every chat keeps its own unfinished prompt; switching must not carry
        // a draft, selection, or attachment into an unrelated conversation.
        self.park_draft();
        self.active_chat = id;
        self.restore_draft(id);
        self.ensure_selected_change();
        self.chat_epoch += 1;
        self.thread_settled = false;
        cx.notify();
    }

    fn ensure_selected_change(&mut self) {
        let count = self
            .chats
            .iter()
            .find(|chat| chat.id == self.active_chat)
            .map(|chat| chat.files)
            .unwrap_or(0);
        let files = changes_for(count);
        if !files.iter().any(|file| file.name == self.selected_change) {
            self.selected_change = files.first().map(|file| file.name).unwrap_or("");
        }
    }

    fn start_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let blank = self
            .chats
            .iter()
            .find(|chat| matches!(chat.kind, ChatKind::New) && chat.title == "New chat")
            .map(|chat| chat.id);
        let id = match blank {
            // An untouched draft is already waiting; typing belongs there.
            Some(id) => id,
            None => {
                let id = self.next_chat;
                self.next_chat += 1;
                self.chats.insert(
                    0,
                    ChatThread {
                        id,
                        title: "New chat".to_string(),
                        meta: "draft · no files".to_string(),
                        files: 0,
                        group: ChatGroup::Today,
                        kind: ChatKind::New,
                    },
                );
                id
            }
        };
        if self.active_chat != id {
            self.park_draft();
            self.active_chat = id;
            self.restore_draft(id);
            self.ensure_selected_change();
            self.chat_epoch += 1;
        }
        self.model_menu = false;
        self.thread_settled = false;
        window.focus(&self.composer_focus);
        cx.notify();
    }

    fn toggle_model_menu(&mut self, cx: &mut Context<Self>) {
        self.model_menu = !self.model_menu;
        cx.notify();
    }

    fn set_model(&mut self, model: usize, cx: &mut Context<Self>) {
        self.model = model.min(MODELS.len() - 1);
        self.model_menu = false;
        cx.notify();
    }

    fn fix_typo(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.draft_selection.range(&self.draft).is_empty() {
            return false;
        }
        let Some(fix) = spell::nearest(&self.draft, self.draft_selection.head) else {
            return false;
        };
        let caret = self.draft_selection.head;
        let start = fix.range.start;
        let end = fix.range.end;
        let new_len = fix.replacement.len();
        self.draft.replace_range(fix.range, &fix.replacement);
        // Distant fixes must not yank the caret away from where the user is
        // typing; only the word under the caret takes the caret with it.
        let head = if caret >= end {
            caret - (end - start) + new_len
        } else if caret <= start {
            caret
        } else {
            start + new_len
        };
        self.draft_selection.anchor = head;
        self.draft_selection.head = head;
        self.draft_selection.set_marked(None);
        self.typo_notice = None;
        self.thread_settled = false;
        cx.notify();
        true
    }

    fn toggle_web_search(&mut self, cx: &mut Context<Self>) {
        self.web_search = !self.web_search;
        cx.notify();
    }

    fn attach_files(&mut self, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Attach files".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = picked.await else {
                return;
            };
            this.update(cx, |this, cx| {
                this.attachments
                    .extend(paths.iter().map(|path| attachment_for_path(path)));
                this.thread_settled = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn remove_attachment(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.attachments.len() {
            self.attachments.remove(index);
            cx.notify();
        }
    }
}

impl EntityInputHandler for GlassPreview {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let (text, _) = self.input_ref();
        let range = crate::ui_kit::from_utf16(text, range.start)
            ..crate::ui_kit::from_utf16(text, range.end);
        actual.replace(crate::ui_kit::to_utf16(text, range.clone()));
        Some(text[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let (text, selection) = self.input_ref();
        Some(UTF16Selection {
            range: crate::ui_kit::to_utf16(text, selection.range(text)),
            reversed: selection.head < selection.anchor,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        let (text, selection) = self.input_ref();
        selection
            .marked()
            .map(|range| crate::ui_kit::to_utf16(text, range))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.edit_input(|_, selection| selection.set_marked(None));
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = normalize_input(text, self.search_open);
        self.edit_input(|value, selection| {
            selection.replace(value, range, &text);
        });
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = normalize_input(text, self.search_open);
        self.edit_input(|value, selection| {
            let start = range
                .as_ref()
                .map(|range| crate::ui_kit::from_utf16(value, range.start))
                .or_else(|| selection.marked().map(|range| range.start))
                .unwrap_or_else(|| selection.range(value).start);
            if selection.replace(value, range, &text) {
                selection.set_marked((!text.is_empty()).then_some(start..start + text.len()));
                if let Some(selected) = selected {
                    selection.anchor = start + crate::ui_kit::from_utf16(&text, selected.start);
                    selection.head = start + crate::ui_kit::from_utf16(&text, selected.end);
                }
            }
        });
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _: Range<usize>,
        bounds: gpui::Bounds<gpui::Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<gpui::Bounds<gpui::Pixels>> {
        Some(bounds)
    }

    fn character_index_for_point(
        &mut self,
        _: Point<gpui::Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

impl Render for GlassPreview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mode = self.mode;
        let day = self.day;
        let nav_open = self.nav_open;
        let nav_epoch = self.nav_epoch;
        let nav_pinned = self.nav_pinned;
        let section = self.section;
        let section_epoch = self.section_epoch;
        let search_open = self.search_open;
        let search_epoch = self.search_epoch;
        let search_query = self.search_query.clone();
        let search_selection = self.search_selection.clone();
        let search_index = self.search_index;
        let show_tokens = self.show_tokens;
        let show_warnings = self.show_warnings;
        let editor_file = self.editor_file;
        let selected_change = self.selected_change;
        let all_runs = self.all_runs;
        let applied = self.applied_chats.contains(&self.active_chat);
        let chats = self.chats.clone();
        let active_chat = self.active_chat;
        let chat_epoch = self.chat_epoch;
        let model = self.model;
        let model_menu = self.model_menu;
        let web_search = self.web_search;
        let attachments = self.attachments.clone();
        let chat = self.active_thread().clone();
        let changes = changes_for(chat.files);
        let live_run = matches!(chat.kind, ChatKind::Workbench);
        let draft = self.draft.clone();
        let draft_selection = self.draft_selection.clone();
        let typo_notice = self.typo_notice;
        let sent: Vec<SentMessage> = self
            .sent
            .iter()
            .filter(|message| message.chat == active_chat)
            .cloned()
            .collect();
        let nav_focus = self.nav_focus.clone();
        let search_focus = self.search_focus.clone();
        let composer_focus = self.composer_focus.clone();
        let thread_scroll = self.thread_scroll.clone();
        if !self.focus_settled && !search_open {
            // Nothing holds focus on a cold launch, so window-level shortcuts
            // never reach the root. A chat starts ready to type.
            window.focus(&composer_focus);
            self.focus_settled = true;
        }
        let composer_focused = composer_focus.is_focused(window);
        let search_focused = search_focus.is_focused(window);
        let entity = cx.entity();
        if !self.thread_settled {
            // The composer's attachment row reserves height in the same layout
            // that first measures the thread, so pin the latest turn from here
            // rather than from `new`, where the thread has not been sized yet.
            self.thread_scroll.scroll_to_bottom();
            self.thread_settled = true;
        }
        // The nav floats over the thread now, so a closed drawer reserves
        // nothing; below ~600 the header drops the redundant file chip and Auto.
        let reserve = if nav_open { DRAWER_W + EDGE_INSET } else { 0.0 };
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
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.is_held {
                    return;
                }
                if event.keystroke.modifiers.control && event.keystroke.key == "k" {
                    if this.search_open {
                        window.focus(&this.search_focus);
                    } else {
                        this.open_search(window);
                    }
                    cx.notify();
                } else if event.keystroke.key == "escape" && this.model_menu {
                    this.model_menu = false;
                    cx.notify();
                }
            }))
            .child(backdrop())
            .child(titlebar(day, search_open, cx))
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
                            .child(rail(&chats, active_chat, cx))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .h_full()
                                    .child(conversation(
                                        &thread_scroll,
                                        ConversationState {
                                            pinned: nav_pinned,
                                            show_tokens,
                                            show_warnings,
                                            compact,
                                            applied,
                                            chat: chat.clone(),
                                            chat_epoch,
                                            sent,
                                            draft,
                                            draft_selection,
                                            typo_notice,
                                            focused: composer_focused,
                                            composer_focus,
                                            composer_scroll: self.composer_scroll.clone(),
                                            entity: entity.clone(),
                                            model,
                                            model_menu,
                                            web_search,
                                            attachments: attachments.clone(),
                                        },
                                        cx,
                                    ))
                                    .with_animation(
                                        ("glass-conv", nav_epoch),
                                        Animation::new(motion(320)).with_easing(ease_out_quint()),
                                        move |el, t| {
                                            let k = edge_reveal(nav_open, nav_epoch, t);
                                            // Only an open panel pushes the thread;
                                            // the floating nav never claims a gutter.
                                            el.pr(px((DRAWER_W + EDGE_INSET) * k))
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
                            show_warnings,
                            editor_file,
                            selected_change,
                            all_runs,
                            active_chat,
                            changes,
                            applied,
                            live: live_run,
                        },
                        &nav_focus,
                        &self.drawer_scroll,
                        cx,
                    ));
                if search_open {
                    content.child(search_overlay(
                        SearchState {
                            epoch: search_epoch,
                            query: search_query,
                            selection: search_selection,
                            index: search_index,
                            focused: search_focused,
                            focus: search_focus,
                            entity,
                            files: chat.files,
                        },
                        cx,
                    ))
                } else {
                    content
                }
            })
            .child(status_bar(mode, nav_pinned, chat.files))
    }
}

// ----------------------------------------------------------------- backdrop

fn backdrop() -> Div {
    div().size_full().absolute().top_0().left_0().bg(if day() {
        linear_gradient(
            180.0,
            linear_color_stop(rgb(0xEFE8DB), 0.0),
            linear_color_stop(rgb(0xE5DDCD), 1.0),
        )
    } else {
        linear_gradient(
            180.0,
            linear_color_stop(rgb(0x0A0A0A), 0.0),
            linear_color_stop(rgb(0x050505), 1.0),
        )
    })
}

// ----------------------------------------------------------------- titlebar

fn titlebar(day: bool, search_open: bool, cx: &mut Context<GlassPreview>) -> Div {
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
        .child(search(search_open, cx))
        .child(div().flex_1())
        .child(theme_toggle(day, cx))
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

fn theme_toggle(day: bool, cx: &mut Context<GlassPreview>) -> Stateful<Div> {
    div()
        .id("glass-theme")
        .flex()
        .items_center()
        .justify_center()
        .size(px(30.0))
        .rounded(px(R_SM))
        .cursor_pointer()
        // The label stays theme-neutral: flipping the theme under an open
        // tooltip would otherwise leave the old wording on screen.
        .tooltip(hint("Toggle day / night theme"))
        .hover(|style| style.bg(hover_wash()))
        .active(|style| style.bg(press_wash()))
        .on_click(cx.listener(|this, _event, _window, cx| this.toggle_theme(cx)))
        .child(icon(if day { "moon" } else { "sun" }, 14.0, ink_muted()))
}

fn brand_tile() -> Div {
    div()
        .size(px(32.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .child(branding::logo(22.0))
}

fn search(open: bool, cx: &mut Context<GlassPreview>) -> Stateful<Div> {
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
        .border_color(tint(0x24))
        .shadow(soft_shadow())
        .cursor_pointer()
        .when(!open, |this| {
            this.hover(|style| style.bg(glass_fill_strong()).border_color(glass_border()))
                .active(|style| style.bg(tint(0x12)))
                .tooltip(hint("Search files, panels, commands · Ctrl K"))
        })
        .on_click(cx.listener(|this, _event, window, cx| {
            if this.search_open {
                window.focus(&this.search_focus);
            } else {
                this.open_search(window);
            }
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
                .child("Search files, panels, commands…"),
        )
        .child(keycap("Ctrl K"))
}

struct SearchState {
    epoch: usize,
    query: String,
    selection: InputSelection,
    index: usize,
    focused: bool,
    focus: FocusHandle,
    entity: Entity<GlassPreview>,
    files: usize,
}

fn input_bridge(entity: Entity<GlassPreview>, focus: FocusHandle) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| {
            window.handle_input(&focus, ElementInputHandler::new(bounds, entity.clone()), cx);
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

fn input_text(
    value: &str,
    selection: &InputSelection,
    focused: bool,
    placeholder: &str,
    size: f32,
    single_line: bool,
    typos: &[Range<usize>],
) -> Div {
    let range = selection.range(value);
    let empty = value.is_empty();
    let marker = focused && !empty && range.is_empty();
    let shown = if empty {
        if focused {
            format!("{placeholder}▏")
        } else {
            placeholder.to_string()
        }
    } else if marker {
        format!("{}▏{}", &value[..range.start], &value[range.end..])
    } else {
        value.to_string()
    };
    // The caret glyph is inserted before highlighting, so ranges at or past
    // it shift by its byte length.
    const MARKER: usize = "▏".len();
    let mut highlights = Vec::new();
    for typo in typos {
        if !range.is_empty() && typo.start < range.end && range.start < typo.end {
            continue;
        }
        let start = if marker && typo.start >= range.start {
            typo.start + MARKER
        } else {
            typo.start
        };
        let end = if marker && typo.end > range.start {
            typo.end + MARKER
        } else {
            typo.end
        };
        highlights.push((
            start..end,
            HighlightStyle {
                underline: Some(UnderlineStyle {
                    thickness: px(1.0),
                    color: Some(danger().into()),
                    wavy: true,
                }),
                ..Default::default()
            },
        ));
    }
    if !empty && focused && !range.is_empty() {
        highlights.push((
            range,
            HighlightStyle {
                background_color: Some(rgba(0xFDCC0240).into()),
                ..Default::default()
            },
        ));
    }
    div()
        .w_full()
        .min_w_0()
        .when(single_line, |this| {
            this.overflow_hidden().whitespace_nowrap()
        })
        .text_size(px(size))
        .line_height(px(size + 7.0))
        .text_color(if empty { ink_muted() } else { ink() })
        .child(StyledText::new(shown).with_highlights(highlights))
}

fn search_overlay(state: SearchState, cx: &mut Context<GlassPreview>) -> impl IntoElement {
    let SearchState {
        epoch,
        query,
        selection,
        index,
        focused,
        focus,
        entity,
        files,
    } = state;
    let results = search_results(SEARCH_ENTRIES, &query, files);
    let total = results.len();
    let shown = total.min(SEARCH_LIMIT);
    let selected = if shown == 0 { 0 } else { index.min(shown - 1) };
    let label = if query.trim().is_empty() {
        "RECENT"
    } else {
        "RESULTS"
    };
    let count = if total > shown && total > 0 {
        format!("{shown} of {total} results")
    } else if total == 1 {
        "1 result".to_string()
    } else {
        format!("{total} results")
    };
    let rows: Vec<AnyElement> = results
        .iter()
        .take(SEARCH_LIMIT)
        .enumerate()
        .map(|(row, entry)| {
            search_result(entry, &query, row == selected, row, cx).into_any_element()
        })
        .collect();
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
        .bg(sheet_scrim())
        .occlude()
        .on_click(cx.listener(|this, _event, window, cx| {
            this.close_search(window);
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
                .track_focus(&focus)
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
                        .border_color(tint(0x14))
                        .child(icon("search", 15.0, brand()))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .relative()
                                .child(input_text(
                                    &query,
                                    &selection,
                                    focused,
                                    "Search files, panels, commands…",
                                    13.0,
                                    true,
                                    &[],
                                ))
                                .child(input_bridge(entity.clone(), focus.clone())),
                        )
                        .when(!query.is_empty(), |this| {
                            this.child(
                                div()
                                    .id("glass-search-clear")
                                    .size(px(22.0))
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .hover(|style| style.bg(hover_wash()))
                                    .active(|style| style.bg(press_wash()))
                                    .tooltip(hint("Clear search"))
                                    .on_click(cx.listener(|this, _event, window, cx| {
                                        cx.stop_propagation();
                                        this.clear_search();
                                        window.focus(&this.search_focus);
                                        cx.notify();
                                    }))
                                    .child(icon("x", 12.0, ink_muted())),
                            )
                        })
                        .child(keycap("Esc")),
                )
                .when(shown > 0, |this| this.child(search_group_label(label)))
                .children(rows)
                .when(shown == 0, |this| this.child(search_empty(&query)))
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
                        .border_color(tint(0x14))
                        .text_size(px(10.5))
                        .text_color(ink_muted())
                        .when(shown > 0, |this| {
                            this.child("↑↓ navigate")
                                .child(div().size(px(2.0)).rounded_full().bg(ink_muted()))
                                .child("↵ open")
                                .child(div().size(px(2.0)).rounded_full().bg(ink_muted()))
                        })
                        .child(if query.is_empty() {
                            "Esc close"
                        } else {
                            "Esc clear"
                        })
                        .child(div().flex_1())
                        .child(count),
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

fn search_empty(query: &str) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(6.0))
        .py(px(22.0))
        .px(px(16.0))
        .child(
            div()
                .size(px(36.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(tint(0x0E))
                .child(icon("search", 16.0, ink_muted())),
        )
        .child(
            div()
                .mt(px(2.0))
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(ink_soft())
                .child(format!("No matches for “{}”", query.trim())),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(ink_muted())
                .child("Try a file name, a panel, or a command."),
        )
}

fn search_result(
    entry: &'static SearchEntry,
    query: &str,
    active: bool,
    index: usize,
    cx: &mut Context<GlassPreview>,
) -> Stateful<Div> {
    let action = entry.action;
    div()
        .id(entry.id)
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
        .cursor_pointer()
        .when(active, |this| this.bg(brand_tint()))
        .when(!active, |this| {
            this.hover(|style| style.bg(hover_wash()))
                .active(|style| style.bg(press_wash()))
        })
        .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
            if *hovered && this.search_index != index {
                this.search_index = index;
                cx.notify();
            }
        }))
        .on_click(cx.listener(move |this, _event, window, cx| {
            cx.stop_propagation();
            this.run_search(action, window, cx);
        }))
        .child(
            div()
                .size(px(26.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(7.0))
                .bg(if active { rgba(0xFDCC0226) } else { well() })
                .child(icon(
                    entry.icon,
                    13.0,
                    if active { brand() } else { ink_muted() },
                )),
        )
        .child(search_title(entry.title, query, active))
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(10.5))
                .text_color(ink_muted())
                .child(entry.meta),
        )
        .when(active, |this| this.child(keycap("↵")))
}

fn search_title(title: &'static str, query: &str, active: bool) -> Div {
    let mut ranges: Vec<Range<usize>> = Vec::new();
    if title.is_ascii() {
        let lowered = title.to_lowercase();
        for token in query.split_whitespace() {
            let needle = token.to_lowercase();
            let mut from = 0;
            while let Some(found) = lowered[from..].find(&needle) {
                let start = from + found;
                ranges.push(start..start + needle.len());
                from = start + needle.len();
            }
        }
    }
    ranges.sort_by_key(|range| range.start);
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in ranges {
        match merged.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    let highlights: Vec<_> = merged
        .into_iter()
        .map(|range| {
            (
                range,
                HighlightStyle {
                    background_color: Some(rgba(0xFDCC0240).into()),
                    ..Default::default()
                },
            )
        })
        .collect();
    div()
        .flex_1()
        .min_w_0()
        .truncate()
        .text_size(px(12.5))
        .text_color(if active { ink() } else { ink_soft() })
        .child(StyledText::new(title).with_highlights(highlights))
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
        .text_color(brand_text())
        .child("VA")
}

// --------------------------------------------------------------------- rail

fn rail(chats: &[ChatThread], active_chat: u64, cx: &mut Context<GlassPreview>) -> Div {
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
                .child(bare_icon(
                    "glass-chat-new",
                    "plus",
                    "New chat",
                    |this, window, cx| this.start_chat(window, cx),
                    cx,
                )),
        )
        .child(div().h(px(8.0)))
        .child(chat_section_label("TODAY"))
        .children(
            chats
                .iter()
                .filter(|chat| chat.group == ChatGroup::Today)
                .map(|chat| chat_row(chat, chat.id == active_chat, cx)),
        )
        .child(div().h(px(12.0)))
        .child(chat_section_label("YESTERDAY"))
        .children(
            chats
                .iter()
                .filter(|chat| chat.group == ChatGroup::Yesterday)
                .map(|chat| chat_row(chat, chat.id == active_chat, cx)),
        )
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

fn chat_row(chat: &ChatThread, active: bool, cx: &mut Context<GlassPreview>) -> Stateful<Div> {
    let id = chat.id;
    // GPUI clips this row's text without painting an ellipsis, so trim the
    // rail label ourselves; the header still carries the longer title.
    let title = ellipsize(&chat.title, 24);
    let meta = chat.meta.clone();
    div()
        .id(ElementId::named_usize("glass-chat-row", id as usize))
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
        .on_click(cx.listener(move |this, _event, window, cx| {
            this.select_chat(id, window, cx);
        }))
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
        .tooltip(hint("Current workspace"))
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
}

// -------------------------------------------------------------- conversation

struct ConversationState {
    pinned: bool,
    show_tokens: bool,
    show_warnings: bool,
    compact: bool,
    applied: bool,
    chat: ChatThread,
    chat_epoch: usize,
    sent: Vec<SentMessage>,
    draft: String,
    draft_selection: InputSelection,
    typo_notice: Option<usize>,
    focused: bool,
    composer_focus: FocusHandle,
    composer_scroll: ScrollHandle,
    entity: Entity<GlassPreview>,
    model: usize,
    model_menu: bool,
    web_search: bool,
    attachments: Vec<Attachment>,
}

fn conversation(
    scroll: &ScrollHandle,
    state: ConversationState,
    cx: &mut Context<GlassPreview>,
) -> Div {
    let ConversationState {
        pinned,
        show_tokens,
        show_warnings,
        compact,
        applied,
        chat,
        chat_epoch,
        sent,
        draft,
        draft_selection,
        typo_notice,
        focused,
        composer_focus,
        composer_scroll,
        entity,
        model,
        model_menu,
        web_search,
        attachments,
    } = state;
    let blank = matches!(chat.kind, ChatKind::New) && sent.is_empty();
    div()
        .flex_1()
        .min_w_0()
        .h_full()
        .relative()
        .flex()
        .flex_col()
        .overflow_hidden()
        .rounded(px(R_LG))
        .bg(panel_fill())
        .border_1()
        .border_color(panel_border())
        .shadow(panel_shadow())
        .child(conversation_header(compact, &chat, model, model_menu, cx))
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
                                .pt(px(14.0))
                                .pb(px(14.0))
                                .child(
                                    div()
                                        .w_full()
                                        .max_w(px(720.0))
                                        .flex()
                                        .flex_col()
                                        .gap(px(14.0))
                                        .when(matches!(chat.kind, ChatKind::Workbench), |this| {
                                            this.child(intro_turn())
                                                .child(user_turn())
                                                .child(work_turn(compact, applied, cx))
                                        })
                                        .when_some(
                                            archive_parts(chat.kind),
                                            |this, (prompt, reply)| {
                                                this.child(sent_turn(prompt, &[]))
                                                    .child(reply_turn(reply))
                                            },
                                        )
                                        .when(blank, |this| this.child(empty_thread()))
                                        .children(sent.iter().map(|message| {
                                            sent_turn(&message.text, &message.attachments)
                                        }))
                                        .with_animation(
                                            ("glass-chat", chat_epoch),
                                            Animation::new(motion(180))
                                                .with_easing(ease_out_quint()),
                                            |el, t| el.opacity(0.55 + 0.45 * t),
                                        ),
                                ),
                        ),
                )
                // Content slides under this fade, so a scrolled thread never
                // shows a bubble sliced by a hard edge.
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .h(px(16.0))
                        .bg(panel_top_fade(0xFF)),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(16.0))
                        .left_0()
                        .right_0()
                        .h(px(28.0))
                        .bg(linear_gradient(
                            180.0,
                            linear_color_stop(panel_top_fade(0xFF), 0.0),
                            linear_color_stop(panel_top_fade(0x00), 1.0),
                        )),
                )
                // Scrolled-up content also meets the composer; soften that cut
                // the same way as the header edge.
                .child(
                    div()
                        .absolute()
                        .bottom_0()
                        .left_0()
                        .right_0()
                        .h(px(20.0))
                        .bg(linear_gradient(
                            180.0,
                            linear_color_stop(panel_top_fade(0x00), 0.0),
                            linear_color_stop(panel_top_fade(0xFF), 1.0),
                        )),
                ),
        )
        .child(suggestions(
            pinned,
            show_tokens,
            show_warnings,
            chat.files > 0,
            cx,
        ))
        .child(composer(
            compact,
            draft,
            draft_selection,
            focused,
            composer_focus,
            composer_scroll,
            entity,
            &attachments,
            web_search,
            show_tokens,
            chat.files,
            typo_notice,
            cx,
        ))
        .child(
            div().w_full().flex().justify_center().pb(px(12.0)).child(
                div()
                    .text_size(px(10.5))
                    .text_color(ink_muted())
                    .child("Vibe can make mistakes — review every change before you apply."),
            ),
        )
        .when(model_menu, |this| {
            this.child(
                div()
                    .id("glass-model-scrim")
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .occlude()
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.model_menu = false;
                        cx.notify();
                    })),
            )
        })
}

fn model_menu_panel(model: usize, cx: &mut Context<GlassPreview>) -> Stateful<Div> {
    div()
        .id("glass-model-menu")
        .absolute()
        .top(px(38.0))
        .right(px(-60.0))
        .w(px(248.0))
        .flex()
        .flex_col()
        .p(px(6.0))
        .rounded(px(14.0))
        .bg(sheet_fill())
        .border_1()
        .border_color(panel_border())
        .shadow(glass_shadow())
        .on_click(cx.listener(|_, _event, _window, cx| cx.stop_propagation()))
        .child(
            div().h(px(26.0)).px(px(10.0)).flex().items_center().child(
                div()
                    .text_size(px(10.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(ink_muted())
                    .child("MODEL"),
            ),
        )
        .children(MODELS.iter().enumerate().map(|(index, name)| {
            let selected = index == model;
            let blurb = MODEL_BLURBS[index];
            div()
                .id(("glass-model-option", index))
                .flex()
                .items_center()
                .gap(px(10.0))
                .h(px(44.0))
                .px(px(10.0))
                .rounded(px(10.0))
                .cursor_pointer()
                .when(selected, |this| this.bg(brand_tint()))
                .when(!selected, |this| this.hover(|style| style.bg(hover_wash())))
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    this.set_model(index, cx);
                }))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(1.0))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if selected { brand_text() } else { ink() })
                                .child(*name),
                        )
                        .child(
                            div()
                                .text_size(px(10.5))
                                .text_color(ink_muted())
                                .child(blurb),
                        ),
                )
                .when(selected, |this| {
                    this.child(icon("check", 13.0, brand_text()))
                })
        }))
}

fn conversation_header(
    compact: bool,
    chat: &ChatThread,
    model: usize,
    model_menu: bool,
    cx: &mut Context<GlassPreview>,
) -> Div {
    let status = match chat.kind {
        ChatKind::Workbench => "Vibe · Code mode · this session",
        ChatKind::Archive { .. } => "Vibe · Code mode · earlier",
        ChatKind::New if chat.title == "New chat" => "Vibe · Code mode · new session",
        ChatKind::New => "Vibe · Code mode · this session",
    };
    let chip = if chat.files > 0 {
        format!("{} files changed", chat.files)
    } else {
        "No files yet".to_string()
    };
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
                        .flex_1()
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
                                .child(chat.title.clone()),
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
                                        .child(if compact { "Vibe · Code mode" } else { status }),
                                ),
                        ),
                ),
        )
        .child(
            div()
                .relative()
                .flex_shrink_0()
                .child(model_pill(compact, model, model_menu, cx))
                .when(model_menu, |this| {
                    this.child(deferred(model_menu_panel(model, cx)))
                }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(4.0))
                // At full width both sides grow so the pill stays centred; in
                // compact the title needs the whole remainder instead.
                .when(!compact, |this| this.flex_1().child(soft_chip(chip)))
                .child(bare_icon(
                    "glass-history",
                    "history",
                    "Run history",
                    |this, _window, cx| {
                        this.reveal_section(DrawerSection::History, true, cx);
                    },
                    cx,
                ))
                .child(bare_icon(
                    "glass-new",
                    "plus",
                    "New chat",
                    |this, window, cx| this.start_chat(window, cx),
                    cx,
                )),
        )
}

fn model_pill(
    compact: bool,
    model: usize,
    open: bool,
    cx: &mut Context<GlassPreview>,
) -> Stateful<Div> {
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
        .border_color(if open { brand_line() } else { tint(0x2E) })
        .shadow(soft_shadow())
        .cursor_pointer()
        .hover(|style| style.bg(glass_fill_strong()))
        .active(|style| style.bg(tint(0x12)))
        .on_click(cx.listener(|this, _event, _window, cx| this.toggle_model_menu(cx)))
        .child(icon("sparkles", 12.0, brand()))
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(ink())
                .child(MODELS[model.min(MODELS.len() - 1)]),
        )
        .child(icon(
            if open { "chevron-up" } else { "chevron-down" },
            11.0,
            ink_muted(),
        ))
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
                 History. Pin keeps it open; the context ring sits beneath the nav.",
            ),
    )
}

fn archive_parts(kind: ChatKind) -> Option<(&'static str, &'static str)> {
    match kind {
        ChatKind::Archive { prompt, reply } => Some((prompt, reply)),
        ChatKind::Workbench | ChatKind::New => None,
    }
}

fn reply_turn(text: &str) -> Div {
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
            .child(text.to_string()),
    )
}

fn empty_thread() -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(10.0))
        .pt(px(72.0))
        .pb(px(36.0))
        .child(
            div()
                .size(px(44.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(brand_tint())
                .border_1()
                .border_color(brand_line())
                .child(icon("sparkles", 20.0, brand())),
        )
        .child(
            div()
                .text_size(px(15.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ink())
                .child("Start a new chat"),
        )
        .child(
            div()
                .max_w(px(380.0))
                .text_center()
                .text_size(px(12.5))
                .line_height(px(19.0))
                .text_color(ink_muted())
                .child(
                    "Ask for a change, attach a file, or search the codebase. \
                     Nothing leaves this machine until you send.",
                ),
        )
}

fn user_turn() -> Div {
    div().w_full().flex().justify_end().child(
        div()
            .w_full()
            .max_w(px(500.0))
            .min_w_0()
            .p(px(12.0))
            .rounded(px(14.0))
            .bg(raised())
            .border_1()
            .border_color(panel_border())
            .shadow(soft_shadow())
            .text_size(px(13.0))
            .line_height(px(20.0))
            .text_color(ink())
            .child(
                "Move Session and Changes into the auto-hide bar, animate it, and \
                 give the context window a quiet progress ring.",
            ),
    )
}

fn sent_turn(text: &str, attachments: &[Attachment]) -> Div {
    div().w_full().flex().justify_end().child(
        div()
            .max_w(px(500.0))
            .min_w_0()
            .p(px(12.0))
            .rounded(px(14.0))
            .bg(raised())
            .border_1()
            .border_color(panel_border())
            .shadow(soft_shadow())
            .flex()
            .flex_col()
            .gap(px(6.0))
            .when(!attachments.is_empty(), |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .children(attachments.iter().map(|attachment| {
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .h(px(24.0))
                                .pl(px(7.0))
                                .pr(px(9.0))
                                .rounded(px(7.0))
                                .bg(well())
                                .border_1()
                                .border_color(panel_border_soft())
                                .child(icon(attachment.icon, 11.0, ink_muted()))
                                .child(
                                    div()
                                        .max_w(px(220.0))
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(10.5))
                                        .text_color(ink_soft())
                                        .child(attachment.name.clone()),
                                )
                        })),
                )
            })
            .child(
                div()
                    .text_size(px(13.0))
                    .line_height(px(20.0))
                    .text_color(ink())
                    .child(text.to_string()),
            ),
    )
}

fn work_turn(compact: bool, applied: bool, cx: &mut Context<GlassPreview>) -> Div {
    vibe_turn(work_card(compact, applied, cx))
}

fn work_card(compact: bool, applied: bool, cx: &mut Context<GlassPreview>) -> Div {
    div()
        .w_full()
        .p(px(14.0))
        .rounded(px(R_LG))
        .bg(work_surface())
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
        .child(check_line("Pin in the panel header keeps it open"))
        .child(check_line("Progress ring shows the 62% context window"))
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
                .child(apply_button(applied, cx))
                .child(review_button(cx))
                .when(applied, |this| this.child(undo_button(cx)))
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
                .flex_1()
                .min_w_0()
                .text_size(px(12.5))
                .line_height(px(18.0))
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
                .pr(px(DRAWER_CARD_RIGHT))
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
        .child(diff_body(name))
}

// The preview follows the file selected in the Changed files card, so each
// file needs a hunk that could plausibly belong to it.
fn diff_body(name: &'static str) -> Div {
    let body = div().w_full().flex().flex_col().py(px(5.0));
    match name {
        "theme.rs" => body
            .child(diff_row(" ", None, &[cm("@@ warm canvas @@")]))
            .child(diff_row(
                "-",
                Some(danger_tint()),
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
            .child(diff_row(
                "+",
                Some(success_tint()),
                &[
                    kw("pub const "),
                    ty("CANVAS"),
                    p(": "),
                    ty("Rgba"),
                    p(" = "),
                    fx("rgb"),
                    p("("),
                    nu("0x0A0A0B"),
                    p(");"),
                ],
            ))
            .child(diff_row(" ", None, &[cm("@@ brand accent @@")]))
            .child(diff_row(
                "-",
                Some(danger_tint()),
                &[
                    kw("pub const "),
                    ty("BRAND"),
                    p(": "),
                    ty("Rgba"),
                    p(" = "),
                    fx("rgb"),
                    p("("),
                    nu("0xF5C400"),
                    p(");"),
                ],
            ))
            .child(diff_row(
                "+",
                Some(success_tint()),
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
            )),
        "widgets.rs" => body
            .child(diff_row(" ", None, &[cm("@@ quiet button @@")]))
            .child(diff_row(
                "-",
                Some(danger_tint()),
                &[p("    .h("), fx("px"), p("("), nu("26.0"), p("))")],
            ))
            .child(diff_row(
                "+",
                Some(success_tint()),
                &[p("    .h("), fx("px"), p("("), nu("28.0"), p("))")],
            ))
            .child(diff_row(" ", None, &[cm("@@ primary fill @@")]))
            .child(diff_row(
                "-",
                Some(danger_tint()),
                &[p("    .bg("), fx("rgb"), p("("), nu("0xF5C400"), p("))")],
            ))
            .child(diff_row(
                "+",
                Some(success_tint()),
                &[p("    .bg("), fx("rgb"), p("("), nu("0xFDCC02"), p("))")],
            )),
        "left_dock.rs" => body
            .child(diff_row(" ", None, &[cm("@@ chat rows @@")]))
            .child(diff_row(
                "-",
                Some(danger_tint()),
                &[
                    p("    .px("),
                    fx("px"),
                    p("("),
                    nu("10.0"),
                    p(")).gap("),
                    fx("px"),
                    p("("),
                    nu("8.0"),
                    p("))"),
                ],
            ))
            .child(diff_row(
                "+",
                Some(success_tint()),
                &[
                    p("    .px("),
                    fx("px"),
                    p("("),
                    nu("12.0"),
                    p(")).gap("),
                    fx("px"),
                    p("("),
                    nu("10.0"),
                    p("))"),
                ],
            )),
        "assistant_panel.rs" => body
            .child(diff_row(" ", None, &[cm("@@ attachment chips @@")]))
            .child(diff_row(
                "-",
                Some(danger_tint()),
                &[
                    p("    .h("),
                    fx("px"),
                    p("("),
                    nu("22.0"),
                    p(")).rounded("),
                    fx("px"),
                    p("("),
                    nu("6.0"),
                    p("))"),
                ],
            ))
            .child(diff_row(
                "+",
                Some(success_tint()),
                &[
                    p("    .h("),
                    fx("px"),
                    p("("),
                    nu("24.0"),
                    p(")).rounded("),
                    fx("px"),
                    p("("),
                    nu("7.0"),
                    p("))"),
                ],
            )),
        "editor_pane.rs" => body
            .child(diff_row(" ", None, &[cm("@@ preview footer @@")]))
            .child(diff_row(
                "-",
                Some(danger_tint()),
                &[p("    .h("), fx("px"), p("("), nu("26.0"), p("))")],
            ))
            .child(diff_row(
                "+",
                Some(success_tint()),
                &[p("    .h("), fx("px"), p("("), nu("28.0"), p("))")],
            )),
        "icon.rs" => body
            .child(diff_row(" ", None, &[cm("@@ icon set @@")]))
            .child(diff_row(
                " ",
                None,
                &[
                    p("    "),
                    p("\"dock\" => "),
                    fx("svg"),
                    p("("),
                    p("\"dock\""),
                    p("),"),
                ],
            ))
            .child(diff_row(
                "+",
                Some(success_tint()),
                &[
                    p("    "),
                    p("\"edge\" => "),
                    fx("svg"),
                    p("("),
                    p("\"edge\""),
                    p("),"),
                ],
            )),
        _ => body
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
                &[p("    "), fx("context_ring"), p("("), p("epoch"), p(")")],
            ))
            .child(diff_row(
                "+",
                Some(success_tint()),
                &[p("    "), fx("context_ring"), p("()")],
            )),
    }
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
    has_checks: bool,
    cx: &mut Context<GlassPreview>,
) -> Div {
    div()
        .w_full()
        .flex()
        .justify_center()
        .px(px(20.0))
        .pb(px(14.0))
        .child(
            div()
                .w_full()
                .max_w(px(720.0))
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
                    |this, cx| this.set_pinned(!this.nav_pinned, cx),
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
                    |this, _cx| this.toggle_tokens(),
                    cx,
                ))
                // A chat that has not run anything has no warnings to explain;
                // offering the chip would promise content that does not exist.
                .when(has_checks, |this| {
                    this.child(suggestion(
                        "glass-suggest-warnings",
                        if show_warnings {
                            "Hide warnings"
                        } else {
                            "Explain warnings"
                        },
                        show_warnings,
                        |this, cx| this.explain_warnings(cx),
                        cx,
                    ))
                }),
        )
}

fn suggestion(
    id: &'static str,
    label: &'static str,
    active: bool,
    action: impl Fn(&mut GlassPreview, &mut Context<GlassPreview>) + 'static,
    cx: &mut Context<GlassPreview>,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .h(px(28.0))
        .px(px(11.0))
        .rounded_full()
        .bg(tint(0x0C))
        .border_1()
        .border_color(if active { brand_line() } else { panel_border() })
        .text_size(px(11.5))
        .text_color(if active { ink() } else { ink_soft() })
        .cursor_pointer()
        .when(active, |this| this.bg(brand_tint()))
        .when(!active, |this| this.hover(|style| style.bg(hover_wash())))
        .active(|style| style.bg(press_wash()))
        .on_click(cx.listener(move |this, _event, _window, cx| {
            action(this, cx);
            cx.notify();
        }))
        .child(label)
}

#[allow(clippy::too_many_arguments)]
fn composer(
    compact: bool,
    draft: String,
    selection: InputSelection,
    focused: bool,
    focus: FocusHandle,
    scroll: ScrollHandle,
    entity: Entity<GlassPreview>,
    attachments: &[Attachment],
    web_search: bool,
    show_tokens: bool,
    files: usize,
    typo_notice: Option<usize>,
    cx: &mut Context<GlassPreview>,
) -> Div {
    let ready = !draft.trim().is_empty();
    // Keep the token figure and the context ring in agreement: both derive
    // from the active chat, so an empty chat reads zero.
    let token_label = if files == 0 {
        if compact {
            "0".to_string()
        } else {
            "0 / 120k".to_string()
        }
    } else {
        let tokens = (context_usage(files) * 120.0).round() as u32;
        if compact {
            format!("{tokens}k")
        } else {
            format!("{tokens}k / 120k")
        }
    };
    let token_tip = if files == 0 {
        "No context used yet"
    } else {
        "Context window used by this chat"
    };
    let typos: Vec<Range<usize>> = spell::suggestions(&draft)
        .into_iter()
        .map(|fix| fix.range)
        .collect();
    let active = if focused && selection.range(&draft).is_empty() {
        spell::nearest(&draft, selection.head)
    } else {
        None
    };
    let footer_status: Option<AnyElement> = if let Some(fix) = active {
        Some(typo_fix_chip(fix, cx).into_any_element())
    } else {
        typo_notice.map(|count| typo_fixed_chip(count).into_any_element())
    };
    div()
        .w_full()
        .flex()
        .justify_center()
        .px(px(20.0))
        .pb(px(14.0))
        .child(
            div()
                .relative()
                .overflow_hidden()
                .w_full()
                .max_w(px(720.0))
                .flex()
                .flex_col()
                .gap(px(14.0))
                .p(px(16.0))
                .rounded(px(24.0))
                .bg(glass_fill_strong())
                .border_1()
                .border_color(if focused {
                    brand_line()
                } else {
                    glass_border()
                })
                .shadow(glass_shadow())
                .when(!attachments.is_empty(), |this| {
                    this.child(attachment_row(attachments, compact, cx))
                })
                .child(
                    div()
                        .id("glass-composer-input")
                        .relative()
                        .w_full()
                        .min_h(px(22.0))
                        .max_h(px(110.0))
                        .overflow_y_scroll()
                        .track_scroll(&scroll)
                        .flex()
                        .items_start()
                        .px(px(2.0))
                        .track_focus(&focus)
                        .on_key_down(cx.listener(GlassPreview::on_composer_key))
                        .on_click(cx.listener(|this, _event, window, cx| {
                            window.focus(&this.composer_focus);
                            cx.notify();
                        }))
                        .child(input_text(
                            &draft,
                            &selection,
                            focused,
                            "Ask me anything…",
                            15.0,
                            false,
                            &typos,
                        ))
                        .child(input_bridge(entity, focus)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(composer_icon(
                            "glass-attach",
                            "plus",
                            "Attach files",
                            false,
                            |this, cx| this.attach_files(cx),
                            cx,
                        ))
                        .child(composer_icon(
                            "glass-web",
                            "globe",
                            // Tooltips do not repaint while hovered, so this one
                            // stays state-neutral; the yellow tint reports on/off.
                            "Search the web",
                            web_search,
                            |this, cx| this.toggle_web_search(cx),
                            cx,
                        ))
                        .when_some(footer_status, |this, status| this.child(status))
                        .child(div().flex_1())
                        // Usage sits next to the composer controls, so the
                        // toggle gives feedback even when the drawer is shut.
                        .when(show_tokens, |this| {
                            this.child(
                                div()
                                    .id("glass-token-count")
                                    .pr(px(6.0))
                                    .font_family(theme::code_font())
                                    .text_size(px(10.5))
                                    .text_color(ink_muted())
                                    .tooltip(hint(token_tip))
                                    .child(token_label),
                            )
                        })
                        .child(
                            div()
                                .pr(px(4.0))
                                .font_family(theme::code_font())
                                .text_size(px(10.5))
                                .text_color(ink_muted())
                                .child("Rust"),
                        )
                        .child(send_button(ready, cx)),
                ),
        )
}

fn typo_fix_chip(fix: spell::Suggestion, cx: &mut Context<GlassPreview>) -> Stateful<Div> {
    div()
        .id("glass-typo-fix")
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap(px(6.0))
        .h(px(26.0))
        .px(px(9.0))
        .max_w(px(280.0))
        .overflow_hidden()
        .whitespace_nowrap()
        .rounded_full()
        .bg(tint(0x0C))
        .border_1()
        .border_color(panel_border())
        .cursor_pointer()
        .tooltip(hint("Fix this typo · Tab"))
        .hover(|style| style.bg(hover_wash()))
        .active(|style| style.bg(press_wash()))
        .on_click(cx.listener(|this, _event, _window, cx| {
            this.fix_typo(cx);
        }))
        .child(icon("sparkles", 11.0, brand_text()))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(ink_muted())
                .child(fix.original),
        )
        .child(div().text_size(px(11.0)).text_color(ink_muted()).child("→"))
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(ink())
                .child(fix.replacement),
        )
        .child(keycap("Tab"))
}

fn typo_fixed_chip(count: usize) -> Div {
    let label = if count == 1 {
        "1 typo fixed".to_string()
    } else {
        format!("{count} typos fixed")
    };
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap(px(6.0))
        .h(px(26.0))
        .px(px(9.0))
        .rounded_full()
        .bg(success_tint())
        .border_1()
        .border_color(success_line())
        .child(icon("check", 11.0, success()))
        .child(div().text_size(px(11.0)).text_color(success()).child(label))
}

fn attachment_row(
    attachments: &[Attachment],
    compact: bool,
    cx: &mut Context<GlassPreview>,
) -> Div {
    let row = div()
        .w_full()
        .flex()
        .flex_wrap()
        .gap(px(if compact { 6.0 } else { 8.0 }));
    if compact {
        // A narrow composer cannot afford three full-width cards; pills keep
        // the thread's viewport taller while the filename stays readable.
        row.children(
            attachments
                .iter()
                .enumerate()
                .map(|(index, attachment)| attachment_chip(attachment, index, cx)),
        )
    } else {
        row.children(
            attachments
                .iter()
                .enumerate()
                .map(|(index, attachment)| attachment_card(attachment, index, cx)),
        )
    }
}

fn attachment_chip(
    attachment: &Attachment,
    index: usize,
    cx: &mut Context<GlassPreview>,
) -> Stateful<Div> {
    let name = ellipsize(&attachment.name, 14);
    div()
        .id(ElementId::named_usize("glass-attach", index))
        .w(px(140.0))
        .h(px(30.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(6.0))
        .pl(px(8.0))
        .pr(px(2.0))
        .rounded_full()
        .bg(card_fill())
        .border_1()
        .border_color(panel_border_soft())
        .child(icon(attachment.icon, 12.0, ink_muted()))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(11.0))
                .text_color(ink_soft())
                .child(name),
        )
        .child(
            div()
                .id(ElementId::named_usize("glass-attach-remove", index))
                .size(px(20.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(5.0))
                .cursor_pointer()
                .hover(|style| style.bg(hover_wash()))
                .active(|style| style.bg(press_wash()))
                .tooltip(hint("Remove attachment"))
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    this.remove_attachment(index, cx);
                }))
                .child(icon("x", 11.0, ink_muted())),
        )
}

fn attachment_card(
    attachment: &Attachment,
    index: usize,
    cx: &mut Context<GlassPreview>,
) -> Stateful<Div> {
    let name = ellipsize(&attachment.name, 24);
    div()
        .id(ElementId::named_usize("glass-attach", index))
        .w(px(222.0))
        .h(px(46.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .pl(px(6.0))
        .pr(px(4.0))
        .rounded(px(11.0))
        .bg(card_fill())
        .border_1()
        .border_color(panel_border_soft())
        .shadow(soft_shadow())
        .child(
            div()
                .size(px(32.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(8.0))
                .bg(well())
                .border_1()
                .border_color(panel_border_soft())
                .child(icon(attachment.icon, 14.0, ink_muted())),
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
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(ink())
                        .child(name),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(px(10.0))
                        .text_color(ink_muted())
                        .child(attachment.meta.clone()),
                ),
        )
        .child(
            div()
                .id(ElementId::named_usize("glass-attach-remove", index))
                .size(px(24.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.0))
                .cursor_pointer()
                .hover(|style| style.bg(hover_wash()))
                .active(|style| style.bg(press_wash()))
                .tooltip(hint("Remove attachment"))
                .on_click(cx.listener(move |this, _event, _window, cx| {
                    this.remove_attachment(index, cx);
                }))
                .child(icon("x", 12.0, ink_muted())),
        )
}

fn composer_icon(
    id: &'static str,
    name: &'static str,
    label: &'static str,
    active: bool,
    action: impl Fn(&mut GlassPreview, &mut Context<GlassPreview>) + 'static,
    cx: &mut Context<GlassPreview>,
) -> Stateful<Div> {
    div()
        .id(id)
        .size(px(30.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(8.0))
        .cursor_pointer()
        .when(active, |this| this.bg(brand_tint()))
        .when(!active, |this| {
            this.hover(|style| style.bg(hover_wash()))
                .active(|style| style.bg(press_wash()))
        })
        .tooltip(hint(label))
        .on_click(cx.listener(move |this, _event, _window, cx| {
            action(this, cx);
            cx.notify();
        }))
        .child(icon(
            name,
            15.0,
            if active { brand_text() } else { ink_soft() },
        ))
}

fn send_button(ready: bool, cx: &mut Context<GlassPreview>) -> Stateful<Div> {
    div()
        .id("glass-send")
        .size(px(32.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(if ready { brand() } else { tint(0x14) })
        .shadow(if ready { brand_glow() } else { soft_shadow() })
        .when(ready, |this| {
            this.cursor_pointer()
                .hover(|style| style.bg(brand_hover()))
                .active(|style| style.bg(brand_press()))
                .tooltip(hint("Send · Enter"))
        })
        .when(!ready, |this| this.tooltip(hint("Type a message first")))
        .on_click(cx.listener(|this, _event, window, cx| {
            if !this.draft.trim().is_empty() {
                this.send_draft(window, cx);
            }
        }))
        .child(icon(
            "arrow-up",
            14.0,
            if ready { brand_ink() } else { ink_muted() },
        ))
}

// ------------------------------------------------------------- session cards

fn drawer_header(
    section: DrawerSection,
    changes: &[ChangeFile],
    pinned: bool,
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
        .child(soft_chip(section_chip(section, changes)))
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
        // The label already reads Pin/Pinned; a stateful tooltip would keep
        // showing the old action after the click.
        .on_click(cx.listener(|this, _event, _window, cx| {
            let next = !this.nav_pinned;
            this.set_pinned(next, cx);
            cx.notify();
        }))
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
            if event.is_held {
                return;
            }
            match event.keystroke.key.as_str() {
                "enter" | "space" => {
                    let next = !this.nav_pinned;
                    this.set_pinned(next, cx);
                    cx.notify();
                }
                // With the model menu open, Escape belongs to it: the root
                // handler closes the menu, and the drawer stays put.
                "escape" if !this.model_menu => {
                    this.close_nav(cx);
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
            this.close_nav(cx);
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

fn changed_files_card(
    files: &'static [ChangeFile],
    selected: &'static str,
    show_open_diff: bool,
    applied: bool,
    cx: &mut Context<GlassPreview>,
) -> Div {
    let (added, removed) = change_totals(files);
    let rows: Vec<AnyElement> = files
        .iter()
        .map(|file| file_change_row(file, file.name == selected, cx).into_any_element())
        .collect();
    panel_card()
        .flex_shrink_0()
        .p(px(12.0))
        .pr(px(DRAWER_CARD_RIGHT))
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
                .when(!files.is_empty(), |this| {
                    this.child(
                        div()
                            .font_family(theme::code_font())
                            .text_size(px(11.0))
                            .text_color(ink_muted())
                            .child(format!("+{added} −{removed}")),
                    )
                }),
        )
        .children(rows)
        .when(files.is_empty(), |this| {
            this.child(
                div()
                    .py(px(8.0))
                    .text_size(px(11.0))
                    .text_color(ink_muted())
                    .child("No files changed in this chat."),
            )
        })
        .when(!files.is_empty(), |this| {
            this.child(
                div()
                    .flex()
                    .items_center()
                    .pt(px(2.0))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(ink_muted())
                            .child(if applied {
                                "Applied to your worktree"
                            } else {
                                "Nothing applied yet"
                            }),
                    )
                    .child(div().flex_1())
                    .when(show_open_diff, |this| {
                        this.child(link_button("glass-open-diff", "Open full diff").on_click(
                            cx.listener(|this, _event, _window, cx| {
                                this.open_section(DrawerSection::Changes, cx);
                                cx.notify();
                            }),
                        ))
                    }),
            )
        })
}

fn file_change_row(
    file: &'static ChangeFile,
    active: bool,
    cx: &mut Context<GlassPreview>,
) -> Stateful<Div> {
    let name = file.name;
    div()
        .id(file.name)
        .flex()
        .items_center()
        .gap(px(9.0))
        .h(px(36.0))
        // Rows share the card's spare height on tall windows.
        .flex_grow()
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
        .on_click(cx.listener(move |this, _event, _window, cx| {
            this.selected_change = name;
            // The matching hunk sits at the top of Changes; bring it into view
            // even when the file list itself is scrolled down.
            this.drawer_scroll.set_offset(point(px(0.0), px(0.0)));
            if this.section != DrawerSection::Changes {
                this.open_section(DrawerSection::Changes, cx);
            }
            cx.notify();
        }))
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
                        .child(file.name),
                )
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(ink_muted())
                        .truncate()
                        .child(file.dir),
                ),
        )
        .child(
            div()
                .font_family(theme::code_font())
                .text_size(px(11.0))
                .text_color(success())
                .child(format!("+{}", file.added)),
        )
        .child(
            div()
                .font_family(theme::code_font())
                .text_size(px(11.0))
                .text_color(danger())
                .child(format!("−{}", file.removed)),
        )
}

fn checks_card(has_run: bool, show_warnings: bool, cx: &mut Context<GlassPreview>) -> Div {
    let card = panel_card()
        .flex_shrink_0()
        .p(px(12.0))
        .pr(px(DRAWER_CARD_RIGHT))
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
                .child(soft_chip(if has_run { "just ran" } else { "not run" })),
        );
    if !has_run {
        return card.child(
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
                        .bg(tint(0x14))
                        .child(icon("minus", 10.0, ink_muted())),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .text_color(ink_soft())
                        .child("No checks yet"),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(ink_muted())
                        .child("Run a task first"),
                ),
        );
    }
    card.child(check_stat_row("Builds clean", "2.1s"))
        .child(check_stat_row("All tests pass", "0.8s"))
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
                if this.show_warnings {
                    this.scroll_to_warnings(cx);
                }
                cx.notify();
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

fn editor_card(file: EditorFile, cx: &mut Context<GlassPreview>) -> Div {
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
                .pr(px(DRAWER_CARD_RIGHT))
                .border_b_1()
                .border_color(panel_border_soft())
                .child(editor_tab(EditorFile::Theme, file == EditorFile::Theme, cx))
                .child(editor_tab(
                    EditorFile::Widgets,
                    file == EditorFile::Widgets,
                    cx,
                ))
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
        .child(editor_body(file))
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
                        // The pane shows a short excerpt, not the whole file,
                        // so it must not claim a total line count.
                        .child("Read-only excerpt"),
                ),
        )
}

fn editor_body(file: EditorFile) -> Div {
    let body = div().p(px(10.0)).flex().flex_col();
    match file {
        EditorFile::Theme => body
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
        EditorFile::Widgets => body
            .child(code_line("1", &[cm("//! Glass button tokens.")]))
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
                    ty("BUTTON_H"),
                    p(": "),
                    ty("f32"),
                    p(" = "),
                    nu("32.0"),
                    p(";"),
                ],
            ))
            .child(code_line(
                "6",
                &[
                    kw("pub const "),
                    ty("BUTTON_R"),
                    p(": "),
                    ty("f32"),
                    p(" = "),
                    nu("999.0"),
                    p(";"),
                ],
            ))
            .child(code_line("7", &[]))
            .child(code_line(
                "8",
                &[
                    kw("pub fn "),
                    fx("primary_fill"),
                    p("() -> "),
                    ty("Rgba"),
                    p(" {"),
                ],
            ))
            .child(code_line(
                "9",
                &[p("    "), fx("rgb"), p("("), nu("0xFDCC02"), p(")")],
            ))
            .child(code_line("10", &[p("}")]))
            .child(code_line("11", &[]))
            .child(code_line(
                "12",
                &[
                    kw("pub fn "),
                    fx("primary_ink"),
                    p("() -> "),
                    ty("Rgba"),
                    p(" {"),
                ],
            ))
            .child(code_line(
                "13",
                &[p("    "), fx("rgb"), p("("), nu("0x1A1700"), p(")")],
            ))
            .child(code_line("14", &[p("}")]))
            .child(code_line("15", &[]))
            .child(code_line(
                "16",
                &[
                    kw("pub fn "),
                    fx("quiet_fill"),
                    p("() -> "),
                    ty("Rgba"),
                    p(" {"),
                ],
            ))
            .child(code_line(
                "17",
                &[p("    "), fx("rgba"), p("("), nu("0xFFFFFF14"), p(")")],
            ))
            .child(code_line("18", &[p("}")])),
    }
}

fn editor_tab(file: EditorFile, active: bool, cx: &mut Context<GlassPreview>) -> Stateful<Div> {
    div()
        .id(file.title())
        .h(px(26.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .gap(px(5.0))
        .rounded(px(6.0))
        .cursor_pointer()
        .when(active, |this| this.bg(tint(0x10)))
        .when(!active, |this| this.hover(|style| style.bg(hover_wash())))
        .on_click(cx.listener(move |this, _event, _window, cx| {
            this.editor_file = file;
            cx.notify();
        }))
        .child(icon(
            "file-code",
            11.0,
            if active { brand() } else { ink_muted() },
        ))
        .child(
            div()
                .text_size(px(11.5))
                .text_color(if active { ink() } else { ink_muted() })
                .child(file.title()),
        )
}

fn outline_card(file: EditorFile) -> Div {
    let rows: Vec<AnyElement> = file
        .outline()
        .iter()
        .map(|(kind, name, line)| outline_row(kind, name, line).into_any_element())
        .collect();
    panel_card()
        .flex_shrink_0()
        .p(px(12.0))
        .pr(px(DRAWER_CARD_RIGHT))
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .flex()
                .items_center()
                .pb(px(4.0))
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ink())
                        .child("Outline"),
                )
                .child(div().flex_1())
                .child(soft_chip(file.title())),
        )
        .children(rows)
}

fn outline_row(kind: &'static str, name: &'static str, line: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .h(px(24.0))
        // Rows share the outline card's spare height on tall windows.
        .flex_grow()
        .px(px(4.0))
        .child(
            div()
                .w(px(30.0))
                .flex_shrink_0()
                .font_family(theme::code_font())
                .text_size(px(9.5))
                .text_color(ink_muted())
                .child(kind),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(12.0))
                .text_color(ink_soft())
                .child(name),
        )
        .child(
            div()
                .font_family(theme::code_font())
                .text_size(px(10.5))
                .text_color(ink_muted())
                .child(line),
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

fn history_card(active_chat: u64, all_runs: bool, cx: &mut Context<GlassPreview>) -> Div {
    let shown = if all_runs { 12 } else { 6 };
    panel_card()
        .flex_shrink_0()
        .p(px(12.0))
        .pr(px(DRAWER_CARD_RIGHT))
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
            Some(0),
            active_chat,
            cx,
        ))
        .child(history_row(
            "glass-hist-2",
            "Glass dock, round two",
            "42m ago · 4 files changed",
            Some(1),
            active_chat,
            cx,
        ))
        .child(history_row(
            "glass-hist-3",
            "Midnight palette pass",
            "2h ago · 9 files changed",
            Some(2),
            active_chat,
            cx,
        ))
        .child(chat_section_label("YESTERDAY"))
        .child(history_row(
            "glass-hist-4",
            "Paper & Ink rollout",
            "21:40 · 12 files changed",
            Some(3),
            active_chat,
            cx,
        ))
        .child(history_row(
            "glass-hist-5",
            "Studio direction",
            "15:15 · 2 files changed",
            Some(4),
            active_chat,
            cx,
        ))
        .child(history_row(
            "glass-hist-6",
            "Obsidian first look",
            "11:02 · no changes",
            Some(5),
            active_chat,
            cx,
        ))
        .when(all_runs, |this| {
            // Archived runs have no chat left in the rail, so they are
            // informational only — no pointer, no click, no chevron.
            this.child(chat_section_label("LAST WEEK"))
                .child(history_row(
                    "glass-hist-7",
                    "Dock spacing pass",
                    "Mon 18:20 · 3 files changed",
                    None,
                    active_chat,
                    cx,
                ))
                .child(history_row(
                    "glass-hist-8",
                    "Context ring math",
                    "Mon 14:05 · 2 files changed",
                    None,
                    active_chat,
                    cx,
                ))
                .child(history_row(
                    "glass-hist-9",
                    "Glass nav overlay",
                    "Sun 16:40 · 7 files changed",
                    None,
                    active_chat,
                    cx,
                ))
                .child(history_row(
                    "glass-hist-10",
                    "Search palette probe",
                    "Sat 11:12 · 5 files changed",
                    None,
                    active_chat,
                    cx,
                ))
                .child(history_row(
                    "glass-hist-11",
                    "Day mode contrast",
                    "Fri 09:30 · 4 files changed",
                    None,
                    active_chat,
                    cx,
                ))
                .child(history_row(
                    "glass-hist-12",
                    "Type scale sweep",
                    "Thu 17:22 · 8 files changed",
                    None,
                    active_chat,
                    cx,
                ))
        })
        .child(
            div()
                .flex()
                .items_center()
                .pt(px(4.0))
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(ink_muted())
                        .child(format!("{shown} of 24 runs")),
                )
                .child(div().flex_1())
                .child(
                    link_button(
                        "glass-open-history",
                        if all_runs {
                            "Show recent"
                        } else {
                            "View more runs"
                        },
                    )
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.all_runs = !this.all_runs;
                        cx.notify();
                    })),
                ),
        )
}

fn run_stats_card() -> Div {
    div().flex_shrink_0().child(
        panel_card()
            .p(px(12.0))
            .pr(px(DRAWER_CARD_RIGHT))
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
                            .child("This week"),
                    )
                    .child(div().flex_1())
                    .child(soft_chip("+18% vs last")),
            )
            .child(
                div()
                    .flex()
                    .gap(px(6.0))
                    .child(stat_tile("24", "Runs"))
                    .child(stat_tile("18", "Applied"))
                    .child(stat_tile("86", "Files")),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .pt(px(2.0))
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(ink_muted())
                            .child("Runs per day"),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(ink_muted())
                            .child("7 days"),
                    ),
            )
            .child(week_chart()),
    )
}

fn stat_tile(value: &'static str, label: &'static str) -> Div {
    div()
        .flex_1()
        .p(px(8.0))
        .rounded(px(10.0))
        .bg(well())
        .border_1()
        .border_color(panel_border_soft())
        .flex()
        .flex_col()
        .gap(px(1.0))
        .child(
            div()
                .text_size(px(16.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ink())
                .child(value),
        )
        .child(
            div()
                .text_size(px(10.0))
                .text_color(ink_muted())
                .child(label),
        )
}

fn week_chart() -> Div {
    const DAYS: [(&str, usize); 7] = [
        ("M", 3),
        ("T", 5),
        ("W", 2),
        ("T", 6),
        ("F", 4),
        ("S", 7),
        ("S", 2),
    ];
    div()
        .h(px(48.0))
        .w_full()
        .flex()
        .items_end()
        .gap(px(6.0))
        .children(DAYS.iter().enumerate().map(|(index, (day, count))| {
            let height = 8.0 + *count as f32 * 4.5;
            div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_end()
                .gap(px(4.0))
                .child(
                    div()
                        .w_full()
                        .h(px(height))
                        .rounded(px(3.0))
                        .bg(if index == 5 { brand() } else { tint(0x24) }),
                )
                .child(div().text_size(px(8.5)).text_color(ink_muted()).child(*day))
        }))
}

fn history_row(
    id: &'static str,
    title: &'static str,
    meta: &'static str,
    chat: Option<u64>,
    active_chat: u64,
    cx: &mut Context<GlassPreview>,
) -> Stateful<Div> {
    let active = chat == Some(active_chat);
    let clickable = chat.is_some();
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
        .when(clickable, |this| this.cursor_pointer())
        .when(clickable && !active, |this| {
            this.hover(|style| style.bg(hover_wash()))
                .active(|style| style.bg(press_wash()))
        })
        .when_some(chat, |this, chat| {
            this.on_click(cx.listener(move |this, _event, window, cx| {
                this.select_chat(chat, window, cx);
            }))
        })
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
        .when(clickable, |this| {
            this.child(icon("chevron-right", 12.0, ink_muted()))
        })
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
                .bg(tint(0x12))
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
                            .bg(bubble())
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
                                     in Session. Use Pin to keep the panel open.",
                                ),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(icon("message-circle", 11.0, ink_muted()))
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(ink_muted())
                        .child("Live chat rides along — open Session to follow the run."),
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

fn status_bar(mode: ExecutionMode, pinned: bool, files: usize) -> Div {
    div()
        .h(px(STATUS))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(16.0))
        .bg(status_fill())
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
                .bg(tint(0x0C))
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
            "Running on this machine",
        ))
        .child(status_chip(
            "status-branch",
            "git-branch",
            "main",
            "Current branch",
        ))
        .child(separator())
        .child(div().size(px(5.0)).rounded_full().bg(success()))
        .child(div().text_color(ink_soft()).child("Everything builds"))
        .child(separator())
        .child(div().child("3 minor warnings"))
        .child(div().flex_1())
        .child(div().child(if files > 0 {
            format!("{files} files changed")
        } else {
            "No files yet".to_string()
        }))
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
        .bg(tint(0x0A))
        .border_1()
        .border_color(panel_border_soft())
        .tooltip(hint(tip))
        .child(icon(name, 11.0, ink_muted()))
        .child(div().text_color(ink_soft()).child(label))
}

#[derive(Clone, Copy)]
struct EdgeState {
    open: bool,
    epoch: usize,
    pinned: bool,
    section: DrawerSection,
    section_epoch: usize,
    show_warnings: bool,
    editor_file: EditorFile,
    selected_change: &'static str,
    all_runs: bool,
    active_chat: u64,
    changes: &'static [ChangeFile],
    applied: bool,
    live: bool,
}

fn edge_overlay(
    state: EdgeState,
    focus: &FocusHandle,
    scroll: &ScrollHandle,
    cx: &mut Context<GlassPreview>,
) -> impl IntoElement {
    let EdgeState { open, epoch, .. } = state;
    div()
        .id("glass-edge-zone")
        .absolute()
        .top_0()
        .bottom(px(8.0))
        .right(px(EDGE_INSET))
        .on_hover(cx.listener(|this, hovered, _window, cx| {
            if *hovered {
                // The nav is always on screen; only its icons open a section.
                this.nav_hovered = true;
            } else {
                this.nav_hovered = false;
                if this.nav_open && !this.nav_pinned {
                    this.set_nav_open(false, cx);
                    cx.notify();
                }
            }
        }))
        .child(edge_drawer(state, focus, scroll, cx))
        .child(nav_column(state, focus, cx))
        .with_animation(
            ("glass-edge", epoch),
            Animation::new(motion(320)).with_easing(ease_out_quint()),
            move |el, t| {
                let k = edge_reveal(open, epoch, t);
                el.w(px(NAV_INSET + NAV_W + (DRAWER_W - NAV_INSET - NAV_W) * k))
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
    scroll: &ScrollHandle,
    cx: &mut Context<GlassPreview>,
) -> impl IntoElement {
    let EdgeState {
        open,
        epoch,
        pinned,
        section,
        section_epoch,
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
        .border_color(if pinned { brand_line() } else { tint(0x2E) })
        .shadow(glass_shadow())
        .child(
            // Content keeps its width while the panel's left edge sweeps out,
            // so labels never reflow mid-animation. It runs full-bleed under
            // the nav — the nav is the top layer, not a reserved gutter.
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .right_0()
                .w(px(DRAWER_W))
                .flex()
                .flex_col()
                .gap(px(10.0))
                .p(px(12.0))
                .child(drawer_header(section, state.changes, pinned, focus, cx))
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
                                .track_scroll(scroll)
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .pt(px(12.0))
                                        .flex_grow()
                                        .flex_shrink_0()
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
                                linear_color_stop(drawer_top_fade(0xEB), 0.0),
                                linear_color_stop(drawer_top_fade(0x00), 1.0),
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
                                    linear_color_stop(drawer_bottom_fade(0x00), 0.0),
                                    linear_color_stop(drawer_bottom_fade(0xF5), 1.0),
                                )),
                        ),
                )
                .when(section == DrawerSection::Session && state.live, |this| {
                    this.child(live_chat_panel())
                }),
        )
        .with_animation(
            ("glass-drawer", epoch),
            Animation::new(motion(320)).with_easing(ease_out_quint()),
            move |el, t| {
                let k = edge_reveal(open, epoch, t);
                el.w(px(DRAWER_W * k)).opacity(k)
            },
        )
}

fn section_body(state: EdgeState, cx: &mut Context<GlassPreview>) -> Div {
    let file = state
        .changes
        .iter()
        .find(|file| file.name == state.selected_change)
        .or_else(|| state.changes.first());
    let body = match state.section {
        DrawerSection::Session => div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(changed_files_card(
                state.changes,
                state.selected_change,
                true,
                state.applied,
                cx,
            ))
            .child(checks_card(
                !state.changes.is_empty(),
                state.show_warnings,
                cx,
            )),
        DrawerSection::Changes => div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .when_some(file, |this, file| {
                this.child(diff_preview(file.name, file.dir, file.added, file.removed))
            })
            .when(file.is_none(), |this| this.child(no_changes_card()))
            // The file list closes the panel; its footer sits on the card's
            // bottom edge when the window leaves spare height.
            .when(!state.changes.is_empty(), |this| {
                this.child(
                    changed_files_card(
                        state.changes,
                        state.selected_change,
                        false,
                        state.applied,
                        cx,
                    )
                    .flex_grow(),
                )
            }),
        DrawerSection::Editor => div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(editor_card(state.editor_file, cx))
            .child(outline_card(state.editor_file).flex_grow()),
        DrawerSection::History => div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(history_card(state.active_chat, state.all_runs, cx))
            .child(run_stats_card().flex_grow()),
    };
    // The scroll content fills the viewport when it is short, so the section
    // can absorb spare height instead of ending in a band of empty drawer.
    body.flex_grow().flex_shrink_0()
}

fn no_changes_card() -> Div {
    panel_card()
        .flex_shrink_0()
        .p(px(16.0))
        .pr(px(DRAWER_CARD_RIGHT))
        .flex()
        .flex_col()
        .items_center()
        .gap(px(6.0))
        .child(icon("files", 18.0, ink_muted()))
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(ink_soft())
                .child("Nothing to diff yet"),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(ink_muted())
                .child("This chat has not produced any changes."),
        )
}

fn context_usage(files: usize) -> f32 {
    match files {
        0 => 0.0,
        // The workbench run is the one the intro cards describe.
        6 => 0.62,
        count => (0.06 + count as f32 * 0.065).min(0.9),
    }
}

fn nav_column(state: EdgeState, focus: &FocusHandle, cx: &mut Context<GlassPreview>) -> Div {
    let EdgeState { section, .. } = state;
    div()
        .absolute()
        .top_0()
        .bottom_0()
        .right(px(NAV_INSET))
        .w(px(NAV_W))
        // Icons float over the drawer; clicks on the rail must not also hit the
        // rows underneath it, while the wheel still scrolls the panel.
        .block_mouse_except_scroll()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(14.0))
        .child(nav_capsule(section, focus, cx))
        .child(context_indicator(state.changes.len()))
}

fn context_indicator(files: usize) -> Stateful<Div> {
    let used = context_usage(files);
    div()
        .id("glass-context")
        .size(px(CONTEXT_SIZE))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(nav_fill())
        .border_1()
        .border_color(nav_border())
        .shadow(glass_shadow())
        .tooltip(hint(if files == 0 {
            "No context used yet"
        } else {
            "Context window used by this chat"
        }))
        .child(context_ring(used))
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
        .border_color(nav_border())
        .shadow(glass_shadow())
        .cursor_pointer()
        .track_focus(focus)
        .focus(focus_ring_neutral)
        .on_click(cx.listener(|this, _event, window, cx| {
            window.focus(&this.nav_focus);
            let section = this.section;
            this.open_section(section, cx);
            cx.notify();
        }))
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
            if event.is_held {
                return;
            }
            match event.keystroke.key.as_str() {
                // With the model menu open, Escape belongs to it: the root
                // handler closes the menu, and the drawer stays put.
                "escape" if !this.model_menu => this.close_nav(cx),
                "enter" | "space" => {
                    let section = this.section;
                    this.open_section(section, cx);
                    window.focus(&this.nav_focus);
                }
                "up" | "left" => {
                    let next = this.section.cycle(false);
                    this.open_section(next, cx);
                }
                "down" | "right" => {
                    let next = this.section.cycle(true);
                    this.open_section(next, cx);
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
            this.child(active_halo())
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
            this.open_section(section, cx);
            cx.notify();
        }))
        .on_click(cx.listener(move |this, _event, window, cx| {
            cx.stop_propagation();
            this.open_section(section, cx);
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

fn context_ring(used: f32) -> impl IntoElement {
    div()
        .size(px(CONTEXT_RING))
        .flex_shrink_0()
        .relative()
        .child(
            canvas(
                |_bounds, _window, _cx| (),
                move |bounds, _prepaint, window, _cx| {
                    let stroke = px(3.0);
                    let center = bounds.center();
                    let radius = (bounds.size.width - stroke) / 2.0;
                    let top = point(center.x, center.y - radius);
                    let bottom = point(center.x, center.y + radius);
                    let mut track = PathBuilder::stroke(stroke);
                    track.move_to(top);
                    track.arc_to(point(radius, radius), px(0.0), false, true, bottom);
                    track.arc_to(point(radius, radius), px(0.0), false, true, top);
                    if let Ok(path) = track.build() {
                        window.paint_path(path, tint(0x24));
                    }
                    let end_angle = -FRAC_PI_2 + TAU * used;
                    let end = point(
                        center.x + radius * end_angle.cos(),
                        center.y + radius * end_angle.sin(),
                    );
                    let mut arc = PathBuilder::stroke(stroke);
                    arc.move_to(top);
                    arc.arc_to(point(radius, radius), px(0.0), used > 0.5, true, end);
                    if let Ok(path) = arc.build() {
                        window.paint_path(path, brand());
                    }
                },
            )
            .size_full(),
        )
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_size(px(10.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(if used > 0.0 {
                            brand_text()
                        } else {
                            ink_muted()
                        })
                        .child(format!("{}%", (used * 100.0).round() as i32)),
                ),
        )
}

fn active_halo() -> Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .rounded_full()
        .bg(rgba(0xFDCC022E))
        .opacity(0.5)
}

// ------------------------------------------------------------------- pieces

fn bare_icon(
    id: &'static str,
    name: &'static str,
    label: &'static str,
    action: impl Fn(&mut GlassPreview, &mut Window, &mut Context<GlassPreview>) + 'static,
    cx: &mut Context<GlassPreview>,
) -> Stateful<Div> {
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
        .on_click(cx.listener(move |this, _event, window, cx| {
            action(this, window, cx);
            cx.notify();
        }))
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
        .bg(tint(0x12))
        .text_size(px(10.0))
        .text_color(ink_muted())
        .child(label)
}

fn soft_chip(label: impl Into<String>) -> Div {
    div()
        .h(px(22.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .rounded(px(6.0))
        .bg(tint(0x0E))
        .text_size(px(11.0))
        .text_color(ink_soft())
        .child(label.into())
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
                .text_color(brand_text())
                .child(label),
        )
        .child(icon("chevron-right", 11.0, brand_text()))
}

fn separator() -> Div {
    div().w(px(1.0)).h(px(12.0)).flex_shrink_0().bg(tint(0x1F))
}

fn apply_button(applied: bool, cx: &mut Context<GlassPreview>) -> Stateful<Div> {
    div()
        .id("glass-apply")
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .h(px(32.0))
        .px(px(14.0))
        .rounded_full()
        .bg(if applied { success_tint() } else { brand() })
        .border_1()
        .border_color(if applied {
            success_line()
        } else {
            rgba(0x00000000)
        })
        .shadow(if applied { soft_shadow() } else { brand_glow() })
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(if applied { success() } else { brand_ink() })
        .when(applied, |this| this.tooltip(hint("Changes applied")))
        .when(!applied, |this| {
            this.cursor_pointer()
                .hover(|style| style.bg(brand_hover()))
                .active(|style| style.bg(brand_press()))
                .on_click(cx.listener(|this, _event, _window, cx| {
                    if !this.applied_chats.contains(&this.active_chat) {
                        this.applied_chats.push(this.active_chat);
                        cx.notify();
                    }
                }))
        })
        .child(icon(
            "check",
            12.0,
            if applied { success() } else { brand_ink() },
        ))
        .child(if applied { "Applied" } else { "Apply changes" })
}

fn undo_button(cx: &mut Context<GlassPreview>) -> Stateful<Div> {
    div()
        .id("glass-undo")
        .flex()
        .items_center()
        .justify_center()
        .gap(px(5.0))
        .h(px(32.0))
        .px(px(12.0))
        .rounded_full()
        .bg(raised())
        .border_1()
        .border_color(panel_border())
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(ink_soft())
        .cursor_pointer()
        .hover(|style| style.bg(hover_wash()))
        .active(|style| style.bg(press_wash()))
        .tooltip(hint("Undo the applied changes"))
        .on_click(cx.listener(|this, _event, _window, cx| {
            let active = this.active_chat;
            this.applied_chats.retain(|chat| *chat != active);
            cx.notify();
        }))
        .child(icon("undo-2", 12.0, ink_soft()))
        .child("Undo")
}

fn review_button(cx: &mut Context<GlassPreview>) -> Stateful<Div> {
    div()
        .id("glass-review")
        .flex()
        .items_center()
        .justify_center()
        .h(px(32.0))
        .px(px(14.0))
        .rounded_full()
        .bg(raised())
        .border_1()
        .border_color(panel_border())
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(ink_soft())
        .cursor_pointer()
        .hover(|style| style.bg(hover_wash()))
        .active(|style| style.bg(press_wash()))
        .tooltip(hint("Open the diff in Changes"))
        .on_click(cx.listener(|this, _event, _window, cx| {
            this.reveal_section(DrawerSection::Changes, true, cx);
            cx.notify();
        }))
        .child("Review diff")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_preferences_round_trip_and_tolerate_noise() {
        assert_eq!(parse_theme(format_theme(true)), Some(true));
        assert_eq!(parse_theme(format_theme(false)), Some(false));
        assert_eq!(parse_theme("theme=day\r\n"), Some(true));
        assert_eq!(parse_theme("  theme = night  "), Some(false));
        assert_eq!(parse_theme("sidebar=wide\ntheme=day\n"), Some(true));
        assert_eq!(parse_theme("theme=sepia"), None);
        assert_eq!(parse_theme(""), None);
        assert_eq!(parse_theme("theme"), None);
        // A malformed theme line must not mask an unambiguous one below it.
        assert_eq!(parse_theme("theme=nope\ntheme=night\n"), Some(false));
    }

    #[test]
    fn empty_search_lists_recent_entries_only() {
        let hits = filter_entries(SEARCH_ENTRIES, "");
        assert!(!hits.is_empty());
        assert!(hits.iter().all(|entry| entry.recent));
        assert_eq!(
            hits.len(),
            SEARCH_ENTRIES.iter().filter(|e| e.recent).count()
        );
        assert_eq!(hits.len(), filter_entries(SEARCH_ENTRIES, "   ").len());
    }

    #[test]
    fn search_matches_title_or_meta_and_groups_by_rank() {
        // Two editor-file rows, then the three panels whose blurbs mention files.
        let files = filter_entries(SEARCH_ENTRIES, "FILE");
        assert_eq!(files.len(), 5);
        assert!(files[..2].iter().all(|entry| entry.action.rank() == 1));
        assert!(files[2..].iter().all(|entry| entry.action.rank() == 2));

        let appearance = filter_entries(SEARCH_ENTRIES, "appearance");
        assert_eq!(appearance.len(), 1);
        assert!(matches!(appearance[0].action, SearchAction::Theme));

        let tokens = filter_entries(SEARCH_ENTRIES, "token counts");
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].action, SearchAction::Tokens));

        let changes = filter_entries(SEARCH_ENTRIES, "  CHANGES  ");
        assert_eq!(changes.len(), 1);
        assert!(matches!(
            changes[0].action,
            SearchAction::Section(DrawerSection::Changes)
        ));

        let panels = filter_entries(SEARCH_ENTRIES, "panel");
        assert!(panels.len() >= 3);
        let mut ranks = panels.iter().map(|entry| entry.action.rank());
        let mut previous = ranks.next().unwrap();
        for rank in ranks {
            assert!(rank >= previous);
            previous = rank;
        }

        assert!(filter_entries(SEARCH_ENTRIES, "zzz-no-match").is_empty());
    }

    #[test]
    fn search_matches_every_word_across_punctuation() {
        let theme = filter_entries(SEARCH_ENTRIES, "day night");
        assert_eq!(theme.len(), 1);
        assert!(matches!(theme[0].action, SearchAction::Theme));

        let pin = filter_entries(SEARCH_ENTRIES, "pin panel");
        assert_eq!(pin.len(), 1);
        assert!(matches!(pin[0].action, SearchAction::Pin));

        assert!(filter_entries(SEARCH_ENTRIES, "day zzz").is_empty());
    }

    #[test]
    fn search_hides_diffs_the_chat_never_produced() {
        assert!(search_results(SEARCH_ENTRIES, "left_dock", 0).is_empty());
        let hits = search_results(SEARCH_ENTRIES, "left_dock", 6);
        assert_eq!(hits.len(), 1);
        assert!(matches!(
            hits[0].action,
            SearchAction::Change("left_dock.rs")
        ));

        // The recent list must not offer a diff row to a chat without diffs.
        assert!(search_results(SEARCH_ENTRIES, "", 0)
            .iter()
            .all(|entry| !matches!(entry.action, SearchAction::Change(_))));
    }

    #[test]
    fn drawer_reveal_tracks_animation_and_ignores_pre_animation_epoch() {
        assert_eq!(edge_reveal(false, 0, 0.0), 0.0);
        assert_eq!(edge_reveal(true, 0, 1.0), 0.0);
        assert_eq!(edge_reveal(true, 1, 0.0), 0.0);
        assert_eq!(edge_reveal(true, 1, 1.0), 1.0);
        assert_eq!(edge_reveal(false, 1, 1.0), 0.0);
        assert_eq!(edge_reveal(false, 1, 0.0), 1.0);
        assert!((edge_reveal(true, 2, 0.25) - 0.25).abs() < f32::EPSILON);
        assert!((edge_reveal(false, 2, 0.25) - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn ellipsize_marks_only_trimmed_labels() {
        assert_eq!(ellipsize("short.rs", 12), "short.rs");
        assert_eq!(ellipsize("exact-fit", 9), "exact-fit");
        assert_eq!(ellipsize("design-preview-glass.png", 14), "design-preview…");
        assert_eq!(ellipsize("éééé", 2), "éé…");
    }

    #[test]
    fn history_rows_open_the_chat_with_the_same_title() {
        let chats = seeded_chats();
        for (id, title) in [
            (0, "Restyle the workbench"),
            (1, "Glass dock, round two"),
            (2, "Midnight palette pass"),
            (3, "Paper & Ink rollout"),
            (4, "Studio direction"),
            (5, "Obsidian first look"),
        ] {
            let chat = chats
                .iter()
                .find(|chat| chat.id == id)
                .expect("history row should point at a seeded chat");
            assert_eq!(chat.title, title);
        }
    }
}
