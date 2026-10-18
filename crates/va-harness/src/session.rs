//! Append-only transactional session trajectory (DAG).
//!
//! # Design note — relationship to the upstream reference
//!
//! The reference implementation (`deepseek-harness`, TypeScript) is a **flat,
//! seq-addressed append-only event log**: it has no DAG, no state hashing, no
//! rollback, and no re-execution replay. Its fork is a slice of the event array
//! plus synthetic closers.
//!
//! This module implements the *behaviors* the Vibe Assembly wave requires —
//! branching trajectories, verifiable state, rollback and replay — rather than
//! claiming to port code that does not exist upstream. Concretely it adds:
//!
//! * a real DAG: nodes carry `parents: Vec<NodeId>`, so a node may join several
//!   branches (a true merge, not just a range replace);
//! * per-node state hashing, so drift is detectable rather than assumed;
//! * `rollback_to`, which is distinct from `fork` (no new branch is created);
//! * `replay`, which folds stored events forward without re-executing tools,
//!   mirroring the upstream's "reuse stored results" property.
//!
//! Every mutation is transactional: nodes are only visible once `commit` runs,
//! mirroring the upstream's flush-before-side-effect checkpoint policy.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// FNV-1a 64-bit. Deterministic, dependency-free, and adequate for integrity
/// checks over content we control. Not a cryptographic hash — collision
/// resistance against a *hostile* editor is out of scope; this detects
/// accidental drift.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

pub fn hash_str(s: &str) -> u64 {
    fnv1a64(s.as_bytes())
}

pub type NodeId = u64;

/// What kind of trajectory entry a node represents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeKind {
    /// A user or system instruction.
    Prompt { role: String, text: String },
    /// A model response.
    Thought { text: String },
    /// An attempted tool or patch invocation.
    ToolCall { name: String, arguments: String },
    /// The result of a tool invocation.
    Observation { text: String },
    /// A patch applied through the patcher.
    Patch { path: String, summary: String },
    /// A verification outcome (compile, test, judge).
    Verification { passed: bool, detail: String },
    /// An explicit branch point marker.
    Fork { reason: String },
    /// A compacted summary replacing a range of intermediate nodes on the surface.
    Compacted {
        start: NodeId,
        end: NodeId,
        summary: String,
    },
}

impl NodeKind {
    pub fn label(&self) -> &'static str {
        match self {
            NodeKind::Prompt { .. } => "prompt",
            NodeKind::Thought { .. } => "thought",
            NodeKind::ToolCall { .. } => "tool_call",
            NodeKind::Observation { .. } => "observation",
            NodeKind::Patch { .. } => "patch",
            NodeKind::Verification { .. } => "verification",
            NodeKind::Fork { .. } => "fork",
            NodeKind::Compacted { .. } => "compacted",
        }
    }
}

/// An immutable trajectory node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    /// Zero or more parents. More than one makes this a merge point.
    pub parents: Vec<NodeId>,
    pub kind: NodeKind,
    /// Hash over this node's own content — detects tampering with a stored log.
    pub content_hash: u64,
    /// Hash over the full ancestry state, so a reordered or dropped ancestor
    /// changes every descendant's hash.
    pub state_hash: u64,
    /// Micro-dollar cost telemetry ingested from Vibe Router (`X-VR-Cost-Micros`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_micros: Option<u64>,
    /// Latency telemetry ingested from Vibe Router (`X-VR-Latency-Ms`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// Deterministic anti-slop verification hash recorded on verified checkpoints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judge_verified_hash: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    UnknownNode(NodeId),
    DuplicateNode(NodeId),
    /// A node references a parent that does not exist.
    MissingParent(NodeId),
    /// `rollback`/`replay` to a node that is not on the current branch.
    NotOnCurrentPath(NodeId),
    /// The recorded ancestry hash does not match a recomputation — the log was
    /// modified out of band.
    IntegrityViolation {
        node: NodeId,
        expected: u64,
        actual: u64,
    },
    /// A node was committed with no parents and no prior head (empty session).
    EmptySession,
    /// Compaction range does not correspond to a valid forward slice.
    InvalidCompactionRange {
        start: NodeId,
        end: NodeId,
    },
    /// JSONL parse error.
    JsonlParseError(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::UnknownNode(id) => write!(f, "unknown node {id}"),
            SessionError::DuplicateNode(id) => write!(f, "duplicate node {id}"),
            SessionError::MissingParent(id) => write!(f, "missing parent {id}"),
            SessionError::NotOnCurrentPath(id) => {
                write!(f, "node {id} is not on the current branch")
            }
            SessionError::IntegrityViolation {
                node,
                expected,
                actual,
            } => write!(
                f,
                "integrity violation at node {node}: expected state hash {expected:#018x}, got {actual:#018x}"
            ),
            SessionError::EmptySession => write!(f, "session has no committed nodes"),
            SessionError::InvalidCompactionRange { start, end } => {
                write!(f, "invalid compaction range from node {start} to {end}")
            }
            SessionError::JsonlParseError(err) => write!(f, "JSONL parse error: {err}"),
        }
    }
}

impl std::error::Error for SessionError {}

/// An append-only transactional session trajectory.
#[derive(Debug, Clone, Default)]
pub struct Session {
    nodes: BTreeMap<NodeId, Node>,
    /// Nodes staged but not yet visible. Mirrors the upstream's flush-before-
    /// side-effect policy: work is not durable until committed.
    pending: Vec<Node>,
    next_id: NodeId,
    head: Option<NodeId>,
    /// Set when a `fork` or `rollback` moves the head backwards, so the next
    /// committed node starts a branch rather than extending the old one.
    detached: bool,
}

impl Session {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            pending: Vec::new(),
            next_id: 1,
            head: None,
            detached: false,
        }
    }

    pub fn head(&self) -> Option<NodeId> {
        self.head
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains_key(&id)
    }

    /// Every committed node, in id order.
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    /// The chain from the root to `id`, inclusive.
    pub fn path_to(&self, id: NodeId) -> Result<Vec<NodeId>, SessionError> {
        if !self.nodes.contains_key(&id) {
            return Err(SessionError::UnknownNode(id));
        }
        let mut chain = vec![id];
        let mut cursor = self.nodes[&id].parents.clone();
        let mut guard = HashSet::new();
        while let Some(parent) = cursor.pop() {
            if !guard.insert(parent) {
                break; // defensive: malformed multi-parent cycle
            }
            let node = self
                .nodes
                .get(&parent)
                .ok_or(SessionError::MissingParent(parent))?;
            chain.push(parent);
            cursor.extend(node.parents.iter().copied());
        }
        chain.reverse();
        Ok(chain)
    }

    /// The current branch: root → head along first parents.
    pub fn current_path(&self) -> Result<Vec<NodeId>, SessionError> {
        match self.head {
            Some(head) => self.path_to(head),
            None => Ok(Vec::new()),
        }
    }

    /// Stage a node as a child of `head`. Nothing is visible until `commit`.
    pub fn push(&mut self, kind: NodeKind) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;

        let parents: Vec<NodeId> = match self.head {
            Some(h) => vec![h],
            None => Vec::new(),
        };
        self.pending.push(Node {
            id,
            parents,
            kind,
            content_hash: 0,
            state_hash: 0,
            cost_micros: None,
            latency_ms: None,
            judge_verified_hash: None,
        });
        self.detached = false;
        id
    }

    /// Stage a node with explicit parents, creating a merge point.
    pub fn push_merging(&mut self, parents: Vec<NodeId>, kind: NodeKind) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        self.pending.push(Node {
            id,
            parents,
            kind,
            content_hash: 0,
            state_hash: 0,
            cost_micros: None,
            latency_ms: None,
            judge_verified_hash: None,
        });
        self.detached = false;
        id
    }

    /// Discard staged nodes without committing.
    pub fn discard_pending(&mut self) {
        self.pending.clear();
        // Roll the id counter back so a discarded attempt leaves no gap.
        if let Some(last) = self.pending_staged_max() {
            self.next_id = last;
        }
    }

    fn pending_staged_max(&self) -> Option<NodeId> {
        self.pending.iter().map(|n| n.id).max()
    }

    fn compute_content_hash(node: &Node) -> u64 {
        let mut buf = String::new();
        buf.push_str(node.kind.label());
        buf.push('\u{1f}');
        buf.push_str(&format!("{:?}", node.parents));
        match &node.kind {
            NodeKind::Prompt { role, text } => {
                buf.push_str(role);
                buf.push('\u{1e}');
                buf.push_str(text);
            }
            NodeKind::Thought { text } | NodeKind::Observation { text } => buf.push_str(text),
            NodeKind::Patch { path, summary } => {
                buf.push_str(path);
                buf.push('\u{1e}');
                buf.push_str(summary);
            }
            NodeKind::ToolCall { name, arguments } => {
                buf.push_str(name);
                buf.push('\u{1e}');
                buf.push_str(arguments);
            }
            NodeKind::Verification { passed, detail } => {
                buf.push_str(if *passed { "pass" } else { "fail" });
                buf.push('\u{1e}');
                buf.push_str(detail);
            }
            NodeKind::Fork { reason } => buf.push_str(reason),
            NodeKind::Compacted {
                start,
                end,
                summary,
            } => {
                buf.push_str(&format!("{start}..{end}"));
                buf.push('\u{1e}');
                buf.push_str(summary);
            }
        }
        hash_str(&buf)
    }

    /// Make every staged node visible and advance the head.
    pub fn commit(&mut self) -> Result<Vec<NodeId>, SessionError> {
        if self.pending.is_empty() {
            return Ok(Vec::new());
        }
        let staged = std::mem::take(&mut self.pending);
        let mut committed = Vec::with_capacity(staged.len());

        for mut node in staged {
            if self.nodes.contains_key(&node.id) {
                return Err(SessionError::DuplicateNode(node.id));
            }
            for parent in &node.parents {
                if !self.nodes.contains_key(parent) {
                    return Err(SessionError::MissingParent(*parent));
                }
            }
            node.content_hash = Self::compute_content_hash(&node);

            // State hash folds in the parents' state hashes, so any change to
            // ancestry invalidates every descendant.
            let mut acc = node.content_hash;
            for parent in &node.parents {
                acc = fnv1a64(&acc.to_le_bytes());
                acc ^= self.nodes[parent].state_hash;
                acc = acc.wrapping_mul(0x100_0000_01b3);
            }
            node.state_hash = acc;

            let id = node.id;
            self.nodes.insert(id, node);
            self.head = Some(id);
            committed.push(id);
        }
        Ok(committed)
    }

    /// Create a new branch from `at`, leaving this session's head on the new
    /// branch. History up to `at` is shared, not copied.
    pub fn fork(&mut self, at: NodeId, reason: &str) -> Result<NodeId, SessionError> {
        if !self.nodes.contains_key(&at) {
            return Err(SessionError::UnknownNode(at));
        }
        let marker = NodeKind::Fork {
            reason: reason.to_string(),
        };
        self.head = Some(at);
        self.push(marker);
        let id = self.pending[0].id;
        self.commit()?;
        self.detached = true;
        Ok(id)
    }

    /// Move the head back to `at` **without** creating a branch marker. The
    /// nodes after `at` remain in the log; the next `push` starts a new branch
    /// that still references `at`.
    pub fn rollback(&mut self, at: NodeId) -> Result<(), SessionError> {
        if !self.nodes.contains_key(&at) {
            return Err(SessionError::UnknownNode(at));
        }
        self.head = Some(at);
        self.detached = true;
        Ok(())
    }

    /// The last node on the current path whose verification passed — the
    /// safe place to resume from after a dead end.
    pub fn last_verified(&self) -> Result<Option<NodeId>, SessionError> {
        let path = self.current_path()?;
        Ok(path.iter().rev().copied().find(|id| {
            matches!(
                self.nodes[id].kind,
                NodeKind::Verification { passed: true, .. }
            )
        }))
    }

    /// Roll back to the last verified node, or to the root if none exists.
    pub fn rollback_to_last_verified(&mut self) -> Result<Option<NodeId>, SessionError> {
        match self.last_verified()? {
            Some(id) => {
                self.rollback(id)?;
                Ok(Some(id))
            }
            None => {
                // No verification on this path — rewind to the first node.
                let path = self.current_path()?;
                if let Some(root) = path.first().copied() {
                    self.rollback(root)?;
                    Ok(Some(root))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Fold the stored trajectory forward from `from` to the head, returning
    /// the ordered labels. No tool is re-executed — results are reused, which
    /// is the upstream's defining property.
    pub fn replay(&self, from: Option<NodeId>) -> Result<Vec<String>, SessionError> {
        let path = self.current_path()?;
        let start = match from {
            Some(id) => {
                if !self.nodes.contains_key(&id) {
                    return Err(SessionError::UnknownNode(id));
                }
                let pos = path
                    .iter()
                    .position(|p| *p == id)
                    .ok_or(SessionError::NotOnCurrentPath(id))?;
                pos
            }
            None => 0,
        };
        Ok(path[start..]
            .iter()
            .map(|id| self.nodes[id].kind.label().to_string())
            .collect())
    }

    /// Recompute every state hash and compare against what was recorded.
    /// Detects out-of-band edits to the log.
    pub fn verify_integrity(&self) -> Result<(), SessionError> {
        for node in self.nodes.values() {
            let content = Self::compute_content_hash(node);
            if content != node.content_hash {
                return Err(SessionError::IntegrityViolation {
                    node: node.id,
                    expected: content,
                    actual: node.content_hash,
                });
            }
            let mut acc = content;
            for parent in &node.parents {
                acc = fnv1a64(&acc.to_le_bytes());
                acc ^= self.nodes[parent].state_hash;
                acc = acc.wrapping_mul(0x100_0000_01b3);
            }
            if acc != node.state_hash {
                return Err(SessionError::IntegrityViolation {
                    node: node.id,
                    expected: acc,
                    actual: node.state_hash,
                });
            }
        }
        Ok(())
    }

    /// Children of `id`, in id order.
    pub fn children_of(&self, id: NodeId) -> Vec<NodeId> {
        self.nodes
            .values()
            .filter(|n| n.parents.contains(&id))
            .map(|n| n.id)
            .collect()
    }

    /// Return the active branch as a `ContextSurface`.
    pub fn context_surface(&self) -> Result<ContextSurface, SessionError> {
        let path = self.current_path()?;
        let mut nodes = Vec::with_capacity(path.len());
        for id in path {
            nodes.push(
                self.nodes
                    .get(&id)
                    .ok_or(SessionError::UnknownNode(id))?
                    .clone(),
            );
        }
        Ok(ContextSurface::from_nodes(nodes))
    }

    /// Record cost and latency telemetry on an existing node.
    pub fn record_telemetry(
        &mut self,
        id: NodeId,
        cost_micros: Option<u64>,
        latency_ms: Option<u64>,
    ) -> Result<(), SessionError> {
        let node = self
            .nodes
            .get_mut(&id)
            .ok_or(SessionError::UnknownNode(id))?;
        if cost_micros.is_some() {
            node.cost_micros = cost_micros;
        }
        if latency_ms.is_some() {
            node.latency_ms = latency_ms;
        }
        Ok(())
    }

    /// Record a deterministic judge_verified_hash on an existing node.
    pub fn record_judge_verified(
        &mut self,
        id: NodeId,
        verified_hash: u64,
    ) -> Result<(), SessionError> {
        let node = self
            .nodes
            .get_mut(&id)
            .ok_or(SessionError::UnknownNode(id))?;
        node.judge_verified_hash = Some(verified_hash);
        Ok(())
    }

    /// Sum of all recorded cost_micros in this session.
    pub fn total_cost_micros(&self) -> u64 {
        self.nodes.values().filter_map(|n| n.cost_micros).sum()
    }

    /// Sum of all recorded latency_ms in this session.
    pub fn total_latency_ms(&self) -> u64 {
        self.nodes.values().filter_map(|n| n.latency_ms).sum()
    }

    /// Serialize to a JSONL-compatible string: one node per line.
    /// Uses strict RFC-8259 escaping via serde_json to ensure arbitrary payloads
    /// with nested braces, quotes, Windows CRLF, and Unicode round-trip cleanly.
    pub fn to_jsonl(&self) -> String {
        let mut out = String::new();
        for node in self.nodes.values() {
            if let Ok(line) = serde_json::to_string(node) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        out
    }

    /// Parse the `to_jsonl` form back.
    /// Uses serde_json for strict RFC-8259 parsing, and falls back to
    /// legacy field parsing if a line is from an older version.
    pub fn from_jsonl(raw: &str) -> Result<Session, SessionError> {
        let mut session = Session::new();

        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(node) = serde_json::from_str::<Node>(line) {
                if session.nodes.contains_key(&node.id) {
                    return Err(SessionError::DuplicateNode(node.id));
                }
                for parent in &node.parents {
                    if !session.nodes.contains_key(parent) {
                        return Err(SessionError::MissingParent(*parent));
                    }
                }
                let id = node.id;
                session.next_id = session.next_id.max(id + 1);
                session.head = Some(id);
                session.nodes.insert(id, node);
                continue;
            }

            // Legacy line fallback
            let id = field_u64(line, "id")?;
            let label = field_string(line, "label")?;
            let parents = field_u64_list(line, "parents")?;

            if session.nodes.contains_key(&id) {
                return Err(SessionError::DuplicateNode(id));
            }
            for parent in &parents {
                if !session.nodes.contains_key(parent) {
                    return Err(SessionError::MissingParent(*parent));
                }
            }
            session.nodes.insert(
                id,
                Node {
                    id,
                    parents,
                    kind: NodeKind::Thought { text: label },
                    content_hash: field_u64(line, "content_hash")?,
                    state_hash: field_u64(line, "state_hash")?,
                    cost_micros: None,
                    latency_ms: None,
                    judge_verified_hash: None,
                },
            );
            session.next_id = session.next_id.max(id + 1);
            session.head = Some(id);
        }
        Ok(session)
    }

    /// Labels of every node, in id order — the human-readable trajectory.
    pub fn kind_labels(&self) -> Vec<String> {
        self.nodes
            .values()
            .map(|n| n.kind.label().to_string())
            .collect()
    }

    /// Whether the head was moved backwards by fork/rollback.
    pub fn is_detached(&self) -> bool {
        self.detached
    }
}

// ---- minimal field extraction (avoids a serde_json dependency here) -------

fn find_field(line: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":", key);
    let start = line.find(&needle)? + needle.len();
    Some(line[start..].to_string())
}

fn field_u64(line: &str, key: &str) -> Result<u64, SessionError> {
    let raw = find_field(line, key).ok_or(SessionError::UnknownNode(0))?;
    // find_field returns everything to end-of-line, so take only the digits up
    // to the first delimiter rather than trimming trailing punctuation.
    let digits: String = raw
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits
        .parse::<u64>()
        .map_err(|_| SessionError::UnknownNode(0))
}

fn field_string(line: &str, key: &str) -> Result<String, SessionError> {
    let raw = find_field(line, key).ok_or(SessionError::UnknownNode(0))?;
    // Everything up to the first `"` after the opening quote is the value.
    let inner = raw
        .trim_start()
        .strip_prefix('"')
        .unwrap_or(raw.trim_start());
    let value: String = inner.chars().take_while(|c| *c != '"').collect();
    Ok(value)
}

fn field_u64_list(line: &str, key: &str) -> Result<Vec<u64>, SessionError> {
    let raw = find_field(line, key).ok_or(SessionError::UnknownNode(0))?;
    // Take only the bracketed group; anything after `]` belongs to later fields.
    let trimmed = raw.trim_start();
    let after_open = trimmed.strip_prefix('[').unwrap_or(trimmed);
    let inner: String = after_open.chars().take_while(|c| *c != ']').collect();
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    Ok(inner
        .split(',')
        .filter_map(|p| p.trim().parse::<u64>().ok())
        .collect())
}

// ---- ContextSurface view layer ----------------------------------------------

/// Upstream durable constant for compacted prompt formatting (pi/packages/durable/src/harness/compaction.ts).
pub const SUMMARY_PREFIX: &str =
    "The conversation history before this point was compacted into the following summary:\n\n<summary>\n";
pub const SUMMARY_SUFFIX: &str = "\n</summary>";

/// An item on the context surface view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "surface_type", rename_all = "snake_case")]
pub enum SurfaceItem {
    /// An uncompacted DAG node on the active branch.
    Node(Node),
    /// A compacted range of nodes represented as a summary.
    Summary {
        start: NodeId,
        end: NodeId,
        summary: String,
    },
}

impl SurfaceItem {
    pub fn id_range(&self) -> (NodeId, NodeId) {
        match self {
            SurfaceItem::Node(n) => (n.id, n.id),
            SurfaceItem::Summary { start, end, .. } => (*start, *end),
        }
    }
}

/// A compaction-aware view layer over the active DAG branch.
///
/// Keeps the underlying multi-parent DAG with 64-bit FNV-1a ancestry hashes
/// intact for full auditability, rollback, and replay, while providing a
/// collapsed context view for LLM prompt construction without mutating or
/// losing DAG nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextSurface {
    pub items: Vec<SurfaceItem>,
}

impl ContextSurface {
    /// Create a surface view directly from an ordered sequence of nodes on the active branch.
    pub fn from_nodes(nodes: Vec<Node>) -> Self {
        Self {
            items: nodes.into_iter().map(SurfaceItem::Node).collect(),
        }
    }

    /// Collapse a range of nodes [start_node, end_node] into a single summary item.
    pub fn compact_range(
        &mut self,
        start_node: NodeId,
        end_node: NodeId,
        summary_payload: &str,
    ) -> Result<(), SessionError> {
        let start_pos = self.items.iter().position(|item| match item {
            SurfaceItem::Node(n) => n.id == start_node,
            SurfaceItem::Summary { start, .. } => *start == start_node,
        });
        let end_pos = self.items.iter().position(|item| match item {
            SurfaceItem::Node(n) => n.id == end_node,
            SurfaceItem::Summary { end, .. } => *end == end_node,
        });

        let (s_idx, e_idx) = match (start_pos, end_pos) {
            (Some(s), Some(e)) if s <= e => (s, e),
            _ => {
                return Err(SessionError::InvalidCompactionRange {
                    start: start_node,
                    end: end_node,
                })
            }
        };

        let summary_item = SurfaceItem::Summary {
            start: start_node,
            end: end_node,
            summary: summary_payload.to_string(),
        };

        // Replace the slice with the single summary item
        self.items
            .splice(s_idx..=e_idx, std::iter::once(summary_item));
        Ok(())
    }

    /// Number of surface items (compacted summaries count as 1 item).
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Render the surface into prompt context for an LLM.
    pub fn to_prompt_context(&self) -> String {
        let mut out = String::new();
        for item in &self.items {
            match item {
                SurfaceItem::Summary { summary, .. } => {
                    out.push_str(SUMMARY_PREFIX);
                    out.push_str(summary);
                    out.push_str(SUMMARY_SUFFIX);
                    out.push_str("\n\n");
                }
                SurfaceItem::Node(node) => match &node.kind {
                    NodeKind::Prompt { role, text } => {
                        out.push_str(&format!("[{role}]: {text}\n\n"));
                    }
                    NodeKind::Thought { text } => {
                        out.push_str(&format!("[thought]: {text}\n\n"));
                    }
                    NodeKind::ToolCall { name, arguments } => {
                        out.push_str(&format!("[tool_call:{name}]: {arguments}\n\n"));
                    }
                    NodeKind::Observation { text } => {
                        out.push_str(&format!("[observation]: {text}\n\n"));
                    }
                    NodeKind::Patch { path, summary } => {
                        out.push_str(&format!("[patch:{path}]: {summary}\n\n"));
                    }
                    NodeKind::Verification { passed, detail } => {
                        out.push_str(&format!(
                            "[verification:{}]: {detail}\n\n",
                            if *passed { "pass" } else { "fail" }
                        ));
                    }
                    NodeKind::Fork { reason } => {
                        out.push_str(&format!("[fork]: {reason}\n\n"));
                    }
                    NodeKind::Compacted { summary, .. } => {
                        out.push_str(SUMMARY_PREFIX);
                        out.push_str(summary);
                        out.push_str(SUMMARY_SUFFIX);
                        out.push_str("\n\n");
                    }
                },
            }
        }
        out
    }
}
