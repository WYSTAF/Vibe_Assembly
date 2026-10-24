//! Execution harness for Vibe Assembly.
//!
//! * [`session`] — append-only transactional trajectory DAG with fork, rollback,
//!   replay, and per-node state hashing.
//! * [`patcher`] — deterministic hash-anchored source patching with 1-hop
//!   structural recovery and hard rejection of ambiguous targets.
//! * [`review`] — git checkpointing and per-step delta isolation.
//! * [`judge`] — deterministic anti-slop and minimalism gatekeeper.
//! * [`engine`] — the Observe → Think → Patch → Verify → Judge loop.

pub mod diagnostics;
pub mod engine;
pub mod experience;
pub mod judge;
pub mod live_planner;
pub mod patcher;
pub mod repomap;
pub mod review;
pub mod sandbox;
pub mod session;
pub mod tournament;
