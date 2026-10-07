//! Visual language borrowed from Zed's dark theme: a near-black layered
//! surface, hairline borders, one accent hue, and color reserved almost
//! entirely for *status* so it always carries meaning.

use gpui::{hsla, rgb, Hsla, Rgba};

pub fn surface_0() -> Rgba {
    // Window background — deepest layer.
    rgb(0x0d0e10)
}

pub fn surface_1() -> Rgba {
    // Panels sitting on the background.
    rgb(0x141619)
}

pub fn surface_2() -> Rgba {
    // Raised rows, hover states.
    rgb(0x1b1e22)
}

pub fn border() -> Rgba {
    rgb(0x26292e)
}

pub fn border_strong() -> Rgba {
    rgb(0x363a41)
}

pub fn text_primary() -> Rgba {
    rgb(0xe6e8ec)
}

pub fn text_secondary() -> Rgba {
    rgb(0x9aa0a8)
}

pub fn text_muted() -> Rgba {
    rgb(0x6b7178)
}

pub fn accent() -> Rgba {
    // Zed's blue.
    rgb(0x3b8eea)
}

/// Status colors. These are the only saturated hues in the app, so a red
/// block is never decorative.
pub fn status_ok() -> Rgba {
    rgb(0x3fb950)
}

pub fn status_warn() -> Rgba {
    rgb(0xd29922)
}

pub fn status_error() -> Rgba {
    rgb(0xf85149)
}

pub fn status_idle() -> Rgba {
    rgb(0x6b7178)
}

pub fn status_done() -> Rgba {
    rgb(0x8b949e)
}

/// The status color for a ticket row.
pub fn ticket_status_color(status: &str) -> Rgba {
    match status {
        "done" => status_done(),
        "in_progress" => accent(),
        "blocked" | "failed" => status_error(),
        "pending" => status_warn(),
        _ => status_idle(),
    }
}

/// Convert an opaque status color to Hsla at the given alpha. Rgba has no
/// alpha helper in gpui 0.2.2, so washes go through Hsla.
pub fn wash(color: Rgba, a: f32) -> Hsla {
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
    hsla(h, s, l, a)
}

/// A faint wash of the status color, for row backgrounds.
pub fn status_wash(status: &str) -> Hsla {
    wash(ticket_status_color(status), 0.14)
}

/// A fainter wash, for panel headers and inline chips.
pub fn status_wash_faint(status: &str) -> Hsla {
    wash(ticket_status_color(status), 0.10)
}
