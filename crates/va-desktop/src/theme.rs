//! Zed One Dark design tokens for the Vibe Assembly workbench.
//!
//! The palette is deliberately small: four sunken surfaces, one editor surface,
//! two elevated tones, three text weights, two diff tones, and four mode accents.
//! Every border is a 1px hairline; panes are separated by borders, not cards.

use gpui::{hsla, rgb, Hsla, Rgba};
use va_core::modes::ExecutionMode;

// ------------------------------------------------------------------ surfaces

/// Titlebar, activity bar, left dock, assistant dock, bottom drawer, status bar.
pub fn sunken() -> Rgba {
    rgb(0x181A1F)
}

/// Editor and primary canvas.
pub fn editor() -> Rgba {
    rgb(0x21252B)
}

/// Active tab, command palette, hover states.
pub fn elevated() -> Rgba {
    rgb(0x282C34)
}

/// Composer input and pressed/hover raised rows.
pub fn elevated_alt() -> Rgba {
    rgb(0x2C313A)
}

/// Structural hairline border. Also the raised hover tone in Zed.
pub fn border() -> Rgba {
    rgb(0x2C313A)
}

/// Slightly stronger border for focused controls.
pub fn border_strong() -> Rgba {
    rgb(0x3E4451)
}

// -------------------------------------------------------------------- text

pub fn text_primary() -> Rgba {
    rgb(0xDCDFE4)
}

pub fn text_secondary() -> Rgba {
    rgb(0xABB2BF)
}

pub fn text_muted() -> Rgba {
    rgb(0x5C6370)
}

// -------------------------------------------------------------------- diff

pub fn diff_add_bg() -> Rgba {
    rgb(0x1E3A29)
}

pub fn diff_add_text() -> Rgba {
    rgb(0x98C379)
}

pub fn diff_delete_bg() -> Rgba {
    rgb(0x3E1F24)
}

pub fn diff_delete_text() -> Rgba {
    rgb(0xE06C75)
}

// ------------------------------------------------------------------ accents

/// Soft mode accent used for borders, pills, and active indicators.
pub fn accent_rgb(mode: ExecutionMode) -> u32 {
    match mode {
        ExecutionMode::Code => 0x00E5_C07B,
        ExecutionMode::Web => 0x0056_B6C2,
        ExecutionMode::Chat => 0x0061_AFEF,
        ExecutionMode::Debug => 0x00E0_6C75,
    }
}

/// Vivid mode accent used for solid badges and focus glows.
pub fn accent_strong_rgb(mode: ExecutionMode) -> u32 {
    mode.accent_rgb()
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
        ExecutionMode::Code => "#E5C07B",
        ExecutionMode::Web => "#56B6C2",
        ExecutionMode::Chat => "#61AFEF",
        ExecutionMode::Debug => "#E06C75",
    }
}

#[cfg(test)]
pub fn accent_strong_hex(mode: ExecutionMode) -> &'static str {
    mode.accent_hex()
}

// --------------------------------------------------------------- status hues

pub fn success() -> Rgba {
    rgb(0x98C379)
}

pub fn warning() -> Rgba {
    rgb(0xE5C07B)
}

pub fn danger() -> Rgba {
    rgb(0xE06C75)
}

pub fn info() -> Rgba {
    rgb(0x61AFEF)
}

pub fn purple() -> Rgba {
    rgb(0xC678DD)
}

pub fn cyan() -> Rgba {
    rgb(0x56B6C2)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zed_one_dark_tokens_are_exact() {
        assert_eq!(sunken(), rgb(0x181A1F));
        assert_eq!(editor(), rgb(0x21252B));
        assert_eq!(elevated(), rgb(0x282C34));
        assert_eq!(elevated_alt(), rgb(0x2C313A));
        assert_eq!(border(), rgb(0x2C313A));
        assert_eq!(text_primary(), rgb(0xDCDFE4));
        assert_eq!(text_secondary(), rgb(0xABB2BF));
        assert_eq!(text_muted(), rgb(0x5C6370));
        assert_eq!(diff_add_bg(), rgb(0x1E3A29));
        assert_eq!(diff_add_text(), rgb(0x98C379));
        assert_eq!(diff_delete_bg(), rgb(0x3E1F24));
        assert_eq!(diff_delete_text(), rgb(0xE06C75));
    }

    #[test]
    fn mode_accents_match_the_workbench_palette() {
        assert_eq!(accent_hex(ExecutionMode::Code), "#E5C07B");
        assert_eq!(accent_hex(ExecutionMode::Web), "#56B6C2");
        assert_eq!(accent_hex(ExecutionMode::Chat), "#61AFEF");
        assert_eq!(accent_hex(ExecutionMode::Debug), "#E06C75");

        assert_eq!(accent_strong_hex(ExecutionMode::Code), "#F5B800");
        assert_eq!(accent_strong_hex(ExecutionMode::Web), "#06B6D4");
        assert_eq!(accent_strong_hex(ExecutionMode::Chat), "#3B82F6");
        assert_eq!(accent_strong_hex(ExecutionMode::Debug), "#EF4444");

        for mode in ExecutionMode::ALL {
            assert_eq!(accent_for(mode), rgb(accent_rgb(mode)));
            assert_eq!(accent_strong_for(mode), rgb(accent_strong_rgb(mode)));
        }
    }
}
