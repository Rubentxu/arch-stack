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
}
