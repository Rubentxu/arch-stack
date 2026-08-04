//! Agent descriptor and policy types.

use serde::{Deserialize, Serialize};

/// Which model class a agent is permitted to invoke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelPolicy {
    Heuristic,
    LocalLLM,
    PowerfulLLM,
    Human,
}

/// Budget constraints for a agent invocation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentBudget {
    /// Max tokens in + out combined.
    pub tokens: Option<u32>,
    /// Max wall-clock time in milliseconds.
    pub time_ms: Option<u64>,
    /// Max cost in cents (for paid models).
    pub cost_cents: Option<u64>,
}

/// A tool the agent may call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub id: String,
    pub description: String,
    pub input_schema: String,
}

/// Registered capability of an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub tool_id: String,
    pub max_calls_per_invocation: Option<u32>,
}

/// Static description of a agent instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDescriptor {
    pub id: String,
    pub version: String,
    pub subscriptions: Vec<String>, // event pattern names
    pub required_views: Vec<String>,
    pub output_schema: String,
    pub model_policy: ModelPolicy,
    pub budget: AgentBudget,
    pub capabilities: Vec<Capability>,
    pub deterministic: bool,
    pub idempotent: bool,
}

/// A single escalation rule loaded from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Human-readable name for this rule.
    pub name: String,
    /// Conditions that trigger this rule (all must match).
    pub conditions: RuleConditions,
    /// The model policy to escalate to when matched.
    pub escalate_to: ModelPolicy,
    /// Optional budget override for this level.
    pub budget_override: Option<AgentBudget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConditions {
    /// Confidence below this threshold triggers.
    pub confidence_below: Option<f64>,
    /// Trigger when evidence type is present.
    pub has_evidence_type: Option<String>,
    /// Trigger when view name matches regex.
    pub view_name_matches: Option<String>,
    /// Trigger on specific goal keyword.
    pub goal_contains: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_policy_serde() {
        let mp = ModelPolicy::LocalLLM;
        let json = serde_json::to_string(&mp).unwrap();
        let back: ModelPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ModelPolicy::LocalLLM);
    }

    #[test]
    fn agent_budget_default() {
        let b = AgentBudget::default();
        assert!(b.tokens.is_none());
        assert!(b.time_ms.is_none());
        assert!(b.cost_cents.is_none());
    }

    #[test]
    fn rule_conditions() {
        let json = r#"{"confidence_below":0.5,"goal_contains":"refactor"}"#;
        let c: RuleConditions = serde_json::from_str(json).unwrap();
        assert_eq!(c.confidence_below, Some(0.5));
        assert_eq!(c.goal_contains, Some("refactor".into()));
    }
}
