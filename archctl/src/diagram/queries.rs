//! Graph read queries for diagram bundle export.
//!
//! Four read-only Cypher queries through `GraphStore::query`. User input
//! is validated via `graph::validate_identifier` before interpolation.
//! Note: `GraphStore::prepare` + `execute` (M51) ARE available, but
//! these queries return rows with column names (via `query`, not
//! `execute` which is positional). The interpolation path remains the
//! canonical read API for column-typed result rows.

use crate::diagram::export_types::EvidenceEntry;
use crate::graph::{ElementRow, SemanticEdgeRow, VersionPropsRow};
use crate::store::DiagramRepository;

/// Query 1: elements filtered by category and scope.
///
/// `kind`: optional C4Kind string ("container", "component", etc.)
///         When provided, adds `AND e.kind_id STARTS WITH '{kind}'`
///         Per ADR-024: kind_id stores the projection-specific identifier
///         (e.g., "mt.container"), not the bare C4Kind string.
pub fn query_elements(
    store: &dyn DiagramRepository,
    category: &str,
    scope_ident: Option<&str>,
    kind: Option<&str>,
) -> anyhow::Result<Vec<ElementRow>> {
    DiagramRepository::list_elements(store, category, scope_ident, kind)
}

/// Query 2: semantic relations within the given category.
pub fn query_semantic_edges(
    store: &dyn DiagramRepository,
    category: &str,
) -> anyhow::Result<Vec<SemanticEdgeRow>> {
    DiagramRepository::list_semantic_edges(store, category)
}

/// Query 3: evidence for given version IDs (status filtering happens in Rust).
pub fn query_evidence_for_versions(
    store: &dyn DiagramRepository,
    version_ids: &[String],
) -> anyhow::Result<Vec<EvidenceEntry>> {
    DiagramRepository::list_evidence_for_versions(store, version_ids)
}

/// Query 4: element version properties.
pub fn query_version_props(
    store: &dyn DiagramRepository,
    version_ids: &[String],
) -> anyhow::Result<Vec<VersionPropsRow>> {
    DiagramRepository::list_version_props(store, version_ids)
}

/// Check that a query string contains no write keywords.
pub fn is_read_only_query(cypher: &str) -> bool {
    let upper = cypher.to_uppercase();
    !upper.contains("MERGE")
        && !upper.contains("CREATE")
        && !upper.contains("DELETE")
        && !upper.contains("SET")
        && !upper.contains("REMOVE")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_read_only_query_accepts_read_queries() {
        assert!(is_read_only_query("MATCH (e:Element) RETURN e.id"));
        assert!(is_read_only_query("MATCH (e:Element)-[r]->(f) RETURN e, f"));
        assert!(is_read_only_query(
            "MATCH (e:Element) WHERE e.id = 'foo' RETURN e"
        ));
    }

    #[test]
    fn is_read_only_query_rejects_write_queries() {
        assert!(!is_read_only_query("MATCH (e:Element) MERGE (e)"));
        assert!(!is_read_only_query("CREATE (e:Element)"));
        assert!(!is_read_only_query("MATCH (e) DELETE e"));
        assert!(!is_read_only_query("MATCH (e) SET e.foo = 'bar'"));
        assert!(!is_read_only_query("MATCH (e) REMOVE e.foo"));
    }
}
