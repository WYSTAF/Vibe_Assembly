//! Obsidian Gold — the brand token set for the Vibe Assembly workbench.
//!
//! Warm black surfaces (a trace of yellow in every grey) so the brand yellow
//! reads as light rather than as a sticker. Nothing here is consumed by the
//! shipped UI yet: the design preview is the only caller until the migration
//! lands.

use gpui::{rgb, Hsla, Rgba};
use va_core::modes::ExecutionMode;

// ------------------------------------------------------------------ surfaces

/// Titlebar, docks, drawer, status bar — the frame around the canvas.
pub fn sunken() -> Rgba {
    rgb(0x0B0B0A)
}

/// Editor and primary canvas.
pub fn editor() -> Rgba {
    rgb(0x101010)
}

/// Raised cards, active tab, palette.
pub fn elevated() -> Rgba {
    rgb(0x161614)
}

/// Inputs, pressed rows, hover on elevated.
pub fn elevated_alt() -> Rgba {
    rgb(0x1D1D1B)
}

/// Structural hairline.
pub fn border() -> Rgba {
    rgb(0x262622)
}

/// Focused control and floating panel edge.
pub fn border_strong() -> Rgba {
    rgb(0x3A3A33)
}

// ---------------------------------------------------------------------- text

pub fn text_primary() -> Rgba {
    rgb(0xF2F0E9)
}

pub fn text_secondary() -> Rgba {
    rgb(0xB5B2A8)
}

pub fn text_muted() -> Rgba {
    rgb(0x928F85)
}

// --------------------------------------------------------------------- brand

/// Sampled from the supplied logo.
pub fn brand() -> Rgba {
    rgb(0xFDCC02)
}

pub fn brand_hover() -> Rgba {
    rgb(0xFFD83D)
}

pub fn brand_press() -> Rgba {
    rgb(0xE6B800)
}

/// Text and icons placed on a solid brand fill.
pub fn on_brand() -> Rgba {
    rgb(0x0B0B0A)
}

// -------------------------------------------------------------------- status

pub fn success() -> Rgba {
    rgb(0x34D399)
}

pub fn warning() -> Rgba {
    rgb(0xFB923C)
}

pub fn danger() -> Rgba {
    rgb(0xF87171)
}

pub fn violet() -> Rgba {
    rgb(0xA78BFA)
}

pub fn cyan() -> Rgba {
    rgb(0x22D3EE)
}

// ---------------------------------------------------------------------- diff

pub fn diff_add_bg() -> Rgba {
    rgb(0x0E1F19)
}

pub fn diff_add_text() -> Rgba {
    rgb(0x4ADE80)
}

pub fn diff_delete_bg() -> Rgba {
    rgb(0x241318)
}

pub fn diff_delete_text() -> Rgba {
    rgb(0xF87171)
}

// --------------------------------------------------------------------- modes

/// Code keeps the brand yellow; the other modes shift hue but not value.
pub fn mode_accent(mode: ExecutionMode) -> Rgba {
    match mode {
        ExecutionMode::Code => brand(),
        ExecutionMode::Web => cyan(),
        ExecutionMode::Chat => violet(),
        ExecutionMode::Debug => rgb(0xFB7185),
    }
}

/// Convert an opaque color to Hsla at the requested alpha.
pub fn wash(color: Rgba, alpha: f32) -> Hsla {
    crate::theme::wash(color, alpha)
}

// ------------------------------------------------------------------- metrics

pub const SPACE_XS: f32 = 4.0;
pub const SPACE_SM: f32 = 6.0;
pub const SPACE_MD: f32 = 8.0;
pub const SPACE_LG: f32 = 12.0;
pub const SPACE_XL: f32 = 16.0;
pub const SPACE_XXL: f32 = 24.0;

pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 6.0;
pub const RADIUS_LG: f32 = 10.0;

pub const HEIGHT_CONTROL: f32 = 26.0;
pub const HEIGHT_ICON_BTN: f32 = 28.0;
pub const HEIGHT_TITLEBAR: f32 = 36.0;
pub const HEIGHT_TAB: f32 = 34.0;
pub const HEIGHT_ROW: f32 = 26.0;
pub const HEIGHT_STATUS: f32 = 26.0;

pub const TYPE_MICRO: f32 = 11.0;
pub const TYPE_BODY: f32 = 12.5;
pub const TYPE_HEADING: f32 = 14.0;

pub const DOCK_WIDTH: f32 = 250.0;
pub const ASSISTANT_WIDTH: f32 = 400.0;
pub const DRAWER_HEIGHT: f32 = 170.0;
