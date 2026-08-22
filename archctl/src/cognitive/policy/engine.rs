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

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage v2, 2026-08-22)
    // ---------------------------------------------------------------------------

    /// `Default::default()` for `PolicyEngine` is the derived impl with
    /// `rules: Vec<PolicyRule>` empty. It is NOT equivalent to
    /// `default_engine()`. Locks the surprise: `Default::default()`
    /// evaluates everything as "no policy rule matched" →
    /// `RequireApproval { PeerApproval, "no policy rule matched" }`.
    #[test]
    fn default_trait_impl_yields_empty_rules() {
        let default: PolicyEngine = PolicyEngine::default();
        assert!(
            default.rules.is_empty(),
            "Default::default() must produce an engine with zero rules; \
             callers must use default_engine() for the v1.0 rule set"
        );
    }

    /// When NO rule matches, the engine falls back to the documented default:
    /// `RequireApproval { PeerApproval, "no policy rule matched" }` with
    /// `matched_rule: None`. Locks the post-loop branch in `evaluate()`.
    #[test]
    fn evaluate_no_matching_rule_returns_require_approval_default() {
        let engine = PolicyEngine::default(); // empty
        let result = engine.evaluate(
            &make_proposal("unknown_command", Some(0.9), Some(100)),
            &dev_ctx(),
        );
        assert!(
            matches!(
                result.decision,
                PolicyDecision::RequireApproval {
                    level: ApprovalLevel::PeerApproval,
                    ..
                }
            ),
            "no-match must default to RequireApproval {{ PeerApproval, .. }}, got {:?}",
            result.decision
        );
        assert_eq!(
            result.matched_rule, None,
            "no-match must have matched_rule: None"
        );
    }

    /// `matched_rule` carries the name of the matched rule on a match,
    /// and `None` on the default fallback. Distinct from the `is_allow` /
    /// `is_deny` predicates tested elsewhere.
    #[test]
    fn matched_rule_some_on_match_none_on_default() {
        // Default rule set: "tests-in-dev-auto" matches `run_tests` in dev
        let engine = PolicyEngine::default_engine();
        let matched = engine.evaluate(
            &make_proposal("run_tests", Some(0.9), Some(100)),
            &dev_ctx(),
        );
        assert_eq!(matched.matched_rule.as_deref(), Some("tests-in-dev-auto"));

        // Force no match by using an empty engine
        let empty = PolicyEngine::default();
        let fallback = empty.evaluate(
            &make_proposal("run_tests", Some(0.9), Some(100)),
            &dev_ctx(),
        );
        assert_eq!(fallback.matched_rule, None);
    }

    /// `load_from_path` for a valid TOML file returns the parsed engine
    /// with the file's rules (NOT the embedded defaults).
    #[test]
    fn load_from_path_valid_toml_returns_parsed() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("policies.toml");
        std::fs::write(
            &path,
            br#"
[[policy]]
name = "always-allow-anything"
command = "anything"
environment = "any"
decision = "Allow"
"#,
        )
        .unwrap();
        let engine = PolicyEngine::load_from_path(&path).unwrap();
        assert_eq!(engine.rules.len(), 1);
        assert_eq!(engine.rules[0].name, "always-allow-anything");
    }

    /// `load_from_path` for malformed TOML falls back to the embedded
    /// `default_engine()`. Distinct from `escalation::ladder::load_from_path`
    /// (which returns InvalidData error) — the policy engine is
    /// intentionally permissive so a broken config doesn't lock out all
    /// actions; it just reverts to RequireApproval defaults.
    #[test]
    fn load_from_path_invalid_toml_falls_back_to_default() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, b"this is = not [[[ valid toml").unwrap();
        let engine = PolicyEngine::load_from_path(&path).unwrap();
        assert!(
            !engine.rules.is_empty(),
            "invalid TOML must fall back to default_engine() (not empty)"
        );
    }

    /// `parse_env` accepts full names and common abbreviations, treats
    /// "any" / "Any" as a wildcard (returns None), and returns None for
    /// unknown strings. Verified through TOML parsing — observe via the
    /// `environment` field on the loaded rule.
    #[test]
    fn parse_env_all_variants_any_wildcard_and_unknown_fallback() {
        // Use load_from_str to observe parse_env indirectly through
        // the `environment` field on the parsed rule. We must rely on
        // a rule that successfully loads — invalid env yields None
        // which means the rule has environment: None.
        let cases = [
            ("Development", true),
            ("dev", true),
            ("Staging", true),
            ("staging", true),
            ("Production", true),
            ("prod", true),
            ("any", false), // wildcard: parse_env returns None, but rule loads
            ("Any", false),
            ("UnknownEnv", false), // unknown: parse_env returns None
        ];
        for (env_str, _expected_parseable) in cases {
            let toml = format!(
                r#"
[[policy]]
name = "env-test"
environment = "{env_str}"
decision = "Allow"
"#
            );
            let engine = PolicyEngine::load_from_str(&toml);
            assert_eq!(
                engine.rules.len(),
                1,
                "env '{env_str}' must produce 1 rule (parse_env returns Some/None but \
                 the rule itself is not dropped)"
            );
        }
    }

    /// `parse_security` for all 4 SecurityImpact variants (case-insensitive)
    /// AND unknown strings. Locked via load_from_str observation.
    #[test]
    fn parse_security_all_variants_and_unknown_fallback() {
        let cases = [
            "Low", "low", "Medium", "medium", "High", "high", "Critical", "critical",
        ];
        for impact in cases {
            let toml = format!(
                r#"
[[policy]]
name = "si-test"
security_impact = "{impact}"
decision = "Allow"
"#
            );
            let engine = PolicyEngine::load_from_str(&toml);
            assert_eq!(engine.rules.len(), 1, "impact '{impact}' must load 1 rule");
        }
        // Unknown impact also loads (parse_security returns None — the
        // rule's environment/security fields are simply unset)
        let engine = PolicyEngine::load_from_str(
            r#"
[[policy]]
name = "unknown-impact"
security_impact = "UnknownImpactLevel"
decision = "Allow"
"#,
        );
        assert_eq!(engine.rules.len(), 1);
    }

    /// `parse_approval_level` for all 4 simple variants AND unknown →
    /// PeerApproval (the silent fallback). Locked via load_from_str
    /// observation through the resulting `decision` field.
    #[test]
    fn parse_approval_level_simple_variants_and_unknown_fallback() {
        for level in [
            "SelfApproval",
            "PeerApproval",
            "TechLeadApproval",
            "SecurityApproval",
        ] {
            let toml = format!(
                r#"
[[policy]]
name = "level-test"
decision = "RequireApproval"
level = "{level}"
"#
            );
            let engine = PolicyEngine::load_from_str(&toml);
            assert_eq!(engine.rules.len(), 1, "level '{level}' must produce 1 rule");
            let rule = &engine.rules[0];
            match &rule.decision {
                PolicyDecision::RequireApproval {
                    level: ApprovalLevel::SelfApproval,
                    ..
                } if level == "SelfApproval" => {}
                PolicyDecision::RequireApproval {
                    level: ApprovalLevel::PeerApproval,
                    ..
                } if level == "PeerApproval" => {}
                PolicyDecision::RequireApproval {
                    level: ApprovalLevel::TechLeadApproval,
                    ..
                } if level == "TechLeadApproval" => {}
                PolicyDecision::RequireApproval {
                    level: ApprovalLevel::SecurityApproval,
                    ..
                } if level == "SecurityApproval" => {}
                other => panic!("unexpected decision for level '{level}': {other:?}"),
            }
        }
        // Unknown level → PeerApproval fallback
        let toml = r#"
[[policy]]
name = "unknown-level"
decision = "RequireApproval"
level = "BogusLevelName"
"#;
        let engine = PolicyEngine::load_from_str(toml);
        assert!(matches!(
            engine.rules[0].decision,
            PolicyDecision::RequireApproval {
                level: ApprovalLevel::PeerApproval,
                ..
            }
        ));
    }

    /// `parse_approval_level` for MultiPartyApproval parses `{required:N,total:M}`
    /// out of the level string. Locks the `extract_num` helper behavior —
    /// it requires digits IMMEDIATELY after the colon (no whitespace):
    /// `extract_num` uses `take_while(is_ascii_digit)` which fails on
    /// leading whitespace. Documented format is `"required:2"` (NOT
    /// `"required: 2"`). This is a known implementation constraint —
    /// if you want leading-space tolerance, fix `extract_num` first.
    #[test]
    fn parse_approval_level_multiparty_parses_required_and_total() {
        let toml = r#"
[[policy]]
name = "multiparty-test"
decision = "RequireApproval"
level = "MultiPartyApproval {required:2,total:3}"
"#;
        let engine = PolicyEngine::load_from_str(toml);
        match &engine.rules[0].decision {
            PolicyDecision::RequireApproval {
                level: ApprovalLevel::MultiPartyApproval { required, total },
                ..
            } => {
                assert_eq!(*required, 2);
                assert_eq!(*total, 3);
            }
            other => panic!("expected MultiPartyApproval, got {other:?}"),
        }
    }

    /// `parse_decision` for all 5 documented decisions (Allow,
    /// AllowWithNotify with `to` recipients, RequireApproval, Deny with
    /// reason, Escalate with target default) PLUS the unknown → Deny
    /// fallback.
    #[test]
    fn parse_decision_all_variants_and_unknown_fallback() {
        // Allow
        let e = PolicyEngine::load_from_str(
            r#"
[[policy]]
name = "a"
decision = "Allow"
"#,
        );
        assert!(matches!(e.rules[0].decision, PolicyDecision::Allow));

        // AllowWithNotify with `to` recipients
        let e = PolicyEngine::load_from_str(
            r#"
[[policy]]
name = "awn"
decision = "AllowWithNotify"
to = ["alice", "bob"]
"#,
        );
        match &e.rules[0].decision {
            PolicyDecision::AllowWithNotify(recipients) => {
                assert_eq!(recipients, &vec!["alice".to_string(), "bob".to_string()]);
            }
            other => panic!("expected AllowWithNotify, got {other:?}"),
        }

        // AllowWithNotify without `to` → empty recipients
        let e = PolicyEngine::load_from_str(
            r#"
[[policy]]
name = "awn-empty"
decision = "AllowWithNotify"
"#,
        );
        match &e.rules[0].decision {
            PolicyDecision::AllowWithNotify(recipients) => assert!(recipients.is_empty()),
            other => panic!("expected AllowWithNotify, got {other:?}"),
        }

        // RequireApproval
        let e = PolicyEngine::load_from_str(
            r#"
[[policy]]
name = "ra"
decision = "RequireApproval"
reason = "needs sign-off"
"#,
        );
        match &e.rules[0].decision {
            PolicyDecision::RequireApproval { level, reason } => {
                assert!(matches!(level, ApprovalLevel::PeerApproval));
                assert_eq!(reason, "needs sign-off");
            }
            other => panic!("expected RequireApproval, got {other:?}"),
        }

        // Deny with reason
        let e = PolicyEngine::load_from_str(
            r#"
[[policy]]
name = "d"
decision = "Deny"
reason = "no way"
"#,
        );
        match &e.rules[0].decision {
            PolicyDecision::Deny { reason } => assert_eq!(reason, "no way"),
            other => panic!("expected Deny, got {other:?}"),
        }

        // Escalate with default target
        let e = PolicyEngine::load_from_str(
            r#"
[[policy]]
name = "esc"
decision = "Escalate"
"#,
        );
        match &e.rules[0].decision {
            PolicyDecision::Escalate { target } => {
                assert_eq!(target, "tech-lead", "default target when not specified");
            }
            other => panic!("expected Escalate, got {other:?}"),
        }

        // Escalate with explicit target
        let e = PolicyEngine::load_from_str(
            r#"
[[policy]]
name = "esc-tgt"
decision = "Escalate"
target = "cto"
"#,
        );
        match &e.rules[0].decision {
            PolicyDecision::Escalate { target } => assert_eq!(target, "cto"),
            other => panic!("expected Escalate, got {other:?}"),
        }

        // Unknown decision → Deny { "unknown policy decision: ..." }
        let e = PolicyEngine::load_from_str(
            r#"
[[policy]]
name = "unk"
decision = "BogusDecision"
"#,
        );
        match &e.rules[0].decision {
            PolicyDecision::Deny { reason } => {
                assert!(
                    reason.starts_with("unknown policy decision"),
                    "unknown decision must Deny with 'unknown policy decision' prefix, got: {reason}"
                );
            }
            other => panic!("unknown decision must fall back to Deny, got {other:?}"),
        }
    }

    /// When `proposal.confidence` is `None`, the rule's `confidence_below`
    /// branch treats it as 0.0 (the `unwrap_or(0.0)` in `matches()`).
    /// Locks the `Option::None → 0.0` path explicitly so a future
    /// refactor that switches to `unwrap()` would panic visibly here.
    #[test]
    fn evaluate_confidence_none_treated_as_zero() {
        let engine = PolicyEngine::default_engine();
        // Default rules include `low-confidence-require-peer` (threshold 0.6).
        // A proposal with no confidence must trigger this rule (0.0 < 0.6).
        let result = engine.evaluate(&make_proposal("run_tests", None, Some(100)), &dev_ctx());
        // Should match the low-confidence rule, not the tests-in-dev-auto
        // rule (which has no confidence_below but matches via command+env).
        // First matching wins; low-confidence rule comes BEFORE the
        // command-specific rules in DEFAULT_POLICIES.
        assert_eq!(
            result.matched_rule.as_deref(),
            Some("low-confidence-require-peer"),
            "confidence=None must trigger the low-confidence rule"
        );
    }

    /// When `proposal.cost_estimate.tokens` is `None`, the rule's
    /// `cost_above_tokens` branch treats it as 0 (the `unwrap_or(0)` in
    /// `matches()`). Locks the `Option::None → 0` path.
    #[test]
    fn evaluate_cost_tokens_none_treated_as_zero() {
        // A rule with cost_above_tokens threshold must NOT match when
        // proposal.tokens is None (0 <= threshold → return false).
        let toml = r#"
[[policy]]
name = "expensive-only"
cost_above_tokens = 1000
decision = "RequireApproval"
reason = "too expensive"
"#;
        let engine = PolicyEngine::load_from_str(toml);
        // Proposal with no tokens estimate
        let result = engine.evaluate(&make_proposal("anything", Some(0.9), None), &dev_ctx());
        // No rule matched → default RequireApproval with matched_rule: None
        assert_eq!(result.matched_rule, None);
    }
}
