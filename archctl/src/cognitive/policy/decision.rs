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

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage v5, 2026-08-22)
    // ---------------------------------------------------------------------------

    /// `PolicyDecision` derives `PartialEq`. Two variants of different kinds
    /// are NOT equal — locks the derive contract for `is_allow` shortcuts.
    #[test]
    fn policy_decision_partial_eq_across_variants() {
        let allow = PolicyDecision::Allow;
        let deny = PolicyDecision::Deny { reason: "x".into() };
        let allow2 = PolicyDecision::Allow;
        assert_eq!(allow, allow2);
        assert_ne!(allow, deny);
        assert_ne!(
            PolicyDecision::Allow,
            PolicyDecision::AllowWithNotify(vec!["a".into()])
        );
    }

    /// `PolicyDecision` Clone preserves all fields. Locks the clone contract
    /// across all 5 variants.
    #[test]
    fn policy_decision_clone_preserves_all_fields() {
        let original = PolicyDecision::RequireApproval {
            level: ApprovalLevel::MultiPartyApproval {
                required: 2,
                total: 3,
            },
            reason: "low confidence".into(),
        };
        let cloned = original.clone();
        assert_eq!(original, cloned);

        let escalate = PolicyDecision::Escalate {
            target: "human-board".into(),
        };
        let cloned_esc = escalate.clone();
        assert_eq!(escalate, cloned_esc);
    }

    /// `PolicyDecision::Debug` includes the variant name for tracing. Locks
    /// the `#[derive(Debug)]` contract.
    #[test]
    fn policy_decision_debug_includes_variant() {
        let allow = PolicyDecision::Allow;
        let dbg = format!("{allow:?}");
        assert!(dbg.contains("Allow"), "got: {dbg}");

        let deny = PolicyDecision::Deny {
            reason: "policy".into(),
        };
        let dbg = format!("{deny:?}");
        assert!(dbg.contains("Deny"), "got: {dbg}");
        assert!(dbg.contains("policy"), "got: {dbg}");

        let multiparty = PolicyDecision::RequireApproval {
            level: ApprovalLevel::MultiPartyApproval {
                required: 2,
                total: 3,
            },
            reason: "low conf".into(),
        };
        let dbg = format!("{multiparty:?}");
        assert!(dbg.contains("RequireApproval"), "got: {dbg}");
        assert!(dbg.contains("MultiPartyApproval"), "got: {dbg}");
    }

    /// All 5 `PolicyDecision` variants serialize + deserialize cleanly.
    /// Locks the wire format for each variant.
    #[test]
    fn policy_decision_all_variants_serde() {
        let decisions = vec![
            PolicyDecision::Allow,
            PolicyDecision::AllowWithNotify(vec!["alice".into(), "bob".into()]),
            PolicyDecision::RequireApproval {
                level: ApprovalLevel::TechLeadApproval,
                reason: "deploy to production".into(),
            },
            PolicyDecision::Deny {
                reason: "policy violation".into(),
            },
            PolicyDecision::Escalate {
                target: "human-ethics-board".into(),
            },
        ];
        for d in &decisions {
            let json = serde_json::to_string(d).expect("serialize");
            let back: PolicyDecision = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(&back, d, "round-trip mismatch for {:?}", d);
        }
    }

    /// `to_approval_requirement()` for `Escalate` always returns
    /// `Review(TechLeadApproval)` regardless of the `target` field. Locks
    /// the deliberate simplification (escalation maps to a fixed level,
    /// not a parsed-from-target level).
    #[test]
    fn escalate_to_approval_requirement_ignores_target() {
        for target in ["alice", "bob", "human-board", "policy-admin", ""] {
            let decision = PolicyDecision::Escalate {
                target: target.into(),
            };
            let req = decision.to_approval_requirement();
            assert_eq!(
                req,
                ApprovalRequirement::Review(ApprovalLevel::TechLeadApproval),
                "target '{target}' must map to TechLeadApproval"
            );
        }
    }

    /// `PolicyResult` with `matched_rule: None` roundtrips. Distinct from
    /// `policy_result_serialization_round_trip` which uses `Escalate` +
    /// None, this uses `Allow` + None.
    #[test]
    fn policy_result_with_allow_and_no_matched_rule() {
        let result = PolicyResult {
            decision: PolicyDecision::Allow,
            matched_rule: None,
        };
        let json = serde_json::to_string(&result).expect("serialize");
        assert!(json.contains(r#""matched_rule":null"#), "got: {json}");
        let back: PolicyResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.decision, PolicyDecision::Allow);
        assert!(back.matched_rule.is_none());
    }

    /// `PolicyResult` Debug includes both the decision and the rule name.
    #[test]
    fn policy_result_debug_includes_fields() {
        let result = PolicyResult {
            decision: PolicyDecision::Deny {
                reason: "blocked".into(),
            },
            matched_rule: Some("no-prod-deploys".into()),
        };
        let dbg = format!("{result:?}");
        assert!(dbg.contains("PolicyResult"), "got: {dbg}");
        assert!(dbg.contains("Deny"), "got: {dbg}");
        assert!(dbg.contains("no-prod-deploys"), "got: {dbg}");
    }

    /// `to_approval_requirement()` for `AllowWithNotify` preserves user
    /// order (not sorted).
    #[test]
    fn allow_with_notify_preserves_user_order() {
        let decision =
            PolicyDecision::AllowWithNotify(vec!["zara".into(), "alice".into(), "bob".into()]);
        match decision.to_approval_requirement() {
            ApprovalRequirement::Notify(users) => {
                let names: Vec<&str> = users.iter().map(|u| u.0.as_str()).collect();
                assert_eq!(names, vec!["zara", "alice", "bob"]);
            }
            other => panic!("expected Notify, got {other:?}"),
        }
    }
}
