//! JSONL append-only audit log.
//!
//! Each evaluated ActionProposal produces exactly one AuditEntry.
//! File lives at `$XDG_DATA_HOME/archctl/audit.jsonl`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use crate::xdg::resolve_xdg;

/// Outcome of an action after policy evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionOutcome {
    /// Policy allowed execution; action was performed.
    Executed,
    /// Policy denied execution.
    Denied,
    /// Awaiting human approval (HITL).
    PendingApproval,
    /// Required human review but was ultimately approved.
    Approved,
    /// Rejected by human reviewer.
    Rejected,
    /// Action failed during execution.
    Failed,
    /// Rollback was triggered.
    RolledBack,
    /// Action expired before being decided (TTL).
    Expired,
}

/// Single immutable entry in the audit log.
///
/// Written once, never modified or deleted (append-only).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEntry {
    /// When this entry was written.
    pub timestamp: DateTime<Utc>,
    /// Who requested this action.
    pub agent_id: String,
    /// Unique proposal identifier.
    pub proposal_id: String,
    /// Human-readable goal description.
    pub goal: String,
    /// What was decided by the policy engine.
    pub policy_decision: PolicyDecisionSummary,
    /// What actually happened.
    pub outcome: ActionOutcome,
    /// Evidence IDs emitted as a result.
    #[serde(default)]
    pub evidence_emitted: Vec<String>,
    /// Who approved this (if a human was involved).
    #[serde(default)]
    pub user_who_approved: Option<String>,
    /// Whether rollback was triggered.
    #[serde(default)]
    pub rollback_executed: bool,
    /// Environment where this action was evaluated.
    #[serde(default)]
    pub environment: Option<String>,
    /// Cost in tokens (if estimated).
    #[serde(default)]
    pub tokens: Option<u32>,
    /// Cost in US cents (if estimated).
    #[serde(default)]
    pub cost_cents: Option<u32>,
    /// Confidence score (0.0–1.0).
    #[serde(default)]
    pub confidence: Option<f32>,
}

/// Condensed policy decision for audit serialization.
///
/// We store a summary rather than the full PolicyDecision enum so the
/// audit log is stable across policy rule changes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecisionSummary {
    Allow,
    AllowWithNotify,
    RequireApproval,
    Deny,
    Escalate,
}

impl PolicyDecisionSummary {
    /// Human-readable reason field stored alongside the summary.
    pub fn reason(&self) -> &'static str {
        match self {
            PolicyDecisionSummary::Allow => "policy allowed",
            PolicyDecisionSummary::AllowWithNotify => "allowed with notification",
            PolicyDecisionSummary::RequireApproval => "requires human approval",
            PolicyDecisionSummary::Deny => "policy denied",
            PolicyDecisionSummary::Escalate => "escalated to higher authority",
        }
    }
}

/// JSONL append-only audit logger.
///
/// # File location
/// `$XDG_DATA_HOME/archctl/audit.jsonl` (created on first write).
///
/// # Format
/// One JSON object per line, no trailing comma, no enclosing array.
/// Each write is flushed to ensure durability.
///
/// # Concurrency
/// Not safe for concurrent writes from multiple processes.
/// Use process-level locking in front of this for multi-process scenarios.
#[derive(Debug)]
pub struct AuditLogger {
    path: PathBuf,
}

impl AuditLogger {
    /// Create a new logger targeting `$XDG_DATA_HOME/archctl/audit.jsonl`.
    pub fn new() -> Self {
        let layout = resolve_xdg();
        Self {
            path: layout.data.join("audit.jsonl"),
        }
    }

    /// Create a new logger with an explicit path (useful for tests).
    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    /// Path where the audit log is written.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Append an entry to the audit log.
    ///
    /// Creates the file and parent directories if they don't exist.
    /// Each entry is written as a single newline-delimited JSON object.
    ///
    /// # Errors
    /// Returns an error if the file cannot be opened or written.
    pub fn append(&self, entry: &AuditEntry) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string(entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(writer, "{}", json)?;
        writer.flush()?;
        Ok(())
    }

    /// Read all entries from the audit log.
    ///
    /// Returns entries in chronological order (oldest first).
    ///
    /// # Errors
    /// Returns an error if the file cannot be opened or a line is malformed.
    pub fn read_all(&self) -> std::io::Result<Vec<AuditEntry>> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let content = fs::read_to_string(&self.path)?;
        let mut entries = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let entry: AuditEntry = serde_json::from_str(trimmed)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            entries.push(entry);
        }
        Ok(entries)
    }

    /// Number of entries currently in the audit log.
    pub fn len(&self) -> std::io::Result<usize> {
        Ok(self.read_all()?.len())
    }

    /// Whether the audit log is empty.
    pub fn is_empty(&self) -> std::io::Result<bool> {
        Ok(self.len()? == 0)
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::TempDir;

    fn temp_logger() -> (AuditLogger, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("audit.jsonl");
        (AuditLogger::with_path(path), dir)
    }

    #[test]
    fn append_writes_valid_jsonl() {
        let (logger, _dir) = temp_logger();
        let entry = AuditEntry {
            timestamp: Utc::now(),
            agent_id: "test-agent".into(),
            proposal_id: "prop-001".into(),
            goal: "discover c4 components".into(),
            policy_decision: PolicyDecisionSummary::Allow,
            outcome: ActionOutcome::Executed,
            evidence_emitted: vec!["ev:1".into()],
            user_who_approved: None,
            rollback_executed: false,
            environment: Some("dev".into()),
            tokens: Some(500),
            cost_cents: Some(5),
            confidence: Some(0.85),
        };
        logger.append(&entry).unwrap();
        let mut file = std::fs::File::open(logger.path()).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        let parsed: AuditEntry = serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(parsed.agent_id, "test-agent");
        assert_eq!(parsed.proposal_id, "prop-001");
        assert_eq!(parsed.outcome, ActionOutcome::Executed);
    }

    #[test]
    fn read_all_returns_entries_in_order() {
        let (logger, _dir) = temp_logger();
        for i in 0..3 {
            let entry = AuditEntry {
                timestamp: Utc::now(),
                agent_id: format!("agent-{i}"),
                proposal_id: format!("prop-{i}"),
                goal: format!("goal-{i}"),
                policy_decision: PolicyDecisionSummary::Allow,
                outcome: ActionOutcome::Executed,
                evidence_emitted: vec![],
                user_who_approved: None,
                rollback_executed: false,
                environment: None,
                tokens: None,
                cost_cents: None,
                confidence: None,
            };
            logger.append(&entry).unwrap();
        }
        let entries = logger.read_all().unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].proposal_id, "prop-0");
        assert_eq!(entries[2].proposal_id, "prop-2");
    }

    #[test]
    fn is_empty_true_when_no_file() {
        let (logger, _dir) = temp_logger();
        assert!(logger.is_empty().unwrap());
    }

    #[test]
    fn append_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("sub").join("deep").join("audit.jsonl");
        let logger = AuditLogger::with_path(path);
        logger
            .append(&AuditEntry {
                timestamp: Utc::now(),
                agent_id: "a".into(),
                proposal_id: "p".into(),
                goal: "g".into(),
                policy_decision: PolicyDecisionSummary::Allow,
                outcome: ActionOutcome::Executed,
                evidence_emitted: vec![],
                user_who_approved: None,
                rollback_executed: false,
                environment: None,
                tokens: None,
                cost_cents: None,
                confidence: None,
            })
            .unwrap();
        assert!(logger.path().exists());
    }

    #[test]
    fn policy_decision_summary_reason() {
        assert_eq!(
            PolicyDecisionSummary::RequireApproval.reason(),
            "requires human approval"
        );
        assert_eq!(PolicyDecisionSummary::Deny.reason(), "policy denied");
    }

    #[test]
    fn action_outcome_serde() {
        let outcomes = [
            ActionOutcome::Executed,
            ActionOutcome::Denied,
            ActionOutcome::PendingApproval,
            ActionOutcome::Approved,
            ActionOutcome::Rejected,
            ActionOutcome::Failed,
            ActionOutcome::RolledBack,
            ActionOutcome::Expired,
        ];
        for o in outcomes {
            let json = serde_json::to_string(&o).unwrap();
            let back: ActionOutcome = serde_json::from_str(&json).unwrap();
            assert_eq!(o, back);
        }
    }
}
