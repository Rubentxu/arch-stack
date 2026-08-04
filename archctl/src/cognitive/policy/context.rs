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
