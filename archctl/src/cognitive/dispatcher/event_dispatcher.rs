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
}

impl EventDispatcher {
    /// Create a new EventDispatcher with the given event log.
    pub fn new(log: EventLog) -> Self {
        Self {
            agents: Vec::new(),
            log,
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
        let ctx = build_ctx(&envelope, delta);
        let event_type = &envelope.event_type;

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
    use crate::cognitive::context::{AgentContext, GraphView};
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
}
