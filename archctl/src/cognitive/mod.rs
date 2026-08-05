//! Cognitive layer — Agent contract types.
//!
//! v1.0: synchronous dispatcher, deterministic heuristics only.

mod agents;
pub mod context;
pub mod descriptor;
mod dispatcher;
mod escalation;
mod mcp;
pub mod observer;
pub mod output;
pub mod policy;

pub use agents::*;
pub use context::*;
pub use descriptor::*;
pub use dispatcher::*;
pub use escalation::*;
pub use mcp::*;
pub use observer::*;
pub use output::*;
pub use policy::*;
