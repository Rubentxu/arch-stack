//! Graph read queries for diagram bundle export.
//!
//! Four read-only Cypher queries through `GraphStore::query`. User input
//! is validated via `graph::validate_identifier` before interpolation.
//! Note: `GraphStore::prepare` + `execute` (M51) ARE available, but
//! these queries return rows with column names (via `query`, not
//! `execute` which is positional). The interpolation path remains the
//! canonical read API for column-typed result rows.

use crate::diagram::export_types::EvidenceEntry;
use crate::graph::validate_identifier;
use crate::store::GraphStore;
use anyhow::Context;

/// An element row from Query 1.
#[derive(Debug, Clone)]
pub struct ElementRow {
    pub id: String,
    pub kind_id: String,
    pub category: String,
    pub canonical_key: String,
    pub current_name: String,
    pub current_status: String,
    pub current_confidence: f64,
    pub current_version_id: String,
}

/// A semantic edge row from Query 2.
#[derive(Debug, Clone)]
pub struct SemanticEdgeRow {
    pub relation_id: String,
    pub predicate_id: String,
    pub source_id: String,
    pub target_id: String,
    pub order_key: String,
    pub props: serde_json::Map<String, serde_json::Value>,
}

/// A version props row from Query 4.
#[derive(Debug)]
pub struct VersionPropsRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub props: serde_json::Map<String, serde_json::Value>,
}

/// Convert a Cell to a serde_json::Map (for Object variants).
fn cell_to_json_map(cell: &crate::row::Cell) -> serde_json::Map<String, serde_json::Value> {
    let json = cell.to_json();
    json.as_object().cloned().unwrap_or_default()
}

/// Extract a f64 from a Cell (via JSON conversion).
fn cell_as_f64(cell: &crate::row::Cell) -> Option<f64> {
    cell.to_json().as_f64()
}

/// Query 1: elements filtered by category and scope.
///
/// `kind`: optional C4Kind string ("container", "component", etc.)
///         When provided, adds `AND e.kind_id STARTS WITH '{kind}'`
///         Per ADR-024: kind_id stores the projection-specific identifier
///         (e.g., "mt.container"), not the bare C4Kind string.
pub fn query_elements(
    store: &dyn GraphStore,
    category: &str,
    scope_ident: Option<&str>,
    kind: Option<&str>,
) -> anyhow::Result<Vec<ElementRow>> {
    let safe_category = validate_identifier(category)?;

    let cypher = match (scope_ident, kind) {
        (Some(key), Some(k)) => {
            let safe_key = validate_identifier(key)?;
            let safe_kind = validate_identifier(k)?;
            // SCN-417: prefix matching — scope `src/auth` matches `src/auth/user.rs`
            // ADR-024: kind_id CONTAINS handles both 'container' and 'mt.container' formats
            format!(
                "MATCH (e:Element) \
                 WHERE e.category = '{safe_category}' \
                   AND e.canonical_key STARTS WITH '{safe_key}' \
                   AND e.kind_id CONTAINS '{safe_kind}' \
                 RETURN e.id, e.kind_id, e.category, e.canonical_key, \
                        e.current_name, e.current_status, e.current_confidence, \
                        e.current_version_id;"
            )
        }
        (Some(key), None) => {
            let safe_key = validate_identifier(key)?;
            format!(
                "MATCH (e:Element) \
                 WHERE e.category = '{safe_category}' \
                   AND e.canonical_key STARTS WITH '{safe_key}' \
                 RETURN e.id, e.kind_id, e.category, e.canonical_key, \
                        e.current_name, e.current_status, e.current_confidence, \
                        e.current_version_id;"
            )
        }
        (None, Some(k)) => {
            let safe_kind = validate_identifier(k)?;
            format!(
                "MATCH (e:Element) \
                 WHERE e.category = '{safe_category}' \
                   AND e.kind_id CONTAINS '{safe_kind}' \
                 RETURN e.id, e.kind_id, e.category, e.canonical_key, \
                        e.current_name, e.current_status, e.current_confidence, \
                        e.current_version_id;"
            )
        }
        (None, None) => format!(
            "MATCH (e:Element) \
             WHERE e.category = '{safe_category}' \
             RETURN e.id, e.kind_id, e.category, e.canonical_key, \
                    e.current_name, e.current_status, e.current_confidence, \
                    e.current_version_id;"
        ),
    };

    let rows = store.query(&cypher).context("query_elements")?;
    rows.into_iter()
        .map(|r| {
            Ok(ElementRow {
                id: r
                    .get("e.id")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                kind_id: r
                    .get("e.kind_id")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                category: r
                    .get("e.category")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                canonical_key: r
                    .get("e.canonical_key")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                current_name: r
                    .get("e.current_name")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                current_status: r
                    .get("e.current_status")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                current_confidence: r
                    .get("e.current_confidence")
                    .and_then(cell_as_f64)
                    .unwrap_or(0.0),
                current_version_id: r
                    .get("e.current_version_id")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// Query 2: semantic relations within the given category.
pub fn query_semantic_edges(
    store: &dyn GraphStore,
    category: &str,
) -> anyhow::Result<Vec<SemanticEdgeRow>> {
    let safe_category = validate_identifier(category)?;

    let cypher = format!(
        "MATCH (src:Element)-[edge:SEMANTIC_EDGE]->(tgt:Element) \
         WHERE src.category = '{safe_category}' \
           AND tgt.category = '{safe_category}' \
           AND edge.active = true \
         RETURN edge.relation_id, edge.predicate_id, src.id AS source_id, tgt.id AS target_id, \
                edge.order_key, edge.props;"
    );

    let rows = store.query(&cypher).context("query_semantic_edges")?;
    rows.into_iter()
        .map(|r| {
            let props = r
                .get("edge.props")
                .map(cell_to_json_map)
                .unwrap_or_default();

            Ok(SemanticEdgeRow {
                relation_id: r
                    .get("edge.relation_id")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                predicate_id: r
                    .get("edge.predicate_id")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                source_id: r
                    .get("source_id")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                target_id: r
                    .get("target_id")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                order_key: r
                    .get("edge.order_key")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                props,
            })
        })
        .collect()
}

/// Query 3: evidence for given version IDs (status filtering happens in Rust).
///
/// Note: lbug 0.18.3 has no JSON WHERE. Evidence status lives in
/// `e.props["status"]`, so we fetch all and filter in Rust.
pub fn query_evidence_for_versions(
    store: &dyn GraphStore,
    version_ids: &[String],
) -> anyhow::Result<Vec<EvidenceEntry>> {
    if version_ids.is_empty() {
        return Ok(vec![]);
    }

    let safe_ids: Result<Vec<_>, _> = version_ids
        .iter()
        .map(|id| validate_identifier(id).map(|s| s.to_string()))
        .collect();
    let safe_ids = safe_ids.context("version id validation failed")?;
    let id_list = safe_ids
        .iter()
        .map(|id| format!("'{}'", id))
        .collect::<Vec<_>>()
        .join(", ");

    let cypher = format!(
        "MATCH (ev:ElementVersion)-[r:SUPPORTED_BY]->(e:Evidence) \
         WHERE ev.id IN [{id_list}] \
         RETURN e.id, e.kind, e.claim, e.path, e.start_line, e.end_line, \
                e.tool_name, e.tool_version, e.rule_id, e.props, \
                e.content_hash, e.observed_at;"
    );

    let rows = store
        .query(&cypher)
        .context("query_evidence_for_versions")?;

    rows.into_iter()
        .filter_map(|r| {
            // Filter to only Accepted evidence (status in props["status"])
            let props = r.get("e.props").map(cell_to_json_map).unwrap_or_default();

            let status = props.get("status").and_then(|v| v.as_str()).unwrap_or("");

            if status != "accepted" {
                return None;
            }

            Some(Ok(EvidenceEntry {
                id: r
                    .get("e.id")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                kind: r
                    .get("e.kind")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                claim: r
                    .get("e.claim")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                path: r
                    .get("e.path")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                start_line: r.get("e.start_line").and_then(|c| c.as_i64()).unwrap_or(0) as u64,
                end_line: r.get("e.end_line").and_then(|c| c.as_i64()).unwrap_or(0) as u64,
                tool_name: r
                    .get("e.tool_name")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                tool_version: r
                    .get("e.tool_version")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                rule_id: r
                    .get("e.rule_id")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                content_hash: r
                    .get("e.content_hash")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                observed_at: r
                    .get("e.observed_at")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
            }))
        })
        .collect()
}

/// Query 4: element version properties.
pub fn query_version_props(
    store: &dyn GraphStore,
    version_ids: &[String],
) -> anyhow::Result<Vec<VersionPropsRow>> {
    if version_ids.is_empty() {
        return Ok(vec![]);
    }

    let safe_ids: Result<Vec<_>, _> = version_ids
        .iter()
        .map(|id| validate_identifier(id).map(|s| s.to_string()))
        .collect();
    let safe_ids = safe_ids.context("version id validation failed")?;
    let id_list = safe_ids
        .iter()
        .map(|id| format!("'{}'", id))
        .collect::<Vec<_>>()
        .join(", ");

    let cypher = format!(
        "MATCH (v:ElementVersion) \
         WHERE v.id IN [{id_list}] \
         RETURN v.id, v.name, v.description, v.props;"
    );

    let rows = store.query(&cypher).context("query_version_props")?;
    rows.into_iter()
        .map(|r| {
            let props = r.get("v.props").map(cell_to_json_map).unwrap_or_default();

            Ok(VersionPropsRow {
                id: r
                    .get("v.id")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                name: r
                    .get("v.name")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                description: r
                    .get("v.description")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                props,
            })
        })
        .collect()
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
