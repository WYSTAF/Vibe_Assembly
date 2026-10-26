//! Obsidian / zinc semantic design tokens for the Vibe Assembly workbench.
//!
//! The palette is deliberately small: four sunken surfaces, one editor surface,
//! two elevated tones, three text weights, two diff tones, and four mode accents.
//! Every border is a 1px hairline; panes are separated by borders, not cards.

use std::sync::OnceLock;

use gpui::{hsla, rgb, App, Hsla, Rgba};
use va_core::modes::ExecutionMode;

// ------------------------------------------------------------------ surfaces

/// Titlebar, activity bar, left dock, assistant dock, bottom drawer, status bar.
pub fn sunken() -> Rgba {
    rgb(0x090A0F)
}

/// Editor and primary canvas.
pub fn editor() -> Rgba {
    rgb(0x0D0F17)
}

/// Active tab, command palette, hover states.
pub fn elevated() -> Rgba {
    rgb(0x131620)
}

/// Composer input and pressed/hover raised rows.
pub fn elevated_alt() -> Rgba {
    rgb(0x1A1E2C)
}

/// Structural hairline border. Also the raised hover tone in Zed.
pub fn border() -> Rgba {
    rgb(0x1E2330)
}

/// Slightly stronger border for focused controls.
pub fn border_strong() -> Rgba {
    rgb(0x2A3142)
}

// -------------------------------------------------------------------- text

pub fn text_primary() -> Rgba {
    rgb(0xE4E7EF)
}

pub fn text_secondary() -> Rgba {
    rgb(0xA1A9BC)
}

pub fn text_muted() -> Rgba {
    rgb(0x737F98)
}

// -------------------------------------------------------------------- diff

pub fn diff_add_bg() -> Rgba {
    rgb(0x0D2521)
}

pub fn diff_add_text() -> Rgba {
    rgb(0x10B981)
}

pub fn diff_delete_bg() -> Rgba {
    rgb(0x2A1522)
}

pub fn diff_delete_text() -> Rgba {
    rgb(0xF43F5E)
}

// ------------------------------------------------------------------ accents

/// Soft mode accent used for borders, pills, and active indicators.
pub fn accent_rgb(mode: ExecutionMode) -> u32 {
    match mode {
        ExecutionMode::Code => 0x00F5_9E0B,
        ExecutionMode::Web => 0x0006_B6D4,
        ExecutionMode::Chat => 0x0063_66F1,
        ExecutionMode::Debug => 0x00F4_3F5E,
    }
}

/// Vivid mode accent used for solid badges and focus glows.
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
        ExecutionMode::Code => "#F59E0B",
        ExecutionMode::Web => "#06B6D4",
        ExecutionMode::Chat => "#6366F1",
        ExecutionMode::Debug => "#F43F5E",
    }
}

#[cfg(test)]
pub fn accent_strong_hex(mode: ExecutionMode) -> &'static str {
    accent_hex(mode)
}

// --------------------------------------------------------------- status hues

pub fn success() -> Rgba {
    rgb(0x10B981)
}

pub fn warning() -> Rgba {
    rgb(0xF59E0B)
}

pub fn danger() -> Rgba {
    rgb(0xF43F5E)
}

pub fn info() -> Rgba {
    rgb(0x6366F1)
}

pub fn purple() -> Rgba {
    rgb(0xA78BFA)
}

pub fn cyan() -> Rgba {
    rgb(0x06B6D4)
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
pub const ROW: f32 = 26.0;
pub const SIDEBAR: f32 = 250.0;
pub const ASSISTANT: f32 = 400.0;
pub const DRAWER: f32 = 170.0;

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
        "rs" => rgb(0xE9A77A),
        "toml" | "lock" => rgb(0xB4A5E8),
        "md" => rgb(0x85B5F5),
        "ts" | "tsx" => rgb(0x60A5FA),
        "py" => rgb(0xF3CE70),
        "go" => cyan(),
        "js" | "jsx" | "json" => warning(),
        _ => text_secondary(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obsidian_tokens_are_exact() {
        assert_eq!(sunken(), rgb(0x090A0F));
        assert_eq!(editor(), rgb(0x0D0F17));
        assert_eq!(elevated(), rgb(0x131620));
        assert_eq!(elevated_alt(), rgb(0x1A1E2C));
        assert_eq!(border(), rgb(0x1E2330));
        assert_eq!(text_primary(), rgb(0xE4E7EF));
        assert_eq!(text_secondary(), rgb(0xA1A9BC));
        assert_eq!(text_muted(), rgb(0x737F98));
        assert_eq!(diff_add_bg(), rgb(0x0D2521));
        assert_eq!(diff_add_text(), rgb(0x10B981));
        assert_eq!(diff_delete_bg(), rgb(0x2A1522));
        assert_eq!(diff_delete_text(), rgb(0xF43F5E));
    }

    #[test]
    fn mode_accents_match_the_workbench_palette() {
        assert_eq!(accent_hex(ExecutionMode::Code), "#F59E0B");
        assert_eq!(accent_hex(ExecutionMode::Web), "#06B6D4");
        assert_eq!(accent_hex(ExecutionMode::Chat), "#6366F1");
        assert_eq!(accent_hex(ExecutionMode::Debug), "#F43F5E");

        assert_eq!(accent_strong_hex(ExecutionMode::Code), "#F59E0B");
        assert_eq!(accent_strong_hex(ExecutionMode::Web), "#06B6D4");
        assert_eq!(accent_strong_hex(ExecutionMode::Chat), "#6366F1");
        assert_eq!(accent_strong_hex(ExecutionMode::Debug), "#F43F5E");

        for mode in ExecutionMode::ALL {
            assert_eq!(accent_for(mode), rgb(accent_rgb(mode)));
            assert_eq!(accent_strong_for(mode), rgb(accent_strong_rgb(mode)));
        }
    }
}
