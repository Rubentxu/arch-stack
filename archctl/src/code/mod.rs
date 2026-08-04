//! Deterministic code analysis: C4 boundary inference + call graph extraction.

pub mod apply_common;
pub mod c4_discover;
pub mod call_graph;
pub mod call_rules;
pub mod class_diagram;
pub mod output;
pub mod sequence;
pub mod state_machine;
pub mod strategies;

// Public re-exports
pub use c4_discover::{
    Container, ContainerCandidate, DiscoverError, DiscoverReport, ProjectMeta, discover,
};
pub use state_machine::{Language, StateMachine, StateMachineReport, apply, extract};
pub use strategies::{Strategy, register_strategies};
