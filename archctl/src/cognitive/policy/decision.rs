//! Policy decision types.

use serde::{Deserialize, Serialize};

use super::{ApprovalLevel, ApprovalRequirement};

/// Result of evaluating a proposal against the policy engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyResult {
    /// The policy decision.
    pub decision: PolicyDecision,
    /// Name of the rule that matched (for audit).
    pub matched_rule: Option<String>,
}

/// The policy engine's decision on a proposal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyDecision {
    /// Execute immediately — no approval needed.
    Allow,
    /// Execute and notify these users.
    AllowWithNotify(Vec<String>),
    /// Block until approved at the required level.
    RequireApproval {
        level: ApprovalLevel,
        reason: String,
    },
    /// Blocked — do not execute.
    Deny { reason: String },
    /// Pass to a higher authority.
    Escalate { target: String },
}

impl PolicyDecision {
    /// Returns true if the proposal can proceed without further approval.
    pub fn is_allow(&self) -> bool {
        matches!(self, PolicyDecision::Allow)
    }

    /// Returns the ApprovalRequirement equivalent for this decision.
    pub fn to_approval_requirement(&self) -> ApprovalRequirement {
        match self {
            PolicyDecision::Allow => ApprovalRequirement::Auto,
            PolicyDecision::AllowWithNotify(users) => ApprovalRequirement::Notify(
                users
                    .iter()
                    .map(|s| crate::cognitive::output::UserId(s.clone()))
                    .collect(),
            ),
            PolicyDecision::RequireApproval { level, .. } => ApprovalRequirement::Review(*level),
            PolicyDecision::Deny { .. } => ApprovalRequirement::Forbidden,
            PolicyDecision::Escalate { .. } => {
                ApprovalRequirement::Review(ApprovalLevel::TechLeadApproval)
            }
        }
    }
}
