//! Mission Control — the render layer.
//!
//! Design rule, in priority order:
//!   1. Answer "what do I do next?" within five seconds, from anywhere.
//!   2. Make a contract violation impossible to miss.
//!   3. Everything else earns its space or does not get rendered.
//!
//! The previous Electron build violated rule 3 — four raw-markdown panels
//! competed for equal weight with the one thing that mattered.

use gpui::prelude::*;
use gpui::{
    div, px, rgb, size, App, Application, Bounds, Context, Div, FontWeight, Window,
    WindowBounds, WindowOptions,
};

use crate::parser::Step;
use crate::state::Workspace;
use crate::theme::*;

pub struct MissionControl {
    pub ws: Workspace,
    pub error: Option<String>,
    pub selected_step: Option<i64>,
}

// ---------------------------------------------------------------- primitives

/// A panel: hairline-bordered, on the raised surface.
fn panel(title: &str, count: Option<String>) -> Div {
    div()
        .flex()
        .flex_col()
        .bg(surface_1())
        .border_1()
        .border_color(border())
        .rounded(px(6.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .px_3()
                .py_2()
                .rounded_t(px(6.0))
                .border_b_1()
                .border_color(border())
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(text_secondary())
                        .child(title.to_uppercase().to_string()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(text_muted())
                        .child(count.unwrap_or_default()),
                ),
        )
}

/// Monospace label/value pair on one line.
fn kv(label: &str, value: &str, value_color: gpui::Rgba) -> impl IntoElement {
    div()
        .flex()
        .items_start()
        .gap_2()
        .px_3()
        .py_1()
        .child(
            div()
                .w(px(96.0))
                .flex_shrink_0()
                .text_xs()
                .text_color(text_muted())
                .child(label.to_string()),
        )
        .child(
            div()
                .flex_1()
                .text_xs()
                .text_color(value_color)
                .child(value.to_string()),
        )
}

/// A small uppercase status chip.
fn chip(text: &str, color: gpui::Rgba) -> impl IntoElement {
    div()
        .px_2()
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(wash(color, 0.15))
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color)
        .child(text.to_string())
}

/// Horizontal progress meter built from two divs — no canvas needed.
fn meter(pct: usize, color: gpui::Rgba) -> impl IntoElement {
    div()
        .h(px(4.0))
        .w_full()
        .bg(surface_2())
        .rounded(px(2.0))
        .overflow_hidden()
        .child(
            div()
                .h_full()
                .w(px((pct.clamp(0, 100) as f32) * 2.6))
                .bg(color)
                .rounded(px(2.0)),
        )
}

// ------------------------------------------------------------------ sections

/// Rule 1. Always the first thing on screen, and the only element allowed
/// to be large.
fn next_action(ws: &Workspace, copy_phrase: Option<&str>) -> impl IntoElement {
    let (headline, detail, color) = if let Some(v) = &ws.violation {
        (
            "CONTRACT VIOLATION".to_string(),
            v.clone(),
            status_error(),
        )
    } else if let Some(phrase) = ws.suggested_phrase() {
        let resume = ws.active_task.resume_step();
        let detail = match resume {
            Some(n) if ws.active_task.has_work() => {
                format!("{phrase} — resume at step {n} of {}", ws.active_task.steps_total())
            }
            _ => phrase.to_string(),
        };
        (phrase.to_string(), detail, accent())
    } else if let Some(b) = ws.blockers() {
        ("BLOCKED".to_string(), b.to_string(), status_warn())
    } else {
        (
            "NOTHING TO EXECUTE".to_string(),
            "No active ticket. Hand back to 👑 Boss to scope the next wave.".to_string(),
            status_done(),
        )
    };

    div()
        .flex()
        .flex_col()
        .gap_2()
        .px_4()
        .py_3()
        .bg(status_wash_faint(
            if ws.violation.is_some() {
                "failed"
            } else if ws.suggested_phrase().is_some() {
                "in_progress"
            } else {
                "done"
            },
        ))
        .border_b_1()
        .border_color(border())
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .text_color(color)
                .child("NEXT ACTION"),
        )
        .child(
            div()
                .text_xl()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(text_primary())
                .child(headline),
        )
        .child(
            div()
                .text_sm()
                .text_color(text_secondary())
                .child(detail),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .flex_1()
                        .child(meter(ws.progress_pct(), color)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(text_muted())
                        .child(format!(
                            "{} steps verified · {}/{} tickets",
                            ws.active_task.verified_steps().len(),
                            ws.queue_done(),
                            ws.queue_total()
                        )),
                ),
        )
        .when_some(copy_phrase.map(|p| p.to_string()), |this, phrase| {
            this.child(
                div()
                    .mt_1()
                    .px_2()
                    .py_1()
                    .bg(surface_2())
                    .border_1()
                    .border_color(border_strong())
                    .rounded(px(4.0))
                    .text_xs()
                    .text_color(text_secondary())
                    .child(format!("`{phrase}`")),
            )
        })
}

/// The queue as a compact list. A 6-column table is the wrong shape for the
/// common case of one or two rows; this stays readable at n=1 and grows.
fn queue_panel(ws: &Workspace) -> impl IntoElement {
    let rows = ws.active_task.queue.clone();
    panel(
        "Queue",
        Some(format!("{}/{}", ws.queue_done(), ws.queue_total())),
    )
    .child(
        div()
            .flex()
            .flex_col()
            .children(rows.into_iter().map(|r| {
                let color = ticket_status_color(&r.status);
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(border())
                    .bg(status_wash_faint(&r.status))
                    .child(div().w(px(6.0)).h(px(6.0)).rounded_full().bg(color))
                    .child(
                        div()
                            .w(px(44.0))
                            .flex_shrink_0()
                            .text_xs()
                            .text_color(text_muted())
                            .child(r.id.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(text_primary())
                            .child(r.title.clone()),
                    )
                    .child(chip(&r.status, color))
            })),
    )
}

/// The active ticket: steps with verified evidence marked. This is where the
/// relay system's payload is finally visible.
fn ticket_panel(ws: &Workspace, selected: Option<i64>) -> impl IntoElement {
    let ticket = match &ws.active_task.active {
        Some(t) => t.clone(),
        None => {
            return panel("Active Ticket", None).child(
                div()
                    .px_3()
                    .py_3()
                    .text_xs()
                    .text_color(text_muted())
                    .child("No ticket assigned. The Active Ticket pointer is `none`."),
            );
        }
    };
    let verified = ws.active_task.verified_steps();

    panel("Active Ticket", Some(ticket.id.clone()))
        .child(
            div()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(border())
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(text_primary())
                        .child(ticket.title.clone()),
                )
                .when(!ticket.files_allowed.is_empty(), |this| {
                    this.child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(text_muted())
                            .child(format!(
                                "writable: {}",
                                ticket.files_allowed.join(", ")
                            )),
                    )
                }),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .children(ticket.steps.iter().map(move |s| {
                    step_row(s, verified.contains(&s.n), selected == Some(s.n))
                })),
        )
}

fn step_row(step: &Step, verified: bool, selected: bool) -> impl IntoElement {
    let color = if verified { status_ok() } else { text_muted() };
    div()
        .flex()
        .items_start()
        .gap_2()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(border())
        .bg(if selected {
            wash(surface_2(), 1.0)
        } else {
            wash(rgb(0x000000), 0.0)
        })
        .child(
            div()
                .w(px(16.0))
                .flex_shrink_0()
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .text_color(color)
                .child(if verified { "\u{2713}" } else { "\u{25cb}" }),
        )
        .child(
            div()
                .flex_1()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(if verified {
                            wash(text_secondary(), 0.8)
                        } else {
                            wash(text_primary(), 1.0)
                        })
                        .child(step.text.clone()),
                )
                .when(step.verify_command.is_some(), |this| {
                    this.child(
                        div()
                            .px_2()
                            .py(px(2.0))
                            .bg(surface_0())
                            .rounded(px(3.0))
                            .text_xs()
                            .text_color(text_muted())
                            .child(format!("$ {}", step.verify_command.clone().unwrap_or_default())),
                    )
                }),
        )
}

/// Operational state, but only the fields that are actionable. The old UI
/// dumped six `<dt>`/`<dd>` rows, four of which were prose paragraphs.
fn state_panel(ws: &Workspace) -> impl IntoElement {
    let mut rows: Vec<(&str, String, gpui::Rgba)> = Vec::new();

    if let Some(m) = ws.milestone() {
        rows.push(("Milestone", m.to_string(), text_primary()));
    }
    match ws.state_file.get("Planning Status") {
        Some(v) => rows.push(("Planning", v.to_string(), text_secondary())),
        None => {}
    }
    match ws.state_file.get("Resume Anchor") {
        Some(v) if !v.eq_ignore_ascii_case("n/a") => {
            rows.push(("Resume", v.to_string(), accent()))
        }
        _ => {}
    }
    rows.push((
        "Blockers",
        ws.blockers().unwrap_or("None").to_string(),
        if ws.blockers().is_some() {
            status_warn()
        } else {
            status_ok()
        },
    ));

    // Vibe Router allocation status
    rows.push((
        "Router",
        if ws.router_assignment.is_live {
            "🟢 Live Assignments Active".to_string()
        } else {
            "Offline Fallback".to_string()
        },
        if ws.router_assignment.is_live {
            status_ok()
        } else {
            text_muted()
        },
    ));

    let lock_status = crate::lock::check_task_lock(&ws.root);
    if let crate::lock::LockStatus::Held(info) = lock_status {
        rows.push((
            "Task Lock",
            format!("🔒 Held by PID {} ({})", info.pid, info.holder),
            status_warn(),
        ));
    }

    panel("Operational State", None).child(
        div()
            .flex()
            .flex_col()
            .children(rows.into_iter().map(|(k, v, c)| kv(k, &v, c))),
    )
}

/// Known bugs, split into real entries. The old renderer showed one entry's
/// field labels as if they were six separate bugs.
fn bugs_panel(ws: &Workspace) -> impl IntoElement {
    let bugs = ws.bugs.clone();
    panel("Known Bugs", Some(bugs.len().to_string())).child(
        div().flex().flex_col().children(bugs.into_iter().map(|b| {
            let headline = format!("{} — {}", b.id, b.title);
            let body = extract_field(&b.body, "Symptom");
            div()
                .flex()
                .flex_col()
                .gap_1()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(border())
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(text_primary())
                        .child(headline),
                )
                .when(!body.is_empty(), |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(text_secondary())
                            .child(truncate(&body, 150)),
                    )
                })
        })),
    )
}

fn decisions_panel(ws: &Workspace) -> impl IntoElement {
    let adrs = ws.adrs.clone();
    panel("Decisions", Some(adrs.len().to_string())).child(
        div().flex().flex_col().children(adrs.into_iter().map(|a| {
            div()
                .flex()
                .gap_2()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(border())
                .child(
                    div()
                        .w(px(72.0))
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(accent())
                        .child(a.id.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .text_xs()
                        .text_color(text_secondary())
                        .child(a.title.clone()),
                )
        })),
    )
}

fn progress_panel(ws: &Workspace) -> impl IntoElement {
    let lines = ws.progress_lines.clone();
    panel("Recent Progress", Some(lines.len().to_string())).child(
        div()
            .flex()
            .flex_col()
            .children(lines.into_iter().take(12).map(|l| {
                div()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(border())
                    .text_xs()
                    .text_color(text_secondary())
                    .child(truncate(&l, 200))
            })),
    )
}

// --------------------------------------------------------------------- utils

fn extract_field(body: &str, key: &str) -> String {
    for line in body.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("- **") {
            if let Some(end) = rest.find(":**") {
                if rest[..end].trim().eq_ignore_ascii_case(key) {
                    return rest[end + 3..].trim().to_string();
                }
            }
        }
    }
    String::new()
}

fn truncate(s: &str, max: usize) -> String {
    let clean: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        format!("{clean}…")
    } else {
        clean
    }
}

// --------------------------------------------------------------------- render

impl Render for MissionControl {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ws = self.ws.clone();
        let selected = self.selected_step;

        let body = if let Some(err) = &self.error {
            div()
                .size_full()
                .bg(surface_0())
                .items_center()
                .justify_center()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_6()
                        .bg(status_wash("failed"))
                        .border_1()
                        .border_color(status_error())
                        .rounded(px(8.0))
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::BOLD)
                                .text_color(status_error())
                                .child("No Vibe Assembly workspace found"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(text_secondary())
                                .child(err.clone()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(text_muted())
                                .child("Launch from a directory containing a .ai/ folder."),
                        ),
                )
        } else {
            div()
                .size_full()
                .bg(surface_0())
                .flex()
                .flex_col()
                // Header: workspace identity, always visible.
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_4()
                        .py_2()
                        .bg(surface_1())
                        .border_b_1()
                        .border_color(border())
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(text_primary())
                                .child("MISSION CONTROL"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_xs()
                                .text_color(text_muted())
                                .child(
                                    ws.active_task
                                        .wave_name
                                        .clone()
                                        .unwrap_or_else(|| "—".into()),
                                ),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(text_muted())
                                .child(format!(
                                    "{}/{} steps",
                                    ws.active_task.verified_steps().len(),
                                    ws.active_task.steps_total()
                                )),
                        )
                        .child(
                            if ws.router_assignment.is_live {
                                chip("🟢 ROUTER: LIVE", status_ok())
                            } else {
                                chip("○ ROUTER: OFFLINE", text_muted())
                            },
                        ),
                )
                .child(next_action(&ws, ws.suggested_phrase()))
                // Two columns: work on the left, context on the right.
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .flex_row()
                        .gap_3()
                        .p_3()
                        .id("body-scroll")
                        .overflow_scroll()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .w(px(560.0))
                                .flex_shrink_0()
                                .child(ticket_panel(&ws, selected))
                                .child(queue_panel(&ws)),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .flex_1()
                                .min_w_0()
                                .child(state_panel(&ws))
                                .child(bugs_panel(&ws))
                                .child(decisions_panel(&ws))
                                .child(progress_panel(&ws)),
                        ),
                )
        };

        let _ = cx;
        body
    }
}

pub fn run(ws: Workspace, error: Option<String>) {
    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.0), px(860.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| MissionControl {
                    ws,
                    error,
                    selected_step: None,
                })
            },
        )
        .ok();
        cx.activate(true);
    });
}
