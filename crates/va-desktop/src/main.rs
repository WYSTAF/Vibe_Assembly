//! Mission Control — a native desktop dashboard for Vibe Assembly.
//!
//! Reads the `.ai/` workspace state from disk and renders it with GPUI, the
//! same UI framework Zed is built on. Zero web view, zero npm runtime.

mod theme;
mod ui;

use std::path::PathBuf;

use va_core::state::Workspace;

fn main() {
    // Discovery order: `--root`, a positional path, then the current directory.
    let mut start: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => start = args.next().map(PathBuf::from),
            "--mode" => {
                if let Some(value) = args.next() {
                    std::env::set_var("VIBE_MODE", value);
                }
            }
            "--lang" => {
                if let Some(value) = args.next() {
                    std::env::set_var("VIBE_LANG", value);
                }
            }
            "--model" => {
                if let Some(value) = args.next() {
                    std::env::set_var("VIBE_MODEL", value);
                }
            }
            other if !other.starts_with('-') => start = Some(PathBuf::from(other)),
            _ => {}
        }
    }
    let start =
        start.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    match Workspace::discover(&start) {
        Some(root) => ui::run(Workspace::load(&root), None),
        None => {
            let shown = start.display().to_string();
            ui::run(
                Workspace::load(&start),
                Some(format!("Searched upward from: {shown}")),
            );
        }
    }
}
