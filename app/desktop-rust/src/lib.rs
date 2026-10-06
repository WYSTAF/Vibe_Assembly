//! Mission Control — a native desktop dashboard for Vibe Assembly.
//!
//! The parser and state layers are exposed as a library so they can be
//! tested directly, without standing up a GPU window.

pub mod lock;
pub mod parser;
pub mod router;
pub mod state;
pub mod theme;
pub mod ui;
