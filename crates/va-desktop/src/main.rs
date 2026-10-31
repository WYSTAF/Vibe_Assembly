//! Vibe Assembly desktop — a native Zed-style workbench.
//!
//! The workbench reads the real `.ai/` workspace, the RepoMap, the immutable
//! session DAG, compiler diagnostics, and the eight harness surfaces. It uses
//! GPUI directly: no web view, no npm runtime, no Electron shell.

mod assistant_panel;
mod bottom_drawer;
mod branding;
mod command_palette;
mod design;
mod editor_pane;
mod left_dock;
mod state;
mod theme;
mod ui_kit;
mod widgets;

use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{
    px, size, App, Application, TitlebarOptions, WindowBackgroundAppearance, WindowBounds,
    WindowOptions,
};
use va_core::modes::{ExecutionMode, LanguageLens};
use va_core::state::Workspace;

use crate::state::{Workbench, WorkbenchState};

fn main() {
    // Discovery order: `--root`, a positional path, then the current directory.
    let mut start: Option<PathBuf> = None;
    let mut sample: Option<Sample> = None;
    let mut workbench = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--design-preview" => sample = Some(Sample::Obsidian),
            "--design-preview-friendly" => sample = Some(Sample::Paper),
            "--design-preview-midnight" => sample = Some(Sample::Midnight),
            "--design-preview-studio" => sample = Some(Sample::Studio),
            "--design-preview-glass" => sample = Some(Sample::Glass),
            "--workbench" => workbench = true,
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

    let mode = std::env::var("VIBE_MODE")
        .ok()
        .and_then(|value| value.parse::<ExecutionMode>().ok())
        .unwrap_or(ExecutionMode::Code);

    if let Some(sample) = sample {
        run_preview(mode, sample);
        return;
    }

    // A bare launch opens the Liquid Glass surface; the workspace workbench
    // needs an explicit `--workbench` or a root path because it requires a repo.
    if !workbench && start.is_none() {
        run_preview_shipped(mode);
        return;
    }

    let language = std::env::var("VIBE_LANG")
        .ok()
        .and_then(|value| value.parse::<LanguageLens>().ok())
        .unwrap_or(LanguageLens::Auto);
    let model = std::env::var("VIBE_MODEL").unwrap_or_else(|_| "auto-combo".to_string());

    let start =
        start.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    match Workspace::discover(&start) {
        Some(root) => run(Workspace::load(&root), None, mode, language, model),
        None => {
            let shown = start.display().to_string();
            run(
                Workspace::load(&start),
                Some(format!("Searched upward from: {shown}")),
                mode,
                language,
                model,
            );
        }
    }
}

fn run(
    workspace: Workspace,
    error: Option<String>,
    mode: ExecutionMode,
    language: LanguageLens,
    model: String,
) {
    Application::new()
        .with_assets(branding::BrandAssets)
        .run(move |cx: &mut App| {
            theme::init_fonts(cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(1440.0), px(900.0)), cx)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Vibe Assembly".into()),
                    appears_transparent: true,
                    traffic_light_position: None,
                }),
                window_min_size: Some(size(px(1120.0), px(720.0))),
                window_background: WindowBackgroundAppearance::Opaque,
                app_id: Some("vibe-assembly".to_string()),
                ..Default::default()
            };

            let _window = cx
                .open_window(options, move |window, cx| {
                    let state = WorkbenchState::new(workspace, error, mode, language, model);
                    let view = cx.new(|cx| Workbench::new(state, cx));
                    let focus = view.read(cx).composer_focus.clone();
                    focus.focus(window);
                    view
                })
                .expect("failed to open the Vibe Assembly workbench window");

            cx.activate(true);
        });
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sample {
    Obsidian,
    Paper,
    Midnight,
    Studio,
    Glass,
}

fn run_preview_shipped(mode: ExecutionMode) {
    run_preview_window(mode, Sample::Glass, "Vibe Assembly");
}

fn run_preview(mode: ExecutionMode, sample: Sample) {
    let title = match sample {
        Sample::Obsidian => "Vibe Assembly — Design Preview",
        Sample::Paper => "Vibe Assembly — Paper & Ink",
        Sample::Midnight => "Vibe Assembly — Midnight Paper",
        Sample::Studio => "Vibe Assembly — Studio",
        Sample::Glass => "Vibe Assembly — Liquid Glass",
    };
    run_preview_window(mode, sample, title);
}

fn run_preview_window(mode: ExecutionMode, sample: Sample, title: &'static str) {
    Application::new()
        .with_assets(branding::BrandAssets)
        .run(move |cx: &mut App| {
            theme::init_fonts(cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(1440.0), px(900.0)), cx)),
                titlebar: Some(TitlebarOptions {
                    title: Some(title.into()),
                    appears_transparent: true,
                    traffic_light_position: None,
                }),
                window_min_size: Some(size(px(1120.0), px(720.0))),
                window_background: WindowBackgroundAppearance::Opaque,
                app_id: Some("vibe-assembly".to_string()),
                ..Default::default()
            };

            match sample {
                Sample::Obsidian => {
                    cx.open_window(options, move |_window, cx| {
                        cx.new(|_| design::preview::DesignPreview::new(mode))
                    })
                    .expect("failed to open the design preview window");
                }
                Sample::Paper => {
                    cx.open_window(options, move |_window, cx| {
                        cx.new(|_| design::friendly::FriendlyPreview::new(mode))
                    })
                    .expect("failed to open the design preview window");
                }
                Sample::Midnight => {
                    cx.open_window(options, move |_window, cx| {
                        cx.new(|_| design::midnight::MidnightPreview::new(mode))
                    })
                    .expect("failed to open the design preview window");
                }
                Sample::Studio => {
                    cx.open_window(options, move |_window, cx| {
                        cx.new(|_| design::studio::StudioPreview::new(mode))
                    })
                    .expect("failed to open the design preview window");
                }
                Sample::Glass => {
                    cx.open_window(options, move |_window, cx| {
                        cx.new(|cx| design::glass::GlassPreview::new(mode, cx))
                    })
                    .expect("failed to open the design preview window");
                }
            }

            cx.activate(true);
        });
}
