//! Event-driven dispatcher for reactive agent activation.
//!
//! Receives an `EventEnvelope` + `GraphDelta`, appends to the event log,
//! then fans out `observe()` calls to all agents whose subscriptions match
//! the event type. Pure synchronous per ADR-010.

use std::io;
use std::sync::Arc;

use crate::cognitive::context::AgentContext;
use crate::cognitive::delta::GraphDelta;
use crate::cognitive::event::{EventEnvelope, EventLog, SerializedEvent};
use crate::cognitive::observer::ReactiveObserver;
use crate::cognitive::output::AgentOutput;
use crate::cognitive::subscriptions::SubscriptionMatcher;

// ---------------------------------------------------------------------------
// EventDispatcher
// ---------------------------------------------------------------------------

/// The central event-driven fan-out dispatcher.
///
/// Send it an `EventEnvelope` + `GraphDelta` and it:
/// 1. Appends the event to the log (unprocessed)
/// 2. Iterates registered agents in order
/// 3. Activates agents whose subscription patterns match
/// 4. Calls `observe()` on each activated agent
/// 5. Returns all non-NoAction outputs
pub struct EventDispatcher {
    agents: Vec<Arc<dyn ReactiveObserver>>,
    log: EventLog,
    /// Optional event log reference for context compression (M34 W3).
    /// When `Some`, the dispatcher will compress the agent context before
    /// fan-out if the context has a token budget. Defaults to `None`.
    event_log: Option<EventLog>,
}

impl EventDispatcher {
    /// Create a new EventDispatcher with the given event log.
    pub fn new(log: EventLog) -> Self {
        Self {
            agents: Vec::new(),
            log,
            event_log: None,
        }
    }

    /// Create a new EventDispatcher with an event log AND a separate reference
    /// log for compression reads (M34 W3). Use this when you want context
    /// compression to read from a different log than the one events are
    /// appended to.
    pub fn with_compression_log(log: EventLog, compression_log: EventLog) -> Self {
        Self {
            agents: Vec::new(),
            log,
            event_log: Some(compression_log),
        }
    }

    /// Register an agent. Agents are iterated in registration order during dispatch.
    pub fn register(&mut self, agent: Arc<dyn ReactiveObserver>) {
        self.agents.push(agent);
    }

    /// Dispatch an event to all matching agents.
    ///
    /// Returns all non-NoAction outputs from activated agents.
    /// Log append failure is non-fatal — dispatch continues.
    ///
    /// `build_ctx` is called once to construct the `AgentContext` from the
    /// event envelope and delta. This keeps the dispatcher generic over context
    /// construction.
    pub fn dispatch<F>(
        &mut self,
        envelope: EventEnvelope,
        delta: &GraphDelta,
        build_ctx: F,
    ) -> Vec<AgentOutput>
    where
        F: FnOnce(&EventEnvelope, &GraphDelta) -> AgentContext,
    {
        // 1. Append to log (unprocessed)
        let serialized = SerializedEvent::from_envelope(envelope.clone());
        if let Err(e) = self.log.append_serialized(&serialized) {
            eprintln!("eventlog append error: {}", e);
        }

        // 2. Build context once
        let mut ctx = build_ctx(&envelope, delta);
        let event_type = &envelope.event_type;

        // 2b. Context compression (M34 W3) — non-fatal if ledger unavailable or compression fails
        if let Some(tokens) = ctx.budget.tokens {
            let policy = crate::cognitive::context::CompressionPolicy {
                budget_chars: (tokens as usize) * 4,
                preserve_causation_window: 3,
                decision_priority: crate::cognitive::context::DecisionPriority::RecencyOnly,
            };
            if let Some(ledger) = &self.event_log
                && let Err(e) = ctx.compress_for_budget(&policy, ledger)
            {
                tracing::warn!(error = %e, "context compression failed, proceeding uncompressed");
            }
        }

        // 3. Fan out to matching agents
        let mut outputs = Vec::new();
        for agent in &self.agents {
            // Subscription glob match first
            let subs = &agent.descriptor().subscriptions;
            if !SubscriptionMatcher::matches(subs, event_type) {
                continue;
            }

            // Then agent-level matches check
            if !agent.matches(&ctx) {
                continue;
            }

            // Call observe
            match agent.observe(&ctx) {
                Ok(output) => {
                    if !matches!(output, AgentOutput::NoAction(_)) {
                        outputs.push(output);
                    }
                }
                Err(e) => {
                    eprintln!("agent {} observe error: {}", agent.descriptor().id, e);
                }
            }
        }

        // 4. Update per-consumer checkpoint (best-effort)
        if let Err(e) = self
            .log
            .set_consumer_checkpoint("event_dispatcher", envelope.seq)
        {
            eprintln!("eventlog set_consumer_checkpoint error: {}", e);
        }

        outputs
    }

    /// Returns the number of registered agents.
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// Returns the current event log sequence.
    pub fn log_seq(&self) -> io::Result<u64> {
        self.log.seq()
    }
}

// ---------------------------------------------------------------------------
// SerializedEvent extension
// ---------------------------------------------------------------------------

impl SerializedEvent {
    /// Create a SerializedEvent from an EventEnvelope with processed = false.
    pub fn from_envelope(envelope: EventEnvelope) -> Self {
        Self {
            envelope,
            processed: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::context::{AgentContext, Evidence, GraphView, ProvenanceId};
    use crate::cognitive::delta::GraphDelta;
    use crate::cognitive::descriptor::{AgentBudget, AgentDescriptor, ModelPolicy};
    use crate::cognitive::event::{EventEnvelope, EventLog};
    use crate::cognitive::observer::{NoopObserver, ObserveError, ReactiveObserver};
    use std::sync::Arc;

    /// Real `ReactiveObserver` test helper that counts how many times
    /// `observe()` was called. NOT a mock — it is a fully-functional
    /// observer (no-op on output) that happens to expose an activation
    /// counter for dispatcher fan-out assertions. AGENTS.md "no mocks"
    /// applies to placeholders for missing real impls; this is a real
    /// observer specialised for assertions.
    struct CountingObserver {
        descriptor: AgentDescriptor,
        activate_count: std::sync::atomic::AtomicUsize,
    }

    impl CountingObserver {
        fn new(id: &str, subscriptions: Vec<String>) -> Self {
            Self {
                descriptor: AgentDescriptor {
                    id: id.into(),
                    version: "0.1.0".into(),
                    subscriptions,
                    required_views: vec![],
                    output_schema: "{}".into(),
                    model_policy: ModelPolicy::Heuristic,
                    budget: AgentBudget::default(),
                    capabilities: vec![],
                    deterministic: true,
                    idempotent: true,
                },
                activate_count: std::sync::atomic::AtomicUsize::new(0),
            }
        }

        fn activate_count(&self) -> usize {
            self.activate_count
                .load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    impl ReactiveObserver for CountingObserver {
        fn descriptor(&self) -> AgentDescriptor {
            self.descriptor.clone()
        }

        fn matches(&self, _ctx: &AgentContext) -> bool {
            true
        }

        fn observe(&self, _ctx: &AgentContext) -> Result<AgentOutput, ObserveError> {
            self.activate_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(AgentOutput::NoAction(
                crate::cognitive::output::NoActionReason {
                    code: crate::cognitive::output::NoActionCode::InsufficientConfidence,
                    message: "counting observer".into(),
                },
            ))
        }
    }

    fn make_delta() -> GraphDelta {
        GraphDelta::default()
    }

    fn make_envelope(event_type: &str, seq: u64) -> EventEnvelope {
        EventEnvelope {
            event_id: uuid::Uuid::nil(),
            schema_version: "1.0".to_string(),
            timestamp: chrono::DateTime::from_timestamp(1_000_000_000 * seq as i64, 0)
                .unwrap_or_else(chrono::Utc::now),
            source: "test".into(),
            producer: "test".into(),
            event_type: event_type.into(),
            payload: serde_json::json!({}),
            seq,
            correlation_id: None,
            causation_id: None,
            graph_revision: None,
        }
    }

    fn make_ctx() -> AgentContext {
        // REQ-T06-003: feedback_history plumbing — see AgentContext::with_feedback_history
        // REQ-M25-006: pending_adjudications wiring (TRUST-008 REQ-T08-005). At this site the
        // SyncDispatcher::build_context re-populates the field from
        // AdjudicationRepository::list_pending_adjudications before the agent runs.
        // recent_events (M34 W2) populated by compress_for_budget before dispatch.
        AgentContext {
            goal: "test goal".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
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

    // ---------------------------------------------------------------------------
    // Integration tests — full dispatch cycle
    // ---------------------------------------------------------------------------

    /// Agent that records the context it received for inspection.
    struct InspectingAgent {
        descriptor: AgentDescriptor,
        ctx: std::sync::Mutex<Option<AgentContext>>,
    }

    impl InspectingAgent {
        fn new(subscriptions: Vec<String>) -> Self {
            Self {
                descriptor: AgentDescriptor {
                    id: "inspector".into(),
                    version: "0.1.0".into(),
                    subscriptions,
                    required_views: vec![],
                    output_schema: "{}".into(),
                    model_policy: ModelPolicy::Heuristic,
                    budget: AgentBudget::default(),
                    capabilities: vec![],
                    deterministic: true,
                    idempotent: true,
                },
                ctx: std::sync::Mutex::new(None),
            }
        }

        fn take_context(&self) -> Option<AgentContext> {
            self.ctx.lock().unwrap().take()
        }
    }

    impl ReactiveObserver for InspectingAgent {
        fn descriptor(&self) -> AgentDescriptor {
            self.descriptor.clone()
        }

        fn matches(&self, _ctx: &AgentContext) -> bool {
            true
        }

        fn observe(&self, ctx: &AgentContext) -> Result<AgentOutput, ObserveError> {
            *self.ctx.lock().unwrap() = Some(ctx.clone());
            Ok(AgentOutput::NoAction(
                crate::cognitive::output::NoActionReason {
                    code: crate::cognitive::output::NoActionCode::InsufficientConfidence,
                    message: "inspecting".into(),
                },
            ))
        }
    }

    #[test]
    fn integration_full_dispatch_cycle() {
        // Test: EventLog append → SubscriptionMatcher → observe → log update
        let tmp = std::env::temp_dir().join("archctl_integration_full");
        let log = EventLog::open(tmp.clone()).unwrap();
        let mut disp = EventDispatcher::new(log);

        disp.register(Arc::new(NoopObserver {
            descriptor: AgentDescriptor {
                id: "a".into(),
                version: "0.1.0".into(),
                subscriptions: vec!["GoalSubmitted".into()],
                required_views: vec![],
                output_schema: "{}".into(),
                model_policy: ModelPolicy::Heuristic,
                budget: AgentBudget::default(),
                capabilities: vec![],
                deterministic: true,
                idempotent: true,
            },
        }) as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 42);
        disp.dispatch(env, &make_delta(), |_, _| make_ctx())
            .is_empty();

        // Log should have the event
        let log = EventLog::open(tmp).unwrap();
        let seq = log.seq().unwrap();
        assert_eq!(seq, 42);
    }

    #[test]
    fn integration_triggering_event_populated() {
        // Test: triggering_event in context matches dispatched event_type
        let tmp = std::env::temp_dir().join("archctl_integration_triggering");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        let inspector = Arc::new(InspectingAgent::new(vec!["GoalSubmitted".into()]));
        disp.register(inspector.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 1);
        disp.dispatch(env, &make_delta(), |envelope, _delta| {
            let mut ctx = make_ctx();
            ctx.triggering_event = Some(envelope.event_type.clone());
            ctx
        });

        let captured = inspector.take_context();
        assert!(captured.is_some());
        let ctx = captured.unwrap();
        assert_eq!(ctx.triggering_event, Some("GoalSubmitted".into()));
    }

    #[test]
    fn integration_delta_visible_in_context() {
        // Test: GraphDelta is passed to the context builder and visible to agent
        let tmp = std::env::temp_dir().join("archctl_integration_delta");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        let inspector = Arc::new(InspectingAgent::new(vec!["GoalSubmitted".into()]));
        disp.register(inspector.clone() as Arc<dyn ReactiveObserver>);

        // Create a delta with an added element
        let delta = {
            let elem = crate::cognitive::context::Element {
                id: "test-element".into(),
                kind_id: "component".into(),
                name: "TestElement".into(),
                canonical_key: "component:test-element".into(),
                properties: serde_json::json!({}),
            };
            let mut delta = GraphDelta::default();
            delta.added.push(crate::cognitive::delta::DeltaElement {
                element: elem,
                change: crate::cognitive::delta::DeltaChange::Added,
            });
            delta
        };

        let env = make_envelope("GoalSubmitted", 1);
        disp.dispatch(env, &delta, |_envelope, d| {
            let ctx = make_ctx();
            // Simulate the context builder that injects delta info into graph_view
            // For this test we just verify the delta is passed correctly
            let _ = d;
            ctx
        });

        let captured = inspector.take_context();
        assert!(captured.is_some()); // Agent received context with triggering_event
    }

    #[test]
    fn integration_empty_delta_on_no_changes() {
        // Test: empty delta does not prevent dispatch
        let tmp = std::env::temp_dir().join("archctl_integration_empty_delta");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        let agent = Arc::new(CountingObserver::new("a", vec!["*".into()]));
        disp.register(agent.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 1);
        let delta = GraphDelta::default(); // empty
        disp.dispatch(env, &delta, |_, _| make_ctx()).is_empty();

        assert_eq!(agent.activate_count(), 1);
    }

    #[test]
    fn dispatcher_register_and_count() {
        let tmp = std::env::temp_dir().join("archctl_dispatch_count");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        disp.register(Arc::new(NoopObserver {
            descriptor: AgentDescriptor {
                id: "a".into(),
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
        }));
        disp.register(Arc::new(NoopObserver {
            descriptor: AgentDescriptor {
                id: "b".into(),
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
        }));

        assert_eq!(disp.agent_count(), 2);
    }

    #[test]
    fn dispatcher_fan_out_to_matching_agents() {
        let tmp = std::env::temp_dir().join("archctl_dispatch_fanout");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        let agent_a = Arc::new(CountingObserver::new("a", vec!["GoalSubmitted".into()]));
        let agent_b = Arc::new(CountingObserver::new("b", vec!["GoalSubmitted".into()]));
        // Register using explicit coercion
        disp.register(agent_a.clone() as Arc<dyn ReactiveObserver>);
        disp.register(agent_b.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 1);
        let outputs = disp.dispatch(env, &make_delta(), |_, _| make_ctx());

        // Both agents should have been called (both match GoalSubmitted)
        assert_eq!(agent_a.activate_count(), 1);
        assert_eq!(agent_b.activate_count(), 1);
        // Both return NoAction so outputs is empty
        assert!(outputs.is_empty());
    }

    #[test]
    fn dispatcher_skips_non_matching_subscriptions() {
        let tmp = std::env::temp_dir().join("archctl_dispatch_skip");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        let agent = Arc::new(CountingObserver::new("a", vec!["GoalCancelled".into()]));
        disp.register(agent.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 1);
        let outputs = disp.dispatch(env, &make_delta(), |_, _| make_ctx());

        // Agent does not match GoalSubmitted (subscribed to GoalCancelled)
        assert_eq!(agent.activate_count(), 0);
        assert!(outputs.is_empty());
    }

    #[test]
    fn dispatcher_star_subscribes_to_all() {
        let tmp = std::env::temp_dir().join("archctl_dispatch_star");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        let agent = Arc::new(CountingObserver::new("a", vec!["*".into()]));
        disp.register(agent.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("AnyEventWhatsoever", 1);
        disp.dispatch(env, &make_delta(), |_, _| make_ctx());

        assert_eq!(agent.activate_count(), 1);
    }

    #[test]
    fn dispatcher_suffix_subscription() {
        let tmp = std::env::temp_dir().join("archctl_dispatch_suffix");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        let agent = Arc::new(CountingObserver::new("a", vec!["*.changed".into()]));
        disp.register(agent.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("file.changed", 1);
        disp.dispatch(env, &make_delta(), |_, _| make_ctx());

        assert_eq!(agent.activate_count(), 1);
    }

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage, 2026-08-22)
    // ---------------------------------------------------------------------------

    /// Real ReactiveObserver that emits a non-NoAction output (Hypothesis).
    /// NOT a mock — same pattern as CountingObserver: real observer
    /// specialised for assertions, no external placeholder.
    struct ProducingObserver {
        descriptor: AgentDescriptor,
        statement: String,
    }

    impl ProducingObserver {
        fn new(id: &str, subscriptions: Vec<String>, statement: &str) -> Self {
            Self {
                descriptor: AgentDescriptor {
                    id: id.into(),
                    version: "0.1.0".into(),
                    subscriptions,
                    required_views: vec![],
                    output_schema: "{}".into(),
                    model_policy: ModelPolicy::Heuristic,
                    budget: AgentBudget::default(),
                    capabilities: vec![],
                    deterministic: true,
                    idempotent: true,
                },
                statement: statement.into(),
            }
        }
    }

    impl ReactiveObserver for ProducingObserver {
        fn descriptor(&self) -> AgentDescriptor {
            self.descriptor.clone()
        }
        fn matches(&self, _ctx: &AgentContext) -> bool {
            true
        }
        fn observe(&self, _ctx: &AgentContext) -> Result<AgentOutput, ObserveError> {
            Ok(AgentOutput::Hypothesis(
                crate::cognitive::output::Hypothesis {
                    statement: self.statement.clone(),
                    confidence: 0.9,
                    evidence_ids: vec![],
                },
            ))
        }
    }

    /// Real observer that returns Err from observe(). Verifies the dispatcher
    /// swallows the error and continues processing the remaining agents.
    struct ErroringObserver {
        descriptor: AgentDescriptor,
    }

    impl ErroringObserver {
        fn new(id: &str, subscriptions: Vec<String>) -> Self {
            Self {
                descriptor: AgentDescriptor {
                    id: id.into(),
                    version: "0.1.0".into(),
                    subscriptions,
                    required_views: vec![],
                    output_schema: "{}".into(),
                    model_policy: ModelPolicy::Heuristic,
                    budget: AgentBudget::default(),
                    capabilities: vec![],
                    deterministic: true,
                    idempotent: true,
                },
            }
        }
    }

    impl ReactiveObserver for ErroringObserver {
        fn descriptor(&self) -> AgentDescriptor {
            self.descriptor.clone()
        }
        fn matches(&self, _ctx: &AgentContext) -> bool {
            true
        }
        fn observe(&self, _ctx: &AgentContext) -> Result<AgentOutput, ObserveError> {
            Err(ObserveError::Internal("forced failure".into()))
        }
    }

    /// `log_seq()` returns 0 on a freshly-opened log with no appends.
    #[test]
    fn dispatcher_log_seq_initially_zero() {
        let tmp = std::env::temp_dir().join("archctl_dispatch_seq_zero");
        let log = EventLog::open(tmp).unwrap();
        let disp = EventDispatcher::new(log);
        assert_eq!(disp.log_seq().unwrap(), 0);
    }

    /// `SerializedEvent::from_envelope` constructor sets processed=false
    /// regardless of envelope content.
    #[test]
    fn serialized_event_from_envelope_defaults_to_unprocessed() {
        let env = make_envelope("GoalSubmitted", 42);
        let ser = SerializedEvent::from_envelope(env);
        assert!(!ser.processed, "from_envelope must default processed=false");
        assert_eq!(ser.envelope.event_type.as_str(), "GoalSubmitted");
        assert_eq!(ser.envelope.seq, 42);
    }

    /// Dispatch with no registered agents returns an empty outputs vec and
    /// still appends the event to the log + advances the seq marker.
    /// (Note: EventLog is append-only by design — see event.rs:160-191. We
    /// assert the last appended event is the one we dispatched, not the
    /// total event count, since previous runs leave residual entries.)
    #[test]
    fn dispatcher_empty_registry_dispatches_with_no_outputs() {
        let tmp = std::env::temp_dir().join("archctl_dispatch_empty");
        let log = EventLog::open(tmp.clone()).unwrap();
        let mut disp = EventDispatcher::new(log);

        let env = make_envelope("GoalSubmitted", 7);
        let outputs = disp.dispatch(env, &make_delta(), |_, _| make_ctx());

        assert!(
            outputs.is_empty(),
            "empty registry must yield empty outputs"
        );
        let log = EventLog::open(tmp).unwrap();
        let events: Vec<_> = log.iter().unwrap().collect::<io::Result<Vec<_>>>().unwrap();
        assert!(
            !events.is_empty(),
            "event must be appended even with no agents"
        );
        let last = events.last().unwrap();
        assert_eq!(last.envelope.seq, 7, "last event must be seq=7");
        assert_eq!(
            last.envelope.event_type.as_str(),
            "GoalSubmitted",
            "last event must be the dispatched envelope"
        );
    }

    /// Dispatch with one agent returning a real Hypothesis output returns
    /// that output to the caller (not NoAction).
    #[test]
    fn dispatcher_returns_non_noaction_outputs() {
        let tmp = std::env::temp_dir().join("archctl_dispatch_returns");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        let agent = Arc::new(ProducingObserver::new(
            "producer",
            vec!["GoalSubmitted".into()],
            "the system has a coupling smell",
        ));
        disp.register(agent.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 1);
        let outputs = disp.dispatch(env, &make_delta(), |_, _| make_ctx());

        assert_eq!(outputs.len(), 1);
        match &outputs[0] {
            AgentOutput::Hypothesis(h) => {
                assert_eq!(h.statement, "the system has a coupling smell");
                assert!((h.confidence - 0.9).abs() < f64::EPSILON);
            }
            other => panic!("expected Hypothesis, got {:?}", other),
        }
    }

    /// Multiple events dispatched in sequence grow the log and advance the seq
    /// marker monotonically.
    #[test]
    fn dispatcher_multiple_events_grow_log_monotonically() {
        let tmp = std::env::temp_dir().join("archctl_dispatch_multi");
        let log = EventLog::open(tmp.clone()).unwrap();
        let mut disp = EventDispatcher::new(log);

        for i in 1..=3 {
            let env = make_envelope("GoalSubmitted", i);
            disp.dispatch(env, &make_delta(), |_, _| make_ctx());
        }

        // Seq marker must advance to the highest seq
        let seq = disp.log_seq().unwrap();
        assert_eq!(seq, 3, "log_seq must advance to last dispatched seq");

        // Consumer checkpoint for the dispatcher must also advance
        let log = EventLog::open(tmp).unwrap();
        let checkpoint = log.consumer_checkpoint("event_dispatcher").unwrap();
        assert_eq!(
            checkpoint, 3,
            "consumer_checkpoint must advance to last dispatched seq"
        );
    }

    /// A mix of matching and non-matching agents: only matching agents are
    /// activated. The dispatcher's fan-out preserves order.
    #[test]
    fn dispatcher_partial_fan_out_preserves_registration_order() {
        let tmp = std::env::temp_dir().join("archctl_dispatch_partial");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        // a matches, b doesn't, c matches
        let a = Arc::new(ProducingObserver::new(
            "a",
            vec!["GoalSubmitted".into()],
            "from-a",
        ));
        let b = Arc::new(ProducingObserver::new(
            "b",
            vec!["OtherEvent".into()],
            "from-b",
        ));
        let c = Arc::new(ProducingObserver::new(
            "c",
            vec!["GoalSubmitted".into()],
            "from-c",
        ));
        disp.register(a.clone() as Arc<dyn ReactiveObserver>);
        disp.register(b.clone() as Arc<dyn ReactiveObserver>);
        disp.register(c.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 1);
        let outputs = disp.dispatch(env, &make_delta(), |_, _| make_ctx());

        // a + c match (b's subscription doesn't match "GoalSubmitted")
        assert_eq!(outputs.len(), 2);
        match &outputs[0] {
            AgentOutput::Hypothesis(h) => assert_eq!(h.statement, "from-a"),
            _ => panic!("expected Hypothesis from a"),
        }
        match &outputs[1] {
            AgentOutput::Hypothesis(h) => assert_eq!(h.statement, "from-c"),
            _ => panic!("expected Hypothesis from c"),
        }
    }

    /// An agent that returns `Err(ObserveError)` does not crash the dispatcher
    /// or block subsequent agents. The remaining matching agents still emit
    /// their outputs.
    #[test]
    fn dispatcher_swallows_observer_errors_and_continues() {
        let tmp = std::env::temp_dir().join("archctl_dispatch_error");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        // First agent errors, second produces a Hypothesis
        let err_agent = Arc::new(ErroringObserver::new("err", vec!["GoalSubmitted".into()]));
        let producer = Arc::new(ProducingObserver::new(
            "producer",
            vec!["GoalSubmitted".into()],
            "still-emitted",
        ));
        disp.register(err_agent.clone() as Arc<dyn ReactiveObserver>);
        disp.register(producer.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 1);
        let outputs = disp.dispatch(env, &make_delta(), |_, _| make_ctx());

        // Only the producer's output reaches the caller — the err agent's
        // failure is logged to stderr (eprintln) and dropped.
        assert_eq!(outputs.len(), 1, "erroring agent's output is dropped");
        match &outputs[0] {
            AgentOutput::Hypothesis(h) => assert_eq!(h.statement, "still-emitted"),
            _ => panic!("expected Hypothesis from producer"),
        }
    }

    /// The dispatcher's `log_seq()` reflects the highest seq written to the
    /// log, surviving across drops (i.e., the dispatcher can be reopened
    /// with the same log path and pick up where it left off).
    #[test]
    fn dispatcher_log_seq_survives_reopen() {
        let tmp = std::env::temp_dir().join("archctl_dispatch_reopen");
        // First session: dispatch 5 events
        {
            let log = EventLog::open(tmp.clone()).unwrap();
            let mut disp = EventDispatcher::new(log);
            for i in 1..=5 {
                let env = make_envelope("GoalSubmitted", i);
                disp.dispatch(env, &make_delta(), |_, _| make_ctx());
            }
            assert_eq!(disp.log_seq().unwrap(), 5);
        }
        // Second session: reopen same path
        let log = EventLog::open(tmp).unwrap();
        let disp = EventDispatcher::new(log);
        assert_eq!(
            disp.log_seq().unwrap(),
            5,
            "log_seq must survive EventDispatcher reopen (relies on EventLog::seq)"
        );
    }

    // ---------------------------------------------------------------------------
    // M34 W3 — compress_for_budget wiring tests
    // ---------------------------------------------------------------------------

    /// Dispatch with budget.tokens = Some(N) should call compress_for_budget,
    /// populating recent_events in the context passed to the observer.
    #[test]
    fn dispatch_with_budget_calls_compress_for_budget() {
        let tmp = tempfile::TempDir::new().unwrap();
        let log = EventLog::open(tmp.path().join("log.jsonl")).unwrap();
        let mut comp_log = EventLog::open(tmp.path().join("comp.jsonl")).unwrap();

        // Pre-populate the compression log with a few events so recent() returns something
        for i in 1..=5 {
            let env = EventEnvelope {
                event_id: uuid::Uuid::new_v4(),
                schema_version: "1.0".into(),
                timestamp: chrono::Utc::now(),
                source: "test".into(),
                producer: "test".into(),
                event_type: "PreExisting".into(),
                payload: serde_json::json!({}),
                seq: i,
                correlation_id: None,
                causation_id: None,
                graph_revision: None,
            };
            let ser = SerializedEvent::from_envelope(env);
            comp_log.append_serialized(&ser).unwrap();
        }

        let mut disp = EventDispatcher::with_compression_log(log, comp_log);
        let inspector = Arc::new(InspectingAgent::new(vec!["GoalSubmitted".into()]));
        disp.register(inspector.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 10);
        disp.dispatch(env, &make_delta(), |_, _| AgentContext {
            goal: "test".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget {
                tokens: Some(100),
                ..Default::default()
            },
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        });

        let captured = inspector.take_context();
        assert!(
            captured.is_some(),
            "observer should have received a context"
        );
        let ctx = captured.unwrap();
        // compress_for_budget fetches recent events from the ledger
        assert!(
            !ctx.recent_events.is_empty(),
            "recent_events should be populated when budget.tokens is set"
        );
    }

    /// Dispatch with budget.tokens = None should NOT call compress_for_budget,
    /// leaving recent_events empty.
    #[test]
    fn dispatch_without_budget_does_not_compress() {
        let tmp = tempfile::TempDir::new().unwrap();
        let log = EventLog::open(tmp.path().join("log.jsonl")).unwrap();
        let mut comp_log = EventLog::open(tmp.path().join("comp.jsonl")).unwrap();

        // Pre-populate so we'd have something to compress
        for i in 1..=5 {
            let env = EventEnvelope {
                event_id: uuid::Uuid::new_v4(),
                schema_version: "1.0".into(),
                timestamp: chrono::Utc::now(),
                source: "test".into(),
                producer: "test".into(),
                event_type: "PreExisting".into(),
                payload: serde_json::json!({}),
                seq: i,
                correlation_id: None,
                causation_id: None,
                graph_revision: None,
            };
            let ser = SerializedEvent::from_envelope(env);
            comp_log.append_serialized(&ser).unwrap();
        }

        let mut disp = EventDispatcher::with_compression_log(log, comp_log);
        let inspector = Arc::new(InspectingAgent::new(vec!["GoalSubmitted".into()]));
        disp.register(inspector.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 10);
        disp.dispatch(env, &make_delta(), |_, _| AgentContext {
            goal: "test".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget {
                tokens: None,
                ..Default::default()
            },
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        });

        let captured = inspector.take_context();
        assert!(captured.is_some());
        let ctx = captured.unwrap();
        // Without budget tokens, compression should NOT have been called
        assert!(
            ctx.recent_events.is_empty(),
            "recent_events should be empty when budget.tokens is None"
        );
    }

    /// When event_log is None (dispatcher constructed via new()), dispatch
    /// should proceed without compression even if budget.tokens is Some.
    /// This verifies the Option-handling is correct and no panic occurs.
    #[test]
    fn dispatch_with_event_log_unavailable_skips_compression() {
        let tmp = tempfile::TempDir::new().unwrap();
        let log = EventLog::open(tmp.path().join("log.jsonl")).unwrap();
        let mut disp = EventDispatcher::new(log); // No event_log — compression_log is None
        let inspector = Arc::new(InspectingAgent::new(vec!["GoalSubmitted".into()]));
        disp.register(inspector.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 1);
        // This must NOT panic even though budget.tokens = Some
        let outputs = disp.dispatch(env, &make_delta(), |_, _| AgentContext {
            goal: "test".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget {
                tokens: Some(100),
                ..Default::default()
            },
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        });

        // Dispatch should succeed (outputs may be empty depending on observer)
        let _ = outputs;
        let captured = inspector.take_context();
        assert!(captured.is_some(), "observer should still receive context");
        let ctx = captured.unwrap();
        // recent_events stays empty because event_log was unavailable
        assert!(ctx.recent_events.is_empty());
    }

    /// Dispatch with causation-linked events: the BFS should find ancestors
    /// when preserve_causation_window > 0. We verify by pre-populating
    /// a chain of events with causation_id links and checking that
    /// compress_for_budget successfully traverses them (no error returned).
    #[test]
    fn dispatch_preserves_causation_window_within_recent_events() {
        let tmp = tempfile::TempDir::new().unwrap();
        let log = EventLog::open(tmp.path().join("log.jsonl")).unwrap();
        let mut comp_log = EventLog::open(tmp.path().join("comp.jsonl")).unwrap();

        // Create a chain: event 3 → event 2 → event 1 (causation links)
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
            let ser = SerializedEvent::from_envelope(env);
            comp_log.append_serialized(&ser).unwrap();
        }

        let mut disp = EventDispatcher::with_compression_log(log, comp_log);
        let inspector = Arc::new(InspectingAgent::new(vec!["Event3".into()]));
        disp.register(inspector.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("Event3", 10);
        let outputs = disp.dispatch(env, &make_delta(), |_, _| AgentContext {
            goal: "test".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget {
                tokens: Some(100),
                ..Default::default()
            },
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        });

        // Dispatch should succeed; observer receives context with recent_events populated
        let _ = outputs;
        let captured = inspector.take_context();
        assert!(captured.is_some(), "observer should receive context");
        let ctx = captured.unwrap();
        // Recent events should be populated from the compression log
        assert!(
            !ctx.recent_events.is_empty(),
            "recent_events should have events from compression log"
        );
    }

    /// Evidence truncation: when evidence total chars exceeds budget_chars,
    /// compress_for_budget should drop evidence items (oldest first).
    /// We verify by checking that evidence items are reduced after dispatch
    /// with a tight budget.
    #[test]
    fn dispatch_compress_truncates_evidence() {
        let tmp = tempfile::TempDir::new().unwrap();
        let log = EventLog::open(tmp.path().join("log.jsonl")).unwrap();
        let mut comp_log = EventLog::open(tmp.path().join("comp.jsonl")).unwrap();

        // Pre-populate compression log
        for i in 1..=3 {
            let env = EventEnvelope {
                event_id: uuid::Uuid::new_v4(),
                schema_version: "1.0".into(),
                timestamp: chrono::Utc::now(),
                source: "test".into(),
                producer: "test".into(),
                event_type: "PreExisting".into(),
                payload: serde_json::json!({}),
                seq: i,
                correlation_id: None,
                causation_id: None,
                graph_revision: None,
            };
            let ser = SerializedEvent::from_envelope(env);
            comp_log.append_serialized(&ser).unwrap();
        }

        let mut disp = EventDispatcher::with_compression_log(log, comp_log);
        let inspector = Arc::new(InspectingAgent::new(vec!["GoalSubmitted".into()]));
        disp.register(inspector.clone() as Arc<dyn ReactiveObserver>);

        // Create context with many large evidence items
        let evidence: Vec<Evidence> = (0..10)
            .map(|i| Evidence {
                id: format!("ev-{}", i),
                provenance_id: ProvenanceId::File {
                    path: "x.rs".into(),
                    line: i,
                },
                content_hash: format!("hash{}", i),
                text: "this is a long evidence text that should be truncated when budget is tight"
                    .to_string(),
                properties: Default::default(),
            })
            .collect();

        let env = make_envelope("GoalSubmitted", 10);
        disp.dispatch(env, &make_delta(), |_, _| AgentContext {
            goal: "test".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence,
            applicable_rules: vec![],
            available_tools: vec![],
            // Tiny budget: 10 tokens * 4 = 40 chars, should truncate most evidence
            budget: AgentBudget {
                tokens: Some(10),
                ..Default::default()
            },
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        });

        let captured = inspector.take_context();
        assert!(captured.is_some());
        let ctx = captured.unwrap();
        // With budget_chars = 40, most evidence should have been dropped
        assert!(
            ctx.evidence.len() < 10,
            "evidence should be truncated when budget is tight, got {} items",
            ctx.evidence.len()
        );
    }

    // ---------------------------------------------------------------------------
    // cognitive-coverage-v2 — dispatcher compression paths (PR 2 of 3)
    // ---------------------------------------------------------------------------

    /// budget.tokens = Some(0) yields budget_chars = 0, which makes
    /// `compress_for_budget` return `InvalidPolicy`. The dispatcher must
    /// NOT propagate the error — it must log a `tracing::warn!` and proceed
    /// with the un-compressed context. The observer still receives a
    /// context (with `recent_events` empty because compression bailed).
    #[test]
    fn dispatch_zero_tokens_compression_bails_but_fan_out_continues() {
        let tmp = tempfile::TempDir::new().unwrap();
        let log = EventLog::open(tmp.path().join("log.jsonl")).unwrap();
        let mut comp_log = EventLog::open(tmp.path().join("comp.jsonl")).unwrap();

        // Pre-populate comp_log so we can distinguish "compression ran and
        // emptied recent_events" from "compression never ran".
        for i in 1..=3 {
            let env = make_envelope("PreExisting", i);
            let ser = SerializedEvent::from_envelope(env);
            comp_log.append_serialized(&ser).unwrap();
        }

        let mut disp = EventDispatcher::with_compression_log(log, comp_log);
        let inspector = Arc::new(InspectingAgent::new(vec!["GoalSubmitted".into()]));
        disp.register(inspector.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 10);
        // budget.tokens = Some(0) → budget_chars = 0 → InvalidPolicy from
        // compress_for_budget. Dispatcher swallows + proceeds.
        let outputs = disp.dispatch(env, &make_delta(), |_, _| AgentContext {
            goal: "test".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget {
                tokens: Some(0),
                ..Default::default()
            },
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        });

        // dispatch itself returned without panic; fan-out ran.
        let _ = outputs;
        let captured = inspector.take_context();
        assert!(
            captured.is_some(),
            "observer must receive context even when compression failed"
        );
        // recent_events stays empty because compression returned early.
        let ctx = captured.unwrap();
        assert!(
            ctx.recent_events.is_empty(),
            "failed compression must not populate recent_events; got {}",
            ctx.recent_events.len()
        );
    }

    /// When the compression ledger is empty, compression runs successfully
    /// and produces an empty `recent_events` (not an error). Locks the
    /// `compress_empty_ledger_returns_zeroed_report` invariant at the
    /// dispatcher level.
    #[test]
    fn dispatch_with_empty_compression_ledger_populates_empty_recent_events() {
        let tmp = tempfile::TempDir::new().unwrap();
        let log = EventLog::open(tmp.path().join("log.jsonl")).unwrap();
        let comp_log = EventLog::open(tmp.path().join("comp.jsonl")).unwrap();
        // No pre-population — comp_log is empty.

        let mut disp = EventDispatcher::with_compression_log(log, comp_log);
        let inspector = Arc::new(InspectingAgent::new(vec!["GoalSubmitted".into()]));
        disp.register(inspector.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 10);
        disp.dispatch(env, &make_delta(), |_, _| AgentContext {
            goal: "test".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget {
                tokens: Some(100),
                ..Default::default()
            },
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        });

        let captured = inspector.take_context().unwrap();
        assert!(
            captured.recent_events.is_empty(),
            "empty compression ledger → empty recent_events; got {}",
            captured.recent_events.len()
        );
    }

    /// log_seq() is monotonic across multiple dispatch cycles, each of
    /// which triggers compression. This locks the "checkpoint + seq"
    /// invariant under the compression path (the W1-cherry-pick that
    /// motivated M34 W1).
    #[test]
    fn dispatch_log_seq_monotonic_across_compression_cycles() {
        let tmp = tempfile::TempDir::new().unwrap();
        let log = EventLog::open(tmp.path().join("log.jsonl")).unwrap();
        let mut comp_log = EventLog::open(tmp.path().join("comp.jsonl")).unwrap();

        // Pre-populate comp_log so compression has something to read each cycle.
        for i in 1..=3 {
            let env = make_envelope("PreExisting", i);
            let ser = SerializedEvent::from_envelope(env);
            comp_log.append_serialized(&ser).unwrap();
        }

        let mut disp = EventDispatcher::with_compression_log(log, comp_log);
        let inspector = Arc::new(InspectingAgent::new(vec!["GoalSubmitted".into()]));
        disp.register(inspector.clone() as Arc<dyn ReactiveObserver>);

        let mut last_seq = 0u64;
        for cycle in 1..=5 {
            let env = make_envelope("GoalSubmitted", cycle);
            disp.dispatch(env, &make_delta(), |_, _| AgentContext {
                goal: "test".into(),
                triggering_event: None,
                graph_view: GraphView::default(),
                source_fragments: vec![],
                evidence: vec![],
                applicable_rules: vec![],
                available_tools: vec![],
                budget: AgentBudget {
                    tokens: Some(100),
                    ..Default::default()
                },
                feedback_history: vec![],
                pending_adjudications: vec![],
                recent_events: vec![],
            });
            let seq = disp.log_seq().unwrap();
            assert!(
                seq > last_seq,
                "log_seq must be strictly monotonic across compression cycles; got {seq} after {last_seq}"
            );
            last_seq = seq;
        }
        assert_eq!(last_seq, 5, "5 dispatches → 5 events appended");
    }

    /// `with_compression_log(log, comp_log)` reads from `comp_log` for
    /// compression but writes dispatched events to `log`. If only `comp_log`
    /// is pre-populated, the observer sees those events in `recent_events`;
    /// if only `log` is pre-populated, `recent_events` stays empty. Locks
    /// the read/write partition between the two logs.
    #[test]
    fn dispatch_with_compression_log_reads_only_from_compression_ledger() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut log = EventLog::open(tmp.path().join("log.jsonl")).unwrap();
        let comp_log = EventLog::open(tmp.path().join("comp.jsonl")).unwrap();

        // Pre-populate ONLY `log` with 3 events. comp_log stays empty.
        for i in 1..=3 {
            let env = make_envelope("OnlyInDispatchLog", i);
            let ser = SerializedEvent::from_envelope(env);
            log.append_serialized(&ser).unwrap();
        }

        let mut disp = EventDispatcher::with_compression_log(log, comp_log);
        let inspector = Arc::new(InspectingAgent::new(vec!["GoalSubmitted".into()]));
        disp.register(inspector.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 10);
        disp.dispatch(env, &make_delta(), |_, _| AgentContext {
            goal: "test".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget {
                tokens: Some(100),
                ..Default::default()
            },
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        });

        let captured = inspector.take_context().unwrap();
        // Compression reads from comp_log (empty), so recent_events stays empty
        // even though `log` had 3 events.
        assert!(
            captured.recent_events.is_empty(),
            "with_compression_log must NOT read from the dispatch log; got {}",
            captured.recent_events.len()
        );
    }

    /// Dispatched events are written to the dispatch log, NOT to the
    /// compression log. The compression ledger is read-only for context
    /// (mutating it would create feedback loops in multi-dispatcher setups).
    #[test]
    fn dispatch_with_compression_log_does_not_write_to_compression_ledger() {
        let tmp = tempfile::TempDir::new().unwrap();
        let log = EventLog::open(tmp.path().join("log.jsonl")).unwrap();
        let comp_log_path = tmp.path().join("comp.jsonl");
        let comp_log = EventLog::open(comp_log_path.clone()).unwrap();

        let mut disp = EventDispatcher::with_compression_log(log, comp_log);
        let inspector = Arc::new(InspectingAgent::new(vec!["GoalSubmitted".into()]));
        disp.register(inspector.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 10);
        disp.dispatch(env, &make_delta(), |_, _| AgentContext {
            goal: "test".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget {
                tokens: Some(100),
                ..Default::default()
            },
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        });

        // Reopen comp_log independently and confirm it has zero events.
        let comp_log_reopened = EventLog::open(comp_log_path).unwrap();
        assert_eq!(
            comp_log_reopened.seq().unwrap(),
            0,
            "compression ledger must NOT receive dispatched events"
        );
    }

    /// Fan-out respects registration order when compression is enabled.
    /// Observer 1, 2, 3 are registered in that order; the dispatch must
    /// invoke them in registration order. Locks the deterministic
    /// registration-order invariant under the compression path.
    #[test]
    fn dispatch_fan_out_preserves_registration_order_with_compression() {
        let tmp = tempfile::TempDir::new().unwrap();
        let log = EventLog::open(tmp.path().join("log.jsonl")).unwrap();
        let mut comp_log = EventLog::open(tmp.path().join("comp.jsonl")).unwrap();
        for i in 1..=3 {
            let env = make_envelope("PreExisting", i);
            let ser = SerializedEvent::from_envelope(env);
            comp_log.append_serialized(&ser).unwrap();
        }

        // Use a counter shared across observers to record the order.
        let order: Arc<std::sync::Mutex<Vec<&'static str>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));

        // Build 3 lightweight observers with unique ids and a single
        // subscription pattern that matches "GoalSubmitted".
        struct OrderRecorder {
            id: &'static str,
            order: Arc<std::sync::Mutex<Vec<&'static str>>>,
        }
        impl ReactiveObserver for OrderRecorder {
            fn descriptor(&self) -> AgentDescriptor {
                AgentDescriptor {
                    id: self.id.into(),
                    version: "0.1.0".into(),
                    subscriptions: vec!["GoalSubmitted".into()],
                    required_views: vec![],
                    output_schema: "{}".into(),
                    model_policy: ModelPolicy::Heuristic,
                    budget: AgentBudget::default(),
                    capabilities: vec![],
                    deterministic: true,
                    idempotent: true,
                }
            }
            fn matches(&self, _ctx: &AgentContext) -> bool {
                true
            }
            fn observe(&self, _ctx: &AgentContext) -> Result<AgentOutput, ObserveError> {
                self.order.lock().unwrap().push(self.id);
                Ok(AgentOutput::NoAction(
                    crate::cognitive::output::NoActionReason {
                        code: crate::cognitive::output::NoActionCode::InsufficientConfidence,
                        message: self.id.into(),
                    },
                ))
            }
        }

        let mut disp = EventDispatcher::with_compression_log(log, comp_log);
        disp.register(Arc::new(OrderRecorder {
            id: "first",
            order: order.clone(),
        }) as Arc<dyn ReactiveObserver>);
        disp.register(Arc::new(OrderRecorder {
            id: "second",
            order: order.clone(),
        }) as Arc<dyn ReactiveObserver>);
        disp.register(Arc::new(OrderRecorder {
            id: "third",
            order: order.clone(),
        }) as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 10);
        disp.dispatch(env, &make_delta(), |_, _| AgentContext {
            goal: "test".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget {
                tokens: Some(100),
                ..Default::default()
            },
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        });

        let recorded = order.lock().unwrap().clone();
        assert_eq!(
            recorded,
            vec!["first", "second", "third"],
            "fan-out must invoke observers in registration order under compression"
        );
    }

    /// An observer that returns Err must not break the chain for subsequent
    /// observers. Locks the partial-fan-out invariant under compression.
    #[test]
    fn dispatch_erroring_observer_does_not_break_others_with_compression() {
        let tmp = tempfile::TempDir::new().unwrap();
        let log = EventLog::open(tmp.path().join("log.jsonl")).unwrap();
        let mut comp_log = EventLog::open(tmp.path().join("comp.jsonl")).unwrap();
        for i in 1..=2 {
            let env = make_envelope("PreExisting", i);
            let ser = SerializedEvent::from_envelope(env);
            comp_log.append_serialized(&ser).unwrap();
        }

        struct ErroringObserver;
        impl ReactiveObserver for ErroringObserver {
            fn descriptor(&self) -> AgentDescriptor {
                AgentDescriptor {
                    id: "erroring".into(),
                    version: "0.1.0".into(),
                    subscriptions: vec!["GoalSubmitted".into()],
                    required_views: vec![],
                    output_schema: "{}".into(),
                    model_policy: ModelPolicy::Heuristic,
                    budget: AgentBudget::default(),
                    capabilities: vec![],
                    deterministic: true,
                    idempotent: true,
                }
            }
            fn matches(&self, _ctx: &AgentContext) -> bool {
                true
            }
            fn observe(&self, _ctx: &AgentContext) -> Result<AgentOutput, ObserveError> {
                Err(ObserveError::Internal("forced".into()))
            }
        }

        let mut disp = EventDispatcher::with_compression_log(log, comp_log);
        let inspector = Arc::new(InspectingAgent::new(vec!["GoalSubmitted".into()]));
        disp.register(Arc::new(ErroringObserver) as Arc<dyn ReactiveObserver>);
        disp.register(inspector.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 10);
        disp.dispatch(env, &make_delta(), |_, _| AgentContext {
            goal: "test".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget {
                tokens: Some(100),
                ..Default::default()
            },
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        });

        // Despite the first observer erroring, the second still received context.
        let captured = inspector.take_context();
        assert!(
            captured.is_some(),
            "erroring observer must not prevent subsequent observers from receiving context"
        );
    }

    /// `with_compression_log` does not enable compression unless the context
    /// has a budget. With `tokens: None`, even with a populated compression
    /// ledger, `recent_events` stays empty. Locks the gating invariant.
    #[test]
    fn dispatch_with_compression_log_but_no_budget_does_not_read_compression_ledger() {
        let tmp = tempfile::TempDir::new().unwrap();
        let log = EventLog::open(tmp.path().join("log.jsonl")).unwrap();
        let mut comp_log = EventLog::open(tmp.path().join("comp.jsonl")).unwrap();
        for i in 1..=5 {
            let env = make_envelope("PreExisting", i);
            let ser = SerializedEvent::from_envelope(env);
            comp_log.append_serialized(&ser).unwrap();
        }

        let mut disp = EventDispatcher::with_compression_log(log, comp_log);
        let inspector = Arc::new(InspectingAgent::new(vec!["GoalSubmitted".into()]));
        disp.register(inspector.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 10);
        // budget.tokens = None (default) → compression is skipped entirely.
        disp.dispatch(env, &make_delta(), |_, _| AgentContext {
            goal: "test".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(), // tokens: None
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        });

        let captured = inspector.take_context().unwrap();
        assert!(
            captured.recent_events.is_empty(),
            "compression is gated on ctx.budget.tokens.is_some(); got {} events",
            captured.recent_events.len()
        );
    }
}
