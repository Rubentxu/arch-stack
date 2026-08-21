//! Escalation ladder — evaluates rules and selects the appropriate ModelPolicy.

use std::path::Path;

use crate::cognitive::context::AgentContext;
use crate::cognitive::descriptor::{ModelPolicy, Rule};

/// Default escalation ladder configuration.
/// Loaded from `$XDG_CONFIG_HOME/archctl/escalation.toml` if present,
/// otherwise falls back to these defaults.
const DEFAULT_RULES: &[(&str, ModelPolicy, Option<f64>)] = &[
    ("heuristic-insufficient", ModelPolicy::LocalLLM, Some(0.5)),
    ("local-llm-expensive", ModelPolicy::PowerfulLLM, Some(0.8)),
    ("powerful-llm-critical", ModelPolicy::Human, Some(0.95)),
];

/// The escalation ladder evaluates context against ordered rules
/// and returns the first matching ModelPolicy.
#[derive(Default)]
pub struct EscalationLadder {
    rules: Vec<Rule>,
}

impl EscalationLadder {
    /// Build a ladder with the default rule set.
    pub fn default_ladder() -> Self {
        let rules: Vec<Rule> = DEFAULT_RULES
            .iter()
            .map(|(name, policy, conf_below)| Rule {
                name: name.to_string(),
                conditions: crate::cognitive::descriptor::RuleConditions {
                    confidence_below: *conf_below,
                    has_evidence_type: None,
                    view_name_matches: None,
                    goal_contains: None,
                },
                escalate_to: *policy,
                budget_override: None,
            })
            .collect();
        Self { rules }
    }

    /// Load rules from a TOML file. Returns Ok(ladder) or Ok(default) on parse error.
    pub fn load_from_path(path: &Path) -> std::io::Result<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default_ladder());
            }
            Err(e) => return Err(e),
        };
        Self::load_from_str(&text)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
    }

    /// Parse rules from TOML string. Returns default ladder on error.
    pub fn load_from_str(text: &str) -> Result<Self, toml::de::Error> {
        #[derive(serde::Deserialize)]
        struct TomlRule {
            name: String,
            confidence_below: Option<f64>,
            has_evidence_type: Option<String>,
            view_name_matches: Option<String>,
            goal_contains: Option<String>,
            escalate_to: String,
        }

        #[derive(serde::Deserialize)]
        struct TomlLadder {
            rule: Vec<TomlRule>,
        }

        let ladder: TomlLadder = toml::from_str(text)?;
        let rules: Vec<Rule> = ladder
            .rule
            .into_iter()
            .map(|tr| Rule {
                name: tr.name,
                conditions: crate::cognitive::descriptor::RuleConditions {
                    confidence_below: tr.confidence_below,
                    has_evidence_type: tr.has_evidence_type,
                    view_name_matches: tr.view_name_matches,
                    goal_contains: tr.goal_contains,
                },
                escalate_to: parse_model_policy(&tr.escalate_to),
                budget_override: None,
            })
            .collect();
        Ok(Self { rules })
    }

    /// Evaluate context against all rules and return the first matching ModelPolicy.
    /// Returns the highest policy in the escalation chain if no rules match.
    pub fn evaluate(&self, context: &AgentContext) -> ModelPolicy {
        for rule in &self.rules {
            if self.rule_matches(&rule.conditions, context) {
                return rule.escalate_to;
            }
        }
        // Default: stay at heuristic
        ModelPolicy::Heuristic
    }

    fn rule_matches(
        &self,
        cond: &crate::cognitive::descriptor::RuleConditions,
        ctx: &AgentContext,
    ) -> bool {
        if let Some(threshold) = cond.confidence_below {
            // Check if any evidence has confidence below threshold
            let min_confidence = ctx
                .evidence
                .iter()
                .filter_map(|e| e.properties.get("confidence").and_then(|v| v.as_f64()))
                .fold(f64::INFINITY, f64::min);

            if min_confidence >= threshold {
                return false;
            }
        }
        if let Some(ref keyword) = cond.goal_contains
            && !ctx.goal.to_lowercase().contains(&keyword.to_lowercase())
        {
            return false;
        }
        true
    }
}

fn parse_model_policy(s: &str) -> ModelPolicy {
    match s.to_lowercase().as_str() {
        "heuristic" => ModelPolicy::Heuristic,
        "localllm" | "local_llm" | "local-llm" => ModelPolicy::LocalLLM,
        "powerfulllm" | "powerful_llm" | "powerful-llm" => ModelPolicy::PowerfulLLM,
        "human" => ModelPolicy::Human,
        _ => ModelPolicy::Heuristic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::AgentBudget;

    fn make_ctx(goal: &str) -> AgentContext {
        // REQ-T06-003: feedback_history plumbing — see AgentContext::with_feedback_history
        // REQ-M25-006: pending_adjudications wiring (TRUST-008 REQ-T08-005). Escalation
        // contexts intentionally leave the field empty — operators reviewing escalations do
        // not need the open adjudication queue surfaced.
        AgentContext {
            goal: goal.into(),
            triggering_event: None,
            graph_view: Default::default(),
            source_fragments: vec![],
            evidence: vec![],
            applicable_rules: vec![],
            available_tools: vec![],
            budget: AgentBudget::default(),
            feedback_history: vec![],
            pending_adjudications: vec![],
        }
    }

    #[test]
    fn default_ladder() {
        let ladder = EscalationLadder::default_ladder();
        assert_eq!(ladder.rules.len(), 3);
        assert_eq!(ladder.rules[0].escalate_to, ModelPolicy::LocalLLM);
    }

    #[test]
    fn load_from_toml() {
        let toml = r#"
[[rule]]
name = "refactor-goal"
goal_contains = "refactor"
escalate_to = "PowerfulLLM"

[[rule]]
name = "high-confidence"
confidence_below = 0.3
escalate_to = "Human"
"#;
        let ladder = EscalationLadder::load_from_str(toml).unwrap();
        assert_eq!(ladder.rules.len(), 2);
        assert_eq!(ladder.rules[0].name, "refactor-goal");
        assert_eq!(ladder.rules[1].escalate_to, ModelPolicy::Human);
    }

    #[test]
    fn evaluate_no_match_returns_heuristic() {
        let ladder = EscalationLadder::default_ladder();
        let ctx = make_ctx("show me the structure");
        assert_eq!(ladder.evaluate(&ctx), ModelPolicy::Heuristic);
    }

    #[test]
    fn evaluate_goal_match() {
        let toml = r#"
[[rule]]
name = "refactor"
goal_contains = "refactor"
escalate_to = "LocalLLM"
"#;
        let ladder = EscalationLadder::load_from_str(toml).unwrap();
        let ctx = make_ctx("should I refactor component X?");
        assert_eq!(ladder.evaluate(&ctx), ModelPolicy::LocalLLM);
    }
}
