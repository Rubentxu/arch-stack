//! Audit module — immutable append-only audit trail + sync HITL approval queue.
//!
//! v1.0: JSONL file under `$XDG_DATA_HOME/archctl/audit.jsonl`, in-memory queue.
//! Defer async HITL persistence and in-graph audit to 2.0 per ADR-023.

pub use log::{ActionOutcome, AuditEntry, AuditLogger};
pub use queue::ApprovalQueue;

mod log;
mod queue;
