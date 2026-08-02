//! Source artifact domain type.
//!
//! Represents one analyzed file-version at the point it was extracted.
//! Identity = `relative_path + content_hash` (D2) — two scans of the
//! same file with the same content produce the same `id`; a changed file
//! produces a different `id` and creates a new node.
//!
//! `content_hash` is the SHA-256 already computed by
//! [`evidence::content_hash_of`] — the adapter passes it through, never
//! recomputes it. `blake3` is used only for `id` derivation (matching the
//! existing `evidence_id` pattern at `evidence.rs:328`).

use blake3::Hasher;
use serde::{Deserialize, Serialize};

/// One analyzed file-version. Identity = relative_path + content_hash (D2).
///
/// Maps 1:1 to the `SourceArtifact` node table declared in
/// `docs/schema/001_initial_schema.cypher:118-127`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceArtifact {
    /// `"src:" + blake3(relative_path + content_hash)[..16]` (32 hex chars).
    pub id: String,
    /// Always `"source_file"` in B1.
    pub kind: String,
    /// Forward-slash path, relative to the workspace root.
    pub relative_path: String,
    /// `"rust"` | `"python"` | … — from the inventory language label.
    pub language: String,
    /// The `"sha256:<hex>"` already produced by [`crate::evidence::content_hash_of`].
    pub content_hash: String,
    /// `None` for non-git workspaces (B1 does not resolve per-file hashes).
    pub commit_hash: Option<String>,
    /// Always `false` for source files.
    pub generated: bool,
    /// Extra fields. `first_seen_at`, `extractor`, and `extractor_version`
    /// are written by [`from_content`](Self::from_content).
    pub props: serde_json::Map<String, serde_json::Value>,
}

impl SourceArtifact {
    /// Build a SourceArtifact from content the pipeline already read.
    ///
    /// Pure — no I/O, no `Clock`. The `content_hash` is the value already
    /// produced by [`crate::evidence::content_hash_of`] (`"sha256:<hex>"`).
    /// The `id` is derived from `blake3(relative_path + content_hash)`.
    ///
    /// `props` is pre-populated with:
    /// - `first_seen_at`: set by the caller (typically from a [`crate::clock::Clock`])
    /// - `extractor`: `"archctl:evidence"`
    /// - `extractor_version`: `env!("CARGO_PKG_VERSION")`
    pub fn from_content(
        relative_path: &str,
        language: &str,
        content_hash: &str,
        commit_hash: Option<&str>,
        first_seen_at: &str,
        extractor_version: &str,
    ) -> Self {
        let id = Self::id_for(relative_path, content_hash);
        let mut props = serde_json::Map::new();
        props.insert(
            "first_seen_at".to_string(),
            serde_json::Value::String(first_seen_at.to_string()),
        );
        props.insert(
            "extractor".to_string(),
            serde_json::Value::String("archctl:evidence".to_string()),
        );
        props.insert(
            "extractor_version".to_string(),
            serde_json::Value::String(extractor_version.to_string()),
        );

        Self {
            id,
            kind: "source_file".to_string(),
            relative_path: relative_path.to_string(),
            language: language.to_string(),
            content_hash: content_hash.to_string(),
            commit_hash: commit_hash.map(String::from),
            generated: false,
            props,
        }
    }

    /// Derive the stable id for a `(relative_path, content_hash)` pair.
    /// Two calls with the same inputs always produce the same id.
    ///
    /// Format: `"src:" + blake3(relative_path + content_hash)[..16]`
    pub fn id_for(relative_path: &str, content_hash: &str) -> String {
        let mut h = Hasher::new();
        h.update(relative_path.as_bytes());
        h.update(content_hash.as_bytes());
        format!("src:{}", hex::encode(&h.finalize().as_bytes()[..16]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_artifact_id_is_stable_under_same_path_and_hash() {
        let id1 = SourceArtifact::id_for("src/lib.rs", "sha256:abc123");
        let id2 = SourceArtifact::id_for("src/lib.rs", "sha256:abc123");
        assert_eq!(id1, id2, "same path+hash must produce same id");
        assert!(id1.starts_with("src:"), "id must use src: prefix");
    }

    #[test]
    fn source_artifact_id_changes_when_content_changes() {
        let id1 = SourceArtifact::id_for("src/lib.rs", "sha256:abc123");
        let id2 = SourceArtifact::id_for("src/lib.rs", "sha256:def456");
        assert_ne!(id1, id2, "different content_hash must produce different id");
    }

    #[test]
    fn source_artifact_from_content_populates_all_fields() {
        let sa = SourceArtifact::from_content(
            "src/main.rs",
            "rust",
            "sha256:deadbeef",
            Some("abc123"),
            "2026-07-30T00:00:00Z",
            "0.1.0",
        );
        assert_eq!(sa.relative_path, "src/main.rs");
        assert_eq!(sa.language, "rust");
        assert_eq!(sa.content_hash, "sha256:deadbeef");
        assert_eq!(sa.commit_hash, Some("abc123".to_string()));
        assert_eq!(sa.kind, "source_file");
        assert!(!sa.generated);
        assert_eq!(sa.props["extractor"].as_str().unwrap(), "archctl:evidence");
        assert_eq!(sa.props["extractor_version"].as_str().unwrap(), "0.1.0");
        assert_eq!(
            sa.props["first_seen_at"].as_str().unwrap(),
            "2026-07-30T00:00:00Z"
        );
        // id is derived
        let expected_id = SourceArtifact::id_for("src/main.rs", "sha256:deadbeef");
        assert_eq!(sa.id, expected_id);
    }
}
