//! Policy decision types.

use serde::{Deserialize, Serialize};

use super::{ApprovalLevel, ApprovalRequirement};

/// Result of evaluating a proposal against the policy engine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::output::UserId;

    // ─── PolicyDecision::is_allow (M61) ────────────────────────────────
    //
    // `is_allow` is the gate that fast-paths proposals through the
    // escalation chain. It must be true ONLY for `Allow`, not for
    // `AllowWithNotify` (which still requires a notification step).

    #[test]
    fn is_allow_true_only_for_allow_variant() {
        assert!(PolicyDecision::Allow.is_allow());
        assert!(!PolicyDecision::AllowWithNotify(vec!["alice".into()]).is_allow());
        assert!(
            !PolicyDecision::RequireApproval {
                level: ApprovalLevel::PeerApproval,
                reason: "low confidence".into(),
            }
            .is_allow()
        );
        assert!(
            !PolicyDecision::Deny {
                reason: "policy".into()
            }
            .is_allow()
        );
        assert!(
            !PolicyDecision::Escalate {
                target: "tech-lead".into(),
            }
            .is_allow()
        );
    }

    // ─── PolicyDecision::to_approval_requirement (M61) ────────────────
    //
    // The decision-to-requirement mapping is the contract between the
    // policy engine and the approval subsystem. Each variant must map
    // to exactly one ApprovalRequirement, and the mapping must be a
    // total function (no fall-through).

    #[test]
    fn allow_maps_to_auto() {
        assert_eq!(
            PolicyDecision::Allow.to_approval_requirement(),
            ApprovalRequirement::Auto
        );
    }

    #[test]
    fn allow_with_notify_maps_to_notify_with_user_ids() {
        let decision = PolicyDecision::AllowWithNotify(vec!["alice".into(), "bob".into()]);
        let req = decision.to_approval_requirement();
        match req {
            ApprovalRequirement::Notify(users) => {
                assert_eq!(users, vec![UserId("alice".into()), UserId("bob".into())]);
            }
            other => panic!("expected Notify, got {other:?}"),
        }
    }

    #[test]
    fn allow_with_notify_empty_users_still_maps_to_notify() {
        // No users is unusual but should not panic; the downstream
        // notification dispatcher will decide what to do.
        let decision = PolicyDecision::AllowWithNotify(vec![]);
        match decision.to_approval_requirement() {
            ApprovalRequirement::Notify(users) => assert!(users.is_empty()),
            other => panic!("expected Notify, got {other:?}"),
        }
    }

    #[test]
    fn require_approval_maps_to_review_at_same_level() {
        let decision = PolicyDecision::RequireApproval {
            level: ApprovalLevel::SecurityApproval,
            reason: "production deploy".into(),
        };
        assert_eq!(
            decision.to_approval_requirement(),
            ApprovalRequirement::Review(ApprovalLevel::SecurityApproval)
        );
    }

    #[test]
    fn deny_maps_to_forbidden() {
        let decision = PolicyDecision::Deny {
            reason: "security policy".into(),
        };
        assert_eq!(
            decision.to_approval_requirement(),
            ApprovalRequirement::Forbidden
        );
    }

    #[test]
    fn escalate_maps_to_tech_lead_review() {
        let decision = PolicyDecision::Escalate {
            target: "human-ethics-board".into(),
        };
        assert_eq!(
            decision.to_approval_requirement(),
            ApprovalRequirement::Review(ApprovalLevel::TechLeadApproval)
        );
    }

    // ─── PolicyResult (M61) ────────────────────────────────────────────

    #[test]
    fn policy_result_carries_decision_and_matched_rule() {
        let result = PolicyResult {
            decision: PolicyDecision::Deny {
                reason: "blocked".into(),
            },
            matched_rule: Some("no-prod-deploys-from-juniors".into()),
        };
        assert!(!result.decision.is_allow());
        assert_eq!(
            result.matched_rule.as_deref(),
            Some("no-prod-deploys-from-juniors")
        );
    }

    #[test]
    fn policy_result_serialization_round_trip() {
        let result = PolicyResult {
            decision: PolicyDecision::Escalate {
                target: "policy-admin".into(),
            },
            matched_rule: None,
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let parsed: PolicyResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, result);
    }
}
