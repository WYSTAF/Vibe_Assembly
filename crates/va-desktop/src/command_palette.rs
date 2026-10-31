//! Searchable command, file, symbol, and composer picker surface.
use crate::design::icon::icon;
use crate::state::{FileNode, PaletteCommand, PaletteScope, Workbench, WorkbenchState};
use crate::{theme, ui_kit, widgets};
use gpui::prelude::*;
use gpui::{div, point, px, rgba, BoxShadow, Context, Rgba};
use std::path::PathBuf;
use va_core::modes::{ExecutionMode, LanguageLens};

#[derive(Clone)]
pub enum PaletteAction {
    Command(PaletteCommand),
    File(PathBuf),
    Symbol(PathBuf, u64),
    Model(String),
}

pub struct PaletteItem {
    pub label: String,
    pub detail: String,
    pub group: &'static str,
    pub shortcut: &'static str,
    pub action: PaletteAction,
    pub active: bool,
}

pub fn items(state: &WorkbenchState) -> Vec<PaletteItem> {
    let mut out = Vec::new();
    let mut command = |command: PaletteCommand| {
        out.push(PaletteItem {
            label: command.label(),
            detail: String::new(),
            group: command.group(),
            shortcut: command.shortcut(),
            active: command.is_active(state),
            action: PaletteAction::Command(command),
        })
    };
    if matches!(state.palette_scope, PaletteScope::All | PaletteScope::Mode) {
        for mode in ExecutionMode::ALL {
            command(PaletteCommand::Mode(mode));
        }
    }
    if matches!(
        state.palette_scope,
        PaletteScope::All | PaletteScope::Language
    ) {
        for language in LanguageLens::ALL {
            command(PaletteCommand::Language(language));
        }
    }
    if state.palette_scope == PaletteScope::All {
        for action in [
            PaletteCommand::Compact,
            PaletteCommand::Tournament,
            PaletteCommand::Rollback,
            PaletteCommand::ToggleLeft,
            PaletteCommand::ToggleBottom,
            PaletteCommand::ToggleAssistant,
            PaletteCommand::RefreshRepoMap,
            PaletteCommand::CloseTab,
        ] {
            command(action);
        }
    }
    if matches!(state.palette_scope, PaletteScope::All | PaletteScope::Model) {
        for model in &state.models {
            out.push(PaletteItem {
                label: if model == "auto-combo" {
                    "Auto-Combo".into()
                } else {
                    model.clone()
                },
                detail: String::new(),
                group: "Models",
                shortcut: "",
                action: PaletteAction::Model(model.clone()),
                active: *model == state.model,
            });
        }
    }
    if state.palette_scope == PaletteScope::All {
        fn collect(nodes: &[FileNode], state: &WorkbenchState, out: &mut Vec<PaletteItem>) {
            for node in nodes {
                if node.is_dir {
                    collect(&node.children, state, out);
                } else {
                    out.push(PaletteItem {
                        label: node.name.clone(),
                        detail: state.rel_path(&node.path),
                        group: "Files",
                        shortcut: "",
                        action: PaletteAction::File(node.path.clone()),
                        active: false,
                    });
                }
            }
        }
        collect(&state.tree, state, &mut out);
        let mut files = state.repo_map.files.iter().collect::<Vec<_>>();
        files.sort_by_key(|(path, _)| *path);
        for (path, file) in files {
            for symbol in &file.symbols {
                out.push(PaletteItem {
                    label: symbol.name.clone(),
                    detail: format!("{}:{}", path.display(), symbol.line),
                    group: "Symbols",
                    shortcut: "",
                    action: PaletteAction::Symbol(path.clone(), symbol.block_hash),
                    active: false,
                });
            }
        }
    }
    let query = state.palette_query.to_lowercase();
    let words = query.split_whitespace().collect::<Vec<_>>();
    out.into_iter()
        .filter(|item| {
            let haystack = format!("{} {} {}", item.label, item.detail, item.group).to_lowercase();
            words.iter().all(|word| haystack.contains(word))
        })
        .take(80)
        .collect()
}

pub fn render(
    workbench: &Workbench,
    _accent: Rgba,
    _accent_strong: Rgba,
    cx: &mut Context<Workbench>,
) -> impl IntoElement {
    let state = &workbench.state;
    let results = items(state);
    let mut list = div().flex().flex_col().gap(px(2.0)).p(px(8.0));
    let mut group = "";
    for (index, item) in results.iter().enumerate() {
        if item.group != group {
            group = item.group;
            list = list.child(
                div()
                    .px(px(10.0))
                    .pt(px(10.0))
                    .pb(px(6.0))
                    .child(widgets::section_label(group)),
            );
        }
        let selected = index == state.palette_index;
        let action = item.action.clone();
        list = list.child(
            div()
                .id(("palette-row", index))
                .h(px(36.0))
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(px(10.0))
                .px(px(10.0))
                .rounded(px(8.0))
                .bg(if selected {
                    theme::brand_soft()
                } else {
                    theme::elevated()
                })
                .cursor_pointer()
                .hover(|style| style.bg(theme::elevated_alt()))
                .active(|style| style.bg(theme::border()))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.state.apply_palette_action(action.clone());
                    this.restore_focus(window);
                    cx.notify();
                }))
                .child(div().w(px(16.0)).child(if item.active {
                    icon("check", 14.0, theme::brand_ink())
                } else if item.group == "Files" {
                    icon("file-code", 14.0, theme::text_muted())
                } else if item.group == "Symbols" {
                    icon("braces", 14.0, theme::text_muted())
                } else {
                    icon("chevron-right", 14.0, theme::text_muted())
                }))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(theme::BODY))
                        .text_color(theme::text_primary())
                        .child(item.label.clone()),
                )
                .child(
                    div()
                        .max_w(px(200.0))
                        .truncate()
                        .font_family(theme::code_font())
                        .text_size(px(theme::MICRO))
                        .text_color(theme::text_muted())
                        .child(item.detail.clone()),
                )
                .when(!item.shortcut.is_empty(), |this| {
                    this.child(widgets::keycap(item.shortcut))
                }),
        );
    }
    if results.is_empty() {
        list = list.child(ui_kit::empty_state(
            icon("search", 22.0, theme::brand_ink()),
            "No matches",
            "Try a filename, symbol, mode, or command.",
        ));
    }
    let placeholder = match state.palette_scope {
        PaletteScope::All => "Search files, symbols, or run commands…",
        PaletteScope::Mode => "Choose a mode…",
        PaletteScope::Language => "Choose a language…",
        PaletteScope::Model => "Choose a model…",
    };
    div()
        .id("palette-overlay")
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .flex()
        .justify_center()
        .items_start()
        .pt(px(76.0))
        .bg(rgba(0x2B262066))
        .on_click(cx.listener(|this, _, window, cx| {
            this.state.show_palette = false;
            this.restore_focus(window);
            cx.notify();
        }))
        .child(
            div()
                .id("palette-panel")
                .w(px(560.0))
                .max_h(px(600.0))
                .flex()
                .flex_col()
                .rounded(px(16.0))
                .bg(theme::elevated())
                .border_1()
                .border_color(theme::border_strong())
                .overflow_hidden()
                .shadow(vec![BoxShadow {
                    color: rgba(0x2B262033).into(),
                    offset: point(px(0.0), px(12.0)),
                    blur_radius: px(40.0),
                    spread_radius: px(0.0),
                }])
                .track_focus(&workbench.palette_focus)
                .on_key_down(cx.listener(Workbench::on_palette_key))
                .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                .child(
                    div()
                        .h(px(58.0))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .px(px(20.0))
                        .border_b_1()
                        .border_color(theme::border())
                        .child(icon("search", 18.0, theme::text_muted()))
                        .relative()
                        .child(div().flex_1().text_size(px(theme::HEADING)).child(
                            workbench.palette_selection.display(
                                &state.palette_query,
                                true,
                                placeholder,
                                theme::text_primary(),
                            ),
                        ))
                        .child(ui_kit::capture(
                            cx.entity(),
                            workbench.palette_focus.clone(),
                        ))
                        .child(widgets::keycap("Esc")),
                )
                .child(
                    list.id("palette-scroll")
                        .max_h(px(490.0))
                        .overflow_y_scroll()
                        .track_scroll(&workbench.palette_scroll),
                )
                .child(
                    div()
                        .h(px(32.0))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .px(px(16.0))
                        .border_t_1()
                        .border_color(theme::border())
                        .text_size(px(theme::MICRO))
                        .text_color(theme::text_muted())
                        .child(widgets::keycap("↑ ↓"))
                        .child("Navigate")
                        .child(widgets::keycap("Enter"))
                        .child("Open")
                        .child(div().flex_1())
                        .child(format!("{} results", results.len())),
                ),
        )
}
