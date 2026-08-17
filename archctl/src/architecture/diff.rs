//! Architecture snapshot diff use case.
//!
//! Pure read-only comparison of two `Snapshot` carriers. No graph-store writes.

use serde::{Deserialize, Serialize};

/// Snapshot carrier used in the diff report (subset of full Snapshot fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotSummary {
    /// Canonical snapshot id.
    pub id: String,
    /// Git commit SHA.
    pub commit_hash: String,
    /// Schema version (major number).
    pub schema_version: i64,
    /// Extractor digest from props.
    pub extractor_digest: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// Label from props (empty string if absent).
    pub label: String,
    /// Whether this snapshot is pinned.
    pub pinned: bool,
    /// Repository identity from props.
    pub repo_identity: String,
}

impl SnapshotSummary {
    /// Build a summary from a full `Snapshot` carrier.
    fn from_snapshot(snap: &crate::store::Snapshot) -> Self {
        let extractor_digest = snap
            .props
            .get("extractor_digest")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let repo_identity = snap
            .props
            .get("repo_identity")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let label = snap
            .props
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let pinned = snap
            .props
            .get("pinned")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        Self {
            id: snap.id.clone(),
            commit_hash: snap.commit_hash.clone(),
            schema_version: snap.schema_version,
            extractor_digest,
            created_at: snap.created_at.clone(),
            label,
            pinned,
            repo_identity,
        }
    }
}

/// A single field-level delta between two snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDelta {
    /// The field name that differs.
    pub field: String,
    /// Value in the `before` snapshot.
    pub before: String,
    /// Value in the `after` snapshot.
    pub after: String,
}

/// Schema compatibility assessment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityBlock {
    /// Whether the two snapshots share the same schema version.
    pub schema: String,
    /// Human-readable reason (empty when schema is "same").
    pub reason: String,
}

/// The architecture-diff-report/1 carrier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureDiffReport {
    /// Schema version of this report format.
    pub schema_version: String,
    /// Capability that produced this report.
    pub capability: String,
    /// Before/after snapshot summaries.
    pub snapshots: SnapshotsBlock,
    /// Schema compatibility assessment.
    pub compatibility: CompatibilityBlock,
    /// List of field-level deltas.
    pub differences: Vec<FieldDelta>,
}

/// The before/after snapshot block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotsBlock {
    /// The earlier snapshot.
    pub before: SnapshotSummary,
    /// The later snapshot.
    pub after: SnapshotSummary,
}

/// Errors that can occur during a diff operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffError {
    /// The snapshot id contains invalid characters.
    InvalidIdentifier(String),
    /// No snapshot with the given id exists in the store.
    SnapshotNotFound(String),
}

impl std::fmt::Display for DiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiffError::InvalidIdentifier(id) => {
                write!(
                    f,
                    "invalid snapshot id (contains unsafe characters): {}",
                    id
                )
            }
            DiffError::SnapshotNotFound(id) => {
                write!(
                    f,
                    "snapshot not found: {} — run `archctl architecture list` to see available snapshots",
                    id
                )
            }
        }
    }
}

impl std::error::Error for DiffError {}

/// Compare all 7 field groups between two snapshots and emit a diff report.
///
/// Fields compared: `commit_hash`, `schema_version`, `extractor_digest`,
/// `repo_identity`, `label`, `pinned`, `created_at`.
///
/// Schema version comparison sets `compatibility.schema` to `"same"` or
/// `"different"` with reason `"schema version changed"`.
pub fn diff_snapshots(
    before: &crate::store::Snapshot,
    after: &crate::store::Snapshot,
) -> ArchitectureDiffReport {
    let mut differences = Vec::new();

    // commit_hash
    if before.commit_hash != after.commit_hash {
        differences.push(FieldDelta {
            field: "commitHash".to_string(),
            before: before.commit_hash.clone(),
            after: after.commit_hash.clone(),
        });
    }

    // schema_version
    if before.schema_version != after.schema_version {
        differences.push(FieldDelta {
            field: "schemaVersion".to_string(),
            before: before.schema_version.to_string(),
            after: after.schema_version.to_string(),
        });
    }

    // extractor_digest
    let before_ed = before
        .props
        .get("extractor_digest")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let after_ed = after
        .props
        .get("extractor_digest")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if before_ed != after_ed {
        differences.push(FieldDelta {
            field: "extractorDigest".to_string(),
            before: before_ed.to_string(),
            after: after_ed.to_string(),
        });
    }

    // repo_identity
    let before_ri = before
        .props
        .get("repo_identity")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let after_ri = after
        .props
        .get("repo_identity")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if before_ri != after_ri {
        differences.push(FieldDelta {
            field: "repoIdentity".to_string(),
            before: before_ri.to_string(),
            after: after_ri.to_string(),
        });
    }

    // label
    let before_label = before
        .props
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let after_label = after
        .props
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if before_label != after_label {
        differences.push(FieldDelta {
            field: "label".to_string(),
            before: before_label,
            after: after_label,
        });
    }

    // pinned
    let before_pinned = before
        .props
        .get("pinned")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let after_pinned = after
        .props
        .get("pinned")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if before_pinned != after_pinned {
        differences.push(FieldDelta {
            field: "pinned".to_string(),
            before: before_pinned.to_string(),
            after: after_pinned.to_string(),
        });
    }

    // created_at
    if before.created_at != after.created_at {
        differences.push(FieldDelta {
            field: "createdAt".to_string(),
            before: before.created_at.clone(),
            after: after.created_at.clone(),
        });
    }

    // Compatibility
    let (compat_schema, compat_reason) = if before.schema_version == after.schema_version {
        ("same".to_string(), String::new())
    } else {
        (
            "different".to_string(),
            "schema version changed".to_string(),
        )
    };

    ArchitectureDiffReport {
        schema_version: "1.0".to_string(),
        capability: "architecture-diff-mvp".to_string(),
        snapshots: SnapshotsBlock {
            before: SnapshotSummary::from_snapshot(before),
            after: SnapshotSummary::from_snapshot(after),
        },
        compatibility: CompatibilityBlock {
            schema: compat_schema,
            reason: compat_reason,
        },
        differences,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Snapshot;

    fn make_props(
        repo_identity: &str,
        extractor_digest: &str,
        schema_version: &str,
        label: Option<&str>,
        pinned: bool,
    ) -> serde_json::Map<String, serde_json::Value> {
        let mut props = serde_json::Map::new();
        props.insert(
            "repo_identity".to_string(),
            serde_json::Value::String(repo_identity.to_string()),
        );
        props.insert(
            "extractor_digest".to_string(),
            serde_json::Value::String(extractor_digest.to_string()),
        );
        props.insert(
            "schema_version".to_string(),
            serde_json::Value::String(schema_version.to_string()),
        );
        props.insert(
            "schema_compatibility".to_string(),
            serde_json::Value::String("1.0".to_string()),
        );
        props.insert(
            "remote".to_string(),
            serde_json::Value::String("https://example.com/repo".to_string()),
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
        props
    }

    // Test fixture builder: 8 fields mirror the Snapshot carrier under test.
    #[allow(clippy::too_many_arguments)]
    fn make_snapshot(
        id: &str,
        commit_hash: &str,
        schema_version: i64,
        extractor_digest: &str,
        repo_identity: &str,
        created_at: &str,
        label: Option<&str>,
        pinned: bool,
    ) -> Snapshot {
        Snapshot {
            id: id.to_string(),
            sequence: 1,
            kind: "architecture".to_string(),
            commit_hash: commit_hash.to_string(),
            worktree_id: repo_identity.to_string(),
            schema_version,
            created_at: created_at.to_string(),
            props: make_props(
                repo_identity,
                extractor_digest,
                &format!("{}.0.0", schema_version),
                label,
                pinned,
            ),
        }
    }

    #[test]
    fn identical_snapshots_yield_empty_differences() {
        let snap = make_snapshot(
            "snap-abc123",
            "abcd1234",
            1,
            "blake3:digest",
            "repo-xyz",
            "2026-08-17T10:00:00Z",
            None,
            false,
        );
        let report = diff_snapshots(&snap, &snap);
        assert!(
            report.differences.is_empty(),
            "identical snapshots must have no differences"
        );
        assert_eq!(report.compatibility.schema, "same");
        assert_eq!(report.compatibility.reason, "");
    }

    #[test]
    fn different_commit_hash_yields_one_delta() {
        let before = make_snapshot(
            "snap-aaa",
            "aaaa1111",
            1,
            "blake3:digest",
            "repo-xyz",
            "2026-08-17T10:00:00Z",
            None,
            false,
        );
        let after = make_snapshot(
            "snap-bbb",
            "bbbb2222",
            1,
            "blake3:digest",
            "repo-xyz",
            "2026-08-17T10:00:00Z",
            None,
            false,
        );
        let report = diff_snapshots(&before, &after);
        assert_eq!(report.differences.len(), 1, "only commit_hash differs");
        assert_eq!(report.differences[0].field, "commitHash");
        assert_eq!(report.compatibility.schema, "same");
    }

    #[test]
    fn different_schema_version_yields_compatibility_different() {
        let before = make_snapshot(
            "snap-aaa",
            "aaaa1111",
            1,
            "blake3:digest",
            "repo-xyz",
            "2026-08-17T10:00:00Z",
            None,
            false,
        );
        let after = make_snapshot(
            "snap-bbb",
            "bbbb2222",
            2,
            "blake3:digest",
            "repo-xyz",
            "2026-08-17T10:00:00Z",
            None,
            false,
        );
        let report = diff_snapshots(&before, &after);
        assert_eq!(report.compatibility.schema, "different");
        assert_eq!(report.compatibility.reason, "schema version changed");
        // schema version difference is also a field delta
        let schema_deltas: Vec<_> = report
            .differences
            .iter()
            .filter(|d| d.field == "schemaVersion")
            .collect();
        assert_eq!(schema_deltas.len(), 1);
    }

    #[test]
    fn different_extractor_digest_yields_delta() {
        let before = make_snapshot(
            "snap-aaa",
            "aaaa1111",
            1,
            "blake3:digest-v1",
            "repo-xyz",
            "2026-08-17T10:00:00Z",
            None,
            false,
        );
        let after = make_snapshot(
            "snap-bbb",
            "aaaa1111",
            1,
            "blake3:digest-v2",
            "repo-xyz",
            "2026-08-17T10:00:00Z",
            None,
            false,
        );
        let report = diff_snapshots(&before, &after);
        assert!(!report.differences.is_empty());
        let ed_deltas: Vec<_> = report
            .differences
            .iter()
            .filter(|d| d.field == "extractorDigest")
            .collect();
        assert_eq!(ed_deltas.len(), 1);
        assert_eq!(report.compatibility.schema, "same");
    }
}
