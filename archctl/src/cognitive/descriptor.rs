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

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage v3, 2026-08-22)
    // ---------------------------------------------------------------------------

    /// All 4 `ModelPolicy` variants roundtrip via serde. Locks the canonical
    /// string forms and the `PartialEq`/`Eq` contract.
    #[test]
    fn model_policy_all_variants_serde() {
        for (original, expected_json) in [
            (ModelPolicy::Heuristic, "\"Heuristic\""),
            (ModelPolicy::LocalLLM, "\"LocalLLM\""),
            (ModelPolicy::PowerfulLLM, "\"PowerfulLLM\""),
            (ModelPolicy::Human, "\"Human\""),
        ] {
            let json = serde_json::to_string(&original).unwrap();
            assert_eq!(
                json, expected_json,
                "ModelPolicy::{original:?} must serialize to {expected_json}, got {json}"
            );
            let back: ModelPolicy = serde_json::from_str(&json).unwrap();
            assert_eq!(back, original, "round-trip mismatch for {original:?}");
        }
    }

    /// `AgentBudget` with all 3 fields populated roundtrips through serde.
    /// Distinct from `agent_budget_default` which checks the empty case.
    #[test]
    fn agent_budget_serde_with_values() {
        let b = AgentBudget {
            tokens: Some(2048),
            time_ms: Some(1000),
            cost_cents: Some(5),
        };
        let json = serde_json::to_string(&b).unwrap();
        let back: AgentBudget = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tokens, Some(2048));
        assert_eq!(back.time_ms, Some(1000));
        assert_eq!(back.cost_cents, Some(5));
    }

    /// `ToolDescriptor` roundtrips with all fields populated.
    #[test]
    fn tool_descriptor_serde() {
        let td = ToolDescriptor {
            id: "graph_query".into(),
            description: "Run a Cypher query against the graph".into(),
            input_schema: r#"{"type":"object"}"#.into(),
        };
        let json = serde_json::to_string(&td).unwrap();
        let back: ToolDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id.as_str(), "graph_query");
        assert_eq!(back.description, "Run a Cypher query against the graph");
        assert_eq!(back.input_schema, r#"{"type":"object"}"#);
    }

    /// `Capability` with `max_calls_per_invocation` Some and None roundtrips.
    /// Locks the Optional contract: None serializes as JSON null (not omitted)
    /// since `Option<T>` defaults to `null` serialization in serde.
    #[test]
    fn capability_serde_with_and_without_max_calls() {
        let with_max = Capability {
            tool_id: "t1".into(),
            max_calls_per_invocation: Some(5),
        };
        let json_with = serde_json::to_string(&with_max).unwrap();
        assert!(json_with.contains("\"max_calls_per_invocation\":5"));

        let without_max = Capability {
            tool_id: "t2".into(),
            max_calls_per_invocation: None,
        };
        let json_without = serde_json::to_string(&without_max).unwrap();
        // Option<u32> serializes as null (not omitted) — locks the contract
        assert!(json_without.contains("\"max_calls_per_invocation\":null"));

        let back: Capability = serde_json::from_str(&json_without).unwrap();
        assert!(back.max_calls_per_invocation.is_none());
    }

    /// `AgentDescriptor` roundtrips with all fields populated, including
    /// nested `AgentBudget` and `Vec<Capability>`. End-to-end serde check.
    #[test]
    fn agent_descriptor_full_serde() {
        let original = AgentDescriptor {
            id: "my-agent".into(),
            version: "1.2.3".into(),
            subscriptions: vec!["event.a".into(), "event.b".into()],
            required_views: vec!["v1".into()],
            output_schema: r#"{"type":"object"}"#.into(),
            model_policy: ModelPolicy::LocalLLM,
            budget: AgentBudget {
                tokens: Some(4096),
                time_ms: Some(2000),
                cost_cents: Some(10),
            },
            capabilities: vec![Capability {
                tool_id: "tool-a".into(),
                max_calls_per_invocation: Some(3),
            }],
            deterministic: false,
            idempotent: true,
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: AgentDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id.as_str(), "my-agent");
        assert_eq!(back.subscriptions.len(), 2);
        assert_eq!(back.budget.tokens, Some(4096));
        assert_eq!(back.capabilities.len(), 1);
        assert!(!back.deterministic);
        assert!(back.idempotent);
    }

    /// `Rule` with `budget_override` Some AND None both roundtrip. Locks the
    /// `Option<AgentBudget>` serialization contract.
    #[test]
    fn rule_serde_with_optional_budget_override() {
        let rule_without = Rule {
            name: "low-conf".into(),
            conditions: RuleConditions {
                confidence_below: Some(0.5),
                has_evidence_type: None,
                view_name_matches: None,
                goal_contains: None,
            },
            escalate_to: ModelPolicy::Human,
            budget_override: None,
        };
        let json = serde_json::to_string(&rule_without).unwrap();
        assert!(json.contains("\"budget_override\":null"));
        let back: Rule = serde_json::from_str(&json).unwrap();
        assert!(back.budget_override.is_none());

        let rule_with = Rule {
            name: "low-conf-override".into(),
            conditions: RuleConditions {
                confidence_below: Some(0.3),
                has_evidence_type: None,
                view_name_matches: None,
                goal_contains: None,
            },
            escalate_to: ModelPolicy::PowerfulLLM,
            budget_override: Some(AgentBudget {
                tokens: Some(8192),
                time_ms: Some(5000),
                cost_cents: Some(20),
            }),
        };
        let json_with = serde_json::to_string(&rule_with).unwrap();
        let back_with: Rule = serde_json::from_str(&json_with).unwrap();
        assert_eq!(
            back_with.budget_override.as_ref().and_then(|b| b.tokens),
            Some(8192)
        );
    }

    /// `RuleConditions` with ALL 4 fields populated roundtrips. Distinct from
    /// `rule_conditions` which only sets 2.
    #[test]
    fn rule_conditions_all_fields_serde() {
        let json = r#"{
            "confidence_below": 0.3,
            "has_evidence_type": "file",
            "view_name_matches": ".*auth.*",
            "goal_contains": "refactor"
        }"#;
        let c: RuleConditions = serde_json::from_str(json).unwrap();
        assert_eq!(c.confidence_below, Some(0.3));
        assert_eq!(c.has_evidence_type.as_deref(), Some("file"));
        assert_eq!(c.view_name_matches.as_deref(), Some(".*auth.*"));
        assert_eq!(c.goal_contains.as_deref(), Some("refactor"));

        let back_json = serde_json::to_string(&c).unwrap();
        let back: RuleConditions = serde_json::from_str(&back_json).unwrap();
        assert_eq!(back.confidence_below, Some(0.3));
        assert_eq!(back.has_evidence_type.as_deref(), Some("file"));
    }

    /// `RuleConditions` deserializes from empty JSON: all 4 Option fields
    /// default to None via their per-field `#[serde(default)]` (or struct default).
    /// Locks the permissive shape — partial TOML rules are valid.
    #[test]
    fn rule_conditions_deserializes_empty_to_all_none() {
        let c: RuleConditions = serde_json::from_str("{}").unwrap();
        assert!(c.confidence_below.is_none());
        assert!(c.has_evidence_type.is_none());
        assert!(c.view_name_matches.is_none());
        assert!(c.goal_contains.is_none());
    }
}
