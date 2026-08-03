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
use std::path::Path;

use anyhow::{Context, Result};

use crate::store::GraphStore;

/// Open the project's graph store and ensure the schema is initialized.
///
/// Error is contextualized so the caller can wrap it in its own
/// error type (e.g. `CallGraphError::GraphWrite`).
pub fn open_and_init(project_dir: &Path) -> Result<Box<dyn GraphStore>> {
    let mut store = crate::store::open_default(project_dir).context("failed to acquire DB lock")?;
    store.init().context("graph init")?;
    Ok(store)
}

/// Escape a string for use inside a Cypher single-quoted string.
pub fn escape_cypher_string(s: &str) -> String {
    s.replace('\'', "\\'")
}

/// Fetch the set of `canonical_key`s already present in the graph.
/// Used for idempotency: skip elements whose keys already exist.
pub fn existing_canonical_keys(store: &dyn GraphStore) -> Result<HashSet<String>> {
    store
        .query("MATCH (e:Element) WHERE e.canonical_key IS NOT NULL RETURN e.canonical_key;")?
        .into_iter()
        .filter_map(|row| {
            row.get("e.canonical_key")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect::<HashSet<_>>()
        .pipe(Ok)
}

/// Write a `SourceArtifact` node for a file and return its id.
///
/// Id is derived from the file path (`src:<blake3-hex>`), so the
/// MERGE is idempotent. Callers typically combine this with a
/// per-file dedup map to avoid re-querying for the same file.
pub fn write_source_artifact(store: &mut dyn GraphStore, file: &str) -> Result<String> {
    let id = format!("src:{}", blake3::hash(file.as_bytes()).to_hex());
    let path_escaped = escape_cypher_string(file);
    let cypher = format!(
        "MERGE (s:SourceArtifact {{id: '{id}'}}) SET \
         s.kind = 'manifest', \
         s.relative_path = '{path_escaped}', \
         s.language = '', \
         s.content_hash = '', \
         s.generated = false, \
         s.props = '{{}}';"
    );
    store
        .query(&cypher)
        .with_context(|| format!("put_source_artifact {id}"))?;
    Ok(id)
}

/// Minimal pipe helper used by [`existing_canonical_keys`] to wrap a
/// `Result` in `Ok`. Kept local to avoid a public trait.
trait Pipe<T> {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
        Self: Sized,
    {
        f(self)
    }
}

impl<T> Pipe<T> for T {}

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
    fn test_pipe_wraps_value_in_ok() {
        let x: Result<u32> = 42u32.pipe(Ok);
        assert_eq!(x.unwrap(), 42);
    }
}
