//! Core state engine, parser, router client, and allocator for Vibe Assembly.
//! Pure Rust library with zero Node.js/npm dependencies.

pub mod allocator;
pub mod bootstrap;
pub mod knowledge;
pub mod lock;
pub mod parser;
pub mod prompts;
pub mod router;
pub mod router_client;
pub mod state;

// Re-export common types
pub use allocator::{allocate, classify_model, load_knowledge, AllocationResult, ModelKnowledge};
pub use bootstrap::{init_workspace, reset_template_state, TemplateInfo};
pub use knowledge::{auto_combo, score_model, Assignment, Fitness, Role as ModelRole};
pub use lock::{
    acquire_task_lock, check_task_lock, lock_path, LockInfo, LockStatus, TaskLockGuard,
};
pub use parser::{
    parse_active_task, parse_log_entries, parse_state_file, validate_contract, verified_steps,
    ActiveTask, QueueRow, Step, Ticket,
};
pub use prompts::Role;
pub use router::{fetch_live_assignment, resolve_assignment, RoleAllocation, RouterAssignment};
pub use router_client::{
    fetch_assignment, fetch_default, RouterAssignment as GatewayAssignment, RouterAuth,
    RouterStatus,
};
pub use state::Workspace;
