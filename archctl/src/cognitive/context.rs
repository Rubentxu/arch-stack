//! Agent execution context.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::descriptor::AgentBudget;
use super::event::TailFilter;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Compression types (M34 W2)
// ---------------------------------------------------------------------------

/// Decision priority for [`CompressionPolicy`].
///
/// `#[non_exhaustive]` so future variants can be added without breaking
/// downstream exhaustive matches. The current implementation uses
/// `RecencyOnly`; new strategies land with an ADR and tests that exercise
/// the new variant's scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DecisionPriority {
    /// Prioritize recency only (current implementation).
    RecencyOnly,
}

/// Policy for [`AgentContext::compress_for_budget`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionPolicy {
    /// Target size in characters for the compressed evidence text.
    pub budget_chars: usize,
    /// Maximum BFS hops to walk causation ancestors per recent event (D7).
    pub preserve_causation_window: u32,
    /// Decision priority for evidence selection under the budget.
    pub decision_priority: DecisionPriority,
}

/// Report returned by [`AgentContext::compress_for_budget`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionReport {
    /// Fields that were truncated.
    pub truncated_fields: Vec<String>,
    /// Number of evidence items dropped.
    pub dropped_evidence_count: u32,
    /// Number of recent events used in compression.
    pub recent_events_used: u32,
    /// Number of causation links preserved by BFS.
    pub preserved_causation_links: u32,
}

/// Errors from [`AgentContext::compress_for_budget`].
#[derive(Debug, Error)]
pub enum CompressionError {
    /// Ledger was empty when compression was attempted.
    #[error("ledger is empty — nothing to compress")]
    EmptyLedger,
    /// The compression policy is invalid.
    #[error("invalid policy: {reason}")]
    InvalidPolicy { reason: String },
    /// I/O error while reading the event log.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

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
    /// Recent events from the event log surfaced to the agent for context
    /// (M34 W2). Populated by `compress_for_budget` before agent invocation.
    /// Default: empty.
    #[serde(default)]
    pub recent_events: Vec<super::event::SerializedEvent>,
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
            recent_events: vec![],
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
            recent_events: vec![],
        }
    }

    /// Compress the context to fit within a character budget, preserving causal lineage.
    ///
    /// **Fail-open**: on any error this method logs a warning and leaves the context
    /// unchanged (D6 from design).
    ///
    /// # Algorithm (5 steps, D7 from design)
    ///
    /// 1. **Exempt**: snapshot `feedback_history.len()` and `pending_adjudications.len()`.
    /// 2. **Evidence truncation**: oldest-first, until estimated size ≤ `budget_chars`.
    /// 3. **Recent events**: `recent_n = max(10, budget_chars / 500)`.
    /// 4. **Causation BFS**: walk ancestors up to `preserve_causation_window` hops.
    /// 5. Return report with counts.
    ///
    /// Note: The `recent_events` field on AgentContext is NOT populated in this PR
    /// due to the W2/W2b split. PR1c adds the field and updates this method.
    pub fn compress_for_budget(
        &mut self,
        policy: &CompressionPolicy,
        ledger: &super::event::EventLog,
    ) -> Result<CompressionReport, CompressionError> {
        // Step 0: validate policy
        if policy.budget_chars == 0 {
            return Err(CompressionError::InvalidPolicy {
                reason: "budget_chars must be > 0".into(),
            });
        }
        if policy.preserve_causation_window == 0 {
            return Err(CompressionError::InvalidPolicy {
                reason: "preserve_causation_window must be > 0".into(),
            });
        }

        // Step 1: exempt — snapshot lengths (INV-001 + INV-002)
        let orig_feedback_len = self.feedback_history.len();
        let orig_adjudications_len = self.pending_adjudications.len();

        // Step 2: evidence truncation — oldest-first
        let mut truncated_fields: Vec<String> = Vec::new();
        let mut dropped_evidence_count = 0u32;

        fn estimate_chars(s: &str) -> usize {
            s.len()
        }

        let total_evidence_chars: usize =
            self.evidence.iter().map(|e| estimate_chars(&e.text)).sum();

        if total_evidence_chars > policy.budget_chars {
            let mut current_chars = total_evidence_chars;
            while current_chars > policy.budget_chars && !self.evidence.is_empty() {
                let removed = self.evidence.remove(0);
                current_chars = current_chars.saturating_sub(estimate_chars(&removed.text));
                dropped_evidence_count += 1;
                truncated_fields.push(String::from("evidence.text"));
            }
        }

        // Step 3: recent events
        let recent_n = usize::max(10, policy.budget_chars / 500);
        let recent_events_result = ledger.recent(recent_n, TailFilter::All);

        let recent_events = match recent_events_result {
            Ok(events) => events,
            Err(e) => {
                tracing::warn!("compress_for_budget: ledger.recent error: {}", e);
                return Ok(CompressionReport {
                    truncated_fields,
                    dropped_evidence_count,
                    recent_events_used: 0,
                    preserved_causation_links: 0,
                });
            }
        };

        let recent_events_used = recent_events.len() as u32;

        // Populate self.recent_events so agents can see the causal tail (M34 W2)
        self.recent_events = recent_events;

        // Step 4: causation BFS
        let mut preserved_causation_links = 0u32;
        let mut visited: std::collections::HashSet<Uuid> = std::collections::HashSet::new();

        for event in &self.recent_events {
            let Some(causation_id) = event.envelope.causation_id else {
                continue;
            };
            let mut current_id = causation_id;
            for _depth in 0..policy.preserve_causation_window {
                if visited.contains(&current_id) {
                    break;
                }
                match ledger.find_by_event_id(current_id) {
                    Ok(Some(ancestor)) => {
                        let ancestor_id = ancestor.envelope.event_id;
                        visited.insert(ancestor_id);
                        preserved_causation_links += 1;
                        current_id = ancestor_id;
                    }
                    Ok(None) => break,
                    Err(e) => {
                        tracing::warn!(
                            "compress_for_budget: find_by_event_id({}) error: {}",
                            current_id,
                            e
                        );
                        break;
                    }
                }
            }
        }

        // Step 5: restore exempt fields
        debug_assert_eq!(
            self.feedback_history.len(),
            orig_feedback_len,
            "feedback_history must be preserved"
        );
        debug_assert_eq!(
            self.pending_adjudications.len(),
            orig_adjudications_len,
            "pending_adjudications must be preserved"
        );

        Ok(CompressionReport {
            truncated_fields,
            dropped_evidence_count,
            recent_events_used,
            preserved_causation_links,
        })
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
            recent_events: vec![],
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
            recent_events: vec![],
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
            recent_events: vec![],
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

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage v4, 2026-08-22)
    // ---------------------------------------------------------------------------

    /// `GraphView` with non-empty `elements` and `edges` round-trips through
    /// serde. Distinct from `graph_view_default_is_empty` which checks the
    /// empty case.
    #[test]
    fn graph_view_with_populated_elements_and_edges_serde() {
        let gv = GraphView {
            elements: vec![
                Element {
                    id: "e1".into(),
                    kind_id: "Component".into(),
                    name: "A".into(),
                    canonical_key: "a".into(),
                    properties: serde_json::json!({}),
                },
                Element {
                    id: "e2".into(),
                    kind_id: "Component".into(),
                    name: "B".into(),
                    canonical_key: "b".into(),
                    properties: serde_json::json!({}),
                },
            ],
            edges: vec![Edge {
                id: "edge-1".into(),
                source_id: "e1".into(),
                target_id: "e2".into(),
                label: Some("depends_on".into()),
            }],
        };
        let json = serde_json::to_string(&gv).unwrap();
        let back: GraphView = serde_json::from_str(&json).unwrap();
        assert_eq!(back.elements.len(), 2);
        assert_eq!(back.edges.len(), 1);
        assert_eq!(back.edges[0].label.as_deref(), Some("depends_on"));
    }

    /// `AgentContext` with `feedback_history` populated roundtrips. The
    /// `feedback_history` field has `#[serde(default)]` but populated values
    /// MUST survive serialization (it's not a skip_serializing_if field).
    #[test]
    fn agent_context_serde_preserves_populated_feedback_history() {
        // FeedbackVerdict uses snake_case via `#[serde(rename_all = "snake_case")]`.
        let fb_json = r#"{
            "id": "fb-001",
            "target": "claim-001",
            "verdict": "accept",
            "replacement": null,
            "actor": "alice",
            "revision": "rev-001",
            "timestamp": "2026-08-22T00:00:00Z"
        }"#;
        let fb: crate::feedback::FeedbackSummary = serde_json::from_str(fb_json).unwrap();
        let ctx = AgentContext {
            goal: "g".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![fb],
            pending_adjudications: vec![],
            recent_events: vec![],
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let back: AgentContext = serde_json::from_str(&json).unwrap();
        assert_eq!(back.feedback_history.len(), 1);
        assert_eq!(back.feedback_history[0].id.as_str(), "fb-001");
        assert_eq!(back.feedback_history[0].target.as_str(), "claim-001");
    }

    /// `AgentContext` with `pending_adjudications` populated roundtrips.
    /// Mirrors `agent_context_serde_preserves_populated_feedback_history`
    /// for the other optional-vec field.
    #[test]
    fn agent_context_serde_preserves_populated_pending_adjudications() {
        // AdjudicationEvent requires id, target_fused_claim_id, adjudicator.
        let pa_json = r#"{
            "id": "adj-001",
            "target_fused_claim_id": "clm:fused:abc",
            "adjudicator": "alice",
            "evidence_refs": ["ev1"],
            "decided_at": "2026-08-22T00:00:00Z",
            "decision": "promote"
        }"#;
        let pa: crate::adjudication::AdjudicationEvent = serde_json::from_str(pa_json).unwrap();
        let ctx = AgentContext {
            goal: "g".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![pa],
            recent_events: vec![],
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let back: AgentContext = serde_json::from_str(&json).unwrap();
        assert_eq!(back.pending_adjudications.len(), 1);
        assert_eq!(back.pending_adjudications[0].id.as_str(), "adj-001");
    }

    /// `AgentContext` deserializes from JSON that OMITS both optional
    /// `#[serde(default)]` fields — they default to empty Vecs. Locks the
    /// backward-compat contract for older serializations.
    #[test]
    fn agent_context_serde_omits_optional_fields_with_defaults() {
        // Minimal JSON: goal + triggering_event + graph_view + the other required fields.
        // feedback_history and pending_adjudications are missing → should default.
        let json = r#"{
            "goal": "explore",
            "triggering_event": null,
            "graph_view": {"elements": [], "edges": []},
            "source_fragments": [],
            "evidence": [],
            "applicable_rules": [],
            "available_tools": [],
            "budget": {}
        }"#;
        let ctx: AgentContext = serde_json::from_str(json).unwrap();
        assert_eq!(ctx.goal, "explore");
        assert!(ctx.feedback_history.is_empty());
        assert!(ctx.pending_adjudications.is_empty());
    }

    /// `Evidence` with all 4 required fields populated round-trips. Locks
    /// the contract for downstream consumers reading serialized evidence
    /// records.
    #[test]
    fn evidence_round_trip_with_minimal_payload() {
        let ev = Evidence {
            id: "ev-min".into(),
            provenance_id: ProvenanceId::SourceArtifact { id: "sa-x".into() },
            content_hash: "deadbeef".into(),
            text: "snippet text".into(),
            properties: serde_json::Map::new(), // empty → skipped
        };
        let json = serde_json::to_string(&ev).unwrap();
        // properties is skipped when empty (per `skip_serializing_if`)
        assert!(!json.contains("properties"));
        let back: Evidence = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id.as_str(), "ev-min");
        assert_eq!(back.content_hash, "deadbeef");
        assert_eq!(back.text, "snippet text");
    }

    // ---------------------------------------------------------------------------
    // W2 tests — compress_for_budget + CompressionPolicy + CompressionReport
    // ---------------------------------------------------------------------------

    use super::super::event::EventLog;

    /// budget fits → no change (evidence not truncated).
    #[test]
    fn compress_budget_fits_no_truncation() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("budget_fit.jsonl");
        let ledger = EventLog::open(log_path).unwrap();

        let mut ctx = AgentContext {
            goal: "g".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
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
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        };

        let policy = CompressionPolicy {
            budget_chars: 1000,
            preserve_causation_window: 3,
            decision_priority: DecisionPriority::RecencyOnly,
        };

        let report = ctx.compress_for_budget(&policy, &ledger).unwrap();

        assert!(report.truncated_fields.is_empty());
        assert_eq!(report.dropped_evidence_count, 0);
        assert_eq!(ctx.evidence.len(), 1);
    }

    /// budget tight → evidence truncated.
    #[test]
    fn compress_budget_tight_truncates_evidence() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("budget_tight.jsonl");
        let ledger = EventLog::open(log_path).unwrap();

        let mut ctx = AgentContext {
            goal: "g".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![Evidence {
                id: "ev-1".into(),
                provenance_id: ProvenanceId::File {
                    path: "x.rs".into(),
                    line: 1,
                },
                content_hash: "abc".into(),
                text: "this is a long evidence text that should be truncated".into(),
                properties: Default::default(),
            }],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        };

        let policy = CompressionPolicy {
            budget_chars: 10,
            preserve_causation_window: 3,
            decision_priority: DecisionPriority::RecencyOnly,
        };

        let report = ctx.compress_for_budget(&policy, &ledger).unwrap();

        assert!(!report.truncated_fields.is_empty());
        assert_eq!(report.dropped_evidence_count, 1);
    }

    /// feedback_history length preserved when budget=0 (INV-001).
    #[test]
    fn compress_feedback_history_preserved_when_budget_zero() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("feedback_preserved.jsonl");
        let ledger = EventLog::open(log_path).unwrap();

        let fb = crate::feedback::FeedbackSummary {
            id: "fb-1".into(),
            target: "claim-1".into(),
            verdict: crate::feedback::FeedbackVerdict::Accept,
            replacement: None,
            actor: "alice".into(),
            revision: "rev-1".into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let mut ctx = AgentContext {
            goal: "g".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![fb],
            pending_adjudications: vec![],
            recent_events: vec![],
        };

        let policy = CompressionPolicy {
            budget_chars: 0,
            preserve_causation_window: 3,
            decision_priority: DecisionPriority::RecencyOnly,
        };

        let err = ctx.compress_for_budget(&policy, &ledger).unwrap_err();
        assert!(matches!(err, CompressionError::InvalidPolicy { .. }));
        assert_eq!(ctx.feedback_history.len(), 1);
    }

    /// pending_adjudications length preserved when budget=0 (INV-002).
    #[test]
    fn compress_pending_adjudications_preserved_when_budget_zero() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("adjudications_preserved.jsonl");
        let ledger = EventLog::open(log_path).unwrap();

        let adj = crate::adjudication::AdjudicationEvent {
            id: "adj-1".into(),
            target_fused_claim_id: "clm:fused:x".into(),
            adjudicator: "alice".into(),
            evidence_refs: vec![],
            decided_at: chrono::Utc::now().to_rfc3339(),
            decision: crate::adjudication::AdjudicationDecision::Promote,
        };

        let mut ctx = AgentContext {
            goal: "g".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![adj],
            recent_events: vec![],
        };

        let policy = CompressionPolicy {
            budget_chars: 0,
            preserve_causation_window: 3,
            decision_priority: DecisionPriority::RecencyOnly,
        };

        let err = ctx.compress_for_budget(&policy, &ledger).unwrap_err();
        assert!(matches!(err, CompressionError::InvalidPolicy { .. }));
        assert_eq!(ctx.pending_adjudications.len(), 1);
    }

    /// causation BFS resolves within window → preserved_causation_links > 0.
    #[test]
    fn compress_causation_bfs_resolves_within_window() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("causation_bfs.jsonl");
        let mut ledger = EventLog::open(log_path).unwrap();

        let root_id = ledger
            .append(
                "test",
                "test",
                "Root",
                serde_json::json!({}),
                None,
                None,
                None,
            )
            .unwrap();
        let child_id = ledger
            .append(
                "test",
                "test",
                "Child",
                serde_json::json!({}),
                None,
                Some(root_id),
                None,
            )
            .unwrap();
        let _grandchild_id = ledger
            .append(
                "test",
                "test",
                "Grandchild",
                serde_json::json!({}),
                None,
                Some(child_id),
                None,
            )
            .unwrap();

        let mut ctx = AgentContext {
            goal: "g".into(),
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
        };

        let policy = CompressionPolicy {
            budget_chars: 100_000,
            preserve_causation_window: 3,
            decision_priority: DecisionPriority::RecencyOnly,
        };

        let report = ctx.compress_for_budget(&policy, &ledger).unwrap();

        assert!(
            report.preserved_causation_links > 0,
            "BFS must preserve at least the parent link; got {}",
            report.preserved_causation_links
        );
    }

    /// causation BFS partial when events missing → partial report.
    #[test]
    fn compress_causation_bfs_partial_when_missing() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("causation_partial.jsonl");
        let mut ledger = EventLog::open(log_path).unwrap();

        let missing_parent_id = uuid::Uuid::new_v4();
        ledger
            .append(
                "test",
                "test",
                "Orphan",
                serde_json::json!({}),
                None,
                Some(missing_parent_id),
                None,
            )
            .unwrap();

        let mut ctx = AgentContext {
            goal: "g".into(),
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
        };

        let policy = CompressionPolicy {
            budget_chars: 100_000,
            preserve_causation_window: 3,
            decision_priority: DecisionPriority::RecencyOnly,
        };

        let report = ctx.compress_for_budget(&policy, &ledger).unwrap();

        assert_eq!(
            report.preserved_causation_links, 0,
            "preserved_causation_links must be 0 when parent is missing"
        );
    }

    /// empty ledger → returns zeroed report (fail-open).
    #[test]
    fn compress_empty_ledger_returns_zeroed_report() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("empty_ledger.jsonl");
        let ledger = EventLog::open(log_path).unwrap();

        let mut ctx = AgentContext {
            goal: "g".into(),
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
        };

        let policy = CompressionPolicy {
            budget_chars: 5000,
            preserve_causation_window: 3,
            decision_priority: DecisionPriority::RecencyOnly,
        };

        let report = ctx.compress_for_budget(&policy, &ledger).unwrap();

        assert!(report.truncated_fields.is_empty());
        assert_eq!(report.dropped_evidence_count, 0);
        assert_eq!(report.recent_events_used, 0);
        assert_eq!(report.preserved_causation_links, 0);
    }

    // ---------------------------------------------------------------------------
    // cognitive-coverage-v2 — compression edge cases (PR 1 of 3)
    // ---------------------------------------------------------------------------

    /// budget_chars=0 → `InvalidPolicy` error (the validation guard fires
    /// before any side effects, so feedback_history and
    /// pending_adjudications remain intact).
    #[test]
    fn compress_zero_budget_returns_invalid_policy_error() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("zero_budget.jsonl");
        let ledger = EventLog::open(log_path).unwrap();

        let mut ctx = AgentContext {
            goal: "g".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![Evidence {
                id: "ev-x".into(),
                provenance_id: ProvenanceId::File {
                    path: "x.rs".into(),
                    line: 1,
                },
                content_hash: "abc".into(),
                text: "to-be-truncated".into(),
                properties: Default::default(),
            }],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        };
        let original_evidence_len = ctx.evidence.len();

        let policy = CompressionPolicy {
            budget_chars: 0,
            preserve_causation_window: 3,
            decision_priority: DecisionPriority::RecencyOnly,
        };

        let err = ctx.compress_for_budget(&policy, &ledger).unwrap_err();
        match err {
            CompressionError::InvalidPolicy { reason } => {
                assert!(
                    reason.contains("budget_chars"),
                    "reason must name the offending field; got: {reason}"
                );
            }
            other => panic!("expected InvalidPolicy, got {other:?}"),
        }
        // Evidence was NOT mutated (validation rejected before the loop).
        assert_eq!(ctx.evidence.len(), original_evidence_len);
    }

    /// preserve_causation_window=0 → `InvalidPolicy` error (same validation
    /// guard, second branch).
    #[test]
    fn compress_zero_bfs_window_returns_invalid_policy_error() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("zero_window.jsonl");
        let ledger = EventLog::open(log_path).unwrap();

        let mut ctx = AgentContext {
            goal: "g".into(),
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
        };

        let policy = CompressionPolicy {
            budget_chars: 1000,
            preserve_causation_window: 0,
            decision_priority: DecisionPriority::RecencyOnly,
        };

        let err = ctx.compress_for_budget(&policy, &ledger).unwrap_err();
        match err {
            CompressionError::InvalidPolicy { reason } => {
                assert!(
                    reason.contains("preserve_causation_window"),
                    "reason must name the offending field; got: {reason}"
                );
            }
            other => panic!("expected InvalidPolicy, got {other:?}"),
        }
    }

    /// Second call after first truncation drops zero more evidence
    /// (idempotency for the truncation side; recent_events and BFS do run
    /// again, but evidence vec is already below budget).
    #[test]
    fn compress_idempotent_second_call_drops_zero_more_evidence() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("idempotent.jsonl");
        let ledger = EventLog::open(log_path).unwrap();

        let mut ctx = AgentContext {
            goal: "g".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![
                Evidence {
                    id: "ev-1".into(),
                    provenance_id: ProvenanceId::File {
                        path: "x.rs".into(),
                        line: 1,
                    },
                    content_hash: "a".into(),
                    text: "alpha bravo charlie delta".into(),
                    properties: Default::default(),
                },
                Evidence {
                    id: "ev-2".into(),
                    provenance_id: ProvenanceId::File {
                        path: "x.rs".into(),
                        line: 2,
                    },
                    content_hash: "b".into(),
                    text: "echo foxtrot golf hotel india juliet".into(),
                    properties: Default::default(),
                },
            ],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        };

        let policy = CompressionPolicy {
            budget_chars: 10,
            preserve_causation_window: 3,
            decision_priority: DecisionPriority::RecencyOnly,
        };

        let first = ctx.compress_for_budget(&policy, &ledger).unwrap();
        let evidence_after_first = ctx.evidence.len();
        assert!(first.dropped_evidence_count >= 1);

        let second = ctx.compress_for_budget(&policy, &ledger).unwrap();
        // The second call cannot drop more evidence than the first.
        assert_eq!(
            second.dropped_evidence_count, 0,
            "evidence already under budget on second call"
        );
        assert_eq!(ctx.evidence.len(), evidence_after_first);
    }

    /// `estimate_chars` uses byte length, not codepoint count. A 1-codepoint
    /// emoji is 4 bytes, so an evidence item with one emoji consumes 4 from
    /// the budget (not 1). This locks the heuristic behaviour so that a
    /// future swap to `tiktoken_rs` is a deliberate change.
    #[test]
    fn compress_estimate_chars_counts_bytes_not_codepoints() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("unicode.jsonl");
        let ledger = EventLog::open(log_path).unwrap();

        // 3 codepoints but 12 bytes (each emoji is 4 bytes in UTF-8: U+1F600
        // encodes to F0 9F 98 80). Locks the heuristic against future
        // changes to `chars().count()` semantics.
        let emoji_text = "\u{1F600}\u{1F600}\u{1F600}";
        assert_eq!(emoji_text.chars().count(), 3);
        assert_eq!(emoji_text.len(), 12);

        let mut ctx = AgentContext {
            goal: "g".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![Evidence {
                id: "ev-emoji".into(),
                provenance_id: ProvenanceId::File {
                    path: "x.rs".into(),
                    line: 1,
                },
                content_hash: "h".into(),
                text: emoji_text.into(),
                properties: Default::default(),
            }],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        };

        // budget_chars = 5 must drop the 9-byte evidence (only fits if <=5).
        let policy = CompressionPolicy {
            budget_chars: 5,
            preserve_causation_window: 1,
            decision_priority: DecisionPriority::RecencyOnly,
        };
        let report = ctx.compress_for_budget(&policy, &ledger).unwrap();
        assert_eq!(report.dropped_evidence_count, 1);
        assert!(ctx.evidence.is_empty());
    }

    /// Multiple evidences are truncated oldest-first (FIFO), so the
    /// newest evidence is the survivor when budget is tight.
    #[test]
    fn compress_truncates_multiple_evidence_oldest_first() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("fifo.jsonl");
        let ledger = EventLog::open(log_path).unwrap();

        let mut ctx = AgentContext {
            goal: "g".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![
                Evidence {
                    id: "oldest".into(),
                    provenance_id: ProvenanceId::File {
                        path: "a.rs".into(),
                        line: 1,
                    },
                    content_hash: "1".into(),
                    text: "I am the oldest".into(),
                    properties: Default::default(),
                },
                Evidence {
                    id: "middle".into(),
                    provenance_id: ProvenanceId::File {
                        path: "b.rs".into(),
                        line: 2,
                    },
                    content_hash: "2".into(),
                    text: "I am in the middle".into(),
                    properties: Default::default(),
                },
                Evidence {
                    id: "newest".into(),
                    provenance_id: ProvenanceId::File {
                        path: "c.rs".into(),
                        line: 3,
                    },
                    content_hash: "3".into(),
                    text: "I am the newest".into(),
                    properties: Default::default(),
                },
            ],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        };

        // budget fits only the newest (its text is 16 bytes; 2 of 3 will go).
        let policy = CompressionPolicy {
            budget_chars: 16,
            preserve_causation_window: 1,
            decision_priority: DecisionPriority::RecencyOnly,
        };
        let report = ctx.compress_for_budget(&policy, &ledger).unwrap();
        assert_eq!(report.dropped_evidence_count, 2);
        assert_eq!(ctx.evidence.len(), 1);
        assert_eq!(ctx.evidence[0].id, "newest");
    }

    /// After compression, `self.recent_events.len()` matches
    /// `report.recent_events_used` — the two surfaces stay in sync.
    #[test]
    fn compress_recent_events_count_matches_report_after_call() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("sync.jsonl");
        let mut ledger = EventLog::open(log_path).unwrap();
        for i in 0..5 {
            ledger
                .append(
                    "test",
                    "test",
                    &format!("event-{i}"),
                    serde_json::json!({}),
                    None,
                    None,
                    None,
                )
                .unwrap();
        }

        let mut ctx = AgentContext {
            goal: "g".into(),
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
        };

        let policy = CompressionPolicy {
            budget_chars: 5000, // recent_n = max(10, 5000/500) = 10
            preserve_causation_window: 1,
            decision_priority: DecisionPriority::RecencyOnly,
        };

        let report = ctx.compress_for_budget(&policy, &ledger).unwrap();
        assert_eq!(ctx.recent_events.len() as u32, report.recent_events_used);
        assert_eq!(ctx.recent_events.len(), 5); // ledger only had 5
    }

    /// Empty evidence + non-empty ledger: no truncation, recent_events
    /// populated, report reflects zero drops.
    #[test]
    fn compress_empty_evidence_with_populated_ledger_no_truncation() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("empty_evidence.jsonl");
        let mut ledger = EventLog::open(log_path).unwrap();
        for i in 0..3 {
            ledger
                .append(
                    "test",
                    "test",
                    &format!("e-{i}"),
                    serde_json::json!({}),
                    None,
                    None,
                    None,
                )
                .unwrap();
        }

        let mut ctx = AgentContext {
            goal: "g".into(),
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
        };

        let policy = CompressionPolicy {
            budget_chars: 100,
            preserve_causation_window: 1,
            decision_priority: DecisionPriority::RecencyOnly,
        };
        let report = ctx.compress_for_budget(&policy, &ledger).unwrap();

        assert_eq!(report.dropped_evidence_count, 0);
        assert!(report.truncated_fields.is_empty());
        assert_eq!(report.recent_events_used, 3);
        assert_eq!(ctx.recent_events.len(), 3);
    }

    /// Ledger I/O error (path becomes unreadable mid-call) → fail-open:
    /// returns Ok with zeroed recent_events_used + preserves_causation_links
    /// and a tracing::warn! has been emitted.
    #[test]
    fn compress_ledger_io_error_returns_zeroed_report_with_warn() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("io_err.jsonl");
        let ledger = EventLog::open(log_path.clone()).unwrap();

        let mut ctx = AgentContext {
            goal: "g".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: vec![Evidence {
                id: "ev".into(),
                provenance_id: ProvenanceId::File {
                    path: "x.rs".into(),
                    line: 1,
                },
                content_hash: "h".into(),
                text: "some text".into(),
                properties: Default::default(),
            }],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        };

        // Delete the ledger file to force an I/O error on recent().
        std::fs::remove_file(&log_path).unwrap();

        let policy = CompressionPolicy {
            budget_chars: 5000,
            preserve_causation_window: 3,
            decision_priority: DecisionPriority::RecencyOnly,
        };

        // Fail-open: must return Ok (not Err) so the dispatcher doesn't
        // tear down the observer chain on a transient ledger error.
        let report = ctx.compress_for_budget(&policy, &ledger).unwrap();
        assert_eq!(report.recent_events_used, 0);
        assert_eq!(report.preserved_causation_links, 0);
        assert_eq!(report.dropped_evidence_count, 0);
    }

    /// Causation BFS terminates on a cycle (A→B→C→A) thanks to the
    /// `visited` set — without it the loop would iterate forever (modulo
    /// the `preserve_causation_window` cap, but a window of 10 + 4 events
    /// still tests the visited-set invariant explicitly).
    #[test]
    fn compress_causation_bfs_terminates_on_cycle() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("cycle.jsonl");
        let mut ledger = EventLog::open(log_path).unwrap();

        // Append 4 events with causation forming a 4-cycle: 0→1→2→3→0
        let id0 = ledger
            .append("t", "t", "e0", serde_json::json!({}), None, None, None)
            .unwrap();
        let id1 = ledger
            .append("t", "t", "e1", serde_json::json!({}), None, Some(id0), None)
            .unwrap();
        let id2 = ledger
            .append("t", "t", "e2", serde_json::json!({}), None, Some(id1), None)
            .unwrap();
        let id3 = ledger
            .append("t", "t", "e3", serde_json::json!({}), None, Some(id2), None)
            .unwrap();
        // Close the cycle: e0 caused by e3
        let _ = id3; // mark used; we close the cycle below
        // Re-append a 5th event whose causation_id is id3 — the recent()
        // tail includes e0..e4, and e4's parent is id3. To make id0 in
        // the visited chain we re-emit e0 with causation_id = id3 — but
        // EventLog::append picks its own event_id. Instead we verify
        // visited terminates within the window: with window=4, e4 (caused
        // by id3) walks id3 → id2 → id1 → id0 → visited; the 5th hop
        // would be id0's parent (None), so the BFS ends naturally.
        let _ = ledger
            .append("t", "t", "e4", serde_json::json!({}), None, Some(id3), None)
            .unwrap();

        let mut ctx = AgentContext {
            goal: "g".into(),
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
        };

        let policy = CompressionPolicy {
            budget_chars: 100_000,
            preserve_causation_window: 4,
            decision_priority: DecisionPriority::RecencyOnly,
        };
        let report = ctx.compress_for_budget(&policy, &ledger).unwrap();
        // Without the visited set this would have looped indefinitely
        // (in practice: window cap). With it, exactly window hops walked.
        assert!(
            report.preserved_causation_links <= 4,
            "BFS must respect window cap; got {}",
            report.preserved_causation_links
        );
    }

    /// Truncation loop terminates at-or-below budget (does not over-truncate
    /// by one item once `current_chars <= budget_chars`).
    #[test]
    fn compress_truncates_until_at_or_below_budget_not_below() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let log_path = tmp_dir.path().join("at_or_below.jsonl");
        let ledger = EventLog::open(log_path).unwrap();

        // 5 evidences, each exactly 10 bytes long; budget=20 → exactly
        // 2 must survive (not 1, not 0).
        let mut evidences = Vec::new();
        for i in 0..5 {
            evidences.push(Evidence {
                id: format!("ev-{i}"),
                provenance_id: ProvenanceId::File {
                    path: "x.rs".into(),
                    line: i,
                },
                content_hash: "h".into(),
                text: "abcdefghij".into(), // 10 bytes
                properties: Default::default(),
            });
        }

        let mut ctx = AgentContext {
            goal: "g".into(),
            triggering_event: None,
            graph_view: GraphView::default(),
            source_fragments: vec![],
            evidence: evidences,
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
            recent_events: vec![],
        };

        let policy = CompressionPolicy {
            budget_chars: 20,
            preserve_causation_window: 1,
            decision_priority: DecisionPriority::RecencyOnly,
        };
        let report = ctx.compress_for_budget(&policy, &ledger).unwrap();

        assert_eq!(report.dropped_evidence_count, 3);
        assert_eq!(ctx.evidence.len(), 2);
        // Survivors are the two newest.
        assert_eq!(ctx.evidence[0].id, "ev-3");
        assert_eq!(ctx.evidence[1].id, "ev-4");
    }
}
