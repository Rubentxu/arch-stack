//! ReactiveObserver trait — contract for all cognitive agents.
//!
//! v1.0 (M18): synchronous dispatch via EventDispatcher. The subscription
//! matcher filters agents first; then `matches()` is called on each candidate.
//! `triggering_event` in `AgentContext` carries the dispatched event type.

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
    /// Called by EventDispatcher after subscription matching confirms eligibility.
    /// v1.0: always true for direct invoke.
    fn matches(&self, _context: &AgentContext) -> bool {
        true
    }

    /// Run the agent observation and produce output.
    /// v1.0: synchronous. M18 will make this `async fn`.
    fn observe(&self, context: &AgentContext) -> Result<AgentOutput, ObserveError>;
}

/// A no-op observer that always returns NoAction with a low-confidence
/// reason. Implements the NullObject pattern: useful as a placeholder
/// during agent registration tests and as a deterministic fallback in
/// production when no real observer is configured for a given event.
pub struct NoopObserver {
    pub descriptor: AgentDescriptor,
}

impl ReactiveObserver for NoopObserver {
    fn descriptor(&self) -> AgentDescriptor {
        self.descriptor.clone()
    }

    fn observe(&self, _context: &AgentContext) -> Result<AgentOutput, ObserveError> {
        Ok(AgentOutput::NoAction(super::output::NoActionReason {
            code: super::output::NoActionCode::InsufficientConfidence,
            message: "noop observer".into(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_observer_returns_no_action() {
        use super::super::descriptor::{AgentBudget, ModelPolicy};
        let stub = NoopObserver {
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
        // REQ-T06-003: feedback_history plumbing — see AgentContext::with_feedback_history
        // REQ-M25-006: pending_adjudications wiring (TRUST-008 REQ-T08-005). Observer contexts
        // are read-only; the field is intentionally empty.
        // recent_events (M34 W2) populated by compress_for_budget before dispatch.
        let ctx = AgentContext {
            goal: "test".into(),
            triggering_event: None,
            graph_view: Default::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        };
        let out = stub.observe(&ctx).unwrap();
        assert!(matches!(out, AgentOutput::NoAction(_)));
    }

    // -----------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage, 2026-08-22)
    // -----------------------------------------------------------------------

    fn descriptor_fixture() -> AgentDescriptor {
        use super::super::descriptor::{AgentBudget, ModelPolicy};
        AgentDescriptor {
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
        }
    }

    fn ctx_fixture() -> AgentContext {
        AgentContext {
            goal: "test".into(),
            triggering_event: None,
            graph_view: Default::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: super::super::descriptor::AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        }
    }

    #[test]
    fn observe_error_budget_exceeded_display() {
        let err = ObserveError::BudgetExceeded("tokens=50000".into());
        assert_eq!(err.to_string(), "budget exceeded: tokens=50000");
    }

    #[test]
    fn observe_error_tool_unavailable_display() {
        let err = ObserveError::ToolUnavailable("ast-grep".into());
        assert_eq!(err.to_string(), "tool unavailable: ast-grep");
    }

    #[test]
    fn observe_error_insufficient_context_display() {
        let err = ObserveError::InsufficientContext("missing goal".into());
        assert_eq!(err.to_string(), "context insufficient: missing goal");
    }

    #[test]
    fn observe_error_internal_display() {
        let err = ObserveError::Internal("panic during parse".into());
        assert_eq!(err.to_string(), "internal: panic during parse");
    }

    #[test]
    fn noop_observer_descriptor_returns_stored_value() {
        let stub = NoopObserver {
            descriptor: descriptor_fixture(),
        };
        let d = stub.descriptor();
        assert_eq!(d.id, "stub");
        assert_eq!(d.version, "0.1.0");
    }

    #[test]
    fn reactive_observer_default_matches_returns_true() {
        struct TrivialObserver;
        impl ReactiveObserver for TrivialObserver {
            fn descriptor(&self) -> AgentDescriptor {
                descriptor_fixture()
            }
            fn observe(&self, _ctx: &AgentContext) -> Result<AgentOutput, ObserveError> {
                Ok(AgentOutput::NoAction(
                    super::super::output::NoActionReason {
                        code: super::super::output::NoActionCode::NoRelevantData,
                        message: "trivial".into(),
                    },
                ))
            }
        }
        let obs = TrivialObserver;
        assert!(
            obs.matches(&ctx_fixture()),
            "default ReactiveObserver::matches() must return true"
        );
    }

    #[test]
    fn noop_observer_observe_returns_insufficient_confidence() {
        let stub = NoopObserver {
            descriptor: descriptor_fixture(),
        };
        let out = stub.observe(&ctx_fixture()).unwrap();
        if let AgentOutput::NoAction(reason) = out {
            assert!(matches!(
                reason.code,
                super::super::output::NoActionCode::InsufficientConfidence
            ));
            assert_eq!(reason.message, "noop observer");
        } else {
            panic!("expected NoAction, got {:?}", out);
        }
    }

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage v3, 2026-08-22)
    // ---------------------------------------------------------------------------

    /// `ObserveError`'s `Debug` impl includes the variant name and inner
    /// payload. Locks the contract that downstream `tracing::error!(?err)`
    /// produces searchable logs.
    #[test]
    fn observe_error_debug_includes_variant_and_payload() {
        let err = ObserveError::BudgetExceeded("tokens=50000".into());
        let dbg = format!("{err:?}");
        assert!(
            dbg.contains("BudgetExceeded"),
            "Debug must include variant: {dbg}"
        );
        assert!(
            dbg.contains("tokens=50000"),
            "Debug must include payload: {dbg}"
        );
    }

    /// A `NoopObserver` with a custom `id` returns it from `descriptor()`.
    /// Distinct from `descriptor_returns_stored_value` which uses the
    /// shared fixture (id="stub"); this confirms id is per-instance.
    #[test]
    fn noop_observer_preserves_custom_descriptor_id() {
        let custom_desc = AgentDescriptor {
            id: "my-custom-noop".into(),
            ..descriptor_fixture()
        };
        let stub = NoopObserver {
            descriptor: custom_desc,
        };
        let got = stub.descriptor();
        assert_eq!(got.id.as_str(), "my-custom-noop");
    }

    /// An observer with an EXPLICIT `matches()` override returning `false`
    /// is short-circuited. The default impl returning `true` is documented
    /// in `reactive_observer_default_matches_returns_true`; this confirms
    /// the override path.
    #[test]
    fn reactive_observer_explicit_matches_false_override() {
        struct DecliningObserver;
        impl ReactiveObserver for DecliningObserver {
            fn descriptor(&self) -> AgentDescriptor {
                descriptor_fixture()
            }
            fn matches(&self, _ctx: &AgentContext) -> bool {
                false
            }
            fn observe(&self, _ctx: &AgentContext) -> Result<AgentOutput, ObserveError> {
                panic!("observe() must not run when matches() returns false")
            }
        }
        let obs = DecliningObserver;
        assert!(!obs.matches(&ctx_fixture()));
    }

    /// `ObserveError` Display preserves special characters (quotes,
    /// backslashes, newlines) verbatim — important for error messages
    /// containing tool output or file paths.
    #[test]
    fn observe_error_display_preserves_special_chars() {
        let err = ObserveError::Internal("path=\"C:\\Users\\foo\"\nline:42".into());
        let msg = err.to_string();
        assert_eq!(
            msg, "internal: path=\"C:\\Users\\foo\"\nline:42",
            "special chars must round-trip through Display"
        );
    }

    /// An empty payload is valid (Display returns just the prefix). Locks
    /// the behavior when an observer returns `Internal("")` or similar.
    #[test]
    fn observe_error_display_empty_payload() {
        let err = ObserveError::InsufficientContext("".into());
        assert_eq!(err.to_string(), "context insufficient: ");
    }

    /// `ReactiveObserver` requires `Send + Sync`. This is enforced at
    /// compile time by the trait bound. The `assert_send_sync` helper
    /// would fail to compile if the bound were weakened.
    #[test]
    fn reactive_observer_trait_requires_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NoopObserver>();
        assert_send_sync::<Box<dyn ReactiveObserver>>();
    }
}
