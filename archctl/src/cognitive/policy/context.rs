//! Policy context — information available when evaluating a proposal.

use serde::{Deserialize, Serialize};

use super::{DeploymentEnv, SecurityImpact};

/// Policy evaluation context — everything the PolicyEngine needs to decide.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyContext {
    /// Who is requesting this action.
    pub user_id: String,
    /// Target deployment environment.
    pub environment: DeploymentEnv,
    /// Security impact level of the proposed action.
    pub security_impact: SecurityImpact,
    /// Capabilities being requested.
    pub requesting_capabilities: Vec<String>,
    /// Components that would be affected by this action.
    #[serde(default)]
    pub affected_components: Vec<String>,
    /// Maximum cost authorised by the user.
    #[serde(default)]
    pub cost_ceiling: CostCeiling,
}

/// Cost ceiling authorised by the user for a proposal.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CostCeiling {
    /// Maximum tokens authorised.
    pub tokens: Option<u32>,
    /// Maximum time in milliseconds.
    pub time_ms: Option<u32>,
    /// Maximum cost in US cents.
    pub cost_cents: Option<u32>,
}

impl CostCeiling {
    /// Check if the given cost estimate is within this ceiling.
    pub fn allows(&self, tokens: Option<u32>, cost_cents: Option<u32>) -> bool {
        if let (Some(limit), Some(used)) = (self.tokens, tokens)
            && used > limit
        {
            return false;
        }
        if let (Some(limit), Some(used)) = (self.cost_cents, cost_cents)
            && used > limit
        {
            return false;
        }
        true
    }
}

impl Default for PolicyContext {
    fn default() -> Self {
        Self {
            user_id: "anonymous".into(),
            environment: DeploymentEnv::Development,
            security_impact: SecurityImpact::Low,
            requesting_capabilities: vec![],
            affected_components: vec![],
            cost_ceiling: CostCeiling::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── CostCeiling::allows (M61) ─────────────────────────────────────
    //
    // The cost ceiling is the single user-controllable cost gate in the
    // policy engine. Bugs here silently allow runaway proposals or block
    // legitimate ones. Coverage of the three branches (under, at, over
    // each limit) plus the unspecified-limit case.

    #[test]
    fn cost_ceiling_allows_when_both_limits_unset() {
        // No caps set ⇒ everything is fine.
        let ceiling = CostCeiling::default();
        assert!(ceiling.allows(Some(1_000_000), Some(1_000_000)));
        assert!(ceiling.allows(None, None));
    }

    #[test]
    fn cost_ceiling_allows_when_under_token_limit() {
        let ceiling = CostCeiling {
            tokens: Some(1000),
            time_ms: None,
            cost_cents: None,
        };
        assert!(ceiling.allows(Some(500), None));
    }

    #[test]
    fn cost_ceiling_rejects_over_token_limit() {
        let ceiling = CostCeiling {
            tokens: Some(1000),
            time_ms: None,
            cost_cents: None,
        };
        assert!(!ceiling.allows(Some(1001), None));
    }

    #[test]
    fn cost_ceiling_allows_at_exact_token_limit() {
        // Boundary: equal is allowed (the contract is `>`, not `>=`).
        let ceiling = CostCeiling {
            tokens: Some(1000),
            time_ms: None,
            cost_cents: None,
        };
        assert!(ceiling.allows(Some(1000), None));
    }

    #[test]
    fn cost_ceiling_allows_when_caller_omits_used_tokens() {
        // The caller is not yet supplying a token estimate; the ceiling
        // does not gate on `None` for a `Some` limit.
        let ceiling = CostCeiling {
            tokens: Some(1000),
            time_ms: None,
            cost_cents: None,
        };
        assert!(ceiling.allows(None, None));
    }

    #[test]
    fn cost_ceiling_rejects_over_cost_cents_limit() {
        let ceiling = CostCeiling {
            tokens: None,
            time_ms: None,
            cost_cents: Some(50),
        };
        assert!(!ceiling.allows(None, Some(51)));
    }

    #[test]
    fn cost_ceiling_allows_under_cost_cents_limit() {
        let ceiling = CostCeiling {
            tokens: None,
            time_ms: None,
            cost_cents: Some(50),
        };
        assert!(ceiling.allows(None, Some(49)));
    }

    #[test]
    fn cost_ceiling_rejects_either_branch_failing() {
        // Both limits set; tokens over, cost under. Overall: reject.
        let ceiling = CostCeiling {
            tokens: Some(100),
            time_ms: None,
            cost_cents: Some(50),
        };
        assert!(!ceiling.allows(Some(101), Some(10)));
    }

    #[test]
    fn cost_ceiling_default_is_unrestricted() {
        let ceiling = CostCeiling::default();
        assert!(ceiling.tokens.is_none());
        assert!(ceiling.time_ms.is_none());
        assert!(ceiling.cost_cents.is_none());
    }

    // ─── PolicyContext::default (M61) ──────────────────────────────────

    #[test]
    fn policy_context_default_is_anonymous_development() {
        let ctx = PolicyContext::default();
        assert_eq!(ctx.user_id, "anonymous");
        assert_eq!(ctx.environment, DeploymentEnv::Development);
        assert_eq!(ctx.security_impact, SecurityImpact::Low);
        assert!(ctx.requesting_capabilities.is_empty());
        assert!(ctx.affected_components.is_empty());
        assert_eq!(ctx.cost_ceiling, CostCeiling::default());
    }

    #[test]
    fn policy_context_serializes_round_trip() {
        // The context travels through serde (audit/log, MCP, etc.); if
        // the round-trip drops fields, downstream rules see default
        // values where the caller set explicit ones.
        let ctx = PolicyContext {
            user_id: "alice".into(),
            environment: DeploymentEnv::Production,
            security_impact: SecurityImpact::High,
            requesting_capabilities: vec!["deploy".into(), "read".into()],
            affected_components: vec!["orders-service".into()],
            cost_ceiling: CostCeiling {
                tokens: Some(5000),
                time_ms: Some(60_000),
                cost_cents: Some(200),
            },
        };
        let json = serde_json::to_string(&ctx).expect("serialize");
        let parsed: PolicyContext = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.user_id, "alice");
        assert_eq!(parsed.environment, DeploymentEnv::Production);
        assert_eq!(parsed.security_impact, SecurityImpact::High);
        assert_eq!(parsed.requesting_capabilities, vec!["deploy", "read"]);
        assert_eq!(parsed.affected_components, vec!["orders-service"]);
        assert_eq!(parsed.cost_ceiling.tokens, Some(5000));
        assert_eq!(parsed.cost_ceiling.time_ms, Some(60_000));
        assert_eq!(parsed.cost_ceiling.cost_cents, Some(200));
    }

    #[test]
    fn policy_context_round_trip_with_defaults() {
        // serde(default) on affected_components and cost_ceiling must
        // survive a JSON that omits them.
        let json = r#"{
            "user_id": "bob",
            "environment": "Staging",
            "security_impact": "Medium",
            "requesting_capabilities": []
        }"#;
        let parsed: PolicyContext = serde_json::from_str(json).expect("deserialize with defaults");
        assert_eq!(parsed.user_id, "bob");
        assert!(parsed.affected_components.is_empty());
        assert_eq!(parsed.cost_ceiling, CostCeiling::default());
    }
}
