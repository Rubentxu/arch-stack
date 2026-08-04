//! Policy module — Policy Engine for governed action execution.
//!
//! v1.0: field-equality rules only, no comparison DSL.
//! Mirrors the escalation/ladder.rs pattern (TOML load + first-match evaluate).

pub use super::output::{ApprovalLevel, ApprovalRequirement, DeploymentEnv, SecurityImpact};
pub use context::{CostCeiling, PolicyContext};
pub use decision::{PolicyDecision, PolicyResult};
pub use engine::{Policy, PolicyEngine};

mod context;
mod decision;
mod engine;
