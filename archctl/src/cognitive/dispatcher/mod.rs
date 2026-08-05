//! Cognitive layer dispatcher — synchronous agent invocation.

mod registry;

pub use registry::*;

// M18 PR2: reactive event dispatcher
pub mod event_dispatcher;
pub use event_dispatcher::*;
