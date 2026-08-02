//! Deterministic code analysis: C4 boundary inference + call graph extraction.

pub mod call_graph;
pub mod c4_discover;
pub mod call_rules;
pub mod class_diagram;
pub mod output;
pub mod sequence;
pub mod strategies;

// Public re-exports
pub use c4_discover::{discover, Container, ContainerCandidate, DiscoverError, DiscoverReport, ProjectMeta};
pub use strategies::{register_strategies, Strategy};
