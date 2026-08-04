//! Synchronous in-memory HITL approval queue.
//!
//! v1.0: in-memory only, deny-on-pending semantics.
//! Proposals requiring approval are held here until a human resolves them.
//! Persistence deferred to 2.0 per ADR-023.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A proposal awaiting human approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApproval {
    /// Proposal identifier (matches ActionProposal.id).
    pub proposal_id: String,
    /// Human-readable goal description.
    pub goal: String,
    /// Who requested this action.
    pub agent_id: String,
    /// Minimum approval level required.
    pub required_level: String,
    /// Why human approval is required.
    pub reason: String,
    /// When this was added to the queue.
    pub queued_at: DateTime<Utc>,
    /// Who resolved this (if resolved).
    pub resolved_by: Option<String>,
    /// When it was resolved (if resolved).
    pub resolved_at: Option<DateTime<Utc>>,
    /// Whether it was approved or rejected.
    pub resolution: Option<Resolution>,
}

impl PendingApproval {
    /// Create a new pending approval entry.
    pub fn new(
        proposal_id: impl Into<String>,
        goal: impl Into<String>,
        agent_id: impl Into<String>,
        required_level: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            goal: goal.into(),
            agent_id: agent_id.into(),
            required_level: required_level.into(),
            reason: reason.into(),
            queued_at: Utc::now(),
            resolved_by: None,
            resolved_at: None,
            resolution: None,
        }
    }

    /// Mark this approval as resolved.
    pub fn resolve(&mut self, by: impl Into<String>, approved: bool) {
        self.resolved_by = Some(by.into());
        self.resolved_at = Some(Utc::now());
        self.resolution = Some(if approved {
            Resolution::Approved
        } else {
            Resolution::Rejected
        });
    }

    /// Whether this approval has been resolved.
    pub fn is_resolved(&self) -> bool {
        self.resolution.is_some()
    }
}

/// Resolution of a pending approval.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    Approved,
    Rejected,
}

/// Synchronous in-memory approval queue.
///
/// # v1.0 semantics (deny-on-pending)
/// - `push(proposal)` → added to queue, returns `QueueResult::Queued`
/// - If already pending → returns `QueueResult::AlreadyQueued`
/// - `approve(id, user)` / `reject(id, user)` → resolves and removes
/// - `get(id)` → returns pending entry if present
/// - `is_pending(id)` → true if proposal is waiting for approval
///
/// # Persistence
/// In-memory only. Cleared on restart. Persisted queue is a 2.0 concern.
#[derive(Debug, Clone, Default)]
pub struct ApprovalQueue {
    /// Map from proposal_id → PendingApproval.
    entries: HashMap<String, PendingApproval>,
}

impl ApprovalQueue {
    /// Create a new empty approval queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a proposal to the approval queue.
    ///
    /// If already pending, returns `AlreadyQueued` without modifying state.
    pub fn push(&mut self, approval: PendingApproval) -> QueueResult {
        let id = approval.proposal_id.clone();
        if self.entries.contains_key(&id) {
            return QueueResult::AlreadyQueued;
        }
        self.entries.insert(id, approval);
        QueueResult::Queued
    }

    /// Check whether a proposal is currently pending approval.
    pub fn is_pending(&self, proposal_id: &str) -> bool {
        self.entries
            .get(proposal_id)
            .map(|e| !e.is_resolved())
            .unwrap_or(false)
    }

    /// Get a pending approval entry by proposal ID.
    pub fn get(&self, proposal_id: &str) -> Option<&PendingApproval> {
        let entry = self.entries.get(proposal_id)?;
        if entry.is_resolved() {
            None
        } else {
            Some(entry)
        }
    }

    /// Approve a pending proposal.
    ///
    /// Returns the resolved entry, or `None` if not found or already resolved.
    pub fn approve(
        &mut self,
        proposal_id: &str,
        user: impl Into<String>,
    ) -> Option<PendingApproval> {
        let entry = self.entries.get_mut(proposal_id)?;
        if entry.is_resolved() {
            return None;
        }
        entry.resolve(user, true);
        self.entries.remove(proposal_id)
    }

    /// Reject a pending proposal.
    ///
    /// Returns the resolved entry, or `None` if not found or already resolved.
    pub fn reject(
        &mut self,
        proposal_id: &str,
        user: impl Into<String>,
    ) -> Option<PendingApproval> {
        let entry = self.entries.get_mut(proposal_id)?;
        if entry.is_resolved() {
            return None;
        }
        entry.resolve(user, false);
        self.entries.remove(proposal_id)
    }

    /// All currently pending (unresolved) approvals.
    pub fn pending(&self) -> Vec<&PendingApproval> {
        self.entries.values().filter(|e| !e.is_resolved()).collect()
    }

    /// Number of pending approvals.
    pub fn len(&self) -> usize {
        self.entries.values().filter(|e| !e.is_resolved()).count()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all entries (used in tests).
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Result of a queue push operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueResult {
    /// Proposal was added to the queue.
    Queued,
    /// Proposal was already in the queue (dedup).
    AlreadyQueued,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_approval(id: &str) -> PendingApproval {
        PendingApproval::new(
            id,
            format!("goal-{id}"),
            "test-agent",
            "PeerApproval",
            "test reason",
        )
    }

    #[test]
    fn push_and_is_pending() {
        let mut queue = ApprovalQueue::new();
        assert!(!queue.is_pending("prop-1"));
        queue.push(make_approval("prop-1"));
        assert!(queue.is_pending("prop-1"));
        assert!(!queue.is_pending("prop-2"));
    }

    #[test]
    fn push_dedup() {
        let mut queue = ApprovalQueue::new();
        assert_eq!(queue.push(make_approval("prop-1")), QueueResult::Queued);
        assert_eq!(
            queue.push(make_approval("prop-1")),
            QueueResult::AlreadyQueued
        );
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn approve_removes_and_returns() {
        let mut queue = ApprovalQueue::new();
        queue.push(make_approval("prop-1"));
        let resolved = queue.approve("prop-1", "alice").unwrap();
        assert_eq!(resolved.resolution, Some(Resolution::Approved));
        assert!(!queue.is_pending("prop-1"));
    }

    #[test]
    fn reject_removes_and_returns() {
        let mut queue = ApprovalQueue::new();
        queue.push(make_approval("prop-2"));
        let resolved = queue.reject("prop-2", "bob").unwrap();
        assert_eq!(resolved.resolution, Some(Resolution::Rejected));
        assert!(!queue.is_pending("prop-2"));
    }

    #[test]
    fn get_returns_unresolved() {
        let mut queue = ApprovalQueue::new();
        queue.push(make_approval("prop-1"));
        assert!(queue.get("prop-1").is_some());
        queue.approve("prop-1", "alice");
        assert!(queue.get("prop-1").is_none());
    }

    #[test]
    fn pending_lists_only_unresolved() {
        let mut queue = ApprovalQueue::new();
        queue.push(make_approval("prop-1"));
        queue.push(make_approval("prop-2"));
        queue.approve("prop-1", "alice");
        let pending = queue.pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].proposal_id, "prop-2");
    }

    #[test]
    fn approve_none_if_not_found() {
        let mut queue = ApprovalQueue::new();
        assert!(queue.approve("nonexistent", "alice").is_none());
    }

    #[test]
    fn double_resolve_returns_none() {
        let mut queue = ApprovalQueue::new();
        queue.push(make_approval("prop-1"));
        queue.approve("prop-1", "alice");
        assert!(queue.reject("prop-1", "bob").is_none());
    }

    #[test]
    fn is_empty_after_clear() {
        let mut queue = ApprovalQueue::new();
        queue.push(make_approval("prop-1"));
        assert!(!queue.is_empty());
        queue.clear();
        assert!(queue.is_empty());
    }

    #[test]
    fn resolved_fields_are_set() {
        let mut queue = ApprovalQueue::new();
        queue.push(make_approval("prop-1"));
        let resolved = queue.approve("prop-1", "carol").unwrap();
        assert_eq!(resolved.resolved_by.as_deref(), Some("carol"));
        assert!(resolved.resolved_at.is_some());
        assert_eq!(resolved.resolution, Some(Resolution::Approved));
    }
}
