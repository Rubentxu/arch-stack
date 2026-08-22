//! Agent registry and synchronous dispatcher.

use std::collections::HashMap;

use crate::cognitive::context::AgentContext;
use crate::cognitive::descriptor::AgentDescriptor;
use crate::cognitive::observer::ReactiveObserver;
use crate::cognitive::output::AgentOutput;

/// In-memory registry of available agents.
#[derive(Default)]
pub struct AgentRegistry {
    agents: HashMap<String, Box<dyn ReactiveObserver>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a agent. Panics if id already exists.
    pub fn register(&mut self, agent: impl ReactiveObserver + 'static) {
        let id = agent.descriptor().id.clone();
        if self.agents.contains_key(&id) {
            panic!("agent already registered: {id}");
        }
        self.agents.insert(id, Box::new(agent));
    }

    /// Get a agent descriptor by id.
    pub fn get(&self, id: &str) -> Option<AgentDescriptor> {
        self.agents.get(id).map(|a| a.descriptor())
    }

    /// List all registered agent ids.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.agents.keys().map(|s| s.as_str())
    }

    /// Direct synchronous invocation of a specific agent by id.
    /// v1.0: dispatcher calls this directly. M18 will route via event bus.
    pub fn invoke(
        &self,
        agent_id: &str,
        context: &AgentContext,
    ) -> Result<AgentOutput, DispatchError> {
        let agent = self
            .agents
            .get(agent_id)
            .ok_or_else(|| DispatchError::AgentNotFound(agent_id.to_string()))?;

        if !agent.matches(context) {
            return Ok(AgentOutput::NoAction(
                crate::cognitive::output::NoActionReason {
                    code: crate::cognitive::output::NoActionCode::OutOfScope,
                    message: format!("agent {} does not match context", agent_id),
                },
            ));
        }

        agent.observe(context).map_err(DispatchError::ObserveFailed)
    }
}

/// Errors from dispatch operations.
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("agent not found: {0}")]
    AgentNotFound(String),
    #[error("agent observation failed: {0}")]
    ObserveFailed(crate::cognitive::observer::ObserveError),
}

/// Synchronous dispatcher that runs a goal through all matching agents.
pub struct SyncDispatcher<'a> {
    registry: &'a AgentRegistry,
}

impl<'a> SyncDispatcher<'a> {
    pub fn new(registry: &'a AgentRegistry) -> Self {
        Self { registry }
    }

    /// Dispatch a goal to all matching agents, returning the first actionable output.
    /// v1.0: returns first non-NoAction output, or NoAction if all agents decline.
    pub fn dispatch(&self, context: &AgentContext) -> Result<AgentOutput, DispatchError> {
        let mut best: Option<AgentOutput> = None;

        for id in self.registry.ids() {
            let out = self.registry.invoke(id, context)?;
            if !matches!(out, AgentOutput::NoAction(_)) {
                best = Some(out);
                break;
            }
        }

        Ok(best.unwrap_or_else(|| {
            AgentOutput::NoAction(crate::cognitive::output::NoActionReason {
                code: crate::cognitive::output::NoActionCode::InsufficientConfidence,
                message: "no agent produced output".into(),
            })
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::descriptor::{AgentBudget, ModelPolicy};
    use crate::cognitive::observer::NoopObserver;

    fn make_ctx(goal: &str) -> AgentContext {
        // REQ-T06-003: feedback_history plumbing — see AgentContext::with_feedback_history
        // REQ-M25-006: pending_adjudications wiring (TRUST-008 REQ-T08-005). Registry-level
        // context construction leaves the field empty; re-invoked agents fetch via dispatcher.
        AgentContext {
            goal: goal.into(),
            triggering_event: None,
            graph_view: Default::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
        }
    }

    #[test]
    fn registry_register_and_get() {
        let mut reg = AgentRegistry::new();
        let stub = NoopObserver {
            descriptor: AgentDescriptor {
                id: "test-agent".into(),
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
        reg.register(stub);
        let found = reg.get("test-agent");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id.as_str(), "test-agent");
        assert!(reg.get("missing").is_none());
    }

    #[test]
    fn registry_invoke_not_found() {
        let reg = AgentRegistry::new();
        let ctx = make_ctx("test goal");
        let err = reg.invoke("missing", &ctx).unwrap_err();
        assert!(matches!(err, DispatchError::AgentNotFound(_)));
    }

    #[test]
    fn dispatcher_picks_first_actionable() {
        let mut reg = AgentRegistry::new();
        reg.register(NoopObserver {
            descriptor: AgentDescriptor {
                id: "noop-1".into(),
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
        });

        let disp = SyncDispatcher::new(&reg);
        let ctx = make_ctx("coupling analysis");
        let out = disp.dispatch(&ctx).unwrap();
        // NoopObserver always returns NoAction
        assert!(matches!(out, AgentOutput::NoAction(_)));
    }

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage, 2026-08-22)
    // ---------------------------------------------------------------------------

    fn descriptor_with_id(id: &str) -> AgentDescriptor {
        AgentDescriptor {
            id: id.into(),
            version: "0.1.0".into(),
            subscriptions: vec![],
            required_views: vec![],
            output_schema: "{}".into(),
            model_policy: ModelPolicy::Heuristic,
            budget: AgentBudget::default(),
            capabilities: vec![],
            deterministic: true,
            idempotent: true,
        }
    }

    /// Registering the same agent id twice panics with a clear message.
    /// Per spec: "Panics if id already exists."
    #[test]
    #[should_panic(expected = "agent already registered")]
    fn registry_register_duplicate_panics() {
        let mut reg = AgentRegistry::new();
        reg.register(NoopObserver {
            descriptor: descriptor_with_id("dup-agent"),
        });
        reg.register(NoopObserver {
            descriptor: descriptor_with_id("dup-agent"),
        });
    }

    /// `ids()` iterates all registered agent ids in insertion order.
    /// (HashMap preserves no order, but `count()` is order-independent.)
    #[test]
    fn registry_ids_iterates_all() {
        let mut reg = AgentRegistry::new();
        reg.register(NoopObserver {
            descriptor: descriptor_with_id("alpha"),
        });
        reg.register(NoopObserver {
            descriptor: descriptor_with_id("beta"),
        });
        reg.register(NoopObserver {
            descriptor: descriptor_with_id("gamma"),
        });

        let mut ids: Vec<&str> = reg.ids().collect();
        ids.sort();
        assert_eq!(ids, vec!["alpha", "beta", "gamma"]);
    }

    /// A custom observer whose `matches()` returns false short-circuits to
    /// `NoAction(OutOfScope)` without calling `observe()`.
    struct MismatchObserver {
        descriptor: AgentDescriptor,
    }
    impl ReactiveObserver for MismatchObserver {
        fn descriptor(&self) -> AgentDescriptor {
            self.descriptor.clone()
        }
        fn matches(&self, _context: &AgentContext) -> bool {
            false
        }
        fn observe(
            &self,
            _context: &AgentContext,
        ) -> Result<AgentOutput, crate::cognitive::observer::ObserveError> {
            // Should never be called when matches() returns false.
            panic!("observe() must not be called when matches() returns false");
        }
    }

    #[test]
    fn registry_invoke_returns_noaction_when_mismatch() {
        let mut reg = AgentRegistry::new();
        reg.register(MismatchObserver {
            descriptor: descriptor_with_id("mismatch"),
        });

        let ctx = make_ctx("any goal");
        let out = reg.invoke("mismatch", &ctx).unwrap();
        assert!(matches!(
            out,
            AgentOutput::NoAction(crate::cognitive::output::NoActionReason {
                code: crate::cognitive::output::NoActionCode::OutOfScope,
                ..
            })
        ));
    }

    /// `SyncDispatcher::dispatch` returns `NoAction(InsufficientConfidence)` when
    /// every registered agent declines. Existing test only verifies the broad
    /// `NoAction(_)` variant; this confirms the specific reason code.
    #[test]
    fn dispatcher_dispatch_all_noaction_returns_insufficient_confidence() {
        let mut reg = AgentRegistry::new();
        reg.register(NoopObserver {
            descriptor: descriptor_with_id("n1"),
        });
        reg.register(NoopObserver {
            descriptor: descriptor_with_id("n2"),
        });

        let disp = SyncDispatcher::new(&reg);
        let ctx = make_ctx("any goal");
        let out = disp.dispatch(&ctx).unwrap();
        assert!(matches!(
            out,
            AgentOutput::NoAction(crate::cognitive::output::NoActionReason {
                code: crate::cognitive::output::NoActionCode::InsufficientConfidence,
                ..
            })
        ));
    }

    /// `DispatchError::AgentNotFound` Display includes the agent id.
    #[test]
    fn dispatch_error_display_agent_not_found() {
        let err = DispatchError::AgentNotFound("missing-agent".to_string());
        let msg = format!("{}", err);
        assert!(
            msg.contains("agent not found"),
            "Display must include 'agent not found', got: {msg}"
        );
        assert!(
            msg.contains("missing-agent"),
            "Display must include the agent id, got: {msg}"
        );
    }

    /// `DispatchError::ObserveFailed` Display delegates to the inner `ObserveError`.
    /// Confirms the wrapping error surfaces the underlying cause verbatim.
    #[test]
    fn dispatch_error_display_observe_failed() {
        let inner = crate::cognitive::observer::ObserveError::Internal("boom".to_string());
        let err = DispatchError::ObserveFailed(inner);
        let msg = format!("{}", err);
        assert!(
            msg.contains("agent observation failed"),
            "Display must include 'agent observation failed', got: {msg}"
        );
        assert!(
            msg.contains("boom"),
            "Display must include the inner error message, got: {msg}"
        );
    }
}
