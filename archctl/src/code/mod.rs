//! Deterministic code analysis: C4 boundary inference + future call graph.

pub mod c4_discover;
pub mod output;
pub mod strategies;

// Public re-exports
pub use c4_discover::{discover, Container, ContainerCandidate, DiscoverError, DiscoverReport, ProjectMeta};
pub use strategies::{register_strategies, Strategy};
