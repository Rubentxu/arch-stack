//! Cognitive layer — Agent contract types.
//!
//! v1.0: synchronous dispatcher, deterministic heuristics only.

mod context;
pub mod descriptor;
mod observer;
mod output;

pub use context::*;
pub use descriptor::*;
pub use observer::*;
pub use output::*;
