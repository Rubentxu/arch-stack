//! Cognitive layer — Agent contract types.
//!
//! v1.0: synchronous dispatcher, deterministic heuristics only.

mod context;
pub mod descriptor;
mod dispatcher;
mod escalation;
mod mcp;
mod observer;
mod output;

pub use context::*;
pub use descriptor::*;
pub use dispatcher::*;
pub use escalation::*;
pub use mcp::*;
pub use observer::*;
pub use output::*;
