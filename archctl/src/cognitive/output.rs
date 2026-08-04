//! Agent output types.

use serde::{Deserialize, Serialize};

/// Structured output produced by an agent after observation.
/// All variants carry evidence-backed structured data — never raw text.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum AgentOutput {
    Hypothesis(Hypothesis),
    FindingCandidate(FindingCandidate),
    QueryPlan(QueryPlan),
    ProjectionSpec(ProjectionSpec),
    ActionPlan(ActionPlan),
    ActionProposal(ActionProposal),
    DocumentationPatch(DocumentationPatch),
    ContextRequest(ContextRequest),
    NoAction(NoActionReason),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub statement: String,
    pub confidence: f64,
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingCandidate {
    pub severity: Severity,
    pub title: String,
    pub body: String,
    pub confidence: f64,
    pub evidence_ids: Vec<String>,
    pub recommended_views: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub cypher_steps: Vec<String>,
    pub estimated_rows: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionSpec {
    pub view_kind: ViewKind,
    pub format: DiagramFormat,
    pub focus_elements: Vec<String>,
    pub layout_hints: LayoutHints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewKind {
    #[serde(rename = "c4-context")]
    C4Context,
    #[serde(rename = "c4-container")]
    C4Container,
    #[serde(rename = "c4-component")]
    C4Component,
    Class,
    Sequence,
    State,
    UseCase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiagramFormat {
    PlantUML,
    Mermaid,
    Structurizr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutHints {
    pub direction: Option<LayoutDirection>,
    pub ranksep: Option<f64>,
    pub nodesep: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutDirection {
    TopDown,
    LeftRight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    pub steps: Vec<Step>,
    pub rollback: Option<Vec<Step>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub command: String,
    pub args: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionProposal {
    pub goal: String,
    pub command: String,
    pub args: Vec<String>,
    pub capabilities: Vec<String>,
    pub approval_required: bool,
    pub expected_evidence: String,
    pub rollback: Option<Vec<Step>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationPatch {
    pub file: String,
    pub patch_type: PatchType,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatchType {
    Add,
    Replace,
    Remove,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRequest {
    pub request_id: String,
    pub missing: Vec<String>,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoActionReason {
    pub code: NoActionCode,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoActionCode {
    InsufficientConfidence,
    NoRelevantData,
    OutOfScope,
    RateLimited,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_output_serde() {
        let output = AgentOutput::FindingCandidate(FindingCandidate {
            severity: Severity::Warning,
            title: "Tight coupling".into(),
            body: "Components A and B have mutual import cycle".into(),
            confidence: 0.85,
            evidence_ids: vec!["ev:abc123".into()],
            recommended_views: vec!["c4-component".into()],
        });
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains(r#""kind":"FindingCandidate""#));
        let back: AgentOutput = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, AgentOutput::FindingCandidate(_)));
    }

    #[test]
    fn view_kind_serde() {
        let vk = ViewKind::C4Container;
        let json = serde_json::to_string(&vk).unwrap();
        assert!(json.contains("c4-container"));
    }
}
