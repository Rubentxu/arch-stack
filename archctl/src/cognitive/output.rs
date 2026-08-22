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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    // -----------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage, 2026-08-22)
    // -----------------------------------------------------------------------

    #[test]
    fn newtype_ids_serde_round_trip() {
        // ProposalId
        let p = ProposalId("prop-007".into());
        let json = serde_json::to_string(&p).unwrap();
        let back: ProposalId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);

        // EventId
        let e = EventId("evt-007".into());
        let json = serde_json::to_string(&e).unwrap();
        let back: EventId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, e);

        // AgentId
        let a = AgentId("agent-007".into());
        let json = serde_json::to_string(&a).unwrap();
        let back: AgentId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, a);

        // UserId
        let u = UserId("user-007".into());
        let json = serde_json::to_string(&u).unwrap();
        let back: UserId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, u);
    }

    #[test]
    fn approval_level_all_variants_serde() {
        for level in [
            ApprovalLevel::SelfApproval,
            ApprovalLevel::PeerApproval,
            ApprovalLevel::TechLeadApproval,
            ApprovalLevel::SecurityApproval,
            ApprovalLevel::MultiPartyApproval {
                required: 3,
                total: 5,
            },
        ] {
            let json = serde_json::to_string(&level).unwrap();
            let back: ApprovalLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(back, level, "round-trip failed for {:?}", level);
        }

        // MultiPartyApproval struct shape
        let mpa = ApprovalLevel::MultiPartyApproval {
            required: 2,
            total: 3,
        };
        let json = serde_json::to_string(&mpa).unwrap();
        assert!(json.contains("\"required\":2"));
        assert!(json.contains("\"total\":3"));
    }

    #[test]
    fn approval_requirement_default_is_auto() {
        let ap: ApprovalRequirement = Default::default();
        assert_eq!(ap, ApprovalRequirement::Auto);
    }

    #[test]
    fn approval_requirement_forbidden_serde() {
        let ap = ApprovalRequirement::Forbidden;
        let json = serde_json::to_string(&ap).unwrap();
        assert!(json.contains("Forbidden"));
        let back: ApprovalRequirement = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ApprovalRequirement::Forbidden);
    }

    #[test]
    fn approval_requirement_notify_with_users_serde() {
        let ap = ApprovalRequirement::Notify(vec![UserId("alice".into()), UserId("bob".into())]);
        let json = serde_json::to_string(&ap).unwrap();
        assert!(json.contains("Notify"));
        assert!(json.contains("alice"));
        let back: ApprovalRequirement = serde_json::from_str(&json).unwrap();
        match back {
            ApprovalRequirement::Notify(users) => {
                assert_eq!(users.len(), 2);
                assert_eq!(users[0], UserId("alice".into()));
            }
            other => panic!("expected Notify, got {:?}", other),
        }
    }

    #[test]
    fn agent_output_no_action_serde() {
        let out = AgentOutput::NoAction(NoActionReason {
            code: NoActionCode::RateLimited,
            message: "backoff 30s".into(),
        });
        let json = serde_json::to_string(&out).unwrap();
        assert!(json.contains(r#""kind":"NoAction""#));
        let back: AgentOutput = serde_json::from_str(&json).unwrap();
        if let AgentOutput::NoAction(reason) = back {
            assert!(matches!(reason.code, NoActionCode::RateLimited));
            assert_eq!(reason.message, "backoff 30s");
        } else {
            panic!("expected NoAction");
        }
    }

    #[test]
    fn agent_output_hypothesis_serde() {
        let out = AgentOutput::Hypothesis(Hypothesis {
            statement: "Service X likely has memory leak".into(),
            confidence: 0.72,
            evidence_ids: vec!["ev:001".into(), "ev:002".into()],
        });
        let json = serde_json::to_string(&out).unwrap();
        assert!(json.contains(r#""kind":"Hypothesis""#));
        let back: AgentOutput = serde_json::from_str(&json).unwrap();
        if let AgentOutput::Hypothesis(h) = back {
            assert_eq!(h.confidence, 0.72);
            assert_eq!(h.evidence_ids.len(), 2);
        } else {
            panic!("expected Hypothesis");
        }
    }

    #[test]
    fn agent_output_query_plan_serde() {
        let out = AgentOutput::QueryPlan(QueryPlan {
            cypher_steps: vec![
                "MATCH (n:Element) RETURN n LIMIT 100".into(),
                "MATCH (n)-[r:DEPENDS_ON]->(m) RETURN r".into(),
            ],
            estimated_rows: Some(250),
        });
        let json = serde_json::to_string(&out).unwrap();
        assert!(json.contains(r#""kind":"QueryPlan""#));
        let back: AgentOutput = serde_json::from_str(&json).unwrap();
        if let AgentOutput::QueryPlan(qp) = back {
            assert_eq!(qp.cypher_steps.len(), 2);
            assert_eq!(qp.estimated_rows, Some(250));
        } else {
            panic!("expected QueryPlan");
        }
    }

    #[test]
    fn agent_output_projection_spec_serde() {
        let out = AgentOutput::ProjectionSpec(ProjectionSpec {
            view_kind: ViewKind::C4Component,
            format: DiagramFormat::Structurizr,
            focus_elements: vec!["auth-svc".into()],
            layout_hints: LayoutHints {
                direction: Some(LayoutDirection::LeftRight),
                ranksep: Some(1.5),
                nodesep: Some(0.5),
            },
        });
        let json = serde_json::to_string(&out).unwrap();
        assert!(json.contains(r#""kind":"ProjectionSpec""#));
        assert!(json.contains("c4-component"));
        let back: AgentOutput = serde_json::from_str(&json).unwrap();
        if let AgentOutput::ProjectionSpec(ps) = back {
            assert!(matches!(ps.view_kind, ViewKind::C4Component));
            assert!(matches!(ps.format, DiagramFormat::Structurizr));
            assert_eq!(ps.layout_hints.ranksep, Some(1.5));
        } else {
            panic!("expected ProjectionSpec");
        }
    }

    #[test]
    fn agent_output_action_plan_serde() {
        let out = AgentOutput::ActionPlan(ActionPlan {
            steps: vec![Step {
                command: "cargo fmt".into(),
                args: vec!["--check".into()],
                reason: "verify formatting".into(),
            }],
            rollback: Some(vec![Step {
                command: "git".into(),
                args: vec!["checkout".into(), "--".into(), ".".into()],
                reason: "revert any formatting changes".into(),
            }]),
        });
        let json = serde_json::to_string(&out).unwrap();
        assert!(json.contains(r#""kind":"ActionPlan""#));
        let back: AgentOutput = serde_json::from_str(&json).unwrap();
        if let AgentOutput::ActionPlan(plan) = back {
            assert_eq!(plan.steps.len(), 1);
            assert!(plan.rollback.is_some());
            assert_eq!(plan.rollback.as_ref().unwrap().len(), 1);
        } else {
            panic!("expected ActionPlan");
        }
    }

    #[test]
    fn agent_output_documentation_patch_serde() {
        let out = AgentOutput::DocumentationPatch(DocumentationPatch {
            file: "docs/README.md".into(),
            patch_type: PatchType::Replace,
            body: "## Section\nnew content".into(),
        });
        let json = serde_json::to_string(&out).unwrap();
        assert!(json.contains(r#""kind":"DocumentationPatch""#));
        let back: AgentOutput = serde_json::from_str(&json).unwrap();
        if let AgentOutput::DocumentationPatch(dp) = back {
            assert_eq!(dp.file, "docs/README.md");
            assert!(matches!(dp.patch_type, PatchType::Replace));
        } else {
            panic!("expected DocumentationPatch");
        }
    }

    #[test]
    fn agent_output_context_request_serde() {
        let out = AgentOutput::ContextRequest(ContextRequest {
            request_id: "ctx-req-001".into(),
            missing: vec!["source:src/auth.rs".into()],
            reasoning: "Need to inspect auth module".into(),
        });
        let json = serde_json::to_string(&out).unwrap();
        assert!(json.contains(r#""kind":"ContextRequest""#));
        let back: AgentOutput = serde_json::from_str(&json).unwrap();
        if let AgentOutput::ContextRequest(cr) = back {
            assert_eq!(cr.request_id, "ctx-req-001");
            assert_eq!(cr.missing.len(), 1);
        } else {
            panic!("expected ContextRequest");
        }
    }

    #[test]
    fn severity_all_variants_serde() {
        for sev in [
            Severity::Info,
            Severity::Warning,
            Severity::Error,
            Severity::Critical,
        ] {
            let json = serde_json::to_string(&sev).unwrap();
            let back: Severity = serde_json::from_str(&json).unwrap();
            assert_eq!(back, sev, "round-trip failed for {:?}", sev);
        }
    }

    #[test]
    fn view_kind_all_variants_serde() {
        // Existing test only covers C4Container; cover the rest
        for vk in [
            ViewKind::C4Context,
            ViewKind::C4Container,
            ViewKind::C4Component,
            ViewKind::Class,
            ViewKind::Sequence,
            ViewKind::State,
            ViewKind::UseCase,
        ] {
            let json = serde_json::to_string(&vk).unwrap();
            let back: ViewKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, vk, "round-trip failed for {:?}", vk);
        }
        // Verify the C4-family renames are stable
        assert!(
            serde_json::to_string(&ViewKind::C4Context)
                .unwrap()
                .contains("c4-context")
        );
        assert!(
            serde_json::to_string(&ViewKind::C4Container)
                .unwrap()
                .contains("c4-container")
        );
        assert!(
            serde_json::to_string(&ViewKind::C4Component)
                .unwrap()
                .contains("c4-component")
        );
    }

    #[test]
    fn diagram_format_all_variants_serde() {
        for df in [
            DiagramFormat::PlantUML,
            DiagramFormat::Mermaid,
            DiagramFormat::Structurizr,
        ] {
            let json = serde_json::to_string(&df).unwrap();
            let back: DiagramFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(back, df, "round-trip failed for {:?}", df);
        }
    }

    #[test]
    fn layout_direction_serde() {
        for d in [LayoutDirection::TopDown, LayoutDirection::LeftRight] {
            let json = serde_json::to_string(&d).unwrap();
            let back: LayoutDirection = serde_json::from_str(&json).unwrap();
            assert_eq!(back, d);
        }
    }

    #[test]
    fn layout_hints_all_none_serde() {
        let lh = LayoutHints {
            direction: None,
            ranksep: None,
            nodesep: None,
        };
        let json = serde_json::to_string(&lh).unwrap();
        let back: LayoutHints = serde_json::from_str(&json).unwrap();
        assert!(back.direction.is_none());
        assert!(back.ranksep.is_none());
        assert!(back.nodesep.is_none());
    }

    #[test]
    fn no_action_code_all_variants_serde() {
        for code in [
            NoActionCode::InsufficientConfidence,
            NoActionCode::NoRelevantData,
            NoActionCode::OutOfScope,
            NoActionCode::RateLimited,
        ] {
            let json = serde_json::to_string(&code).unwrap();
            let back: NoActionCode = serde_json::from_str(&json).unwrap();
            assert_eq!(back, code, "round-trip failed for {:?}", code);
        }
    }

    #[test]
    fn patch_type_all_variants_serde() {
        for pt in [PatchType::Add, PatchType::Replace, PatchType::Remove] {
            let json = serde_json::to_string(&pt).unwrap();
            let back: PatchType = serde_json::from_str(&json).unwrap();
            assert_eq!(back, pt, "round-trip failed for {:?}", pt);
        }
    }

    #[test]
    fn outcome_predicate_serde_with_all_fields() {
        let op = OutcomePredicate {
            event_type: "coupling_score > 0.8".into(),
            agent_id: Some(AgentId("coupling-detector".into())),
            threshold: Some(0.8),
        };
        let json = serde_json::to_string(&op).unwrap();
        assert!(json.contains("coupling_score > 0.8"));
        assert!(json.contains("coupling-detector"));
        let back: OutcomePredicate = serde_json::from_str(&json).unwrap();
        assert_eq!(back, op);
    }

    #[test]
    fn rollback_strategy_serde_minimal() {
        // undo_command=None, undo_args empty — defaults contract at output.rs:128-132
        let rs = RollbackStrategy {
            description: "manual revert".into(),
            undo_command: None,
            undo_args: vec![],
        };
        let json = serde_json::to_string(&rs).unwrap();
        let back: RollbackStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(back.description, "manual revert");
        assert!(back.undo_command.is_none());
        assert!(back.undo_args.is_empty());
    }

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage v5, 2026-08-22)
    // ---------------------------------------------------------------------------

    /// `Step` round-trips through serde with all fields populated.
    #[test]
    fn step_serde_with_args_and_reason() {
        let step = Step {
            command: "cargo".into(),
            args: vec!["test".into(), "--lib".into()],
            reason: "verify unit tests pass".into(),
        };
        let json = serde_json::to_string(&step).unwrap();
        let back: Step = serde_json::from_str(&json).unwrap();
        assert_eq!(back.command, "cargo");
        assert_eq!(back.args, vec!["test", "--lib"]);
        assert_eq!(back.reason, "verify unit tests pass");
    }

    /// `ActionPlan` with `rollback: None` round-trips. Distinct from
    /// `agent_output_action_plan_serde` which exercises rollback Some.
    #[test]
    fn action_plan_serde_with_rollback_none() {
        let plan = ActionPlan {
            steps: vec![Step {
                command: "ls".into(),
                args: vec!["-la".into()],
                reason: "list directory".into(),
            }],
            rollback: None,
        };
        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains(r#""rollback":null"#), "got: {json}");
        let back: ActionPlan = serde_json::from_str(&json).unwrap();
        assert!(back.rollback.is_none());
        assert_eq!(back.steps.len(), 1);
    }

    /// `CostEstimate` with all 4 fields populated round-trips. Distinct
    /// from `cost_estimate_default` which checks the empty case.
    #[test]
    fn cost_estimate_serde_with_populated_values() {
        let ce = CostEstimate {
            tokens: Some(8192),
            time_ms: Some(2000),
            cost_cents: Some(50),
            side_effects: vec!["network".into(), "log".into()],
        };
        let json = serde_json::to_string(&ce).unwrap();
        let back: CostEstimate = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tokens, Some(8192));
        assert_eq!(back.time_ms, Some(2000));
        assert_eq!(back.cost_cents, Some(50));
        assert_eq!(back.side_effects, vec!["network", "log"]);
    }

    /// `Hypothesis` full roundtrip (statement, confidence, evidence_ids).
    /// Distinct from `agent_output_hypothesis_serde` which only checks
    /// the AgentOutput::Hypothesis variant and 2 fields.
    #[test]
    fn hypothesis_full_serde_roundtrip() {
        let h = Hypothesis {
            statement: "Hypothesis statement".into(),
            confidence: 0.92,
            evidence_ids: vec!["ev-001".into(), "ev-002".into(), "ev-003".into()],
        };
        let json = serde_json::to_string(&h).unwrap();
        let back: Hypothesis = serde_json::from_str(&json).unwrap();
        assert_eq!(back.statement, "Hypothesis statement");
        assert_eq!(back.confidence, 0.92);
        assert_eq!(back.evidence_ids.len(), 3);
    }

    /// `FindingCandidate` full roundtrip with all 6 fields. Distinct
    /// from `agent_output_serde` which only checks the AgentOutput
    /// variant tag and one field.
    #[test]
    fn finding_candidate_full_serde_roundtrip() {
        let fc = FindingCandidate {
            severity: Severity::Critical,
            title: "Critical coupling".into(),
            body: "Components A and B have mutual dependency".into(),
            confidence: 0.95,
            evidence_ids: vec!["ev-100".into()],
            recommended_views: vec!["c4-component".into(), "c4-container".into()],
        };
        let json = serde_json::to_string(&fc).unwrap();
        let back: FindingCandidate = serde_json::from_str(&json).unwrap();
        assert_eq!(back.severity, Severity::Critical);
        assert_eq!(back.title, "Critical coupling");
        assert_eq!(back.confidence, 0.95);
        assert_eq!(back.recommended_views.len(), 2);
    }

    /// `NoActionReason` round-trips through serde with both fields.
    #[test]
    fn no_action_reason_serde_roundtrip() {
        let nar = NoActionReason {
            code: NoActionCode::NoRelevantData,
            message: "graph query returned no rows".into(),
        };
        let json = serde_json::to_string(&nar).unwrap();
        let back: NoActionReason = serde_json::from_str(&json).unwrap();
        assert_eq!(back.code, NoActionCode::NoRelevantData);
        assert_eq!(back.message, "graph query returned no rows");
    }

    /// `OutcomePredicate` with `agent_id: None` and `threshold: None`
    /// round-trips. Distinct from `outcome_predicate_serde_with_all_fields`
    /// which uses Some for both. Locks the `#[serde(default)]` contract
    /// on both Option fields.
    #[test]
    fn outcome_predicate_serde_with_none_fields() {
        let op = OutcomePredicate {
            event_type: "raw".into(),
            agent_id: None,
            threshold: None,
        };
        let json = serde_json::to_string(&op).unwrap();
        // Option fields serialize as `null` (not omitted — no skip_serializing_if)
        assert!(
            json.contains(r#""agent_id":null"#),
            "agent_id None must serialize as null, got: {json}"
        );
        assert!(
            json.contains(r#""threshold":null"#),
            "threshold None must serialize as null, got: {json}"
        );
        let back: OutcomePredicate = serde_json::from_str(&json).unwrap();
        assert!(back.agent_id.is_none());
        assert!(back.threshold.is_none());
    }

    /// `ActionProposal` Debug includes goal and command for tracing.
    /// Locks the `#[derive(Debug)]` contract.
    #[test]
    fn action_proposal_debug_includes_goal_and_command() {
        let ap = ActionProposal {
            id: Some(ProposalId("prop-007".into())),
            cause: None,
            triggering_agent: None,
            goal: "deploy prod".into(),
            command: "deploy".into(),
            args: vec![],
            capabilities: vec![],
            approval: ApprovalRequirement::Auto,
            expected_evidence: vec![],
            rollback: None,
            cost_estimate: CostEstimate::default(),
            confidence: Some(0.85),
            ttl_ms: None,
            security_impact: None,
            deployment_env: None,
            policy_rule_matched: None,
            approval_required: false,
            expected_evidence_old: String::new(),
        };
        let dbg = format!("{ap:?}");
        assert!(dbg.contains("ActionProposal"), "got: {dbg}");
        assert!(dbg.contains("deploy prod"), "got: {dbg}");
        assert!(dbg.contains("deploy"), "got: {dbg}");
    }
}
