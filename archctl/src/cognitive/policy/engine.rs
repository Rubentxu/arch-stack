//! Policy engine — field-equality rule evaluation.

use std::path::Path;

use crate::cognitive::output::{ActionProposal, ApprovalLevel};

use super::context::PolicyContext;
use super::decision::{PolicyDecision, PolicyResult};

/// Policy rule condition — all fields must match for the rule to trigger.
#[derive(Debug, Clone)]
pub struct PolicyRule {
    pub name: String,
    pub command: Option<String>,
    pub environment: Option<crate::cognitive::output::DeploymentEnv>,
    pub security_impact: Option<crate::cognitive::output::SecurityImpact>,
    pub confidence_below: Option<f32>,
    pub cost_above_tokens: Option<u32>,
    pub decision: PolicyDecision,
}

/// Policy trait — implemented by any policy source (file, embedded, custom).
pub trait Policy: Send + Sync {
    fn evaluate(&self, proposal: &ActionProposal, ctx: &PolicyContext) -> PolicyResult;
}

/// Policy engine — evaluates ActionProposals against ordered field-equality rules.
#[derive(Default)]
pub struct PolicyEngine {
    rules: Vec<PolicyRule>,
}

impl PolicyEngine {
    /// Evaluate a proposal and return the first matching decision.
    pub fn evaluate(&self, proposal: &ActionProposal, ctx: &PolicyContext) -> PolicyResult {
        for rule in &self.rules {
            if rule.matches(proposal, ctx) {
                return PolicyResult {
                    decision: rule.decision.clone(),
                    matched_rule: Some(rule.name.clone()),
                };
            }
        }
        // Default: require peer approval for anything unknown
        PolicyResult {
            decision: PolicyDecision::RequireApproval {
                level: ApprovalLevel::PeerApproval,
                reason: "no policy rule matched".into(),
            },
            matched_rule: None,
        }
    }

    /// Load policy rules from a TOML file.
    /// Falls back to embedded defaults if the file is not found or TOML is invalid.
    pub fn load_from_path(path: &Path) -> std::io::Result<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default_engine());
            }
            Err(e) => return Err(e),
        };
        Ok(Self::load_from_str(&text))
    }

    /// Parse policy rules from a TOML string.
    /// Falls back to embedded defaults if parsing fails.
    pub fn load_from_str(text: &str) -> Self {
        #[derive(serde::Deserialize)]
        struct TomlRule {
            name: String,
            command: Option<String>,
            environment: Option<String>,
            security_impact: Option<String>,
            confidence_below: Option<f32>,
            cost_above_tokens: Option<u32>,
            decision: String,
            reason: Option<String>,
            level: Option<String>,
            to: Option<Vec<String>>,
            target: Option<String>,
        }

        #[derive(serde::Deserialize)]
        struct TomlPolicies {
            policy: Vec<TomlRule>,
        }

        let parsed: TomlPolicies = match toml::from_str(text) {
            Ok(p) => p,
            Err(_) => return Self::default_engine(),
        };
        let rules: Vec<PolicyRule> = parsed
            .policy
            .into_iter()
            .map(|tr| PolicyRule {
                name: tr.name,
                command: tr.command,
                environment: tr.environment.and_then(|s| parse_env(&s)),
                security_impact: tr.security_impact.and_then(|s| parse_security(&s)),
                confidence_below: tr.confidence_below,
                cost_above_tokens: tr.cost_above_tokens,
                decision: parse_decision(
                    &tr.decision,
                    tr.reason.as_deref(),
                    tr.level.as_deref(),
                    tr.to,
                    tr.target,
                ),
            })
            .collect();

        Self { rules }
    }

    /// Build the default engine with embedded v1.0 rules.
    pub fn default_engine() -> Self {
        Self::load_from_str(DEFAULT_POLICIES)
    }
}

impl Policy for PolicyEngine {
    fn evaluate(&self, proposal: &ActionProposal, ctx: &PolicyContext) -> PolicyResult {
        PolicyEngine::evaluate(self, proposal, ctx)
    }
}

impl PolicyRule {
    /// Check if this rule matches the given proposal and context.
    fn matches(&self, proposal: &ActionProposal, ctx: &PolicyContext) -> bool {
        if let Some(ref cmd) = self.command
            && &proposal.command != cmd
        {
            return false;
        }
        if let Some(ref env) = self.environment
            && ctx.environment != *env
        {
            return false;
        }
        if let Some(ref si) = self.security_impact
            && ctx.security_impact != *si
        {
            return false;
        }
        if let Some(threshold) = self.confidence_below {
            let conf = proposal.confidence.unwrap_or(0.0);
            if conf >= threshold {
                return false;
            }
        }
        if let Some(min_tokens) = self.cost_above_tokens {
            let tokens = proposal.cost_estimate.tokens.unwrap_or(0);
            if tokens <= min_tokens {
                return false;
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Default v1.0 rules (embedded TOML)
// ---------------------------------------------------------------------------

const DEFAULT_POLICIES: &str = r#"
# Generic/low-specificity rules first (confidence, security impact)
# These are evaluated BEFORE command-specific rules.

[[policy]]
name = "low-confidence-require-peer"
confidence_below = 0.6
decision = "RequireApproval"
level = "PeerApproval"
reason = "low-confidence proposal"

[[policy]]
name = "high-security-impact-require-review"
security_impact = "Critical"
decision = "RequireApproval"
level = "SecurityApproval"
reason = "critical security impact requires security team review"

# Command-specific rules — more specific rules must come before general ones
# within this group.

[[policy]]
name = "tests-in-dev-auto"
command = "run_tests"
environment = "Development"
decision = "Allow"

[[policy]]
name = "tests-in-prod-require-peer"
command = "run_tests"
environment = "Production"
decision = "RequireApproval"
level = "PeerApproval"
reason = "tests on production require peer sign-off"

[[policy]]
name = "modify-source-always-require-tech-lead"
command = "modify_source"
environment = "any"
decision = "RequireApproval"
level = "TechLeadApproval"
reason = "modifying source files requires tech lead"

[[policy]]
name = "deploy-production-always-deny"
command = "deploy_production"
environment = "any"
decision = "Deny"
reason = "production deploys require human + CI"
"#;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_env(s: &str) -> Option<crate::cognitive::output::DeploymentEnv> {
    match s {
        "Development" | "dev" => Some(crate::cognitive::output::DeploymentEnv::Development),
        "Staging" | "staging" => Some(crate::cognitive::output::DeploymentEnv::Staging),
        "Production" | "prod" => Some(crate::cognitive::output::DeploymentEnv::Production),
        "any" | "Any" => None, // wildcard
        _ => None,
    }
}

fn parse_security(s: &str) -> Option<crate::cognitive::output::SecurityImpact> {
    match s {
        "Low" | "low" => Some(crate::cognitive::output::SecurityImpact::Low),
        "Medium" | "medium" => Some(crate::cognitive::output::SecurityImpact::Medium),
        "High" | "high" => Some(crate::cognitive::output::SecurityImpact::High),
        "Critical" | "critical" => Some(crate::cognitive::output::SecurityImpact::Critical),
        _ => None,
    }
}

fn parse_approval_level(s: &str) -> ApprovalLevel {
    match s {
        "SelfApproval" => ApprovalLevel::SelfApproval,
        "PeerApproval" => ApprovalLevel::PeerApproval,
        "TechLeadApproval" => ApprovalLevel::TechLeadApproval,
        "SecurityApproval" => ApprovalLevel::SecurityApproval,
        other if other.starts_with("MultiPartyApproval") => {
            // Parse "MultiPartyApproval { required: 2, total: 3 }"
            let required = extract_num(other, "required");
            let total = extract_num(other, "total");
            ApprovalLevel::MultiPartyApproval {
                required: required.unwrap_or(1),
                total: total.unwrap_or(1),
            }
        }
        _ => ApprovalLevel::PeerApproval,
    }
}

fn extract_num(s: &str, field: &str) -> Option<u32> {
    let pattern = format!("{}:", field);
    s.find(&pattern).and_then(|i| {
        let rest = &s[i + pattern.len()..];
        rest.chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<u32>()
            .ok()
    })
}

fn parse_decision(
    s: &str,
    reason: Option<&str>,
    level: Option<&str>,
    to: Option<Vec<String>>,
    target: Option<String>,
) -> PolicyDecision {
    let reason = reason.unwrap_or("policy rule").to_string();
    match s {
        "Allow" => PolicyDecision::Allow,
        "AllowWithNotify" => PolicyDecision::AllowWithNotify(to.unwrap_or_default()),
        "RequireApproval" => PolicyDecision::RequireApproval {
            level: level
                .map(parse_approval_level)
                .unwrap_or(ApprovalLevel::PeerApproval),
            reason,
        },
        "Deny" => PolicyDecision::Deny { reason },
        "Escalate" => PolicyDecision::Escalate {
            target: target.unwrap_or_else(|| "tech-lead".to_string()),
        },
        _ => PolicyDecision::Deny {
            reason: format!("unknown policy decision: {}", s),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::output::{ActionProposal, CostEstimate, DeploymentEnv, SecurityImpact};

    fn make_proposal(
        command: &str,
        confidence: Option<f32>,
        tokens: Option<u32>,
    ) -> ActionProposal {
        ActionProposal {
            id: None,
            cause: None,
            triggering_agent: None,
            goal: command.into(),
            command: command.into(),
            args: vec![],
            capabilities: vec![],
            approval: Default::default(),
            expected_evidence: vec![],
            rollback: None,
            cost_estimate: CostEstimate {
                tokens,
                time_ms: None,
                cost_cents: None,
                side_effects: vec![],
            },
            confidence,
            ttl_ms: None,
            security_impact: None,
            deployment_env: None,
            policy_rule_matched: None,
            approval_required: false,
            expected_evidence_old: String::new(),
        }
    }

    fn dev_ctx() -> PolicyContext {
        PolicyContext {
            user_id: "dev".into(),
            environment: DeploymentEnv::Development,
            security_impact: SecurityImpact::Low,
            requesting_capabilities: vec![],
            affected_components: vec![],
            cost_ceiling: Default::default(),
        }
    }

    fn prod_ctx() -> PolicyContext {
        PolicyContext {
            user_id: "dev".into(),
            environment: DeploymentEnv::Production,
            security_impact: SecurityImpact::Low,
            requesting_capabilities: vec![],
            affected_components: vec![],
            cost_ceiling: Default::default(),
        }
    }

    #[test]
    fn tests_in_dev_allow() {
        let engine = PolicyEngine::default_engine();
        let result = engine.evaluate(
            &make_proposal("run_tests", Some(0.9), Some(100)),
            &dev_ctx(),
        );
        assert!(matches!(result.decision, PolicyDecision::Allow));
        assert_eq!(result.matched_rule.as_deref(), Some("tests-in-dev-auto"));
    }

    #[test]
    fn tests_in_prod_require_peer() {
        let engine = PolicyEngine::default_engine();
        let result = engine.evaluate(
            &make_proposal("run_tests", Some(0.9), Some(100)),
            &prod_ctx(),
        );
        assert!(matches!(
            result.decision,
            PolicyDecision::RequireApproval {
                level: ApprovalLevel::PeerApproval,
                ..
            }
        ));
    }

    #[test]
    fn modify_source_require_techlead() {
        let engine = PolicyEngine::default_engine();
        let result = engine.evaluate(
            &make_proposal("modify_source", Some(0.9), Some(100)),
            &dev_ctx(),
        );
        assert!(matches!(
            result.decision,
            PolicyDecision::RequireApproval {
                level: ApprovalLevel::TechLeadApproval,
                ..
            }
        ));
    }

    #[test]
    fn deploy_production_deny() {
        let engine = PolicyEngine::default_engine();
        let result = engine.evaluate(
            &make_proposal("deploy_production", Some(0.9), Some(100)),
            &dev_ctx(),
        );
        assert!(matches!(result.decision, PolicyDecision::Deny { .. }));
    }

    #[test]
    fn low_confidence_require_peer() {
        let engine = PolicyEngine::default_engine();
        let result = engine.evaluate(
            &make_proposal("run_tests", Some(0.4), Some(100)),
            &dev_ctx(),
        );
        assert!(matches!(
            result.decision,
            PolicyDecision::RequireApproval {
                level: ApprovalLevel::PeerApproval,
                ..
            }
        ));
    }

    #[test]
    fn default_engine_loads() {
        let engine = PolicyEngine::default_engine();
        assert!(!engine.rules.is_empty());
    }

    #[test]
    fn load_from_str_valid_toml() {
        let toml = r#"
[[policy]]
name = "test-rule"
command = "test_cmd"
decision = "Allow"
"#;
        let engine = PolicyEngine::load_from_str(toml);
        assert_eq!(engine.rules.len(), 1);
        assert_eq!(engine.rules[0].name, "test-rule");
    }

    #[test]
    fn load_from_str_invalid_toml_falls_back_to_default() {
        let engine = PolicyEngine::load_from_str("not valid toml [[]]");
        // Falls back to defaults when TOML is invalid
        assert!(!engine.rules.is_empty());
    }

    #[test]
    fn load_from_path_not_found_falls_back_to_default() {
        let engine =
            PolicyEngine::load_from_path(std::path::Path::new("/nonexistent/path.toml")).unwrap();
        assert!(!engine.rules.is_empty());
    }

    #[test]
    fn policy_engine_trait_object() {
        let engine: Box<dyn Policy> = Box::new(PolicyEngine::default_engine());
        let result = engine.evaluate(&make_proposal("run_tests", Some(0.9), None), &dev_ctx());
        assert!(result.decision.is_allow());
    }
}
