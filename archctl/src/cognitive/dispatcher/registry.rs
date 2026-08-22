//! Agent registry and synchronous dispatcher.

use std::collections::HashMap;

use crate::cognitive::context::{AgentContext, CompressionPolicy, DecisionPriority};
use crate::cognitive::descriptor::AgentDescriptor;
use crate::cognitive::event::EventLog;
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
    /// Optional event log for context compression (M34 W4).
    /// When `Some`, the dispatcher will compress the agent context before
    /// fan-out if the context has a token budget. Defaults to `None`.
    event_log: Option<EventLog>,
}

impl<'a> SyncDispatcher<'a> {
    pub fn new(registry: &'a AgentRegistry) -> Self {
        Self {
            registry,
            event_log: None,
        }
    }

    /// Attach an event log for context compression (M34 W4).
    /// The compression log is read-only; it provides recent events for
    /// causation-window traversal during budget compression.
    pub fn with_compression_log(mut self, ledger: EventLog) -> Self {
        self.event_log = Some(ledger);
        self
    }

    /// Dispatch a goal to all matching agents, returning the first actionable output.
    /// v1.0: returns first non-NoAction output, or NoAction if all agents decline.
    ///
    /// If `context.budget.tokens` is `Some` and `self.event_log` is `Some`,
    /// the context is cloned and compressed via `compress_for_budget` before
    /// fan-out. The original `&AgentContext` is left untouched. The clone cost
    /// is acceptable for the v1 sync path (not in a hot loop).
    pub fn dispatch(&self, context: &AgentContext) -> Result<AgentOutput, DispatchError> {
        // M34 W4: clone + compress if budget + event_log available
        let ctx_for_dispatch = if context.budget.tokens.is_some() {
            let mut ctx_clone = context.clone();
            let policy = CompressionPolicy {
                budget_chars: context.budget.tokens.unwrap_or(0) as usize * 4,
                preserve_causation_window: 3,
                decision_priority: DecisionPriority::RecencyOnly,
            };
            if let Some(ledger) = &self.event_log
                && let Err(e) = ctx_clone.compress_for_budget(&policy, ledger)
            {
                tracing::warn!(
                    error = %e,
                    "sync_dispatch: context compression failed, proceeding with uncompressed clone"
                );
            }
            ctx_clone
        } else {
            context.clone()
        };

        let mut best: Option<AgentOutput> = None;

        for id in self.registry.ids() {
            let out = self.registry.invoke(id, &ctx_for_dispatch)?;
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
    use crate::cognitive::event::{EventEnvelope, EventLog, SerializedEvent};
    use crate::cognitive::observer::NoopObserver;

    fn make_ctx(goal: &str) -> AgentContext {
        // REQ-T06-003: feedback_history plumbing — see AgentContext::with_feedback_history
        // REQ-M25-006: pending_adjudications wiring (TRUST-008 REQ-T08-005). Registry-level
        // context construction leaves the field empty; re-invoked agents fetch via dispatcher.
        // recent_events (M34 W2) populated by compress_for_budget before dispatch.
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
            recent_events: vec![],
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

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage v3, 2026-08-22)
    // ---------------------------------------------------------------------------

    /// `AgentRegistry::new()` is equivalent to `AgentRegistry::default()`.
    /// Both must produce an empty registry (no ids).
    #[test]
    fn registry_new_equiv_default() {
        let via_new = AgentRegistry::new();
        let via_default = AgentRegistry::default();
        assert_eq!(via_new.ids().count(), 0, "new() must yield empty registry");
        assert_eq!(
            via_default.ids().count(),
            0,
            "default() must yield empty registry"
        );
    }

    /// `registry.ids()` on a fresh registry is an empty iterator.
    /// Distinct from `ids_iterates_all` which exercises the non-empty case.
    #[test]
    fn registry_ids_empty_on_new() {
        let reg = AgentRegistry::new();
        let ids: Vec<&str> = reg.ids().collect();
        assert!(ids.is_empty(), "fresh registry must have no ids");
    }

    /// `registry.get()` returns the registered descriptor with all fields
    /// preserved. Distinct from `register_and_get` which only checks `id`.
    #[test]
    fn registry_get_returns_full_descriptor() {
        let mut reg = AgentRegistry::new();
        let mut descriptor = descriptor_with_id("full-desc");
        descriptor.version = "1.2.3".into();
        descriptor.deterministic = false;
        descriptor.idempotent = false;
        reg.register(NoopObserver { descriptor });

        let got = reg.get("full-desc").unwrap();
        assert_eq!(got.id.as_str(), "full-desc");
        assert_eq!(got.version.as_str(), "1.2.3");
        assert!(!got.deterministic);
        assert!(!got.idempotent);
    }

    /// `SyncDispatcher::dispatch` with an EMPTY registry returns
    /// `NoAction(InsufficientConfidence)` (since `best` stays None and the
    /// `unwrap_or_else` falls through to the "no agent produced output"
    /// branch). Distinct from `dispatcher_dispatch_all_noaction_returns_insufficient_confidence`
    /// which exercises the case where agents are registered but all decline.
    #[test]
    fn dispatcher_dispatch_empty_registry_returns_insufficient_confidence() {
        let reg = AgentRegistry::new();
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

    /// `SyncDispatcher::dispatch` returns the FIRST agent that produces a
    /// non-NoAction output. Verifies the deterministic ordering on hit:
    /// agent A returns NoAction, agent B returns Action → dispatch picks B.
    struct ActionObserver {
        descriptor: AgentDescriptor,
        output: AgentOutput,
    }
    impl ReactiveObserver for ActionObserver {
        fn descriptor(&self) -> AgentDescriptor {
            self.descriptor.clone()
        }
        fn observe(
            &self,
            _context: &AgentContext,
        ) -> Result<AgentOutput, crate::cognitive::observer::ObserveError> {
            Ok(self.output.clone())
        }
    }

    #[test]
    fn dispatcher_picks_first_non_noaction_output() {
        let mut reg = AgentRegistry::new();
        reg.register(ActionObserver {
            descriptor: descriptor_with_id("first"),
            output: AgentOutput::NoAction(crate::cognitive::output::NoActionReason {
                code: crate::cognitive::output::NoActionCode::OutOfScope,
                message: "skip".into(),
            }),
        });
        reg.register(ActionObserver {
            descriptor: descriptor_with_id("second"),
            output: AgentOutput::ProjectionSpec(crate::cognitive::output::ProjectionSpec {
                view_kind: crate::cognitive::output::ViewKind::Sequence,
                format: crate::cognitive::output::DiagramFormat::Mermaid,
                focus_elements: vec![],
                layout_hints: crate::cognitive::output::LayoutHints {
                    direction: None,
                    ranksep: None,
                    nodesep: None,
                },
            }),
        });

        let disp = SyncDispatcher::new(&reg);
        let out = disp.dispatch(&make_ctx("sequence diagram")).unwrap();
        assert!(
            matches!(out, AgentOutput::ProjectionSpec(_)),
            "expected ProjectionSpec from second agent, got {:?}",
            out
        );
    }

    /// `DispatchError::ObserveFailed` propagates the inner cause via Display
    /// (via `#[error("...: {0}")]`). The wrapped error's full message is
    /// embedded in the outer Display. (Note: `Error::source()` returns None
    /// because `ObserveFailed` lacks `#[source]` — the chain is purely
    /// Display-based.) Verifies the consumer-facing message includes both
    /// the wrapper prefix and the inner detail.
    #[test]
    fn dispatch_error_observe_failed_preserves_source() {
        let inner = crate::cognitive::observer::ObserveError::ToolUnavailable("ast-grep".into());
        let err = DispatchError::ObserveFailed(inner);
        let msg = err.to_string();
        assert!(
            msg.contains("agent observation failed"),
            "must include wrapper prefix, got: {msg}"
        );
        assert!(
            msg.contains("tool unavailable"),
            "must include ObserveError variant message, got: {msg}"
        );
        assert!(
            msg.contains("ast-grep"),
            "must include inner cause, got: {msg}"
        );
    }

    /// A custom observer whose `observe()` returns `Err` propagates that
    /// error through `registry.invoke()` wrapped as `DispatchError::ObserveFailed`.
    struct FailingObserver {
        descriptor: AgentDescriptor,
    }
    impl ReactiveObserver for FailingObserver {
        fn descriptor(&self) -> AgentDescriptor {
            self.descriptor.clone()
        }
        fn observe(
            &self,
            _context: &AgentContext,
        ) -> Result<AgentOutput, crate::cognitive::observer::ObserveError> {
            Err(crate::cognitive::observer::ObserveError::Internal(
                "kaboom".into(),
            ))
        }
    }

    #[test]
    fn registry_invoke_propagates_observer_error() {
        let mut reg = AgentRegistry::new();
        reg.register(FailingObserver {
            descriptor: descriptor_with_id("failing"),
        });
        let err = reg.invoke("failing", &make_ctx("any")).unwrap_err();
        assert!(matches!(err, DispatchError::ObserveFailed(_)));
        assert!(err.to_string().contains("kaboom"));
    }

    // ---------------------------------------------------------------------------
    // M34 W4 — compress_for_budget wiring tests for SyncDispatcher
    // ---------------------------------------------------------------------------

    /// Helper to make an EventEnvelope for testing.
    fn make_envelope(event_type: &str) -> EventEnvelope {
        EventEnvelope {
            event_id: uuid::Uuid::new_v4(),
            schema_version: "1.0".into(),
            timestamp: chrono::Utc::now(),
            source: "test".into(),
            producer: "test".into(),
            event_type: event_type.into(),
            payload: serde_json::json!({}),
            seq: 1,
            correlation_id: None,
            causation_id: None,
            graph_revision: None,
        }
    }

    /// Helper to make an AgentContext with budget tokens.
    fn make_ctx_with_budget(tokens: Option<u32>) -> AgentContext {
        AgentContext {
            goal: "test".into(),
            triggering_event: None,
            graph_view: Default::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget {
                tokens,
                ..Default::default()
            },
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        }
    }

    /// Dispatch with budget.tokens = Some(N) and event_log = Some should succeed.
    #[test]
    fn sync_dispatch_with_budget_compresses_context() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut comp_log = EventLog::open(tmp.path().join("comp.jsonl")).unwrap();
        for _ in 1..=5 {
            comp_log
                .append_serialized(&SerializedEvent::from_envelope(make_envelope(
                    "PreExisting",
                )))
                .unwrap();
        }
        let reg = AgentRegistry::new();
        let disp = SyncDispatcher::new(&reg).with_compression_log(comp_log);
        let ctx = make_ctx_with_budget(Some(100));
        let _ = disp.dispatch(&ctx);
    }

    /// Dispatch with budget.tokens = None should NOT trigger compression.
    #[test]
    fn sync_dispatch_without_budget_does_not_compress() {
        let tmp = tempfile::TempDir::new().unwrap();
        let comp_log = EventLog::open(tmp.path().join("comp.jsonl")).unwrap();
        let reg = AgentRegistry::new();
        let disp = SyncDispatcher::new(&reg).with_compression_log(comp_log);
        let ctx = make_ctx_with_budget(None);
        let out = disp.dispatch(&ctx).unwrap();
        assert!(matches!(out, AgentOutput::NoAction(_)));
    }

    /// When event_log is None, dispatch must not panic even if budget.tokens = Some.
    #[test]
    fn sync_dispatch_with_event_log_unavailable_skips_compression() {
        let reg = AgentRegistry::new();
        let disp = SyncDispatcher::new(&reg);
        let ctx = make_ctx_with_budget(Some(100));
        let out = disp.dispatch(&ctx).unwrap();
        assert!(matches!(out, AgentOutput::NoAction(_)));
    }

    /// Dispatch with causation-linked events should succeed (BFS traversal ok).
    #[test]
    fn sync_dispatch_preserves_causation_window_within_recent_events() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut comp_log = EventLog::open(tmp.path().join("comp.jsonl")).unwrap();
        let mut prev_id = uuid::Uuid::nil();
        for i in 1..=3 {
            let event_id = uuid::Uuid::new_v4();
            let env = EventEnvelope {
                event_id,
                schema_version: "1.0".into(),
                timestamp: chrono::Utc::now(),
                source: "test".into(),
                producer: "test".into(),
                event_type: format!("Event{}", i),
                payload: serde_json::json!({}),
                seq: i,
                correlation_id: None,
                causation_id: if i > 1 { Some(prev_id) } else { None },
                graph_revision: None,
            };
            prev_id = event_id;
            comp_log
                .append_serialized(&SerializedEvent::from_envelope(env))
                .unwrap();
        }
        let reg = AgentRegistry::new();
        let disp = SyncDispatcher::new(&reg).with_compression_log(comp_log);
        let ctx = make_ctx_with_budget(Some(100));
        let out = disp.dispatch(&ctx).unwrap();
        assert!(matches!(out, AgentOutput::NoAction(_)));
    }
}
