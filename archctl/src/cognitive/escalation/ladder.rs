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

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage v2, 2026-08-22)
    // ---------------------------------------------------------------------------

    use crate::cognitive::context::{Evidence, ProvenanceId};

    fn evidence_with_confidence(conf: f64) -> Evidence {
        let mut props = serde_json::Map::new();
        props.insert("confidence".into(), serde_json::json!(conf));
        Evidence {
            id: format!("ev-{conf}"),
            provenance_id: ProvenanceId::File {
                path: "src/main.rs".into(),
                line: 1,
            },
            content_hash: "blake3:deadbeef".into(),
            text: format!("evidence with confidence {conf}"),
            properties: props,
        }
    }

    fn ctx_with_goal_and_evidence(goal: &str, confidences: &[f64]) -> AgentContext {
        let mut ctx = make_ctx(goal);
        ctx.evidence = confidences
            .iter()
            .map(|c| evidence_with_confidence(*c))
            .collect();
        ctx
    }

    /// `Default::default()` for `EscalationLadder` is the manual `Default` impl
    /// derived on the struct — but the `rules: Vec<Rule>` field is empty.
    /// It is NOT equivalent to `default_ladder()`. This test locks the
    /// current behavior so a future refactor to `#[derive(Default)]` on a
    /// populated ladder would be a deliberate change.
    #[test]
    fn default_trait_impl_yields_empty_rules() {
        let default: EscalationLadder = EscalationLadder::default();
        assert!(
            default.rules.is_empty(),
            "Default::default() must produce a ladder with zero rules; \
             callers must use default_ladder() for the populated default"
        );
    }

    /// `load_from_path` returns the default ladder (not an error) when the
    /// file does not exist. Mirrors the XDG fallback semantics.
    #[test]
    fn load_from_path_missing_file_falls_back_to_default() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("never_exists.toml");
        let ladder = EscalationLadder::load_from_path(&path)
            .expect("missing path must be Ok(default_ladder)");
        assert_eq!(
            ladder.rules.len(),
            3,
            "missing file must yield default ladder"
        );
        assert_eq!(ladder.rules[0].escalate_to, ModelPolicy::LocalLLM);
    }

    /// `load_from_path` returns `InvalidData` IO error when the file exists
    /// but contains malformed TOML.
    #[test]
    fn load_from_path_invalid_toml_returns_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, b"this is not = valid toml [[[").unwrap();
        let result = EscalationLadder::load_from_path(&path);
        match result {
            Ok(_) => panic!("malformed TOML must error"),
            Err(e) => assert_eq!(e.kind(), std::io::ErrorKind::InvalidData),
        }
    }

    /// `load_from_path` for a valid TOML file returns the parsed ladder.
    #[test]
    fn load_from_path_valid_toml_returns_parsed() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("ladder.toml");
        std::fs::write(
            &path,
            br#"
[[rule]]
name = "file-rule"
goal_contains = "ARCH"
escalate_to = "Human"
"#,
        )
        .unwrap();
        let ladder = EscalationLadder::load_from_path(&path).unwrap();
        assert_eq!(ladder.rules.len(), 1);
        assert_eq!(ladder.rules[0].name, "file-rule");
        assert_eq!(ladder.rules[0].escalate_to, ModelPolicy::Human);
    }

    /// `load_from_str` with an empty `rule = []` array produces a ladder
    /// with zero rules.
    #[test]
    fn load_from_str_empty_rules_array_yields_empty_ladder() {
        let toml = r#"rule = []"#;
        let ladder = EscalationLadder::load_from_str(toml).unwrap();
        assert!(ladder.rules.is_empty());
    }

    /// An empty-rules ladder always evaluates to `Heuristic` (the documented
    /// fallback) regardless of context.
    #[test]
    fn evaluate_with_empty_rules_returns_heuristic() {
        let ladder = EscalationLadder::default(); // empty
        assert_eq!(
            ladder.evaluate(&make_ctx("anything")),
            ModelPolicy::Heuristic
        );
        assert_eq!(
            ladder.evaluate(&ctx_with_goal_and_evidence("refactor", &[0.1])),
            ModelPolicy::Heuristic
        );
    }

    /// When multiple rules match, the first one in declaration order wins.
    #[test]
    fn evaluate_first_matching_rule_wins() {
        let toml = r#"
[[rule]]
name = "first"
goal_contains = "refactor"
escalate_to = "LocalLLM"

[[rule]]
name = "second"
goal_contains = "refactor"
escalate_to = "Human"
"#;
        let ladder = EscalationLadder::load_from_str(toml).unwrap();
        let ctx = make_ctx("please refactor module X");
        assert_eq!(
            ladder.evaluate(&ctx),
            ModelPolicy::LocalLLM,
            "first matching rule must win, not the last"
        );
    }

    /// `parse_model_policy` accepts all documented spellings (camelCase,
    /// snake_case, kebab-case) for `LocalLLM`/`PowerfulLLM`, plus plain
    /// `human` and `heuristic`. Unknown strings fall back to `Heuristic`
    /// (NOT a parse error) — silent default is intentional, mirrors the
    /// permissive escalation philosophy.
    #[test]
    fn parse_model_policy_all_spellings_and_unknown_fallback() {
        // Direct access via the in-module function is not possible (private),
        // but the behavior is observable through `load_from_str`. Use a
        // minimal TOML rule per variant.
        let cases = [
            ("heuristic", ModelPolicy::Heuristic),
            ("Heuristic", ModelPolicy::Heuristic),
            ("localllm", ModelPolicy::LocalLLM),
            ("local_llm", ModelPolicy::LocalLLM),
            ("local-llm", ModelPolicy::LocalLLM),
            ("powerfulllm", ModelPolicy::PowerfulLLM),
            ("powerful_llm", ModelPolicy::PowerfulLLM),
            ("powerful-llm", ModelPolicy::PowerfulLLM),
            ("human", ModelPolicy::Human),
            ("Human", ModelPolicy::Human),
            // Unknown spelling silently falls back to Heuristic
            ("unknown-policy-name", ModelPolicy::Heuristic),
            ("", ModelPolicy::Heuristic),
        ];
        for (spelling, expected) in cases {
            let toml = format!(
                r#"
[[rule]]
name = "r"
goal_contains = "match"
escalate_to = "{spelling}"
"#
            );
            let ladder = EscalationLadder::load_from_str(&toml)
                .unwrap_or_else(|e| panic!("parsing '{spelling}' must not fail: {e}"));
            assert_eq!(
                ladder.rules[0].escalate_to, expected,
                "spelling '{spelling}' must map to {expected:?}"
            );
        }
    }

    /// `confidence_below` triggers a match when at least one evidence item
    /// has a `confidence` property strictly below the threshold. Locks the
    /// `f64::min` fold behavior (any evidence below the threshold triggers).
    #[test]
    fn evaluate_confidence_below_triggers_match() {
        let toml = r#"
[[rule]]
name = "low-confidence"
confidence_below = 0.5
escalate_to = "Human"
"#;
        let ladder = EscalationLadder::load_from_str(toml).unwrap();
        // Mix of high and low confidence — the low one triggers
        let ctx = ctx_with_goal_and_evidence("anything", &[0.9, 0.3, 0.8]);
        assert_eq!(ladder.evaluate(&ctx), ModelPolicy::Human);
        // Single low-confidence evidence also triggers
        let ctx = ctx_with_goal_and_evidence("anything", &[0.4]);
        assert_eq!(ladder.evaluate(&ctx), ModelPolicy::Human);
    }

    /// `confidence_below` does NOT match when all evidence is at or above
    /// the threshold (the `min_confidence >= threshold → return false`
    /// branch in `rule_matches`).
    #[test]
    fn evaluate_confidence_above_threshold_no_match() {
        let toml = r#"
[[rule]]
name = "low-confidence"
confidence_below = 0.5
escalate_to = "Human"
"#;
        let ladder = EscalationLadder::load_from_str(toml).unwrap();
        // All evidence above threshold
        let ctx = ctx_with_goal_and_evidence("anything", &[0.9, 0.8, 0.5]);
        assert_eq!(
            ladder.evaluate(&ctx),
            ModelPolicy::Heuristic,
            "rule must NOT match when min(confidence) >= threshold"
        );
        // No evidence at all → min(INFINITY) >= threshold → no match
        let ctx = make_ctx("anything");
        assert_eq!(ladder.evaluate(&ctx), ModelPolicy::Heuristic);
    }

    /// `goal_contains` matching is symmetric (`to_lowercase()` on both
    /// sides): the keyword in the rule and the keyword in the goal are
    /// both lowercased before comparison. Locks the case-insensitive path
    /// for any future refactor that might switch to `==`.
    #[test]
    fn evaluate_goal_contains_is_case_insensitive() {
        let toml = r#"
[[rule]]
name = "case-test"
goal_contains = "REFACTOR"
escalate_to = "Human"
"#;
        let ladder = EscalationLadder::load_from_str(toml).unwrap();
        // Lowercase goal
        assert_eq!(
            ladder.evaluate(&make_ctx("please refactor module X")),
            ModelPolicy::Human
        );
        // Mixed case goal
        assert_eq!(
            ladder.evaluate(&make_ctx("please ReFactor module X")),
            ModelPolicy::Human
        );
        // Uppercase goal
        assert_eq!(
            ladder.evaluate(&make_ctx("PLEASE REFACTOR MODULE X")),
            ModelPolicy::Human
        );
    }

    /// `load_from_str` returns `Err(toml::de::Error)` (not an IO error)
    /// when the TOML is structurally invalid. Distinct from the
    /// `load_from_path` test which wraps the error in `InvalidData`.
    #[test]
    fn load_from_str_invalid_toml_returns_toml_error() {
        let bad_toml = "[[rule]\nname = "; // truncated
        assert!(
            EscalationLadder::load_from_str(bad_toml).is_err(),
            "malformed TOML must error"
        );
    }
}
