//! Cognitive layer — Agent contract types.
//!
//! v1.0: synchronous dispatcher, deterministic heuristics only.

mod agents;
pub mod audit;
pub mod context;
pub mod delta;
pub mod descriptor;
mod dispatcher;
mod escalation;
pub mod event; // M18 PR1: reactive runtime event types
pub mod mcp;
pub mod observer;
pub mod output;
pub mod policy; // M18 PR1: graph delta types

pub use agents::*;
pub use audit::*;
pub use context::*;
pub use descriptor::*;
pub use dispatcher::*;
pub use escalation::*;
pub use event::*;
pub use mcp::*;
pub use observer::*;
pub use output::*;
pub use policy::*;
