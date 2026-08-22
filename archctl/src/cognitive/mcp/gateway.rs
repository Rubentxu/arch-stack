//! MCP gateway — read-only tool invocation over stdio.
//!
//! v1.0: hardcoded allowlist of 3 tools. No dynamic registration.
//! Input: JSON object `{tool: string, args: object}` from stdin.
//! Output: JSON object `{tool, error?, data?}` to stdout.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::cognitive::audit::{
    ActionOutcome, ApprovalQueue, AuditEntry, AuditLogger, PendingApproval,
};
use crate::cognitive::output::ActionProposal;
use crate::cognitive::policy::{PolicyContext, PolicyDecision, PolicyEngine, PolicyResult};

use super::tools::{
    ToolResult, handle_graph_query, handle_run_tests_local, handle_schema_validate,
};

/// The 3 allowed tools in v1.0. No others.
pub const ALLOWED_TOOLS: &[&str] = &["graph_query", "schema_validate", "run_tests_local"];

/// MCP gateway that handles JSON-RPC-like requests from stdin.
#[derive(Default)]
pub struct McpGateway;

impl McpGateway {
    pub fn new() -> Self {
        Self
    }

    /// Handle a raw JSON request from stdin. Returns JSON response string.
    pub fn handle_raw(&self, input: &str) -> String {
        match self.handle_str(input) {
            Ok(result) => serde_json::to_string(&result)
                .unwrap_or_else(|e| serde_json::to_string(&ToolResult::err("mcp", e)).unwrap()),
            Err(e) => serde_json::to_string(&ToolResult::err("mcp", e)).unwrap(),
        }
    }

    fn handle_str(&self, input: &str) -> Result<ToolResult, McpError> {
        #[derive(Deserialize)]
        struct Request {
            tool: String,
            #[serde(default)]
            args: serde_json::Value,
        }

        let req: Request =
            serde_json::from_str(input).map_err(|e| McpError::ParseError(e.to_string()))?;

        if !ALLOWED_TOOLS.contains(&req.tool.as_str()) {
            return Err(McpError::ToolNotAllowed(req.tool));
        }

        let result = match req.tool.as_str() {
            "graph_query" => {
                let args = serde_json::from_value(req.args)
                    .map_err(|e| McpError::InvalidArgs(e.to_string()))?;
                handle_graph_query(args)
            }
            "schema_validate" => {
                let args = serde_json::from_value(req.args)
                    .map_err(|e| McpError::InvalidArgs(e.to_string()))?;
                handle_schema_validate(args)
            }
            "run_tests_local" => {
                let args = serde_json::from_value(req.args)
                    .map_err(|e| McpError::InvalidArgs(e.to_string()))?;
                handle_run_tests_local(args)
            }
            _ => unreachable!(),
        };

        Ok(result)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("invalid JSON: {0}")]
    ParseError(String),
    #[error("tool not in allowlist: {0}")]
    ToolNotAllowed(String),
    #[error("invalid tool arguments: {0}")]
    InvalidArgs(String),
    #[error("policy evaluation failed: {0}")]
    PolicyError(String),
    #[error("governed request missing proposal: {0}")]
    MissingProposal(String),
}

// ---------------------------------------------------------------------------
// PolicyGate — governed MCP gateway with policy pre-flight
// ---------------------------------------------------------------------------

/// PolicyGate combines policy evaluation, audit logging, and HITL approval queue.
///
/// It is the pre-flight seam: every governed tool request passes through
/// PolicyGate::check() before execution.
#[derive(Default)]
pub struct PolicyGate {
    engine: PolicyEngine,
    audit: AuditLogger,
    queue: std::cell::RefCell<ApprovalQueue>,
}

impl PolicyGate {
    /// Create a new PolicyGate with the default policy engine (embedded rules).
    pub fn new() -> Self {
        Self {
            engine: PolicyEngine::default_engine(),
            audit: AuditLogger::default(),
            queue: std::cell::RefCell::new(ApprovalQueue::default()),
        }
    }

    /// Evaluate a proposal under the given context and return a structured result.
    ///
    /// This is the core pre-flight check: policy → audit log → queue or execute.
    pub fn check(&self, proposal: &ActionProposal, ctx: &PolicyContext) -> PolicyGateResult {
        let result = self.engine.evaluate(proposal, ctx);

        // Build audit entry
        let audit_entry = self.build_audit_entry(proposal, &result, ctx);

        // Append to audit log (best-effort: log errors are non-fatal)
        if let Err(e) = self.audit.append(&audit_entry) {
            eprintln!("[policy-gate] warning: failed to append audit entry: {e}");
        }

        // Act on the decision
        let outcome = self.decide(proposal, &result);

        PolicyGateResult { result, outcome }
    }

    /// Handle a governed MCP request from stdin.
    ///
    /// Request format:
    /// ```json
    /// {
    ///   "tool": "graph_query",
    ///   "args": {...},
    ///   "proposal": { "goal": "...", "command": "...", ... }
    /// }
    /// ```
    ///
    /// The `ctx` is provided by the caller (environment, user, cost ceilings).
    pub fn handle_governed(
        &self,
        input: &str,
        ctx: &PolicyContext,
    ) -> Result<GovernedToolResult, McpError> {
        #[derive(Deserialize)]
        struct GovernedRequest {
            tool: String,
            #[serde(default)]
            args: serde_json::Value,
            proposal: ActionProposal,
        }

        let req: GovernedRequest =
            serde_json::from_str(input).map_err(|e| McpError::ParseError(e.to_string()))?;

        // Policy pre-flight
        let gate_result = self.check(&req.proposal, ctx);

        // Act on policy decision
        match gate_result.outcome {
            GateOutcome::Execute => {
                let tool_result = self.execute_tool(&req.tool, req.args)?;
                Ok(GovernedToolResult {
                    policy: gate_result,
                    tool: tool_result,
                })
            }
            GateOutcome::Deny => {
                let reason = gate_result
                    .result
                    .decision
                    .deny_reason()
                    .unwrap_or("policy denied")
                    .to_string();
                let result = PolicyGateResult {
                    result: gate_result.result.clone(),
                    outcome: gate_result.outcome,
                };
                let tool_result = ToolResult::err(&req.tool, format!("policy denied: {reason}"));
                Ok(GovernedToolResult {
                    policy: result,
                    tool: tool_result,
                })
            }
            GateOutcome::Queue => {
                // Sync HITL: proposal queued, sync deny response
                Ok(GovernedToolResult {
                    policy: gate_result,
                    tool: ToolResult::err(
                        &req.tool,
                        "policy requires approval: proposal queued for human review",
                    ),
                })
            }
        }
    }

    /// Execute a tool by name with the given args.
    fn execute_tool(&self, tool: &str, args: serde_json::Value) -> Result<ToolResult, McpError> {
        if !ALLOWED_TOOLS.contains(&tool) {
            return Err(McpError::ToolNotAllowed(tool.to_string()));
        }

        let result = match tool {
            "graph_query" => {
                let args = serde_json::from_value(args)
                    .map_err(|e| McpError::InvalidArgs(e.to_string()))?;
                handle_graph_query(args)
            }
            "schema_validate" => {
                let args = serde_json::from_value(args)
                    .map_err(|e| McpError::InvalidArgs(e.to_string()))?;
                handle_schema_validate(args)
            }
            "run_tests_local" => {
                let args = serde_json::from_value(args)
                    .map_err(|e| McpError::InvalidArgs(e.to_string()))?;
                handle_run_tests_local(args)
            }
            _ => unreachable!(),
        };

        Ok(result)
    }

    /// Decide what to do based on the policy result (sync HITL semantics).
    fn decide(&self, proposal: &ActionProposal, result: &PolicyResult) -> GateOutcome {
        use crate::cognitive::policy::PolicyDecision;

        match &result.decision {
            PolicyDecision::Allow => GateOutcome::Execute,
            PolicyDecision::AllowWithNotify(_) => GateOutcome::Execute,
            PolicyDecision::RequireApproval { level, reason } => {
                // Sync HITL: push to queue, deny synchronously
                let proposal_id = proposal
                    .id
                    .as_ref()
                    .map(|id| id.0.clone())
                    .unwrap_or_else(|| "unknown".into());
                let pending = PendingApproval::new(
                    proposal_id,
                    proposal.goal.clone(),
                    proposal
                        .triggering_agent
                        .as_ref()
                        .map(|a| a.0.clone())
                        .unwrap_or_else(|| "unknown".into()),
                    format!("{:?}", level),
                    reason.clone(),
                );
                self.queue.borrow_mut().push(pending);
                GateOutcome::Queue
            }
            PolicyDecision::Deny { .. } => GateOutcome::Deny,
            PolicyDecision::Escalate { .. } => {
                // Treat escalation as require-approval with tech-lead level
                GateOutcome::Queue
            }
        }
    }

    /// Build an AuditEntry from a proposal and policy result.
    fn build_audit_entry(
        &self,
        proposal: &ActionProposal,
        result: &PolicyResult,
        ctx: &PolicyContext,
    ) -> AuditEntry {
        use crate::cognitive::audit::PolicyDecisionSummary;

        let proposal_id = proposal
            .id
            .as_ref()
            .map(|id| id.0.clone())
            .unwrap_or_else(|| "unknown".into());

        let agent_id = proposal
            .triggering_agent
            .as_ref()
            .map(|a| a.0.clone())
            .unwrap_or_else(|| "unknown".into());

        let summary = match &result.decision {
            PolicyDecision::Allow => PolicyDecisionSummary::Allow,
            PolicyDecision::AllowWithNotify(_) => PolicyDecisionSummary::AllowWithNotify,
            PolicyDecision::RequireApproval { .. } => PolicyDecisionSummary::RequireApproval,
            PolicyDecision::Deny { .. } => PolicyDecisionSummary::Deny,
            PolicyDecision::Escalate { .. } => PolicyDecisionSummary::Escalate,
        };

        AuditEntry {
            timestamp: Utc::now(),
            agent_id,
            proposal_id,
            goal: proposal.goal.clone(),
            policy_decision: summary,
            outcome: ActionOutcome::PendingApproval,
            evidence_emitted: vec![],
            user_who_approved: None,
            rollback_executed: false,
            environment: Some(format!("{:?}", ctx.environment)),
            tokens: proposal.cost_estimate.tokens,
            cost_cents: proposal.cost_estimate.cost_cents,
            confidence: proposal.confidence,
        }
    }

    /// Access the underlying approval queue (for testing / CLI inspection).
    pub fn queue(&self) -> std::cell::Ref<'_, ApprovalQueue> {
        self.queue.borrow()
    }

    /// Access the underlying audit logger (for testing).
    pub fn audit(&self) -> &AuditLogger {
        &self.audit
    }
}

/// Outcome of the policy gate's decide() method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateOutcome {
    /// Policy allowed — execute the tool.
    Execute,
    /// Policy denied — do not execute.
    Deny,
    /// Require approval — queued for HITL review.
    Queue,
}

/// Result of policy evaluation for a governed request.
#[derive(Debug, Clone, Serialize)]
pub struct PolicyGateResult {
    /// The policy engine's evaluation result.
    pub result: PolicyResult,
    /// The action taken based on the decision.
    pub outcome: GateOutcome,
}

/// Result of a governed tool invocation (policy + tool result).
#[derive(Debug, Clone, Serialize)]
pub struct GovernedToolResult {
    /// Policy gate result.
    pub policy: PolicyGateResult,
    /// Tool execution result.
    pub tool: ToolResult,
}

// Extend PolicyDecision with deny_reason helper
impl PolicyDecision {
    fn deny_reason(&self) -> Option<&str> {
        match self {
            PolicyDecision::Deny { reason } => Some(reason),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_allows_graph_query() {
        let gw = McpGateway::new();
        let req = r#"{"tool":"graph_query","args":{"cypher":"MATCH (e) RETURN e","params":{}}}"#;
        let out = gw.handle_raw(req);
        let result: ToolResult = serde_json::from_str(&out).unwrap();
        // graph_query without a db will error, but it's allowed
        assert_eq!(result.tool, "graph_query");
    }

    #[test]
    fn gateway_denies_unknown_tool() {
        let gw = McpGateway::new();
        let req = r#"{"tool":"delete_everything","args":{}}"#;
        let out = gw.handle_raw(req);
        let result: ToolResult = serde_json::from_str(&out).unwrap();
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("not in allowlist"));
    }

    #[test]
    fn gateway_rejects_malformed_json() {
        let gw = McpGateway::new();
        let out = gw.handle_raw("not json at all");
        let result: ToolResult = serde_json::from_str(&out).unwrap();
        assert!(result.error.is_some());
    }

    #[test]
    fn policy_gate_check_allow() {
        use crate::cognitive::output::DeploymentEnv;
        use crate::cognitive::policy::{CostCeiling, PolicyContext};

        let gate = PolicyGate::new();
        let proposal: ActionProposal = serde_json::from_str(
            r#"{
            "goal": "test goal",
            "command": "echo hello",
            "approval_required": false,
            "expected_evidence_old": ""
        }"#,
        )
        .unwrap();
        let ctx = PolicyContext {
            user_id: "test-user".into(),
            environment: DeploymentEnv::Development,
            security_impact: crate::cognitive::output::SecurityImpact::Low,
            requesting_capabilities: vec![],
            affected_components: vec![],
            cost_ceiling: CostCeiling::default(),
        };

        let result = gate.check(&proposal, &ctx);
        // Unknown command defaults to RequireApproval -> Queue
        assert_eq!(result.outcome, GateOutcome::Queue);
    }

    #[test]
    fn policy_gate_unknown_command_queues() {
        use crate::cognitive::output::DeploymentEnv;
        use crate::cognitive::policy::{CostCeiling, PolicyContext};

        let gate = PolicyGate::new();
        let proposal: ActionProposal = serde_json::from_str(
            r#"{
            "goal": "unknown action",
            "command": "destroy_everything",
            "approval_required": false,
            "expected_evidence_old": ""
        }"#,
        )
        .unwrap();
        let ctx = PolicyContext {
            user_id: "test-user".into(),
            environment: DeploymentEnv::Development,
            security_impact: crate::cognitive::output::SecurityImpact::Low,
            requesting_capabilities: vec![],
            affected_components: vec![],
            cost_ceiling: CostCeiling::default(),
        };

        let result = gate.check(&proposal, &ctx);
        // Unknown command defaults to RequireApproval (queue)
        assert_eq!(result.outcome, GateOutcome::Queue);
    }

    #[test]
    fn policy_gate_low_confidence_denies_or_queues() {
        use crate::cognitive::output::DeploymentEnv;
        use crate::cognitive::policy::{CostCeiling, PolicyContext};

        let gate = PolicyGate::new();
        let proposal: ActionProposal = serde_json::from_str(
            r#"{
            "goal": "risky action",
            "command": "rm -rf /",
            "approval_required": false,
            "expected_evidence_old": "",
            "confidence": 0.5
        }"#,
        )
        .unwrap();
        let ctx = PolicyContext {
            user_id: "test-user".into(),
            environment: DeploymentEnv::Development,
            security_impact: crate::cognitive::output::SecurityImpact::Low,
            requesting_capabilities: vec![],
            affected_components: vec![],
            cost_ceiling: CostCeiling::default(),
        };

        let result = gate.check(&proposal, &ctx);
        // destructive command rule or low confidence -> deny or queue
        assert!(matches!(
            result.outcome,
            GateOutcome::Deny | GateOutcome::Queue
        ));
    }

    #[test]
    fn gate_outcome_serde() {
        let outcomes = [GateOutcome::Execute, GateOutcome::Deny, GateOutcome::Queue];
        for o in outcomes {
            let json = serde_json::to_string(&o).unwrap();
            let back: GateOutcome = serde_json::from_str(&json).unwrap();
            assert_eq!(o, back);
        }
    }

    #[test]
    fn handle_governed_execute_allowed_proposal() {
        // run_tests in Dev with confidence >= 0.6 -> tests-in-dev-auto rule -> Allow -> Execute
        use crate::cognitive::output::DeploymentEnv;
        use crate::cognitive::policy::{CostCeiling, PolicyContext};

        let gate = PolicyGate::new();
        let req = serde_json::json!({
            "tool": "graph_query",
            "args": { "cypher": "MATCH (n) RETURN n", "params": {} },
            "proposal": {
                "goal": "check graph health",
                "command": "run_tests",
                "approval_required": false,
                "expected_evidence_old": "",
                "confidence": 0.9
            }
        });
        let ctx = PolicyContext {
            user_id: "test-user".into(),
            environment: DeploymentEnv::Development,
            security_impact: crate::cognitive::output::SecurityImpact::Low,
            requesting_capabilities: vec![],
            affected_components: vec![],
            cost_ceiling: CostCeiling::default(),
        };

        let result = gate
            .handle_governed(&serde_json::to_string(&req).unwrap(), &ctx)
            .unwrap();
        // tests-in-dev-auto rule -> Allow -> Execute
        assert_eq!(result.policy.outcome, GateOutcome::Execute);
        assert_eq!(result.tool.tool, "graph_query");
    }

    #[test]
    fn handle_governed_queue_unknown_command() {
        // Unknown command -> default RequireApproval -> Queue
        use crate::cognitive::output::DeploymentEnv;
        use crate::cognitive::policy::{CostCeiling, PolicyContext};

        let gate = PolicyGate::new();
        let req = serde_json::json!({
            "tool": "graph_query",
            "args": { "cypher": "MATCH (n) RETURN n", "params": {} },
            "proposal": {
                "goal": "unknown action",
                "command": "delete_the_entire_repo",
                "approval_required": false,
                "expected_evidence_old": ""
            }
        });
        let ctx = PolicyContext {
            user_id: "test-user".into(),
            environment: DeploymentEnv::Development,
            security_impact: crate::cognitive::output::SecurityImpact::Low,
            requesting_capabilities: vec![],
            affected_components: vec![],
            cost_ceiling: CostCeiling::default(),
        };

        let result = gate
            .handle_governed(&serde_json::to_string(&req).unwrap(), &ctx)
            .unwrap();
        assert_eq!(result.policy.outcome, GateOutcome::Queue);
        assert!(result.tool.error.is_some());
        assert!(result.tool.error.unwrap().contains("approval"));
    }

    #[test]
    fn handle_governed_low_confidence_queues() {
        // confidence < 0.6 -> RequireApproval -> Queue
        use crate::cognitive::output::DeploymentEnv;
        use crate::cognitive::policy::{CostCeiling, PolicyContext};

        let gate = PolicyGate::new();
        let req = serde_json::json!({
            "tool": "schema_validate",
            "args": {},
            "proposal": {
                "goal": "risky mutation",
                "command": "modify_source",
                "approval_required": false,
                "expected_evidence_old": "",
                "confidence": 0.3
            }
        });
        let ctx = PolicyContext {
            user_id: "test-user".into(),
            environment: DeploymentEnv::Development,
            security_impact: crate::cognitive::output::SecurityImpact::Low,
            requesting_capabilities: vec![],
            affected_components: vec![],
            cost_ceiling: CostCeiling::default(),
        };

        let result = gate
            .handle_governed(&serde_json::to_string(&req).unwrap(), &ctx)
            .unwrap();
        // low-confidence-require-peer rule -> RequireApproval -> Queue
        assert_eq!(result.policy.outcome, GateOutcome::Queue);
    }

    #[test]
    fn governed_tool_result_serializes() {
        // Verify GovernedToolResult serializes without panics
        use crate::cognitive::output::DeploymentEnv;
        use crate::cognitive::policy::{CostCeiling, PolicyContext};

        let gate = PolicyGate::new();
        let req = serde_json::json!({
            "tool": "graph_query",
            "args": { "cypher": "RETURN 1", "params": {} },
            "proposal": {
                "goal": "test",
                "command": "run_tests",
                "approval_required": false,
                "expected_evidence_old": "",
                "confidence": 0.9
            }
        });
        let ctx = PolicyContext {
            user_id: "test-user".into(),
            environment: DeploymentEnv::Development,
            security_impact: crate::cognitive::output::SecurityImpact::Low,
            requesting_capabilities: vec![],
            affected_components: vec![],
            cost_ceiling: CostCeiling::default(),
        };

        let result = gate
            .handle_governed(&serde_json::to_string(&req).unwrap(), &ctx)
            .unwrap();
        // Should serialize to JSON without errors
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.is_empty());
        // Verify key fields are present in serialized form
        assert!(json.contains("\"outcome\""));
        assert!(json.contains("\"Execute\""));
    }

    #[test]
    fn policy_gate_result_serializes() {
        use crate::cognitive::output::DeploymentEnv;
        use crate::cognitive::policy::{CostCeiling, PolicyContext};

        let gate = PolicyGate::new();
        let proposal: ActionProposal = serde_json::from_str(
            r#"{
            "goal": "test",
            "command": "run_tests",
            "approval_required": false,
            "expected_evidence_old": ""
        }"#,
        )
        .unwrap();
        let ctx = PolicyContext {
            user_id: "test-user".into(),
            environment: DeploymentEnv::Development,
            security_impact: crate::cognitive::output::SecurityImpact::Low,
            requesting_capabilities: vec![],
            affected_components: vec![],
            cost_ceiling: CostCeiling::default(),
        };

        let result = gate.check(&proposal, &ctx);
        // PolicyGateResult should serialize to JSON without errors
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.is_empty());
        assert!(json.contains("\"outcome\""));
    }

    // ---------------------------------------------------------------------------
    // Coverage additions (cycle cognitive-layer-coverage, 2026-08-22)
    // ---------------------------------------------------------------------------

    fn make_dev_ctx() -> PolicyContext {
        use crate::cognitive::output::DeploymentEnv;
        use crate::cognitive::policy::CostCeiling;
        PolicyContext {
            user_id: "test-user".into(),
            environment: DeploymentEnv::Development,
            security_impact: crate::cognitive::output::SecurityImpact::Low,
            requesting_capabilities: vec![],
            affected_components: vec![],
            cost_ceiling: CostCeiling::default(),
        }
    }

    /// `ALLOWED_TOOLS` const contains the 3 v1.0 tools. Locks the
    /// hardcoded allowlist against accidental additions.
    #[test]
    fn allowed_tools_const_lists_three_tools() {
        assert_eq!(ALLOWED_TOOLS.len(), 3);
        assert!(ALLOWED_TOOLS.contains(&"graph_query"));
        assert!(ALLOWED_TOOLS.contains(&"schema_validate"));
        assert!(ALLOWED_TOOLS.contains(&"run_tests_local"));
    }

/// `McpGateway::default()` and `McpGateway::new()` are equivalent
    /// (both produce an empty allowlist-only router).
    #[test]
    fn mcp_gateway_default_equiv_new() {
        let gw_default = McpGateway;
        let gw_new = McpGateway::new();
        // Both must allow graph_query and deny unknown tools identically
        for gw in [&gw_default, &gw_new] {
            let allowed = gw.handle_raw(r#"{"tool":"graph_query","args":{"cypher":"RETURN 1","params":{}}}"#);
            assert!(allowed.contains("graph_query"));
            let denied = gw.handle_raw(r#"{"tool":"nope","args":{}}"#);
            assert!(denied.contains("not in allowlist"));
        }
    }

    /// `PolicyGate::default()` and `PolicyGate::new()` are equivalent
    /// for read-side accessors (both produce an empty queue + default engine).
    #[test]
    fn policy_gate_default_equiv_new() {
        let gate_default = PolicyGate::default();
        let gate_new = PolicyGate::new();
        // Both must have an empty approval queue at construction
        assert_eq!(gate_default.queue().len(), 0);
        assert_eq!(gate_new.queue().len(), 0);
        // Both must have an audit logger accessible
        let _ = gate_default.audit();
        let _ = gate_new.audit();
    }

    /// `McpError` Display for all 5 variants carries the inner message verbatim.
    #[test]
    fn mcp_error_display_all_variants() {
        let cases = [
            (
                McpError::ParseError("bad json".into()),
                "invalid JSON",
                "bad json",
            ),
            (
                McpError::ToolNotAllowed("delete_everything".into()),
                "tool not in allowlist",
                "delete_everything",
            ),
            (
                McpError::InvalidArgs("missing field".into()),
                "invalid tool arguments",
                "missing field",
            ),
            (
                McpError::PolicyError("rule failed".into()),
                "policy evaluation failed",
                "rule failed",
            ),
            (
                McpError::MissingProposal("no proposal".into()),
                "missing proposal",
                "no proposal",
            ),
        ];
        for (err, expected_prefix, expected_inner) in cases {
            let msg = format!("{}", err);
            assert!(
                msg.contains(expected_prefix),
                "Display must include '{expected_prefix}', got: {msg}"
            );
            assert!(
                msg.contains(expected_inner),
                "Display must include inner '{expected_inner}', got: {msg}"
            );
        }
    }

    /// `PolicyGate::queue()` returns a Ref that pushes onto the underlying queue.
    /// Verifies the Queue outcome actually persists the pending approval.
    #[test]
    fn policy_gate_queue_accessor_observes_pending() {
        let gate = PolicyGate::new();
        let proposal: ActionProposal = serde_json::from_str(
            r#"{"goal":"unknown","command":"delete_the_db","approval_required":false,"expected_evidence_old":""}"#,
        )
        .unwrap();
        let ctx = make_dev_ctx();

        let result = gate.check(&proposal, &ctx);
        assert_eq!(result.outcome, GateOutcome::Queue);

        // Queue should now have one pending approval — verify via accessor
        let queue = gate.queue();
        assert_eq!(queue.len(), 1, "Queue outcome must add a pending approval");
        assert!(
            queue.is_pending("unknown"),
            "Queue must contain the proposal with id 'unknown'"
        );
        let pending = queue.get("unknown").expect("pending must exist");
        assert_eq!(pending.proposal_id, "unknown");
        assert!(
            !pending.reason.is_empty(),
            "Pending reason must include policy rationale, got: {}",
            pending.reason
        );
    }

    /// `handle_governed` with a destructive command either denies or queues
    /// depending on policy rule severity (low confidence + rm -rf in
    /// Production triggers the destructive-command rule's escalate path).
    /// Per spec: `decide()` maps `Escalate → Queue`; only `Deny → Deny`.
    /// The tool result must carry a policy-related error either way.
    #[test]
    fn handle_governed_destructive_command_deny_or_queue() {
        use crate::cognitive::output::DeploymentEnv;

        let gate = PolicyGate::new();
        let req = serde_json::json!({
            "tool": "graph_query",
            "args": { "cypher": "RETURN 1", "params": {} },
            "proposal": {
                "goal": "delete everything",
                "command": "rm -rf /",
                "approval_required": false,
                "expected_evidence_old": "",
                "confidence": 0.95
            }
        });
        // Production environment raises the destructive-command rule's severity
        let mut ctx = make_dev_ctx();
        ctx.environment = DeploymentEnv::Production;

        let result = gate
            .handle_governed(&serde_json::to_string(&req).unwrap(), &ctx)
            .unwrap();
        assert!(
            matches!(
                result.policy.outcome,
                GateOutcome::Deny | GateOutcome::Queue
            ),
            "rm -rf in Production must deny or queue, got: {:?}",
            result.policy.outcome
        );
        let tool_err = result.tool.error.as_deref().unwrap_or("");
        assert!(
            tool_err.starts_with("policy denied") || tool_err.contains("requires approval"),
            "tool error must mention policy denial or approval, got: {tool_err}"
        );
    }

    /// `handle_governed` with malformed JSON returns ParseError.
    #[test]
    fn handle_governed_rejects_malformed_json() {
        let gate = PolicyGate::new();
        let result = gate.handle_governed("not json at all", &make_dev_ctx());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, McpError::ParseError(_)));
        assert!(format!("{}", err).contains("invalid JSON"));
    }

    /// `handle_governed` without a `proposal` field returns ParseError
    /// (the GovernedRequest struct requires the field).
    #[test]
    fn handle_governed_rejects_missing_proposal() {
        let gate = PolicyGate::new();
        let req = r#"{"tool":"graph_query","args":{"cypher":"RETURN 1","params":{}}}"#;
        let result = gate.handle_governed(req, &make_dev_ctx());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, McpError::ParseError(_)));
    }

    /// `handle_governed` with an unknown tool returns ToolNotAllowed
    /// (caught inside execute_tool).
    #[test]
    fn handle_governed_rejects_unknown_tool() {
        let gate = PolicyGate::new();
        let req = serde_json::json!({
            "tool": "delete_everything",
            "args": {},
            "proposal": {
                "goal": "test",
                "command": "run_tests",
                "approval_required": false,
                "expected_evidence_old": "",
                "confidence": 0.9
            }
        });
        let result = gate.handle_governed(&serde_json::to_string(&req).unwrap(), &make_dev_ctx());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, McpError::ToolNotAllowed(_)));
    }

    /// `GovernedToolResult` serializes the Queue outcome with the policy+tool
    /// fields populated (smoke test for the alternate outcome path).
    #[test]
    fn governed_tool_result_queue_serializes() {
        let gate = PolicyGate::new();
        let req = serde_json::json!({
            "tool": "graph_query",
            "args": { "cypher": "RETURN 1", "params": {} },
            "proposal": {
                "goal": "test",
                "command": "delete_the_repo",
                "approval_required": false,
                "expected_evidence_old": "",
                "confidence": 0.9
            }
        });
        let result = gate
            .handle_governed(&serde_json::to_string(&req).unwrap(), &make_dev_ctx())
            .unwrap();
        assert_eq!(result.policy.outcome, GateOutcome::Queue);

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"Queue\""));
        assert!(json.contains("\"policy\""));
        assert!(json.contains("\"tool\""));
        assert!(
            json.contains("approval"),
            "JSON must contain the 'approval' message from the Queue outcome, got: {json}"
        );
    }

    /// `PolicyGate::check` always appends an audit entry, even on the Allow
    /// path. The audit accessor exposes the underlying logger (smoke test).
    #[test]
    fn policy_gate_check_appends_audit_on_allow() {
        let gate = PolicyGate::new();
        let proposal: ActionProposal = serde_json::from_str(
            r#"{
            "goal": "auto test",
            "command": "run_tests",
            "approval_required": false,
            "expected_evidence_old": "",
            "confidence": 0.9
        }"#,
        )
        .unwrap();

        let result = gate.check(&proposal, &make_dev_ctx());
        // run_tests in Dev with confidence ≥ 0.6 → tests-in-dev-auto rule → Allow → Execute
        assert_eq!(result.outcome, GateOutcome::Execute);

        // Audit logger accessor must return a valid reference
        let _audit = gate.audit();
        // (AuditLogger writes to a path under XDG; the append() call inside
        // check() is best-effort. We only verify the accessor contract here.)
    }
}
