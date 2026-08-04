//! Agent output types.

use serde::{Deserialize, Deserializer, Serialize};

/// Deserialize either the old string format or the new Vec<OutcomePredicate> format.
/// Handles both: `"expected_evidence": "old string"` and `"expected_evidence": [{...}].
fn deserialize_expected_evidence<'de, D>(deserializer: D) -> Result<Vec<OutcomePredicate>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Ed {
        Old(String),
        New(Vec<OutcomePredicate>),
    }
    match Ed::deserialize(deserializer) {
        Ok(Ed::Old(s)) => Ok(vec![OutcomePredicate {
            event_type: s,
            agent_id: None,
            threshold: None,
        }]),
        Ok(Ed::New(v)) => Ok(v),
        Err(e) => Err(e),
    }
}

// ---------------------------------------------------------------------------
// Newtype identifiers
// ---------------------------------------------------------------------------

/// Proposal identifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub struct ProposalId(pub String);

/// Event identifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub struct EventId(pub String);

/// Agent identifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub struct AgentId(pub String);

/// User identifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub struct UserId(pub String);

// ---------------------------------------------------------------------------
// Enums for Policy Engine
// ---------------------------------------------------------------------------

/// Deployment environment.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum DeploymentEnv {
    #[default]
    Development,
    Staging,
    Production,
}

/// Security impact level.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SecurityImpact {
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

/// Approval level required for a proposal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalLevel {
    SelfApproval,
    PeerApproval,
    TechLeadApproval,
    SecurityApproval,
    MultiPartyApproval { required: u32, total: u32 },
}

/// Who must approve an ActionProposal before execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ApprovalRequirement {
    /// Executable without human intervention.
    #[default]
    Auto,
    /// Notify these users but do not block.
    Notify(Vec<UserId>),
    /// Block until a reviewer at the required level approves.
    Review(ApprovalLevel),
    /// Blocked — never executable without a policy override.
    Forbidden,
}

/// Cost estimate for an ActionProposal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CostEstimate {
    /// Estimated token consumption.
    pub tokens: Option<u32>,
    /// Estimated wall-clock time in milliseconds.
    pub time_ms: Option<u32>,
    /// Estimated cost in US cents.
    pub cost_cents: Option<u32>,
    /// Description of side effects.
    #[serde(default)]
    pub side_effects: Vec<String>,
}

/// A predicate describing the expected runtime outcome of a proposal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutcomePredicate {
    /// Human-readable description of the expected event type.
    pub event_type: String,
    /// Agent that should emit the evidence.
    #[serde(default)]
    pub agent_id: Option<AgentId>,
    /// Numeric threshold.
    #[serde(default)]
    pub threshold: Option<f64>,
}

/// Rollback strategy for an ActionProposal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RollbackStrategy {
    /// Human-readable description of how to undo this action.
    pub description: String,
    /// Optional command to reverse the action.
    #[serde(default)]
    pub undo_command: Option<String>,
    /// Args for the undo command.
    #[serde(default)]
    pub undo_args: Vec<String>,
}

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
    ActionProposal(Box<ActionProposal>),
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

/// A structured proposal emitted by an agent for a governed action.
///
/// v1.0 expands the original stub to include identifiers, policy-aware
/// approval, cost estimation, and TTL — backward-compatible via
/// `#[serde(default)]` on all new fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct ActionProposal {
    /// Unique identifier for this proposal.
    #[serde(default)]
    pub id: Option<ProposalId>,
    /// Event that triggered this proposal.
    #[serde(default)]
    pub cause: Option<EventId>,
    /// Agent that emitted this proposal.
    #[serde(default)]
    pub triggering_agent: Option<AgentId>,
    /// What the proposal wants to do.
    pub goal: String,
    /// Command to execute.
    pub command: String,
    /// Arguments for the command.
    #[serde(default)]
    pub args: Vec<String>,
    /// Capabilities required to execute this proposal.
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Who must approve before execution.
    #[serde(default)]
    pub approval: ApprovalRequirement,
    /// Expected evidence predicates that confirm success.
    #[serde(deserialize_with = "deserialize_expected_evidence", default)]
    pub expected_evidence: Vec<OutcomePredicate>,
    /// How to undo this action if it fails.
    #[serde(default)]
    pub rollback: Option<RollbackStrategy>,
    /// Estimated resource cost.
    #[serde(default)]
    pub cost_estimate: CostEstimate,
    /// Confidence score 0.0–1.0.
    #[serde(default)]
    pub confidence: Option<f32>,
    /// Time-to-live in milliseconds; proposal expires if not approved within this window.
    #[serde(default)]
    pub ttl_ms: Option<u64>,
    /// Security impact of executing this proposal.
    #[serde(default)]
    pub security_impact: Option<SecurityImpact>,
    /// Which deployment environment this targets.
    #[serde(default)]
    pub deployment_env: Option<DeploymentEnv>,
    /// Name of the policy rule that matched (for audit).
    #[serde(default)]
    pub policy_rule_matched: Option<String>,

    // -------------------------------------------------------------------------
    // Backward-compatibility shim — old fields kept for deserialization.
    // Old JSON: { "goal": "...", "command": "...", "approval_required": true/false,
    //             "expected_evidence": "...", "capabilities": [...],
    //             "args": [...], "rollback": [...] }
    // These fields are ignored on serialize; their new counterparts are used instead.
    // -------------------------------------------------------------------------
    /// **Deprecated** — use `approval` instead.
    #[serde(skip_serializing, default)]
    pub approval_required: bool,

    /// **Deprecated** — use `expected_evidence` instead.
    #[serde(skip_serializing, default)]
    pub expected_evidence_old: String,
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

    #[test]
    fn action_proposal_backward_compat_old_json() {
        // Old JSON format from before the expansion.
        let old_json = r#"{
            "kind": "ActionProposal",
            "goal": "run tests",
            "command": "cargo test",
            "args": ["--lib"],
            "capabilities": ["run_tests"],
            "approval_required": true,
            "expected_evidence": "all tests pass"
        }"#;
        let output: AgentOutput = serde_json::from_str(old_json).unwrap();
        let AgentOutput::ActionProposal(p) = &output else {
            panic!("expected ActionProposal");
        };
        // New structured fields use defaults (old approval_required doesn't auto-fill approval)
        assert!(p.id.is_none());
        assert!(p.cause.is_none());
        // Old JSON has no structured approval field → defaults to Auto
        assert_eq!(p.approval, ApprovalRequirement::Auto);
        assert_eq!(p.expected_evidence.len(), 1);
        assert_eq!(p.expected_evidence[0].event_type, "all tests pass");
        // Old fields are accessible via shim
        assert!(p.approval_required);
        // expected_evidence_old is a separate field (old JSON uses "expected_evidence")
        assert_eq!(p.expected_evidence_old, "");
        // But the custom deserializer correctly converts old string format
        assert_eq!(p.expected_evidence.len(), 1);
        assert_eq!(p.expected_evidence[0].event_type, "all tests pass");
        // New fields serialize correctly (old fields skipped)
        let re_ser = serde_json::to_string(&output).unwrap();
        assert!(!re_ser.contains("approval_required"));
        assert!(!re_ser.contains("expected_evidence_old"));
        assert!(re_ser.contains("\"approval\":\"Auto\""));
    }

    #[test]
    fn action_proposal_new_format() {
        // New JSON format with structured fields.
        let new_json = r#"{
            "kind": "ActionProposal",
            "id": "prop-001",
            "goal": "merge symbols",
            "command": "merge",
            "approval": "Auto",
            "expected_evidence": [
                {"event_type": "alias_count >= 3", "threshold": 3.0}
            ],
            "confidence": 0.85,
            "deployment_env": "Development",
            "security_impact": "Low"
        }"#;
        let output: AgentOutput = serde_json::from_str(new_json).unwrap();
        let AgentOutput::ActionProposal(p) = &output else {
            panic!("expected ActionProposal");
        };
        assert!(p.id.is_some());
        assert_eq!(p.approval, ApprovalRequirement::Auto);
        assert_eq!(p.expected_evidence.len(), 1);
        assert_eq!(p.expected_evidence[0].event_type, "alias_count >= 3");
        assert_eq!(p.deployment_env, Some(DeploymentEnv::Development));
        assert_eq!(p.security_impact, Some(SecurityImpact::Low));
    }

    #[test]
    fn deployment_env_serde() {
        let env = DeploymentEnv::Production;
        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains("Production"));
        let back: DeploymentEnv = serde_json::from_str(&json).unwrap();
        assert_eq!(back, DeploymentEnv::Production);
    }

    #[test]
    fn security_impact_serde() {
        let impact = SecurityImpact::Critical;
        let json = serde_json::to_string(&impact).unwrap();
        assert!(json.contains("Critical"));
        let back: SecurityImpact = serde_json::from_str(&json).unwrap();
        assert_eq!(back, SecurityImpact::Critical);
    }

    #[test]
    fn approval_requirement_serde() {
        let ap = ApprovalRequirement::Review(ApprovalLevel::TechLeadApproval);
        let json = serde_json::to_string(&ap).unwrap();
        assert!(json.contains("Review"));
        assert!(json.contains("TechLeadApproval"));
        let back: ApprovalRequirement = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ap);
    }

    #[test]
    fn cost_estimate_default() {
        let ce = CostEstimate::default();
        assert!(ce.tokens.is_none());
        assert!(ce.time_ms.is_none());
        assert!(ce.cost_cents.is_none());
        assert!(ce.side_effects.is_empty());
    }

    #[test]
    fn action_proposal_full_new_format() {
        let ap = ActionProposal {
            id: Some(ProposalId("prop-001".into())),
            cause: Some(EventId("evt-001".into())),
            triggering_agent: Some(AgentId("agent-001".into())),
            goal: "deploy".into(),
            command: "deploy_production".into(),
            args: vec![],
            capabilities: vec!["deploy".into()],
            approval: ApprovalRequirement::Review(ApprovalLevel::MultiPartyApproval {
                required: 2,
                total: 3,
            }),
            expected_evidence: vec![OutcomePredicate {
                event_type: "ci_green".into(),
                agent_id: None,
                threshold: Some(1.0),
            }],
            rollback: Some(RollbackStrategy {
                description: "revert deploy".into(),
                undo_command: Some("rollback".into()),
                undo_args: vec!["--confirm".into()],
            }),
            cost_estimate: CostEstimate {
                tokens: Some(5000),
                time_ms: Some(1000),
                cost_cents: Some(10),
                side_effects: vec!["network".into()],
            },
            confidence: Some(0.9),
            ttl_ms: Some(3600000),
            security_impact: Some(SecurityImpact::High),
            deployment_env: Some(DeploymentEnv::Production),
            policy_rule_matched: Some("deploy_prod".into()),
            approval_required: false,
            expected_evidence_old: String::new(),
        };
        let json = serde_json::to_string(&ap).unwrap();
        assert!(json.contains("prop-001"));
        assert!(json.contains("deploy_production"));
        assert!(json.contains("\"approval\":{\"Review\":"));
        // old fields not serialized
        assert!(!json.contains("approval_required"));
        assert!(!json.contains("expected_evidence_old"));
    }
}
