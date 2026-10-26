//! Small shared primitives for the workbench. Keeping these in one place is
//! what prevents the Zed layout from turning back into a pile of one-off cards.

use crate::ui_kit::TintedBadge;
use gpui::prelude::*;
use gpui::{div, px, relative, Div, FontWeight, Rgba};

use crate::theme;

/// Eight-character display form of a 64-bit FNV-1a block hash. The patcher's
/// four-character form stays available for CLI-compatible diagnostics.
pub fn hash8(hash: u64) -> String {
    format!("{:08X}", (hash >> 32) as u32)
}

pub fn truncate(text: &str, max: usize) -> String {
    let mut out = String::new();
    for (i, ch) in text.chars().enumerate() {
        if i >= max {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

pub fn first_line(text: &str) -> String {
    truncate(text.lines().next().unwrap_or("").trim(), 96)
}

pub fn section_label(text: &str) -> Div {
    div()
        .text_size(px(theme::MICRO))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme::text_muted())
        .child(text.to_uppercase())
}

pub fn pill(label: impl Into<String>, active: bool, accent: Rgba) -> Div {
    TintedBadge::render(label, if active { accent } else { theme::text_muted() })
}

pub fn ghost_button(label: impl Into<String>) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .px(px(10.0))
        .h(px(26.0))
        .rounded(px(6.0))
        .bg(theme::elevated())
        .text_size(px(theme::BODY))
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme::text_secondary())
        .cursor_pointer()
        .hover(|style| {
            style
                .bg(theme::elevated_alt())
                .text_color(theme::text_primary())
        })
        .child(label.into())
}

pub fn meter(pct: usize, color: Rgba) -> Div {
    let pct = pct.clamp(0, 100);
    let mut track = div()
        .h(px(3.0))
        .w_full()
        .bg(theme::elevated_alt())
        .rounded_full()
        .overflow_hidden();
    if pct > 0 {
        track = track.child(
            div()
                .h_full()
                .w(relative(pct as f32 / 100.0))
                .bg(color)
                .rounded_full(),
        );
    }
    track
}

pub fn hash_badge(hash: u64, color: Rgba) -> Div {
    TintedBadge::render(format!("#{}", hash8(hash)), color).font_family(theme::code_font())
}

pub fn file_glyph(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "rs" => "rs",
        "md" => "md",
        "toml" => "tm",
        "json" | "jsonl" => "{}",
        "svg" => "svg",
        "png" | "jpg" | "jpeg" | "gif" => "img",
        "html" => "htm",
        "css" => "css",
        "ts" | "tsx" => "ts",
        "js" | "jsx" => "js",
        "py" => "py",
        "c" | "h" => "c",
        "cpp" | "cc" | "hpp" => "c++",
        "go" => "go",
        "lock" => "lk",
        "yml" | "yaml" => "yml",
        _ => "·",
    }
}

pub fn head_tail(text: &str, head: usize, tail: usize) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= head.saturating_add(tail).saturating_add(1) {
        return lines.into_iter().map(ToString::to_string).collect();
    }
    let mut out: Vec<String> = lines.iter().take(head).map(|l| (*l).to_string()).collect();
    out.push(format!("… {} lines hidden …", lines.len() - head - tail));
    out.extend(
        lines
            .iter()
            .skip(lines.len() - tail)
            .map(|l| (*l).to_string()),
    );
    out
}

pub fn keycap(label: &str) -> Div {
    div()
        .px_1()
        .py(px(1.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(theme::border())
        .bg(theme::elevated_alt())
        .text_size(px(theme::MICRO))
        .font_family(theme::code_font())
        .text_color(theme::text_muted())
        .child(label.to_string())
}

pub fn dot(color: Rgba) -> Div {
    div().size(px(5.0)).rounded_full().bg(color)
}
