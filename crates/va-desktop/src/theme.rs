//! Paper & Ink semantic tokens for the Vibe Assembly workbench.
//!
//! Warm paper surfaces, ink text, and a single dark-yellow brand reserved for
//! chrome, primary actions, and active markers. Syntax colors never use brand.
//! Panes are separated by warm hairlines; content sits on raised white cards.

use std::sync::OnceLock;

use gpui::{hsla, rgb, App, Hsla, Rgba};
use va_core::modes::ExecutionMode;

// ------------------------------------------------------------------ surfaces

/// Titlebar, left dock, status bar — the warm canvas behind every pane.
pub fn sunken() -> Rgba {
    rgb(0xF1EADC)
}

/// Conversation and editor canvas.
pub fn editor() -> Rgba {
    rgb(0xFAF6EC)
}

/// Raised cards: messages, dialogs, composer.
pub fn elevated() -> Rgba {
    rgb(0xFFFFFF)
}

/// Pressed and hover rows, meters, and quiet chips.
pub fn elevated_alt() -> Rgba {
    rgb(0xEBE3D3)
}

/// Structural hairline border.
pub fn border() -> Rgba {
    rgb(0xE4DBCB)
}

/// Stronger border for focused controls and floating surfaces.
pub fn border_strong() -> Rgba {
    rgb(0xD1C5B0)
}

// -------------------------------------------------------------------- brand

pub fn brand() -> Rgba {
    rgb(0xFDCC02)
}

pub fn brand_hover() -> Rgba {
    rgb(0xFFD83D)
}

pub fn brand_press() -> Rgba {
    rgb(0xE0B400)
}

/// Readable dark gold for brand-tinted text and active markers.
pub fn brand_ink() -> Rgba {
    rgb(0x7A5F00)
}

/// Soft brand wash for active rows and quiet chips.
pub fn brand_soft() -> Rgba {
    rgb(0xFFF3C4)
}

// -------------------------------------------------------------------- text

pub fn text_primary() -> Rgba {
    rgb(0x2B2620)
}

pub fn text_secondary() -> Rgba {
    rgb(0x5F584C)
}

pub fn text_muted() -> Rgba {
    rgb(0x746B5C)
}

// -------------------------------------------------------------------- diff

pub fn diff_add_bg() -> Rgba {
    rgb(0xE4F2E6)
}

pub fn diff_add_text() -> Rgba {
    rgb(0x2F7D4F)
}

pub fn diff_delete_bg() -> Rgba {
    rgb(0xF8E5E2)
}

pub fn diff_delete_text() -> Rgba {
    rgb(0xA33B2E)
}

// ------------------------------------------------------------------ accents

/// Mode accent used for borders, pills, and active indicators. Never syntax.
pub fn accent_rgb(mode: ExecutionMode) -> u32 {
    match mode {
        ExecutionMode::Code => 0x007A_5F00,
        ExecutionMode::Web => 0x000F_6E7D,
        ExecutionMode::Chat => 0x005B_4DBF,
        ExecutionMode::Debug => 0x00A3_3B2E,
    }
}

/// Solid accents stay identical — brand is chrome, not a gradient.
pub fn accent_strong_rgb(mode: ExecutionMode) -> u32 {
    accent_rgb(mode)
}

pub fn accent_for(mode: ExecutionMode) -> Rgba {
    rgb(accent_rgb(mode))
}

pub fn accent_strong_for(mode: ExecutionMode) -> Rgba {
    rgb(accent_strong_rgb(mode))
}

#[cfg(test)]
pub fn accent_hex(mode: ExecutionMode) -> &'static str {
    match mode {
        ExecutionMode::Code => "#7A5F00",
        ExecutionMode::Web => "#0F6E7D",
        ExecutionMode::Chat => "#5B4DBF",
        ExecutionMode::Debug => "#A33B2E",
    }
}

#[cfg(test)]
pub fn accent_strong_hex(mode: ExecutionMode) -> &'static str {
    accent_hex(mode)
}

// --------------------------------------------------------------- status hues

pub fn success() -> Rgba {
    rgb(0x2F7D4F)
}

pub fn warning() -> Rgba {
    rgb(0xA65F00)
}

pub fn danger() -> Rgba {
    rgb(0xA33B2E)
}

pub fn danger_soft() -> Rgba {
    rgb(0xF8E5E2)
}

pub fn info() -> Rgba {
    rgb(0x5B4DBF)
}

pub fn purple() -> Rgba {
    rgb(0x7C5CB0)
}

pub fn cyan() -> Rgba {
    rgb(0x0F6E7D)
}

pub fn phase_color(phase: &str) -> Rgba {
    match phase {
        "observe" => cyan(),
        "think" => warning(),
        "patch" => success(),
        "verify" => info(),
        "judge" => danger(),
        _ => text_muted(),
    }
}

pub fn symbol_color(kind: &str) -> Rgba {
    match kind {
        "fn" | "method" => success(),
        "struct" | "enum" | "class" | "interface" => warning(),
        "trait" => purple(),
        "impl" => info(),
        "type" | "constant" => cyan(),
        "block" => text_muted(),
        _ => text_secondary(),
    }
}

// ------------------------------------------------------------------- washes

/// Convert an opaque color to Hsla at the requested alpha.
pub fn wash(color: Rgba, alpha: f32) -> Hsla {
    let max = color.r.max(color.g).max(color.b);
    let min = color.r.min(color.g).min(color.b);
    let l = (max + min) / 2.0;
    let d = max - min;
    let s = if d == 0.0 {
        0.0
    } else {
        d / (1.0 - (2.0 * l - 1.0).abs())
    };
    let h = if d == 0.0 {
        0.0
    } else if max == color.r {
        ((color.g - color.b) / d).rem_euclid(6.0) / 6.0
    } else if max == color.g {
        ((color.b - color.r) / d + 2.0) / 6.0
    } else {
        ((color.r - color.g) / d + 4.0) / 6.0
    };
    hsla(h, s, l, alpha)
}

pub const MICRO: f32 = 11.0;
pub const BODY: f32 = 12.5;
pub const HEADING: f32 = 14.0;
pub const ROW: f32 = 28.0;
pub const SIDEBAR: f32 = 260.0;
pub const ASSISTANT: f32 = 430.0;
pub const DRAWER: f32 = 200.0;

static UI_FONT: OnceLock<&str> = OnceLock::new();
static CODE_FONT: OnceLock<&str> = OnceLock::new();

pub fn init_fonts(cx: &App) {
    let fonts = cx.text_system().all_font_names();
    let choose = |choices: &[&'static str], fallback| {
        choices
            .iter()
            .copied()
            .find(|candidate| fonts.iter().any(|font| font == candidate))
            .unwrap_or(fallback)
    };
    let _ = UI_FONT.set(choose(
        &["Segoe UI Variable Display", "Inter", "Segoe UI"],
        ".SystemUIFont",
    ));
    let _ = CODE_FONT.set(choose(
        &["Cascadia Code", "JetBrains Mono", "Consolas"],
        "monospace",
    ));
}

pub fn ui_font() -> &'static str {
    UI_FONT.get().copied().unwrap_or("Segoe UI")
}
pub fn code_font() -> &'static str {
    CODE_FONT.get().copied().unwrap_or("Consolas")
}

pub fn file_color(name: &str) -> Rgba {
    match name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "rs" => rgb(0xB05E2E),
        "toml" | "lock" => rgb(0x6C5BA6),
        "md" => rgb(0x3C6E9E),
        "ts" | "tsx" => rgb(0x2F6DB0),
        "py" => rgb(0x8A6A00),
        "go" => cyan(),
        "js" | "jsx" | "json" => warning(),
        _ => text_secondary(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_tokens_are_exact() {
        assert_eq!(sunken(), rgb(0xF1EADC));
        assert_eq!(editor(), rgb(0xFAF6EC));
        assert_eq!(elevated(), rgb(0xFFFFFF));
        assert_eq!(elevated_alt(), rgb(0xEBE3D3));
        assert_eq!(border(), rgb(0xE4DBCB));
        assert_eq!(text_primary(), rgb(0x2B2620));
        assert_eq!(text_secondary(), rgb(0x5F584C));
        assert_eq!(text_muted(), rgb(0x746B5C));
        assert_eq!(diff_add_bg(), rgb(0xE4F2E6));
        assert_eq!(diff_add_text(), rgb(0x2F7D4F));
        assert_eq!(diff_delete_bg(), rgb(0xF8E5E2));
        assert_eq!(diff_delete_text(), rgb(0xA33B2E));
        assert_eq!(brand(), rgb(0xFDCC02));
        assert_eq!(brand_ink(), rgb(0x7A5F00));
    }

    #[test]
    fn mode_accents_match_the_workbench_palette() {
        assert_eq!(accent_hex(ExecutionMode::Code), "#7A5F00");
        assert_eq!(accent_hex(ExecutionMode::Web), "#0F6E7D");
        assert_eq!(accent_hex(ExecutionMode::Chat), "#5B4DBF");
        assert_eq!(accent_hex(ExecutionMode::Debug), "#A33B2E");

        assert_eq!(accent_strong_hex(ExecutionMode::Code), "#7A5F00");
        assert_eq!(accent_strong_hex(ExecutionMode::Web), "#0F6E7D");
        assert_eq!(accent_strong_hex(ExecutionMode::Chat), "#5B4DBF");
        assert_eq!(accent_strong_hex(ExecutionMode::Debug), "#A33B2E");

        for mode in ExecutionMode::ALL {
            assert_eq!(accent_for(mode), rgb(accent_rgb(mode)));
            assert_eq!(accent_strong_for(mode), rgb(accent_strong_rgb(mode)));
        }
    }
}
