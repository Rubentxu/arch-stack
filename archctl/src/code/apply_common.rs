//! Shared helpers for the `code` bounded-context apply pipelines.
//!
//! `call_graph`, `c4_discover` and `class_diagram` each ship an
//! `apply(project_dir, report, fs)` that opens the graph store,
//! seeds MetaTypes/Predicates, writes Element/ElementVersion nodes,
//! links SemanticEdges, and attaches SourceArtifact + Evidence rows.
//! This module extracts the boilerplate that was previously copied
//! per-module (~150 LOC across 4 private copies of
//! `escape_cypher_string`, 2 of `existing_canonical_keys`, 2 of
//! `write_source_artifact` + 2 local `Pipe` traits).

use std::collections::HashSet;

use anyhow::Result;

use crate::source::SourceArtifact;
use crate::store::GraphStore;

/// Escape a string for use inside a Cypher single-quoted string.
pub fn escape_cypher_string(s: &str) -> String {
    s.replace('\'', "\\'")
}

/// Fetch the set of `canonical_key`s already present in the graph.
/// Used for idempotency: skip elements whose keys already exist.
pub fn existing_canonical_keys(store: &dyn GraphStore) -> Result<HashSet<String>> {
    Ok(store
        .query("MATCH (e:Element) WHERE e.canonical_key IS NOT NULL RETURN e.canonical_key;")?
        .into_iter()
        .filter_map(|row| {
            row.get("e.canonical_key")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect::<HashSet<_>>())
}

/// Write a `SourceArtifact` node for a file and return its canonical id.
///
/// Id follows ADR-017 §D2: `"src:" + blake3(relative_path + content_hash)[..16]`
/// via [`SourceArtifact::id_for`]. Persistence goes through the canonical
/// [`SourceOps::put_source`] path (shared with the `evidence` pipeline), so
/// the `code/*` and `evidence` pipelines converge on the same node for the
/// same file version.
///
/// `language` is the inventory language label (`"rust"`, `"typescript"`, …)
/// or `""` when unknown. `commit_hash` stays `None` for code-derived
/// artifacts (B1 limitation, see `source.rs:33`).
pub fn write_source_artifact(
    store: &mut dyn GraphStore,
    file: &str,
    content_hash: &str,
    language: &str,
) -> Result<String> {
    let artifact = SourceArtifact::from_content(
        file,
        language,
        content_hash,
        None,
        "",
        env!("CARGO_PKG_VERSION"),
    );
    store.put_source(&artifact)?;
    Ok(artifact.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_cypher_string_basic() {
        assert_eq!(escape_cypher_string("foo"), "foo");
        assert_eq!(escape_cypher_string("o'reilly"), "o\\'reilly");
        assert_eq!(escape_cypher_string("a\\b"), "a\\b"); // backslash NOT escaped
        assert_eq!(escape_cypher_string(""), "");
    }

    #[test]
    fn write_source_artifact_uses_d2_canonical_id() {
        // R1: id must equal SourceArtifact::id_for(relative_path, content_hash).
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = crate::filesystem::SystemFilesystem;
        crate::graph::init(&project, &fs).unwrap();
        let mut store = crate::store::open_default(&project).unwrap();
        let id = write_source_artifact(&mut *store, "src/lib.rs", "sha256:abc123", "rust").unwrap();
        let expected = SourceArtifact::id_for("src/lib.rs", "sha256:abc123");
        assert_eq!(id, expected, "apply id must match canonical D2 id");
        assert!(id.starts_with("src:"), "id must use src: prefix");
        assert_eq!(id.len(), 4 + 32, "src: + 32 hex chars");
    }

    #[test]
    fn write_source_artifact_id_changes_with_content_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = crate::filesystem::SystemFilesystem;
        crate::graph::init(&project, &fs).unwrap();
        let mut store = crate::store::open_default(&project).unwrap();
        let id1 =
            write_source_artifact(&mut *store, "src/lib.rs", "sha256:abc123", "rust").unwrap();
        let id2 =
            write_source_artifact(&mut *store, "src/lib.rs", "sha256:def456", "rust").unwrap();
        assert_ne!(id1, id2, "different content_hash must produce different id");
    }

    fn seeded_store_with_canonical_keys(keys: &[&str]) -> Box<dyn GraphStore> {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = crate::filesystem::SystemFilesystem;
        crate::graph::init(&project, &fs).unwrap();
        let mut store = crate::store::open_default(&project).unwrap();
        store.init().unwrap();
        for (i, key) in keys.iter().enumerate() {
            let cypher =
                format!("MERGE (e:Element {{id: 'el:{i}'}}) SET e.canonical_key = '{key}';");
            store.query(&cypher).unwrap();
        }
        store
    }

    #[test]
    fn existing_canonical_keys_empty_on_fresh_store() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = crate::filesystem::SystemFilesystem;
        crate::graph::init(&project, &fs).unwrap();
        let mut store = crate::store::open_default(&project).unwrap();
        store.init().unwrap();
        let keys = existing_canonical_keys(&*store).unwrap();
        assert!(keys.is_empty(), "fresh store must have no canonical keys");
    }

    #[test]
    fn existing_canonical_keys_returns_seeded_keys() {
        let store = seeded_store_with_canonical_keys(&["a:one", "b:two", "c:three"]);
        let keys = existing_canonical_keys(&*store).unwrap();
        assert_eq!(keys.len(), 3, "all seeded keys must be returned");
        assert!(keys.contains("a:one"));
        assert!(keys.contains("b:two"));
        assert!(keys.contains("c:three"));
    }

    #[test]
    fn existing_canonical_keys_ignores_elements_without_key() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = crate::filesystem::SystemFilesystem;
        crate::graph::init(&project, &fs).unwrap();
        let mut store = crate::store::open_default(&project).unwrap();
        store.init().unwrap();
        // Element WITH canonical_key + one WITHOUT (id only).
        store
            .query("MERGE (e:Element {id: 'el:1'}) SET e.canonical_key = 'k:1';")
            .unwrap();
        store.query("MERGE (e:Element {id: 'el:2'});").unwrap();
        let keys = existing_canonical_keys(&*store).unwrap();
        assert_eq!(keys.len(), 1, "only the keyed element counts");
        assert!(keys.contains("k:1"));
    }
}
