//! Mission Control — a native desktop dashboard for Vibe Assembly.
//!
//! Reads the `.ai/` workspace state from disk and renders it with GPUI, the
//! same UI framework Zed is built on. Zero web view, zero npm runtime.

mod theme;
mod ui;

use std::path::PathBuf;

use va_core::state::Workspace;

fn main() {
    // Discovery order: an explicit argument, then the current directory.
    let start: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

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
