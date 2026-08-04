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
    /// Create a new PolicyGate with the default policy engine.
    pub fn new() -> Self {
        Self::default()
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
}
