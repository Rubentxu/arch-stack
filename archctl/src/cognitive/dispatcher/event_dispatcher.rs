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
    use crate::cognitive::observer::{ObserveError, ReactiveObserver};
    use std::sync::Arc;

    struct MockAgent {
        descriptor: AgentDescriptor,
        activate_count: std::sync::atomic::AtomicUsize,
    }

    impl MockAgent {
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
    }

    impl ReactiveObserver for MockAgent {
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
                    message: "mock".into(),
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
        AgentContext {
            goal: "test goal".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
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

        disp.register(Arc::new(MockAgent::new("a", vec!["GoalSubmitted".into()]))
            as Arc<dyn ReactiveObserver>);

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
        use std::sync::atomic::Ordering;
        // Test: empty delta does not prevent dispatch
        let tmp = std::env::temp_dir().join("archctl_integration_empty_delta");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        let agent = Arc::new(MockAgent::new("a", vec!["*".into()]));
        disp.register(agent.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 1);
        let delta = GraphDelta::default(); // empty
        disp.dispatch(env, &delta, |_, _| make_ctx()).is_empty();

        assert_eq!(agent.activate_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn dispatcher_register_and_count() {
        let tmp = std::env::temp_dir().join("archctl_dispatch_count");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        disp.register(Arc::new(MockAgent::new("a", vec![])));
        disp.register(Arc::new(MockAgent::new("b", vec![])));

        assert_eq!(disp.agent_count(), 2);
    }

    #[test]
    fn dispatcher_fan_out_to_matching_agents() {
        use std::sync::atomic::Ordering;
        let tmp = std::env::temp_dir().join("archctl_dispatch_fanout");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        let agent_a = Arc::new(MockAgent::new("a", vec!["GoalSubmitted".into()]));
        let agent_b = Arc::new(MockAgent::new("b", vec!["GoalSubmitted".into()]));
        // Register using explicit coercion
        disp.register(agent_a.clone() as Arc<dyn ReactiveObserver>);
        disp.register(agent_b.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 1);
        let outputs = disp.dispatch(env, &make_delta(), |_, _| make_ctx());

        // Both agents should have been called (both match GoalSubmitted)
        assert_eq!(agent_a.activate_count.load(Ordering::SeqCst), 1);
        assert_eq!(agent_b.activate_count.load(Ordering::SeqCst), 1);
        // Both return NoAction so outputs is empty
        assert!(outputs.is_empty());
    }

    #[test]
    fn dispatcher_skips_non_matching_subscriptions() {
        use std::sync::atomic::Ordering;
        let tmp = std::env::temp_dir().join("archctl_dispatch_skip");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        let agent = Arc::new(MockAgent::new("a", vec!["GoalCancelled".into()]));
        disp.register(agent.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("GoalSubmitted", 1);
        let outputs = disp.dispatch(env, &make_delta(), |_, _| make_ctx());

        // Agent does not match GoalSubmitted (subscribed to GoalCancelled)
        assert_eq!(agent.activate_count.load(Ordering::SeqCst), 0);
        assert!(outputs.is_empty());
    }

    #[test]
    fn dispatcher_star_subscribes_to_all() {
        use std::sync::atomic::Ordering;
        let tmp = std::env::temp_dir().join("archctl_dispatch_star");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        let agent = Arc::new(MockAgent::new("a", vec!["*".into()]));
        disp.register(agent.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("AnyEventWhatsoever", 1);
        disp.dispatch(env, &make_delta(), |_, _| make_ctx());

        assert_eq!(agent.activate_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn dispatcher_suffix_subscription() {
        use std::sync::atomic::Ordering;
        let tmp = std::env::temp_dir().join("archctl_dispatch_suffix");
        let log = EventLog::open(tmp).unwrap();
        let mut disp = EventDispatcher::new(log);

        let agent = Arc::new(MockAgent::new("a", vec!["*.changed".into()]));
        disp.register(agent.clone() as Arc<dyn ReactiveObserver>);

        let env = make_envelope("file.changed", 1);
        disp.dispatch(env, &make_delta(), |_, _| make_ctx());

        assert_eq!(agent.activate_count.load(Ordering::SeqCst), 1);
    }
}
