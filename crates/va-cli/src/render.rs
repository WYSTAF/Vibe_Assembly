//! Terminal rendering helpers for the `vibe-assembly` CLI.

use va_core::parser::{ActiveTask, QueueRow};

pub struct Palette {
    pub green: &'static str,
    pub yellow: &'static str,
    pub red: &'static str,
    pub blue: &'static str,
    pub dim: &'static str,
    pub bold: &'static str,
    pub reset: &'static str,
}

impl Palette {
    pub fn detect() -> Palette {
        if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
            Palette {
                green: "\x1b[32m",
                yellow: "\x1b[33m",
                red: "\x1b[31m",
                blue: "\x1b[36m",
                dim: "\x1b[2m",
                bold: "\x1b[1m",
                reset: "\x1b[0m",
            }
        } else {
            Palette {
                green: "",
                yellow: "",
                red: "",
                blue: "",
                dim: "",
                bold: "",
                reset: "",
            }
        }
    }

    pub fn status_color(&self, status: &str) -> &'static str {
        match status {
            "done" => self.green,
            "in_progress" => self.yellow,
            "blocked" | "failed" | "blocked_escalated" => self.red,
            _ => self.dim,
        }
    }
}

pub fn role_icon(mode: &str) -> &'static str {
    match mode {
        "boss" => "👑",
        "chat" => "💬",
        "code" => "💻",
        "debug" => "🐞",
        "hermes" => "📣",
        _ => "•",
    }
}

/// Render the queue as an aligned table.
pub fn queue_lines(at: &ActiveTask, p: &Palette) -> Vec<String> {
    at.queue
        .iter()
        .map(|r: &QueueRow| {
            let mark = if Some(&r.id) == at.active_id.as_ref() {
                format!("{}▸{}", p.blue, p.reset)
            } else {
                " ".to_string()
            };
            let status = if r.status.is_empty() {
                "—".to_string()
            } else {
                r.status.clone()
            };
            format!(
                " {} {:<7} {}{:<18}{}{}",
                mark,
                r.id,
                p.status_color(&r.status),
                status,
                p.reset,
                r.title
            )
        })
        .collect()
}
