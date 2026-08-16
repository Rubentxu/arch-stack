//! Snapshot use cases — create, list, gc.
//!
//! These functions consume `SnapshotRepository` through `GraphStore` (ADR-044).
//! Write operations use `UnitOfWork::begin_transaction()` for atomicity (ADR-010).

use std::path::Path;

use crate::architecture::digest::extractor_set_digest;
use crate::architecture::errors::{SnapshotError, SnapshotGcReport};
use crate::identity::{RepositoryIdentity, resolve_repository_identity};
use crate::store::{GraphStore, LbugStore, Snapshot, SnapshotRepository, UnitOfWork};

/// Create a new architecture snapshot for the given Git working directory.
///
/// Idempotent: if a snapshot with the same
/// `(repo_identity, commit_hash, schema_version, extractor_digest)` tuple
/// already exists, returns the existing snapshot id without creating a duplicate.
///
/// Uses `UnitOfWork` for atomic writes and proper lbug lock acquisition.
///
/// Returns `(snapshot_id, sequence)`.
pub fn create(
    project_dir: &Path,
    cwd: &str,
    fs: &dyn crate::filesystem::Filesystem,
    kind: &str,
    schema_version: i64,
    label: Option<&str>,
    pinned: bool,
) -> Result<(String, i64), SnapshotError> {
    // Open the store (acquires flock — ADR-010)
    let mut store = LbugStore::open(project_dir)
        .map_err(|e| SnapshotError::Store(anyhow::anyhow!("LbugStore::open: {e}")))?;

    // Resolve stable repository identity
    let repo_identity: RepositoryIdentity = resolve_repository_identity(cwd, fs)?
        .ok_or_else(|| SnapshotError::NotGitRepository(cwd.to_string()))?;

    let extractor_digest = extractor_set_digest();

    // Build snapshot props
    let mut props = serde_json::Map::new();
    props.insert(
        "repo_identity".to_string(),
        serde_json::Value::String(repo_identity.repo_identity.clone()),
    );
    props.insert(
        "extractor_digest".to_string(),
        serde_json::Value::String(extractor_digest),
    );
    props.insert(
        "schema_version".to_string(),
        serde_json::Value::String(format!("{}.0.0", schema_version)),
    );
    props.insert(
        "schema_compatibility".to_string(),
        serde_json::Value::String("1.0".to_string()),
    );
    props.insert(
        "remote".to_string(),
        serde_json::Value::String(repo_identity.remote.clone()),
    );
    if let Some(l) = label {
        props.insert(
            "label".to_string(),
            serde_json::Value::String(l.to_string()),
        );
    }
    if pinned {
        props.insert("pinned".to_string(), serde_json::Value::Bool(true));
    }

    let snap = Snapshot {
        id: String::new(), // computed by create_snapshot
        sequence: 0,
        kind: kind.to_string(),
        commit_hash: repo_identity.first_commit.clone(),
        worktree_id: repo_identity.repo_identity.clone(),
        schema_version,
        created_at: String::new(),
        props,
    };

    let mut tx = UnitOfWork::begin_transaction(&mut store)
        .map_err(|e| SnapshotError::Store(anyhow::anyhow!("begin_transaction: {e}")))?;

    let id =
        SnapshotRepository::create_snapshot(tx.as_mut(), &snap).map_err(SnapshotError::Store)?;

    // Fetch the created row to get sequence
    let created =
        SnapshotRepository::get_snapshot(tx.as_mut(), &id).map_err(SnapshotError::Store)?;

    tx.commit()
        .map_err(|e| SnapshotError::Store(anyhow::anyhow!("commit: {e}")))?;

    Ok((id, created.sequence))
}

/// List all snapshots for the project, ordered by `created_at` descending.
pub fn list(store: &dyn GraphStore) -> Result<Vec<Snapshot>, SnapshotError> {
    SnapshotRepository::list_snapshots(store).map_err(SnapshotError::from)
}

/// Garbage-collect old snapshots, preserving pinned rows and the most recent `keep_last`.
///
/// Default `keep_last = 10`. Pinned rows are always preserved.
/// If `dry_run = true`, returns a report without deleting anything.
/// If `confirmed = false`, returns `Err(SnapshotError::GcRequiresConfirmation)`.
pub fn gc(
    project_dir: &Path,
    keep_last: usize,
    dry_run: bool,
    confirmed: bool,
) -> Result<SnapshotGcReport, SnapshotError> {
    let mut store = LbugStore::open(project_dir)
        .map_err(|e| SnapshotError::Store(anyhow::anyhow!("LbugStore::open: {e}")))?;

    let all = SnapshotRepository::list_snapshots(&store).map_err(SnapshotError::from)?;

    // Separate pinned from unpinned
    let pinned: Vec<_> = all
        .iter()
        .filter(|s| {
            s.props
                .get("pinned")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .collect();

    let unpinned: Vec<_> = all
        .iter()
        .filter(|s| {
            !s.props
                .get("pinned")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .collect();

    // Keep last `keep_last` unpinned by created_at (already sorted DESC)
    let keep_count = keep_last.min(unpinned.len());
    let to_keep: Vec<_> = pinned
        .iter()
        .chain(unpinned.iter().take(keep_count))
        .collect();
    let to_delete: Vec<_> = if keep_count < unpinned.len() {
        unpinned[keep_count..].iter()
    } else {
        [].iter()
    }
    .collect::<Vec<_>>();

    let deleted_ids: Vec<String> = to_delete.iter().map(|s| s.id.clone()).collect();
    let preserved_ids: Vec<String> = to_keep.iter().map(|s| s.id.clone()).collect();

    if !deleted_ids.is_empty() && !dry_run {
        if !confirmed {
            return Err(SnapshotError::GcRequiresConfirmation {
                count: deleted_ids.len(),
            });
        }

        let mut tx = UnitOfWork::begin_transaction(&mut store)
            .map_err(|e| SnapshotError::Store(anyhow::anyhow!("begin_transaction: {e}")))?;

        SnapshotRepository::delete_snapshots(tx.as_mut(), &deleted_ids)
            .map_err(SnapshotError::Store)?;

        tx.commit()
            .map_err(|e| SnapshotError::Store(anyhow::anyhow!("commit: {e}")))?;
    }

    Ok(SnapshotGcReport {
        deleted: deleted_ids,
        preserved: preserved_ids,
        dry_run,
    })
}
