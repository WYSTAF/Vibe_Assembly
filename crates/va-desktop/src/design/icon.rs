//! Lucide icon set, embedded and served through the app asset source.
//!
//! GPUI paints an SVG as an alpha mask in `style.text.color`, so every icon
//! must be given an explicit `.text_color()` or it renders nothing.

use gpui::{px, svg, Rgba, SharedString, Styled, Svg};

macro_rules! icon_set {
    ($($name:literal),* $(,)?) => {
        pub const ICONS: &[(&str, &[u8])] = &[
            $(( $name, include_bytes!(concat!("../../assets/icons/", $name, ".svg")) )),*
        ];
    };
}

icon_set!(
    "arrow-up",
    "braces",
    "check",
    "chevron-down",
    "chevron-left",
    "chevron-right",
    "code",
    "file-code",
    "file-text",
    "files",
    "folder",
    "folder-open",
    "git-branch",
    "globe",
    "history",
    "image",
    "info",
    "lock",
    "message-circle",
    "mic",
    "minus",
    "monitor",
    "moon",
    "more-horizontal",
    "panel-bottom",
    "panel-left",
    "panel-right",
    "pin",
    "play",
    "plus",
    "refresh-cw",
    "scale",
    "search",
    "sparkles",
    "square",
    "sun",
    "triangle-alert",
    "x",
);

const PREFIX: &str = "icon/";
const SUFFIX: &str = ".svg";

pub fn asset(path: &str) -> Option<&'static [u8]> {
    let name = path.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)?;
    ICONS
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, bytes)| *bytes)
}

pub fn list(prefix: &str) -> Vec<SharedString> {
    ICONS
        .iter()
        .map(|(name, _)| format!("{PREFIX}{name}{SUFFIX}"))
        .filter(|path| path.starts_with(prefix))
        .map(SharedString::from)
        .collect()
}

pub fn icon(name: &'static str, size: f32, color: Rgba) -> Svg {
    svg()
        .path(format!("{PREFIX}{name}{SUFFIX}"))
        .size(px(size))
        .flex_shrink_0()
        .text_color(color)
}
