//! Port of `bin/lib/parser.js` to Rust.
//!
//! The contract semantics are load-bearing: `verified_steps` decides where
//! Code resumes, and `validate_contract` fails closed. Both are ported
//! literally rather than "improved" so the native app and the `va` CLI can
//! never disagree about what state a workspace is in.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct QueueRow {
    pub order: i64,
    pub id: String,
    pub title: String,
    pub status: String,
    pub effort: String,
    pub depends_on: String,
}

impl QueueRow {
    pub fn is_done(&self) -> bool {
        self.status == "done"
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub n: i64,
    pub text: String,
    pub verify_command: Option<String>,
    pub verify_expected: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Ticket {
    pub id: String,
    pub title: String,
    pub status: Option<String>,
    pub effort: Option<String>,
    pub files_allowed: Vec<String>,
    pub context_inline: String,
    pub steps: Vec<Step>,
    pub done_when: Vec<String>,
    pub stop_when: Vec<String>,
    pub relay_notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub current_ticket: Option<String>,
    pub completed_tickets: Option<String>,
    pub remaining_tickets: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActiveTask {
    pub wave_name: Option<String>,
    pub snapshot: Snapshot,
    pub active_id: Option<String>,
    pub queue: Vec<QueueRow>,
    pub active: Option<Ticket>,
}

impl ActiveTask {
    /// The resume point: the first step without durable verification evidence.
    /// This is the single most decision-relevant number in the whole dataset.
    pub fn verified_steps(&self) -> Vec<i64> {
        match &self.active {
            Some(t) => verified_steps(&t.steps, &t.relay_notes),
            None => Vec::new(),
        }
    }

    pub fn resume_step(&self) -> Option<i64> {
        let verified = self.verified_steps();
        let total = self.active.as_ref()?.steps.len() as i64;
        (1..=total).find(|n| !verified.contains(n))
    }

    pub fn steps_total(&self) -> usize {
        self.active.as_ref().map_or(0, |t| t.steps.len())
    }

    pub fn has_work(&self) -> bool {
        self.queue.iter().any(|r| !r.is_done())
    }
}

/// Text between `start_marker` and the first `end_marker` after it.
fn between<'a>(text: &'a str, start_marker: &str, end_marker: Option<&str>) -> Option<&'a str> {
    let i = text.find(start_marker)?;
    let rest = &text[i + start_marker.len()..];
    match end_marker {
        None => Some(rest.trim()),
        Some(end) => match rest.find(end) {
            Some(j) => Some(rest[..j].trim()),
            None => Some(rest.trim()),
        },
    }
}

/// Parse a `- **field:** value` line, case-insensitively.
fn field(text: &str, name: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim_start();
        let rest = line.strip_prefix("- ")?;
        let rest = rest.strip_prefix("**")?;
        // Split at the first `:**` so field names may contain ':'.
        let end = rest.find(":**")?;
        if !rest[..end].eq_ignore_ascii_case(name) {
            continue;
        }
        return Some(rest[end + 3..].trim().to_string());
    }
    None
}

/// Windows checkouts produce CRLF; strip it so captures never carry a
/// trailing `\r` and silently fail status comparisons.
fn normalize(raw: &str) -> String {
    raw.replace("\r\n", "\n").replace('\r', "\n")
}

fn parse_queue_rows(section: &str) -> Vec<QueueRow> {
    let mut rows = Vec::new();
    for line in section.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        // | order | id | title | status | effort | depends_on |
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() < 6 {
            continue;
        }
        let order: i64 = match cells[0].parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        if !cells[1].starts_with("T-") {
            continue;
        }
        rows.push(QueueRow {
            order,
            id: cells[1].to_string(),
            title: cells[2].to_string(),
            status: cells[3].to_string(),
            effort: cells[4].to_string(),
            depends_on: cells[5].to_string(),
        });
    }
    rows
}

fn parse_steps(section: &str) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    for line in section.lines() {
        let trimmed = line.trim_start();
        // A step is `N. text`.
        if let Some(dot) = trimmed.find(". ") {
            let (head, tail) = trimmed.split_at(dot);
            if let Ok(n) = head.trim().parse::<i64>() {
                if n > 0 && head.trim().chars().all(|c| c.is_ascii_digit()) {
                    steps.push(Step {
                        n,
                        text: tail[2..].trim().to_string(),
                        verify_command: None,
                        verify_expected: None,
                    });
                    continue;
                }
            }
        }
        // `  - **Verify:** \`cmd\` -> expected`
        if let Some(cur) = steps.last_mut() {
            if let Some((cmd, expected)) = parse_verify(trimmed) {
                cur.verify_command = Some(cmd);
                cur.verify_expected = Some(expected);
            }
        }
    }
    steps
}

fn parse_verify(line: &str) -> Option<(String, String)> {
    let after = line.strip_prefix("- **Verify:**")?.trim_start();
    // The command is the text between the first pair of backticks.
    let inner = after.strip_prefix('`')?;
    let end = inner.find('`')?;
    let cmd = inner[..end].to_string();
    let rest = inner[end + 1..].trim_start();
    // Tolerate both the ASCII `->` and the Unicode `→` separator.
    let rest = rest
        .trim_start_matches("->")
        .trim_start_matches('\u{2192}')
        .trim();
    Some((cmd, rest.to_string()))
}

fn bullets(section: &str) -> Vec<String> {
    section
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("- "))
        .map(|l| l.trim_start_matches("- ").trim())
        .map(|l| {
            // The contract tells authors to leave an empty relay_notes marker.
            let lower = l.to_ascii_lowercase();
            match lower.find("(empty until code runs)") {
                Some(i) => format!("{}{}", &l[..i], &l[i + "(empty until code runs)".len()..])
                    .trim()
                    .to_string(),
                None => l.to_string(),
            }
        })
        .filter(|l| !l.is_empty())
        .collect()
}

pub fn parse_active_task(raw: &str) -> ActiveTask {
    let text = normalize(raw);
    let queue_section = between(&text, "## Queue", Some("\n## ")).unwrap_or("");
    let queue = parse_queue_rows(queue_section);

    // The Active Ticket pointer is the sole execution pointer. The literal
    // value "none" is meaningful (no work assigned) and must survive here —
    // dropping it would turn a quiescent workspace into a false violation.
    let active_id = text
        .split_once("## Active Ticket")
        .and_then(|(_, rest)| {
            rest.lines()
                .map(str::trim)
                .find_map(|l| l.strip_prefix("- **id:**"))
                .map(|v| v.trim().to_string())
        })
        .filter(|v| !v.is_empty());

    let mut active = None;
    if let Some(id) = active_id.as_deref() {
        if id != "none" {
            if let Some(sec) = between(&text, &format!("## Ticket {id}"), Some("\n## ")) {
                let meta = between(sec, "### Meta", Some("\n### ")).unwrap_or("");
                let title = queue
                    .iter()
                    .find(|r| r.id == id)
                    .map(|r| r.title.clone())
                    .unwrap_or_default();

                // files_allowed is a multi-line list under its own Meta key.
                // Collect only the indented bullets that follow that key, so
                // sibling keys (status/effort/depends_on) are never absorbed.
                let files_allowed = {
                    let mut out = Vec::new();
                    let mut in_block = false;
                    for line in meta.lines() {
                        let t = line.trim();
                        // The key line itself opens the block, then continues.
                        if t.starts_with("- **files_allowed:**") {
                            in_block = true;
                            continue;
                        }
                        if !t.starts_with("- ") {
                            continue;
                        }
                        // Any other `- **key:**` line closes it.
                        if t.starts_with("- **") {
                            in_block = false;
                            continue;
                        }
                        if in_block {
                            if let Some(v) = t.strip_prefix("- ") {
                                let v = v.trim();
                                if v != "none" && !v.starts_with('(') && !v.is_empty() {
                                    out.push(v.to_string());
                                }
                            }
                        }
                    }
                    out
                };

                let relay = sec.find("### relay_notes").map(|i| &sec[i..]);

                active = Some(Ticket {
                    id: id.to_string(),
                    title,
                    status: field(meta, "status"),
                    effort: field(meta, "effort"),
                    files_allowed,
                    context_inline: between(sec, "### context_inline", Some("\n### "))
                        .unwrap_or("")
                        .to_string(),
                    steps: parse_steps(between(sec, "### Steps", Some("\n### ")).unwrap_or("")),
                    done_when: bullets(between(sec, "### done_when", Some("\n### ")).unwrap_or("")),
                    stop_when: bullets(between(sec, "### stop_when", Some("\n### ")).unwrap_or("")),
                    relay_notes: bullets(relay.unwrap_or("")),
                });
            }
        }
    }

    let snap_section = between(&text, "## Status Snapshot", Some("\n## ")).unwrap_or("");
    let snapshot = Snapshot {
        current_ticket: field(snap_section, "current_ticket"),
        completed_tickets: field(snap_section, "completed_tickets"),
        remaining_tickets: field(snap_section, "remaining_tickets"),
    };
    let wave_name = field(
        between(&text, "## Wave", Some("\n## ")).unwrap_or(""),
        "name",
    );

    ActiveTask {
        wave_name,
        snapshot,
        active_id,
        queue,
        active,
    }
}

/// A simple `# Title` + `- **k:** v` file. Duplicate keys become a Vec so
/// per-entry fields (each KB's `Date:`) do not overwrite one another.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateFile {
    pub title: String,
    pub fields: Vec<(String, String)>,
}

impl StateFile {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    pub fn all(&self, key: &str) -> Vec<&str> {
        self.fields
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
            .collect()
    }
}

pub fn parse_state_file(raw: &str) -> StateFile {
    let text = normalize(raw);
    let title = text
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .unwrap_or("")
        .trim()
        .to_string();
    let mut fields = Vec::new();
    for line in text.lines() {
        let t = line.trim_start();
        let rest = match t.strip_prefix("- **") {
            Some(r) => r,
            None => continue,
        };
        let end = match rest.find(":**") {
            Some(i) => i,
            None => continue,
        };
        fields.push((
            rest[..end].trim().to_string(),
            rest[end + 3..].trim().to_string(),
        ));
    }
    StateFile { title, fields }
}

/// Split a `## KB-NNN — title` log into discrete entries. The old Electron
/// UI rendered one entry's field labels as if they were separate bugs, so the
/// native app parses entries properly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub title: String,
    pub body: String,
}

pub fn parse_log_entries(raw: &str) -> Vec<LogEntry> {
    let text = normalize(raw);
    let mut out: Vec<LogEntry> = Vec::new();
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            let heading = heading.trim();
            // Expected shape: `KB-001 — title` or `[ADR-008] title`.
            let (id, title) = if let Some(rest) = heading.strip_prefix('[') {
                match rest.split_once(']') {
                    Some((id, title)) => (id.trim().to_string(), title.trim().to_string()),
                    None => (String::new(), heading.to_string()),
                }
            } else {
                match heading
                    .split_once("—")
                    .or_else(|| heading.split_once(" - "))
                {
                    Some((id, title)) => (id.trim().to_string(), title.trim().to_string()),
                    None => (String::new(), heading.to_string()),
                }
            };
            out.push(LogEntry {
                id,
                title,
                body: String::new(),
            });
        } else if let Some(last) = out.last_mut() {
            last.body.push_str(line);
            last.body.push('\n');
        }
    }
    out
}

/// Fail-closed consistency checks mirroring `.ai/ticket_contract.md`.
/// Returns a human-readable violation, or `None` when state is coherent.
pub fn validate_contract(parsed: &ActiveTask) -> Option<String> {
    let q = &parsed.queue;
    let id = parsed.active_id.as_deref();

    // Duplicate ids make the pointer ambiguous before any other check.
    for (i, r) in q.iter().enumerate() {
        if q.iter().skip(i + 1).any(|o| o.id == r.id) {
            return Some(format!("Queue contains duplicate id {}.", r.id));
        }
    }

    let work_remains = q.iter().any(|r| !r.is_done());

    let id = match id {
        None => return Some("Active Ticket pointer section is missing.".to_string()),
        Some("none") => {
            return if work_remains {
                let open = q
                    .iter()
                    .filter(|r| !r.is_done())
                    .map(|r| r.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                Some(format!(
                    "Active Ticket is \"none\" but unfinished Queue rows remain: {open}."
                ))
            } else {
                None
            }
        }
        Some(other) => other,
    };

    let pointed: Vec<&QueueRow> = q.iter().filter(|r| r.id == id).collect();
    if pointed.len() != 1 {
        return Some(format!(
            "Queue must contain exactly one row for Active Ticket {id}, found {}.",
            pointed.len()
        ));
    }
    let row = pointed[0];
    if row.status != "pending" && row.status != "in_progress" {
        return Some(format!(
            "Active Ticket {id} has Queue status \"{}\" — expected pending/in_progress.",
            row.status
        ));
    }
    if let Some(ticket) = &parsed.active {
        if let Some(status) = &ticket.status {
            if status != &row.status {
                return Some(format!(
                    "Status mismatch: Meta \"{status}\" vs Queue \"{}\" for {id}.",
                    row.status
                ));
            }
        }
    }

    // Every dependency must exist and be done.
    for dep in row
        .depends_on
        .split(',')
        .map(str::trim)
        .filter(|d| !d.is_empty() && !d.eq_ignore_ascii_case("none"))
    {
        match q.iter().find(|r| r.id == dep) {
            None => return Some(format!("{id} depends_on {dep}, which is not in the Queue.")),
            Some(dep_row) if dep_row.status != "done" => {
                return Some(format!(
                    "{id} depends_on {dep}, whose status is \"{}\" — must be done.",
                    dep_row.status
                ))
            }
            Some(_) => {}
        }
    }

    None
}

/// A step counts as verified ONLY when a SINGLE relay note both references
/// that exact step (word-boundary matched, so "step 12" never satisfies
/// "step 1") and carries success evidence. Cross-note leakage is impossible.
pub fn verified_steps(steps: &[Step], relay_notes: &[String]) -> Vec<i64> {
    let lowered: Vec<String> = relay_notes.iter().map(|n| n.to_lowercase()).collect();
    steps
        .iter()
        .filter(|s| {
            lowered
                .iter()
                .any(|note| mentions_step(note, s.n) && has_success_evidence(note))
        })
        .map(|s| s.n)
        .collect()
}

fn mentions_step(note: &str, n: i64) -> bool {
    let needle = format!("step {n}");
    let hay: Vec<char> = note.chars().collect();
    let nd: Vec<char> = needle.chars().collect();
    if nd.is_empty() || hay.len() < nd.len() {
        return false;
    }
    for start in 0..=(hay.len() - nd.len()) {
        if hay[start..start + nd.len()] != nd[..] {
            continue;
        }
        // \b — a non-word char before, and a non-digit after, so the prefix
        // trap ("step 1" matching inside "step 12") cannot fire.
        let before_ok = start == 0 || !is_word_char(hay[start - 1]);
        let after_idx = start + nd.len();
        let after_ok = after_idx >= hay.len() || !hay[after_idx].is_ascii_digit();
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn has_success_evidence(note: &str) -> bool {
    // \bpass\b
    if word_occurrences(note, "pass") {
        return true;
    }
    // \bexit(\s*code)?\s*[:=]?\s*0\b
    let bytes: Vec<char> = note.chars().collect();
    for i in 0..bytes.len() {
        if !bytes[i..].starts_with(&['e', 'x', 'i', 't']) {
            continue;
        }
        if i > 0 && is_word_char(bytes[i - 1]) {
            continue;
        }
        let mut j = i + 4;
        let mut saw_code = false;
        // optional whitespace then "code"
        let ws: String = bytes[j..]
            .iter()
            .take_while(|c| c.is_whitespace())
            .collect();
        let rest_start = j + ws.chars().count();
        if note[byte_index(&bytes, rest_start)..].starts_with("code") {
            saw_code = true;
            j = rest_start + 4;
        }
        let _ = saw_code;
        // optional whitespace, optional :/=, optional whitespace, then '0'
        while j < bytes.len() && bytes[j].is_whitespace() {
            j += 1;
        }
        if j < bytes.len() && (bytes[j] == ':' || bytes[j] == '=') {
            j += 1;
        }
        while j < bytes.len() && bytes[j].is_whitespace() {
            j += 1;
        }
        if j < bytes.len() && bytes[j] == '0' {
            let after_ok = j + 1 >= bytes.len() || !is_word_char(bytes[j + 1]);
            if after_ok {
                return true;
            }
        }
    }
    false
}

fn byte_index(chars: &[char], char_idx: usize) -> usize {
    chars[..char_idx].iter().map(|c| c.len_utf8()).sum()
}

fn word_occurrences(hay: &str, needle: &str) -> bool {
    let chars: Vec<char> = hay.chars().collect();
    let nd: Vec<char> = needle.chars().collect();
    for start in 0..=chars.len().saturating_sub(nd.len()) {
        if chars[start..start + nd.len()] != nd[..] {
            continue;
        }
        let before_ok = start == 0 || !is_word_char(chars[start - 1]);
        let after = start + nd.len();
        let after_ok = after >= chars.len() || !is_word_char(chars[after]);
        if before_ok && after_ok {
            return true;
        }
    }
    false
}
