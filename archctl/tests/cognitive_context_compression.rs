// M34: compress_for_budget 100-event flow + serde back-compat.
use archctl::cognitive::{
    ProvenanceId,
    context::{AgentContext, CompressionPolicy, Evidence},
    descriptor::AgentBudget,
    event::{EventLog, SerializedEvent},
};
use tempfile::TempDir;

fn make_event(seq: u64, event_type: &str) -> SerializedEvent {
    SerializedEvent {
        envelope: archctl::cognitive::event::EventEnvelope {
            event_id: uuid::Uuid::nil(),
            schema_version: "1.0".into(),
            timestamp: chrono::DateTime::from_timestamp(0, 0).unwrap(),
            source: "test".into(),
            producer: "test".into(),
            event_type: event_type.into(),
            payload: serde_json::json!({}),
            seq,
            correlation_id: None,
            causation_id: None,
            graph_revision: None,
        },
        processed: false,
    }
}

#[test]
fn compress_for_budget_100_event_flow() {
    let tmp_dir = TempDir::new().unwrap();
    let log_path = tmp_dir.path().join("100_events.jsonl");
    let mut ledger = EventLog::open(log_path).unwrap();
    for i in 0..100 {
        ledger
            .append_serialized(&make_event(i + 1, "TestEvent"))
            .unwrap();
    }

    let mut ctx = AgentContext {
        goal: "g".into(),
        triggering_event: None,
        graph_view: Default::default(),
        source_fragments: vec![],
        evidence: vec![Evidence {
            id: "ev-1".into(),
            provenance_id: ProvenanceId::File {
                path: "x.rs".into(),
                line: 1,
            },
            content_hash: "abc".into(),
            text: "short".into(),
            properties: Default::default(),
        }],
        applicable_rules: vec![],
        available_tools: vec![],
        budget: AgentBudget {
            tokens: Some(500),
            ..Default::default()
        },
        feedback_history: vec![],
        pending_adjudications: vec![],
        recent_events: vec![],
    };

    let policy = CompressionPolicy {
        budget_chars: 5_000,
        preserve_causation_window: 3,
        decision_priority: archctl::cognitive::context::DecisionPriority::RecencyOnly,
    };

    let report = ctx.compress_for_budget(&policy, &ledger).unwrap();
    assert!(ctx.recent_events.len() <= 10, "recent_events cap violated");
    assert_eq!(report.recent_events_used as usize, ctx.recent_events.len());
    assert_eq!(
        ctx.feedback_history.len(),
        0,
        "feedback_history must be preserved"
    );
    assert_eq!(
        ctx.pending_adjudications.len(),
        0,
        "pending_adjudications must be preserved"
    );
}

#[test]
fn pre_m34_agent_context_json_back_compat() {
    let json = r#"{"goal":"explore","triggering_event":null,"graph_view":{"elements":[],"edges":[]},"source_fragments":[],"evidence":[],"applicable_rules":[],"available_tools":[],"budget":{}}"#;
    let ctx: AgentContext = serde_json::from_str(json).unwrap();
    assert!(
        ctx.recent_events.is_empty(),
        "pre-M34 JSON must default recent_events to []"
    );
    assert!(ctx.feedback_history.is_empty());
    assert!(ctx.pending_adjudications.is_empty());
}
