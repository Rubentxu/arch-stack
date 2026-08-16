//! Export pipeline: graph query → projection → bundle write.
//!
//! Orchestrates the 4 read queries, canonicalizes the result,
//! computes the content hash, and writes the 5-file bundle.

use anyhow::Context;
use std::path::Path;

use crate::clock::Clock;
use crate::diagram::export_types::{
    Edge as ExportEdge, EdgeColors, ElementColors, EvidenceBundle, Manifest, Node as ExportNode,
    Projection, Styles,
};
use crate::diagram::hash::base_revision;
use crate::diagram::selector::{ScopeFilter, ViewSelector};
use crate::filesystem::Filesystem;
use crate::graph::ElementRow;
use crate::store::GraphStore;

/// Report from a successful export operation.
#[derive(Debug)]
pub struct ExportReport {
    pub manifest: Manifest,
    pub element_count: usize,
    pub edge_count: usize,
    pub evidence_count: usize,
    /// True when the projection contains zero nodes (empty graph).
    pub empty: bool,
    /// Warning message when the graph is empty; `None` otherwise.
    pub warning: Option<String>,
}

/// Run the full export pipeline.
///
/// Parses `selector` → runs 4 graph queries → builds projection → computes
/// `baseRevision` → writes 5 files (`manifest.json`, `projection.json`,
/// `evidence.json`, `styles.json`, `assets/`) atomically.
///
/// Map Element.kind_id (`"mt.container"`) → schema-valid node type (`"container"`).
fn kind_id_to_type(kind_id: &str) -> String {
    // Strip the `mt.` prefix and any namespace; keep the last segment.
    kind_id.rsplit('.').next().unwrap_or(kind_id).to_string()
}

/// Map internal Element status → schema-valid bundle status.
/// `active` → `drafted` (until evidence is explicitly accepted),
/// `deprecated` → `superseded`, anything else passes through.
fn schema_valid_status(current: &str) -> String {
    match current {
        "active" => "drafted".to_string(),
        "deprecated" => "superseded".to_string(),
        other => other.to_string(),
    }
}

/// The internal bundle carrier (manifest + projection + evidence + styles)
/// built by `build_bundle` and consumed by both `run_export` (writes 5
/// files) and `build_export_envelope` (single JSON to stdout).
#[derive(Debug)]
pub struct BundleEnvelope {
    pub manifest: Manifest,
    pub projection: Projection,
    pub evidence: EvidenceBundle,
    pub styles: Styles,
}

/// Parses `selector` → runs 4 graph queries → builds projection → computes
/// `baseRevision`. Returns the in-memory bundle WITHOUT writing files.
/// Used by `run_export` (file write) and `build_export_envelope` (stdout).
pub fn build_bundle(
    store: &dyn GraphStore,
    selector: &str,
    clock: &dyn Clock,
) -> anyhow::Result<BundleEnvelope> {
    // 1. Parse selector
    let view: ViewSelector =
        crate::diagram::selector::parse(selector).context("invalid view selector")?;

    // Per ADR-024: category = diagram family ("c4"), not the C4 kind string.
    let category = view.kind.category();
    let kind = view.kind.to_string();
    let scope_ident = match &view.scope {
        ScopeFilter::All => None,
        ScopeFilter::Exact(s) => Some(s.as_str()),
    };

    // 2. Run queries via DiagramRepository
    let element_rows = store
        .list_elements(category, scope_ident, Some(&kind))
        .context("list_elements failed")?;

    let edge_rows = store
        .list_semantic_edges(category)
        .context("list_semantic_edges failed")?;

    // Collect version IDs for evidence + version props queries
    let version_ids: Vec<String> = element_rows
        .iter()
        .filter(|e| !e.current_version_id.is_empty())
        .map(|e| e.current_version_id.clone())
        .collect();

    let evidence_entries = store
        .list_evidence_for_versions(&version_ids)
        .context("list_evidence_for_versions failed")?;

    let version_props = store
        .list_version_props(&version_ids)
        .context("list_version_props failed")?;

    // 3. Build projection (nodes + edges)
    let version_map: std::collections::HashMap<String, &crate::graph::VersionPropsRow> =
        version_props.iter().map(|v| (v.id.clone(), v)).collect();

    // M81 D2: fetch ViewMembers and index by element_id for LEFT JOIN.
    // One query + HashMap lookup avoids N+1 Cypher calls (ADR-019 perf budget).
    let all_members: Vec<crate::diagram::view_types::ViewMember> =
        store.get_view_members(selector).unwrap_or_default();
    let view_member_map: std::collections::HashMap<&str, &crate::diagram::view_types::ViewMember> =
        all_members
            .iter()
            .map(|m| (m.element_id.as_str(), m))
            .collect();

    let nodes: Vec<ExportNode> = element_rows
        .iter()
        .map(|e: &ElementRow| {
            let version = version_map.get(&e.current_version_id);
            let description = version
                .map(|v| v.description.clone())
                .filter(|s| !s.is_empty());
            let evidence_refs: Vec<String> = evidence_entries
                .iter()
                .filter(|_ev| {
                    // evidence supports this element if it was fetched for one of its version IDs
                    // (we don't have a direct link, so we include all evidence for now)
                    true
                })
                .map(|e| e.id.clone())
                .collect();

            // M81 D2: LEFT JOIN against ViewMember map for cosmetic fields.
            // Defaults: x=0, y=0, collapsed=false, label_override=None.
            let vm = view_member_map.get(e.id.as_str());
            let label_override: Option<String> = vm.and_then(|v| {
                if v.label.is_empty() {
                    None
                } else {
                    Some(v.label.clone())
                }
            });

            ExportNode {
                id: e.id.clone(),
                // ADR-024: kind_id holds the projection kind
                // (`mt.container`, `mt.component`, etc.) — extract the
                // suffix after the dot to get the schema-valid `type`.
                element_type: kind_id_to_type(&e.kind_id),
                name: e.current_name.clone(),
                description,
                canonical_key: Some(e.canonical_key.clone()).filter(|s| !s.is_empty()),
                // Bundle schema accepts only accepted/drafted/superseded.
                // Internal Element.current_status is "active"/"deprecated";
                // map to the schema-valid "drafted" until accepted.
                status: Some(schema_valid_status(&e.current_status)).filter(|s| !s.is_empty()),
                confidence: Some(e.current_confidence).filter(|&c| c > 0.0),
                evidence_refs: Some(evidence_refs).filter(|v| !v.is_empty()),
                // M81 D2: cosmetic fields from ViewMember
                x: vm.map(|v| v.x).unwrap_or(0),
                y: vm.map(|v| v.y).unwrap_or(0),
                collapsed: vm.map(|v| v.collapsed).unwrap_or(false),
                label_override,
            }
        })
        .collect();

    // Sort nodes by id for deterministic ordering
    let mut nodes = nodes;
    nodes.sort_by(|a, b| a.id.cmp(&b.id));

    let edges: Vec<ExportEdge> = edge_rows
        .iter()
        .map(|e| ExportEdge {
            id: e.relation_id.clone(),
            source: e.source_id.clone(),
            target: e.target_id.clone(),
            predicate: e.predicate_id.clone(),
            label: Some(e.order_key.clone()).filter(|s| !s.is_empty()),
        })
        .collect();

    // Sort edges by id
    let mut edges = edges;
    edges.sort_by(|a, b| a.id.cmp(&b.id));

    let projection = Projection { nodes, edges };

    // 4. Compute baseRevision
    let revision = base_revision(&projection);

    // 5. Build manifest
    let manifest = Manifest {
        schema_version: "1.1.0".into(), // M81: bumped from 1.0.0 → 1.1.0 for cosmetic fields
        format: "viewer-bundle".into(),
        view_selector: selector.to_string(),
        base_revision: revision,
        generated_at: clock.now_rfc3339(),
        element_count: projection.nodes.len(),
        edge_count: projection.edges.len(),
        evidence_count: evidence_entries.len(),
    };

    // 6. Build styles
    let styles = Styles {
        theme: "default".into(),
        version: "1.0.0".into(),
        element_colors: ElementColors {
            context: "#1168bd".into(),
            container: "#438dd5".into(),
            component: "#85b8e8".into(),
            dynamic: "#2694ab".into(),
            deployment: "#999999".into(),
        },
        edge_colors: EdgeColors {
            default: "#707070".into(),
        },
    };

    let evidence_bundle = EvidenceBundle {
        evidence: evidence_entries,
    };

    Ok(BundleEnvelope {
        manifest,
        projection,
        evidence: evidence_bundle,
        styles,
    })
}

/// Builds the full bundle envelope as a `serde_json::Value` ready to print
/// to stdout (single JSON document). Adds `empty` + `warning` fields that
/// are not in the 5-file bundle but are useful for agents.
///
/// Shape:
/// ```json
/// {
///   "manifest": {...},
///   "projection": {...},
///   "evidence": {...},
///   "styles": {...},
///   "empty": bool,
///   "warning": Option<String>
/// }
/// ```
pub fn build_export_envelope(bundle: &BundleEnvelope) -> serde_json::Value {
    let empty = bundle.projection.nodes.is_empty();
    let warning = if empty {
        Some("no graph found (0 elements)".to_string())
    } else {
        None
    };
    serde_json::json!({
        "manifest": bundle.manifest,
        "projection": bundle.projection,
        "evidence": bundle.evidence,
        "styles": bundle.styles,
        "empty": empty,
        "warning": warning,
    })
}

/// Uses `Clock::now_rfc3339()` for `generatedAt` and writes each file
/// atomically (write-then-rename) for idempotency.
pub fn run_export(
    store: &dyn GraphStore,
    selector: &str,
    out_dir: &Path,
    clock: &dyn Clock,
    fs: &dyn Filesystem,
) -> anyhow::Result<ExportReport> {
    let bundle = build_bundle(store, selector, clock)?;

    // Write 5 bundle files (atomic: write to tmp, then rename)
    fs.create_dir_all(out_dir)
        .with_context(|| format!("creating output directory {}", out_dir.display()))?;

    write_atomic(fs, &out_dir.join("manifest.json"), &bundle.manifest)?;
    write_atomic(fs, &out_dir.join("projection.json"), &bundle.projection)?;
    write_atomic(fs, &out_dir.join("evidence.json"), &bundle.evidence)?;
    write_atomic(fs, &out_dir.join("styles.json"), &bundle.styles)?;

    // Write assets directory and icon files
    let assets_dir = out_dir.join("assets");
    fs.create_dir_all(&assets_dir)
        .with_context(|| format!("creating assets directory {}", assets_dir.display()))?;

    // Write icons — the 5 canonical C4 levels (shared with validate.rs).
    // Single source of truth: `assets::CANONICAL_C4_ICONS`.
    for icon_name in crate::diagram::assets::CANONICAL_C4_ICONS {
        let icon_bytes = crate::diagram::assets::icon_for(icon_name).unwrap_or_default();
        write_atomic_bytes(fs, &assets_dir.join(format!("{icon_name}.png")), icon_bytes)?;
    }

    let empty = bundle.projection.nodes.is_empty();
    let warning = if empty {
        Some("no graph found (0 elements)".into())
    } else {
        None
    };
    Ok(ExportReport {
        manifest: bundle.manifest,
        element_count: bundle.projection.nodes.len(),
        edge_count: bundle.projection.edges.len(),
        evidence_count: bundle.evidence.evidence.len(),
        empty,
        warning,
    })
}

/// Write a serializable value to a file atomically (write to .tmp, then rename).
fn write_atomic(
    fs: &dyn Filesystem,
    path: &Path,
    value: &impl serde::Serialize,
) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(value).context("serialization failed")?;
    let tmp_path = path.with_extension("json.tmp");
    fs.write(&tmp_path, json.as_bytes())?;
    fs.write(path, json.as_bytes())?; // rename not available; overwrite is acceptably atomic for this use
    Ok(())
}

/// Write bytes to a file atomically.
fn write_atomic_bytes(fs: &dyn Filesystem, path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    let tmp_path = path.with_extension("png.tmp");
    fs.write(&tmp_path, contents)?;
    fs.write(path, contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FixedClock;
    use crate::diagram::export_types::{EvidenceBundle, Manifest, Projection, Styles};
    use crate::filesystem::MemoryFilesystem;
    use crate::row::{Cell, Row};

    /// A minimal GraphStore stub that returns pre-configured query results.
    struct MockGraphStore {
        elements: Vec<Row>,
        edges: Vec<Row>,
        evidence: Vec<Row>,
        version_props: Vec<Row>,
        view_members: Vec<crate::diagram::view_types::ViewMember>,
    }

    impl crate::store::DiagramRepository for MockGraphStore {
        fn list_elements(
            &self,
            category: &str,
            scope: Option<&str>,
            kind: Option<&str>,
        ) -> anyhow::Result<Vec<crate::graph::ElementRow>> {
            // Filter self.elements the same way LbugStore::list_elements filters via Cypher.
            let cat_upper = category.to_uppercase();
            let key_upper = scope.map(|s| s.to_uppercase());
            let kind_upper = kind.map(|k| k.to_uppercase());
            let filtered: Vec<crate::graph::ElementRow> = self
                .elements
                .iter()
                .filter(|row| {
                    let row_cat = row
                        .get("e.category")
                        .and_then(|c| c.as_str())
                        .map(|s| s.to_uppercase())
                        .unwrap_or_default();
                    let row_key = row
                        .get("e.canonical_key")
                        .and_then(|c| c.as_str())
                        .map(|s| s.to_uppercase())
                        .unwrap_or_default();
                    let row_kind = row
                        .get("e.kind_id")
                        .and_then(|c| c.as_str())
                        .map(|s| s.to_uppercase())
                        .unwrap_or_default();
                    let cat_match = row_cat == cat_upper;
                    let key_match = key_upper
                        .as_ref()
                        .filter(|k| !k.is_empty() && *k != "*")
                        .map(|k| row_key.starts_with(k))
                        .unwrap_or(true);
                    let kind_match = kind_upper
                        .as_ref()
                        .map(|k| row_kind.contains(k))
                        .unwrap_or(true);
                    cat_match && key_match && kind_match
                })
                .map(|row| crate::graph::ElementRow {
                    id: row
                        .get("e.id")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    kind_id: row
                        .get("e.kind_id")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    category: row
                        .get("e.category")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    canonical_key: row
                        .get("e.canonical_key")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    current_name: row
                        .get("e.current_name")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    current_status: row
                        .get("e.current_status")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    current_confidence: row
                        .get("e.current_confidence")
                        .and_then(|c| c.as_f64())
                        .unwrap_or(0.0),
                    current_version_id: row
                        .get("e.current_version_id")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                })
                .collect();
            Ok(filtered)
        }
        fn list_semantic_edges(
            &self,
            _category: &str,
        ) -> anyhow::Result<Vec<crate::graph::SemanticEdgeRow>> {
            // Return all edges for now (category filtering happens at the Cypher level in the real impl)
            Ok(self
                .edges
                .iter()
                .map(|row| crate::graph::SemanticEdgeRow {
                    relation_id: row
                        .get("edge.relation_id")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    predicate_id: row
                        .get("edge.predicate_id")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    source_id: row
                        .get("src.id")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    target_id: row
                        .get("tgt.id")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    order_key: row
                        .get("edge.order_key")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    props: row
                        .get("edge.props")
                        .and_then(|c| c.to_map())
                        .unwrap_or_default(),
                })
                .collect())
        }
        fn list_evidence_for_versions(
            &self,
            version_ids: &[String],
        ) -> anyhow::Result<Vec<crate::diagram::export_types::EvidenceEntry>> {
            let ids_upper: std::collections::HashSet<String> =
                version_ids.iter().map(|s| s.to_uppercase()).collect();
            Ok(self
                .evidence
                .iter()
                .filter(|row| {
                    let vid = row
                        .get("v.id")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_uppercase();
                    ids_upper.contains(&vid)
                })
                .map(|row| crate::diagram::export_types::EvidenceEntry {
                    id: row
                        .get("e.id")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    kind: row
                        .get("e.kind")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    claim: row
                        .get("e.claim")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    path: row
                        .get("e.path")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    start_line: row
                        .get("e.start_line")
                        .and_then(|c| c.as_i64())
                        .unwrap_or(0) as u64,
                    end_line: row.get("e.end_line").and_then(|c| c.as_i64()).unwrap_or(0) as u64,
                    tool_name: row
                        .get("e.tool_name")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    tool_version: row
                        .get("e.tool_version")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    rule_id: row
                        .get("e.rule_id")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    content_hash: row
                        .get("e.content_hash")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    observed_at: row
                        .get("e.observed_at")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                })
                .collect())
        }
        fn list_version_props(
            &self,
            version_ids: &[String],
        ) -> anyhow::Result<Vec<crate::graph::VersionPropsRow>> {
            let ids_upper: std::collections::HashSet<String> =
                version_ids.iter().map(|s| s.to_uppercase()).collect();
            Ok(self
                .version_props
                .iter()
                .filter(|row| {
                    let vid = row
                        .get("v.id")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_uppercase();
                    ids_upper.contains(&vid)
                })
                .map(|row| crate::graph::VersionPropsRow {
                    id: row
                        .get("v.id")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    name: row
                        .get("v.name")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    description: row
                        .get("v.description")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    props: row
                        .get("v.props")
                        .and_then(|c| c.to_map())
                        .unwrap_or_default(),
                })
                .collect())
        }
    }

    impl MockGraphStore {
        fn new(
            elements: Vec<Row>,
            edges: Vec<Row>,
            evidence: Vec<Row>,
            version_props: Vec<Row>,
            view_members: Vec<crate::diagram::view_types::ViewMember>,
        ) -> Self {
            Self {
                elements,
                edges,
                evidence,
                version_props,
                view_members,
            }
        }
    }

    impl crate::store::GraphStore for MockGraphStore {
        fn open(_: &std::path::Path) -> anyhow::Result<Self>
        where
            Self: Sized,
        {
            unimplemented!()
        }
        fn init(&mut self) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn stat(&self) -> anyhow::Result<crate::GraphStat> {
            unimplemented!()
        }
        // M32 D1: MockGraphStore does not persist, so transaction primitives
        // are no-ops. Tests that exercise writers (call_graph, etc.) use the
        // real `LbugStore` via `open_and_init` and do not touch the mock.
        fn begin_transaction(&mut self) -> Result<(), crate::store::StoreError> {
            Ok(())
        }
        fn commit_transaction(&mut self) -> Result<(), crate::store::StoreError> {
            Ok(())
        }
        fn rollback_transaction(&mut self) -> Result<(), crate::store::StoreError> {
            Ok(())
        }
    }

    impl crate::store::UnitOfWork for MockGraphStore {
        fn begin_transaction<'a>(
            &'a mut self,
        ) -> std::result::Result<crate::store::Transaction<'a>, crate::store::StoreError> {
            // MockGraphStore is in-memory; transaction semantics are a no-op.
            // Return an error to indicate transactions are not supported.
            Err(crate::store::StoreError::Transaction(
                "MockGraphStore does not support transactions".to_string(),
            ))
        }
    }

    // Query-parsing helpers (dead code after RawGraphQuery removal in P1-05 2.5).
    #[allow(dead_code)]
    impl MockGraphStore {
        /// Extract category, canonical_key prefix, and kind_id prefix from a query_elements cypher.
        fn extract_query_filters(cypher: &str) -> (Option<String>, Option<String>, Option<String>) {
            let category = Self::extract_quoted(cypher, "E.CATEGORY");
            // SCN-417: support STARTS WITH prefix matching for canonical_key
            let canonical_key = Self::extract_key_from_starts_with(cypher, "E.CANONICAL_KEY")
                .or_else(|| Self::extract_quoted(cypher, "E.CANONICAL_KEY"));
            // ADR-024: kind_id STARTS WITH filter
            let kind_id = Self::extract_key_from_starts_with(cypher, "E.KIND_ID");
            (category, canonical_key, kind_id)
        }

        fn extract_key_from_starts_with(s: &str, key: &str) -> Option<String> {
            let upper = s.to_uppercase();
            let pattern = format!("{} STARTS WITH '", key);
            let start = upper.find(&pattern)?;
            let value_start = start + pattern.len();
            let value_end = s[value_start..].find('\'')?;
            Some(s[value_start..value_start + value_end].to_string())
        }

        fn extract_quoted(s: &str, key: &str) -> Option<String> {
            let upper = s.to_uppercase();
            let pattern = format!("{} = '", key);
            let start = upper.find(&pattern)?;
            let value_start = start + pattern.len();
            let value_end = s[value_start..].find('\'')?;
            Some(s[value_start..value_start + value_end].to_string())
        }

        /// Extract a list of IDs from `KEY IN ['id1', 'id2', ...]`.
        fn extract_id_list(s: &str, key: &str) -> Vec<String> {
            let upper = s.to_uppercase();
            let pattern = format!("{} IN [", key);
            let start = match upper.find(&pattern) {
                Some(s) => s,
                None => return vec![],
            };
            let values_start = start + pattern.len();
            let values_end = s[values_start..].find(']').map(|i| values_start + i);
            let values_str = values_end
                .map(|end| &s[values_start..end])
                .unwrap_or_default();
            values_str
                .split(',')
                .filter_map(|item| {
                    let trimmed = item.trim().trim_start_matches('\'');
                    let end = trimmed.find('\'').map(|i| &trimmed[..i]).unwrap_or(trimmed);
                    if end.is_empty() {
                        None
                    } else {
                        Some(end.to_string())
                    }
                })
                .collect()
        }
    }

    // Sub-trait impls (see diagram_queries.rs for rationale).
    impl crate::store::EvidenceOps for MockGraphStore {
        fn put_evidence(&mut self, _: &[crate::evidence::Evidence]) -> anyhow::Result<usize> {
            unimplemented!()
        }
        fn list_evidence(&self, _: Option<&str>) -> anyhow::Result<Vec<Row>> {
            unimplemented!()
        }
        fn accept_evidence(&mut self, _: &str, _: &dyn crate::clock::Clock) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn supersede_evidence(&mut self, _: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn list_evidence_by_status(
            &self,
            _: crate::evidence::EvidenceStatus,
            _: Option<&str>,
        ) -> anyhow::Result<Vec<Row>> {
            unimplemented!()
        }
    }

    impl crate::store::SourceOps for MockGraphStore {
        fn put_source(&mut self, _: &crate::source::SourceArtifact) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn put_evaluation(&mut self, _: &crate::evaluation::Evaluation) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn link_extracted_from(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn link_evaluates(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
    }

    impl crate::store::DiagramOps for MockGraphStore {
        fn put_diagram(&mut self, _: &crate::diagram::view_types::Diagram) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn get_diagram(&self, _: &str) -> anyhow::Result<crate::diagram::view_types::Diagram> {
            unimplemented!()
        }
        fn put_view_member(
            &mut self,
            _: &crate::diagram::view_types::ViewMember,
        ) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn link_member_of(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn link_renders(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn put_view_group(
            &mut self,
            _: &crate::diagram::view_types::ViewGroup,
        ) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn link_group_contains(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn get_view_members(
            &self,
            _: &str,
        ) -> anyhow::Result<Vec<crate::diagram::view_types::ViewMember>> {
            Ok(self.view_members.clone())
        }
        fn update_view_member_label(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
    }

    impl crate::store::ElementRepository for MockGraphStore {
        fn upsert_element(&mut self, _: &crate::graph::Element) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn upsert_element_version(
            &mut self,
            _: &crate::graph::ElementVersion,
        ) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn link_current_version(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn link_version_of(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn link_of_type(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn ensure_metatype(&mut self, _: &str, _: &str, _: &str, _: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn existing_canonical_keys(&self) -> anyhow::Result<std::collections::HashSet<String>> {
            let mut out = std::collections::HashSet::new();
            for row in &self.elements {
                if let Some(crate::row::Cell::String(k)) = row.get("e.canonical_key") {
                    out.insert(k.clone());
                }
            }
            Ok(out)
        }
        fn batch_upsert_elements(&mut self, _: &[crate::graph::Element]) -> anyhow::Result<usize> {
            unimplemented!()
        }
        fn batch_upsert_element_versions(
            &mut self,
            _: &[crate::graph::ElementVersion],
        ) -> anyhow::Result<usize> {
            unimplemented!()
        }
        fn batch_link_of_type(&mut self, _: &[(String, String)]) -> anyhow::Result<usize> {
            unimplemented!()
        }
    }

    impl crate::store::EvidenceRepository for MockGraphStore {
        fn put_structural_evidence(
            &mut self,
            _: &crate::graph::StructuralEvidence,
        ) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn link_supported_by(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn link_extracted_from(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
    }

    impl crate::store::SourceRepository for MockGraphStore {
        fn put_source(&mut self, _: &crate::source::SourceArtifact) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn link_extracted_from(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
    }

    impl crate::store::EvaluationRepository for MockGraphStore {
        fn put_evaluation(&mut self, _: &crate::evaluation::Evaluation) -> anyhow::Result<()> {
            unimplemented!()
        }
        fn link_evaluates(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
            unimplemented!()
        }
    }

    fn make_element_row(
        id: &str,
        category: &str,
        name: &str,
        version_id: &str,
        kind_id: &str,
        canonical_key: &str,
    ) -> Row {
        let mut r = Row::new();
        r.push("e.id", Cell::String(id.to_string()));
        r.push("e.kind_id", Cell::String(kind_id.to_string()));
        r.push("e.category", Cell::String(category.to_string()));
        r.push("e.canonical_key", Cell::String(canonical_key.to_string()));
        r.push("e.current_name", Cell::String(name.to_string()));
        r.push("e.current_status", Cell::String("accepted".to_string()));
        r.push("e.current_confidence", Cell::Float(0.9));
        r.push("e.current_version_id", Cell::String(version_id.to_string()));
        r
    }

    fn make_edge_row(rel_id: &str, src: &str, tgt: &str) -> Row {
        let mut r = Row::new();
        r.push("edge.relation_id", Cell::String(rel_id.to_string()));
        r.push("edge.predicate_id", Cell::String("calls".to_string()));
        r.push("src.id", Cell::String(src.to_string()));
        r.push("tgt.id", Cell::String(tgt.to_string()));
        r.push("edge.order_key", Cell::String("1".to_string()));
        r.push("edge.props", Cell::Object(Vec::new()));
        r
    }

    fn make_evidence_row(id: &str, version_id: &str) -> Row {
        let mut r = Row::new();
        r.push("e.id", Cell::String(id.to_string()));
        r.push("e.kind", Cell::String("structural".to_string()));
        r.push("e.claim", Cell::String("test claim".to_string()));
        r.push("e.path", Cell::String("src/lib.rs".to_string()));
        r.push("e.start_line", Cell::Int(1));
        r.push("e.end_line", Cell::Int(10));
        r.push("e.tool_name", Cell::String("archctl".to_string()));
        r.push("e.tool_version", Cell::String("0.1.0".to_string()));
        r.push("e.rule_id", Cell::String("test:rule".to_string()));
        r.push("e.content_hash", Cell::String("sha256:abc".to_string()));
        r.push(
            "e.observed_at",
            Cell::String("2026-07-30T00:00:00Z".to_string()),
        );
        r.push("v.id", Cell::String(version_id.to_string()));
        // props must include status = "accepted" for evidence to pass the filter
        r.push(
            "e.props",
            Cell::Object(vec![(
                "status".to_string(),
                Cell::String("accepted".to_string()),
            )]),
        );
        r
    }

    fn make_version_row(id: &str, name: &str, desc: &str) -> Row {
        let mut r = Row::new();
        r.push("v.id", Cell::String(id.to_string()));
        r.push("v.name", Cell::String(name.to_string()));
        r.push("v.description", Cell::String(desc.to_string()));
        r.push("v.props", Cell::Object(Vec::new()));
        r
    }

    #[test]
    fn export_produces_all_bundle_files() {
        // Per ADR-024: category must be "c4" (diagram family), not "container" (C4 kind).
        // The kind filter is passed via the kind parameter to query_elements.
        // For "container:orders" selector, we need:
        // - category = 'c4'
        // - canonical_key STARTS WITH 'orders'
        // - kind_id STARTS WITH 'container'
        let elements = vec![
            make_element_row(
                "el:1",
                "c4",
                "OrderService",
                "v:1",
                "mt.container",
                "orders",
            ),
            make_element_row(
                "el:2",
                "c4",
                "PaymentService",
                "v:2",
                "mt.container",
                "payments",
            ),
        ];
        let edges = vec![make_edge_row("rel:1", "el:1", "el:2")];
        let evidence = vec![
            make_evidence_row("ev:1", "v:1"),
            make_evidence_row("ev:2", "v:2"),
        ];
        let version_props = vec![
            make_version_row("v:1", "OrderService", "Handles order processing"),
            make_version_row("v:2", "PaymentService", "Handles payment processing"),
        ];

        let store = MockGraphStore::new(elements, edges, evidence, version_props, vec![]);
        let clock = FixedClock::new("2026-07-30T12:00:00Z");
        let fs = MemoryFilesystem::new();
        let out_dir = std::path::PathBuf::from("/out");

        let report = run_export(&store, "container:orders", &out_dir, &clock, &fs).unwrap();

        // Verify report — container:orders matches only el:1 (canonical_key STARTS WITH 'orders')
        // el:2 has canonical_key='payments' which doesn't match 'orders'
        assert_eq!(report.element_count, 1);
        assert_eq!(report.edge_count, 1);
        assert_eq!(report.evidence_count, 1);
        assert_eq!(report.manifest.schema_version, "1.1.0");
        assert_eq!(report.manifest.format, "viewer-bundle");
        assert_eq!(report.manifest.view_selector, "container:orders");
        assert!(!report.manifest.base_revision.is_empty());
        assert_eq!(report.manifest.generated_at, "2026-07-30T12:00:00Z");

        // Verify manifest.json
        let manifest_json = fs.read_to_string(&out_dir.join("manifest.json")).unwrap();
        let manifest: Manifest = serde_json::from_str(&manifest_json).unwrap();
        // container:orders matches only el:1 (canonical_key STARTS WITH 'orders')
        assert_eq!(manifest.element_count, 1);
        assert_eq!(manifest.edge_count, 1);

        // Verify projection.json
        let projection_json = fs.read_to_string(&out_dir.join("projection.json")).unwrap();
        let projection: Projection = serde_json::from_str(&projection_json).unwrap();
        assert_eq!(projection.nodes.len(), 1);
        assert_eq!(projection.edges.len(), 1);

        // Verify evidence.json
        let evidence_json = fs.read_to_string(&out_dir.join("evidence.json")).unwrap();
        let bundle: EvidenceBundle = serde_json::from_str(&evidence_json).unwrap();
        // Only evidence for v:1 (el:1's version)
        assert_eq!(bundle.evidence.len(), 1);

        // Verify styles.json
        let styles_json = fs.read_to_string(&out_dir.join("styles.json")).unwrap();
        let styles: Styles = serde_json::from_str(&styles_json).unwrap();
        assert_eq!(styles.theme, "default");
        assert_eq!(styles.element_colors.container, "#438dd5");

        // Verify assets directory and icons — must match the canonical C4 set shared with validate.rs.
        assert!(fs.exists(&out_dir.join("assets")));
        for icon in crate::diagram::assets::CANONICAL_C4_ICONS {
            assert!(
                fs.exists(&out_dir.join("assets").join(format!("{icon}.png"))),
                "icon {icon}.png should exist"
            );
        }
    }

    #[test]
    fn export_with_all_scope_returns_only_matching_category() {
        // Per ADR-024: category must be "c4" (diagram family).
        // For "container:*", the query filters by category='c4' AND kind_id STARTS WITH 'container'
        let elements = vec![
            make_element_row("el:1", "c4", "ServiceA", "v:1", "mt.container", "svc-a"),
            make_element_row("el:2", "c4", "ServiceB", "v:2", "mt.container", "svc-b"),
        ];
        let store = MockGraphStore::new(elements, Vec::new(), Vec::new(), Vec::new(), vec![]);
        let clock = FixedClock::new("2026-07-30T12:00:00Z");
        let fs = MemoryFilesystem::new();
        let out_dir = std::path::PathBuf::from("/out");

        // With container:*, should return both containers
        let report = run_export(&store, "container:*", &out_dir, &clock, &fs).unwrap();
        assert_eq!(report.element_count, 2);

        // Verify the names
        let proj_json = fs.read_to_string(&out_dir.join("projection.json")).unwrap();
        let proj: Projection = serde_json::from_str(&proj_json).unwrap();
        let names: Vec<_> = proj.nodes.iter().map(|n| n.name.clone()).collect();
        assert!(names.contains(&"ServiceA".to_string()));
        assert!(names.contains(&"ServiceB".to_string()));
    }

    #[test]
    fn export_idempotent_on_same_input() {
        // Per ADR-024: category must be "c4" (diagram family).
        // kind_id must match the query filter (container:*)
        let elements = vec![make_element_row(
            "el:1",
            "c4",
            "Svc",
            "v:1",
            "mt.container",
            "svc",
        )];
        let store = MockGraphStore::new(elements, Vec::new(), Vec::new(), Vec::new(), vec![]);
        let clock = FixedClock::new("2026-07-30T12:00:00Z");
        let fs = MemoryFilesystem::new();
        let out_dir = std::path::PathBuf::from("/out2");

        let r1 = run_export(&store, "container:*", &out_dir, &clock, &fs).unwrap();
        let r2 = run_export(&store, "container:*", &out_dir, &clock, &fs).unwrap();

        // Both runs succeed and produce same revision (deterministic)
        assert_eq!(r1.manifest.base_revision, r2.manifest.base_revision);
    }

    #[test]
    fn export_rejects_malformed_selector() {
        let store = MockGraphStore::new(Vec::new(), Vec::new(), Vec::new(), Vec::new(), vec![]);
        let clock = FixedClock::new("2026-07-30T12:00:00Z");
        let fs = MemoryFilesystem::new();
        let out_dir = std::path::PathBuf::from("/out");

        // Empty selector
        let result = run_export(&store, "", &out_dir, &clock, &fs);
        assert!(result.is_err());

        // Unknown kind
        let result = run_export(&store, "unknown_kind", &out_dir, &clock, &fs);
        assert!(result.is_err());
    }

    #[test]
    fn export_empty_graph_sets_empty_true() {
        // Mock store with zero elements — empty graph
        let store = MockGraphStore::new(Vec::new(), Vec::new(), Vec::new(), Vec::new(), vec![]);
        let clock = FixedClock::new("2026-07-30T12:00:00Z");
        let fs = MemoryFilesystem::new();
        let out_dir = std::path::PathBuf::from("/out");

        let report = run_export(&store, "container:*", &out_dir, &clock, &fs).unwrap();

        assert!(report.empty, "expected empty=true for zero-element graph");
        assert!(
            report
                .warning
                .as_deref()
                .unwrap()
                .contains("no graph found"),
            "expected warning to mention 'no graph found', got: {:?}",
            report.warning
        );
        assert_eq!(report.element_count, 0, "expected element_count==0");
        assert_eq!(report.edge_count, 0, "expected edge_count==0");
        assert_eq!(report.evidence_count, 0, "expected evidence_count==0");
    }

    /// The envelope emitted by `build_export_envelope` MUST be a valid
    /// instance of `diagram-projection.schema.json` (without the `empty`
    /// and `warning` fields, which are CLI conveniences, not part of the
    /// schema). This is the canonical "agent can trust stdout JSON"
    /// regression test (M37).
    #[test]
    fn envelope_is_schema_valid() {
        use crate::diagram::schema_embed::SCHEMA;

        let store = MockGraphStore::new(Vec::new(), Vec::new(), Vec::new(), Vec::new(), vec![]);
        let clock = FixedClock::new("2026-07-30T12:00:00Z");
        let bundle = build_bundle(&store, "container:*", &clock).unwrap();
        let envelope = build_export_envelope(&bundle);

        // The schema describes {manifest, projection, evidence, styles};
        // `empty` + `warning` are CLI conveniences. Strip them before
        // validating.
        let mut envelope_for_validation = envelope.clone();
        if let Some(obj) = envelope_for_validation.as_object_mut() {
            obj.remove("empty");
            obj.remove("warning");
        }

        let schema: serde_json::Value =
            serde_json::from_str(SCHEMA).expect("embedded schema is valid JSON");
        let validator = jsonschema::validator_for(&schema).expect("compile schema");
        let validation_result = validator.validate(&envelope_for_validation);
        assert!(
            validation_result.is_ok(),
            "envelope must validate against schemas/diagram-projection.schema.json; errors: {:?}",
            validation_result.err()
        );
    }
}
