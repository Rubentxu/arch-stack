//! Snapshot domain errors.

use thiserror::Error;

/// Errors specific to snapshot operations.
#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error("not a Git repository: {0}")]
    NotGitRepository(String),

    #[error("snapshot not found: {0}")]
    NotFound(String),

    #[error("graph store error: {0}")]
    Store(#[from] anyhow::Error),

    #[error("snapshot create failed: {0}")]
    CreateFailed(String),

    #[error("idempotency conflict during concurrent create")]
    ConcurrentCreateConflict,

    #[error("GC would delete {count} snapshots; use --yes to confirm")]
    GcRequiresConfirmation { count: usize },
}

/// GC outcome report.
#[derive(Debug, Clone)]
pub struct SnapshotGcReport {
    /// Snapshot ids that would be (or were) deleted.
    pub deleted: Vec<String>,
    /// Snapshot ids that were preserved.
    pub preserved: Vec<String>,
    /// True if this was a dry run.
    pub dry_run: bool,
}
