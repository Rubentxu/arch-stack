//! ReactiveObserver trait — contract for all cognitive agents.
//!
//! v1.0: synchronous dispatch only. The async/event-driven seam is
//! designed (trait shape) but backed by SyncDispatcher. When M18
//! arrives, replace SyncDispatcher with an event bus — no contract churn.

use super::context::AgentContext;
use super::descriptor::AgentDescriptor;
use super::output::AgentOutput;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObserveError {
    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),
    #[error("tool unavailable: {0}")]
    ToolUnavailable(String),
    #[error("context insufficient: {0}")]
    InsufficientContext(String),
    #[error("internal: {0}")]
    Internal(String),
}

/// Observer that reacts to graph state and produces structured output.
/// v1.0: always invoked synchronously via SyncDispatcher.
pub trait ReactiveObserver: Send + Sync {
    /// Static descriptor of this agent.
    fn descriptor(&self) -> AgentDescriptor;

    /// Whether this agent should run for the given context.
    /// v1.0: always true for direct invoke. M18 will use event patterns.
    fn matches(&self, _context: &AgentContext) -> bool {
        true
    }

    /// Run the agent observation and produce output.
    /// v1.0: synchronous. M18 will make this `async fn`.
    fn observe(&self, context: &AgentContext) -> Result<AgentOutput, ObserveError>;
}

/// A no-op agent used for testing and scaffolding.
pub struct StubAgent {
    pub descriptor: AgentDescriptor,
}

impl ReactiveObserver for StubAgent {
    fn descriptor(&self) -> AgentDescriptor {
        self.descriptor.clone()
    }

    fn observe(&self, _context: &AgentContext) -> Result<AgentOutput, ObserveError> {
        Ok(AgentOutput::NoAction(super::output::NoActionReason {
            code: super::output::NoActionCode::InsufficientConfidence,
            message: "stub agent".into(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_agent_returns_no_action() {
        use super::super::descriptor::{AgentBudget, ModelPolicy};
        let stub = StubAgent {
            descriptor: AgentDescriptor {
                id: "stub".into(),
                version: "0.1.0".into(),
                subscriptions: vec![],
                required_views: vec![],
                output_schema: "{}".into(),
                model_policy: ModelPolicy::Heuristic,
                budget: AgentBudget::default(),
                capabilities: vec![],
                deterministic: true,
                idempotent: true,
            },
        };
        let ctx = AgentContext {
            goal: "test".into(),
            triggering_event: None,
            graph_view: Default::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
        };
        let out = stub.observe(&ctx).unwrap();
        assert!(matches!(out, AgentOutput::NoAction(_)));
    }
}
