//! Agent execution context.

use serde::{Deserialize, Serialize};

use super::descriptor::AgentBudget;

/// The context passed to an agent on each invocation.
/// v1.0: built by SyncDispatcher from user goal + graph query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    /// The goal the user asked.
    pub goal: String,
    /// Optional event that triggered this invocation (v1.0 = None for direct invoke).
    pub triggering_event: Option<String>,
    /// Subset of the graph relevant to the goal.
    pub graph_view: GraphView,
    /// Source code fragments backing the analysis.
    pub source_fragments: Vec<SourceFragment>,
    /// Evidence nodes available for citation.
    pub evidence: Vec<Evidence>,
    /// Rules applicable to this context.
    pub applicable_rules: Vec<crate::cognitive::Rule>,
    /// Tools the agent may call.
    pub available_tools: Vec<crate::cognitive::ToolDescriptor>,
    /// Budget for this invocation.
    pub budget: AgentBudget,
    /// Prior feedback verdicts sourced from the store at context-build time
    /// (TRUST-006). Re-invoked agents see this and must respect `Reject`
    /// verdicts — they do not re-propose rejected claims as candidates.
    ///
    /// Ordered deterministically by `(target ASC, revision ASC, timestamp ASC, id ASC)`
    /// so agents receive the same history across calls given the same store state.
    /// Default: empty.
    #[serde(default)]
    pub feedback_history: Vec<crate::feedback::FeedbackSummary>,
    /// Prior open adjudications surfaced at context-build time so re-invoked
    /// agents see which FusedClaims still need a human verdict (REQ-M25-006).
    /// Mirrors `feedback_history` (TRUST-006-b). Default: empty.
    ///
    /// Ordered deterministically by `(decided_at DESC, id ASC)` — same as
    /// `AdjudicationRepository::list_pending_adjudications`.
    #[serde(default)]
    pub pending_adjudications: Vec<crate::adjudication::AdjudicationEvent>,
}

impl AgentContext {
    /// Build an `AgentContext` with its `feedback_history` field populated
    /// from a pre-fetched `Vec<FeedbackSummary>`. Use this chokepoint when
    /// `summaries_for_claims` was already called (e.g. by `SyncDispatcher::build_context`)
    /// or when a test path supplies feedback without a live store handle.
    ///
    /// The struct-literal form `feedback_history: vec![]` remains valid for
    /// sites that intentionally construct a feedback-blind context (e.g. the
    /// round-trip serde test at `context.rs:104`).
    ///
    /// Spec: REQ-T06-003 (TRUST-007), invariant ADR-P02.
    #[allow(clippy::too_many_arguments)]
    pub fn with_feedback_history(
        goal: String,
        triggering_event: Option<String>,
        graph_view: GraphView,
        source_fragments: Vec<SourceFragment>,
        evidence: Vec<Evidence>,
        applicable_rules: Vec<crate::cognitive::Rule>,
        available_tools: Vec<crate::cognitive::ToolDescriptor>,
        budget: AgentBudget,
        feedback_history: Vec<crate::feedback::FeedbackSummary>,
    ) -> Self {
        Self {
            goal,
            triggering_event,
            graph_view,
            source_fragments,
            evidence,
            applicable_rules,
            available_tools,
            budget,
            feedback_history,
            pending_adjudications: vec![],
        }
    }

    /// Build an `AgentContext` with its `pending_adjudications` field
    /// populated from a pre-fetched `Vec<AdjudicationEvent>`. Mirrors
    /// `with_feedback_history` (TRUST-006-b). The struct-literal form
    /// `pending_adjudications: vec![]` remains valid at sites that
    /// intentionally construct an adjudication-blind context.
    ///
    /// Spec: REQ-T08-005 (TRUST-008), invariant ADR-063.
    #[allow(clippy::too_many_arguments)]
    pub fn with_pending_adjudications(
        goal: String,
        triggering_event: Option<String>,
        graph_view: GraphView,
        source_fragments: Vec<SourceFragment>,
        evidence: Vec<Evidence>,
        applicable_rules: Vec<crate::cognitive::Rule>,
        available_tools: Vec<crate::cognitive::ToolDescriptor>,
        budget: AgentBudget,
        feedback_history: Vec<crate::feedback::FeedbackSummary>,
        pending_adjudications: Vec<crate::adjudication::AdjudicationEvent>,
    ) -> Self {
        Self {
            goal,
            triggering_event,
            graph_view,
            source_fragments,
            evidence,
            applicable_rules,
            available_tools,
            budget,
            feedback_history,
            pending_adjudications,
        }
    }
}

/// A subgraph extracted for agent consumption.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphView {
    pub elements: Vec<Element>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    pub id: String,
    pub kind_id: String,
    pub name: String,
    pub canonical_key: String,
    pub properties: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFragment {
    pub file: String,
    pub lang: String,
    pub snippet: String,
    pub line_range: LineRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineRange {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub provenance_id: ProvenanceId,
    pub content_hash: String,
    pub text: String,
    /// Optional structured metadata (confidence, source, etc.)
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub properties: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ProvenanceId {
    #[serde(rename = "file")]
    File { path: String, line: u32 },
    #[serde(rename = "sem")]
    Semantic { scheme: String, value: String },
    #[serde(rename = "sa")]
    SourceArtifact { id: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_context_serde() {
        let ctx = AgentContext {
            goal: "what couples A and B".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let back: AgentContext = serde_json::from_str(&json).unwrap();
        assert_eq!(back.goal, "what couples A and B");
    }

    #[test]
    fn provenance_id_serde() {
        let p = ProvenanceId::File {
            path: "src/main.rs".into(),
            line: 42,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains(r#""kind":"file""#));
    }

    // -----------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage, 2026-08-22)
    // -----------------------------------------------------------------------

    #[test]
    fn with_feedback_history_leaves_pending_adjudications_empty() {
        // Verifies the builder at context.rs:58-81: feedback_history is populated,
        // pending_adjudications defaults to empty (mirrors the struct-literal
        // feedback-blind path documented at context.rs:52-54).
        let ctx = AgentContext::with_feedback_history(
            "what couples A".into(),
            None,
            GraphView::default(),
            vec![],
            vec![],
            vec![],
            vec![],
            AgentBudget::default(),
            vec![],
        );
        assert!(ctx.feedback_history.is_empty());
        assert!(
            ctx.pending_adjudications.is_empty(),
            "with_feedback_history must default pending_adjudications to vec![]"
        );
        assert_eq!(ctx.goal, "what couples A");
    }

    #[test]
    fn with_pending_adjudications_preserves_feedback_history() {
        // Verifies the builder at context.rs:91-115: pending_adjudications is
        // populated, feedback_history argument flows through (mirrors
        // with_feedback_history's contract for feedback).
        let ctx = AgentContext::with_pending_adjudications(
            "audit".into(),
            Some("evt-007".into()),
            GraphView::default(),
            vec![],
            vec![],
            vec![],
            vec![],
            AgentBudget::default(),
            vec![],
            vec![],
        );
        assert!(ctx.pending_adjudications.is_empty());
        assert!(ctx.feedback_history.is_empty());
        assert_eq!(ctx.triggering_event.as_deref(), Some("evt-007"));
    }

    #[test]
    fn agent_context_serde_round_trip_with_triggering_event_some_and_none() {
        // Some case
        let ctx_some = AgentContext {
            goal: "analyze coupling".into(),
            triggering_event: Some("evt-001".into()),
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
        };
        let json = serde_json::to_string(&ctx_some).unwrap();
        let back: AgentContext = serde_json::from_str(&json).unwrap();
        assert_eq!(back.goal, "analyze coupling");
        assert_eq!(back.triggering_event.as_deref(), Some("evt-001"));

        // None case
        let ctx_none = AgentContext {
            goal: "explore".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
        };
        let json = serde_json::to_string(&ctx_none).unwrap();
        let back: AgentContext = serde_json::from_str(&json).unwrap();
        assert_eq!(back.triggering_event, None);
    }

    #[test]
    fn provenance_id_file_serde_round_trip() {
        let p = ProvenanceId::File {
            path: "src/main.rs".into(),
            line: 42,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains(r#""kind":"file""#));
        let back: ProvenanceId = serde_json::from_str(&json).unwrap();
        match back {
            ProvenanceId::File { path, line } => {
                assert_eq!(path, "src/main.rs");
                assert_eq!(line, 42);
            }
            other => panic!("expected File, got {:?}", other),
        }
    }

    #[test]
    fn provenance_id_semantic_serde_round_trip() {
        let p = ProvenanceId::Semantic {
            scheme: "arch".into(),
            value: "c4:component:auth-svc".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains(r#""kind":"sem""#));
        let back: ProvenanceId = serde_json::from_str(&json).unwrap();
        match back {
            ProvenanceId::Semantic { scheme, value } => {
                assert_eq!(scheme, "arch");
                assert_eq!(value, "c4:component:auth-svc");
            }
            other => panic!("expected Semantic, got {:?}", other),
        }
    }

    #[test]
    fn provenance_id_source_artifact_serde_round_trip() {
        let p = ProvenanceId::SourceArtifact {
            id: "sa-001".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains(r#""kind":"sa""#));
        let back: ProvenanceId = serde_json::from_str(&json).unwrap();
        match back {
            ProvenanceId::SourceArtifact { id } => assert_eq!(id, "sa-001"),
            other => panic!("expected SourceArtifact, got {:?}", other),
        }
    }

    #[test]
    fn evidence_properties_skipped_when_empty() {
        // serde_json::Map::is_empty() skip_serializing_if contract at context.rs:163
        let ev = Evidence {
            id: "ev-1".into(),
            provenance_id: ProvenanceId::File {
                path: "src/x.rs".into(),
                line: 1,
            },
            content_hash: "abc123".into(),
            text: "snippet".into(),
            properties: serde_json::Map::new(),
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(
            !json.contains("properties"),
            "empty properties must be skipped via skip_serializing_if"
        );
    }

    #[test]
    fn evidence_properties_included_when_present() {
        let mut props = serde_json::Map::new();
        props.insert("confidence".into(), serde_json::json!(0.95));
        props.insert("source".into(), serde_json::json!("ast-grep"));
        let ev = Evidence {
            id: "ev-1".into(),
            provenance_id: ProvenanceId::File {
                path: "src/x.rs".into(),
                line: 1,
            },
            content_hash: "abc123".into(),
            text: "snippet".into(),
            properties: props,
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains("properties"));
        assert!(json.contains("confidence"));
        assert!(json.contains("ast-grep"));
        // Verify the value also round-trips
        let back: Evidence = serde_json::from_str(&json).unwrap();
        assert_eq!(
            back.properties.get("confidence").unwrap(),
            &serde_json::json!(0.95)
        );
    }

    #[test]
    fn element_serde_round_trip() {
        let el = Element {
            id: "el-001".into(),
            kind_id: "Component".into(),
            name: "AuthService".into(),
            canonical_key: "auth-service".into(),
            properties: serde_json::json!({"tech": "rust", "loc": 1234}),
        };
        let json = serde_json::to_string(&el).unwrap();
        let back: Element = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "el-001");
        assert_eq!(back.kind_id, "Component");
        assert_eq!(back.name, "AuthService");
        assert_eq!(back.canonical_key, "auth-service");
        assert_eq!(back.properties["tech"], "rust");
        assert_eq!(back.properties["loc"], 1234);
    }

    #[test]
    fn edge_serde_with_optional_label() {
        // None case — Edge.label is Option<String> with default serde behavior:
        // serializes as `"label":null` (NOT skipped — no skip_serializing_if on
        // this field at context.rs:139). This locks in the contract for
        // downstream parsers that may distinguish missing-vs-null.
        let e1 = Edge {
            id: "edge-1".into(),
            source_id: "a".into(),
            target_id: "b".into(),
            label: None,
        };
        let json = serde_json::to_string(&e1).unwrap();
        assert!(
            json.contains(r#""label":null"#),
            "Edge.label=None must serialize as explicit null (default Option behavior)"
        );

        // Some case — round-trip preserves the label string
        let e2 = Edge {
            id: "edge-2".into(),
            source_id: "a".into(),
            target_id: "b".into(),
            label: Some("depends_on".into()),
        };
        let json = serde_json::to_string(&e2).unwrap();
        assert!(json.contains(r#""label":"depends_on""#));
        let back: Edge = serde_json::from_str(&json).unwrap();
        assert_eq!(back.label.as_deref(), Some("depends_on"));

        // None round-trip: parses back to None
        let none_json = json.replace(r#""label":"depends_on""#, r#""label":null"#);
        let back_none: Edge = serde_json::from_str(&none_json).unwrap();
        assert!(back_none.label.is_none());
    }

    #[test]
    fn graph_view_default_is_empty() {
        let gv = GraphView::default();
        assert!(gv.elements.is_empty());
        assert!(gv.edges.is_empty());
    }

    #[test]
    fn line_range_serde_round_trip() {
        let lr = LineRange { start: 10, end: 20 };
        let json = serde_json::to_string(&lr).unwrap();
        let back: LineRange = serde_json::from_str(&json).unwrap();
        assert_eq!(back.start, 10);
        assert_eq!(back.end, 20);

        // Single-line range (start == end) is valid
        let lr_single = LineRange { start: 5, end: 5 };
        let json = serde_json::to_string(&lr_single).unwrap();
        let back: LineRange = serde_json::from_str(&json).unwrap();
        assert_eq!(back.start, 5);
        assert_eq!(back.end, 5);
    }

    #[test]
    fn source_fragment_serde_round_trip() {
        let sf = SourceFragment {
            file: "src/lib.rs".into(),
            lang: "rust".into(),
            snippet: "pub fn foo() {}".into(),
            line_range: LineRange { start: 1, end: 1 },
        };
        let json = serde_json::to_string(&sf).unwrap();
        let back: SourceFragment = serde_json::from_str(&json).unwrap();
        assert_eq!(back.file, "src/lib.rs");
        assert_eq!(back.lang, "rust");
        assert_eq!(back.snippet, "pub fn foo() {}");
        assert_eq!(back.line_range.start, 1);
    }
}
