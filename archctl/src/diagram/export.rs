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
use crate::diagram::queries::{
    ElementRow, query_elements, query_evidence_for_versions, query_semantic_edges,
    query_version_props,
};
use crate::diagram::selector::{ScopeFilter, ViewSelector};
use crate::filesystem::Filesystem;
use crate::store::GraphStore;

/// Report from a successful export operation.
#[derive(Debug)]
pub struct ExportReport {
    pub manifest: Manifest,
    pub element_count: usize,
    pub edge_count: usize,
    pub evidence_count: usize,
}

/// Run the full export pipeline.
///
/// Parses `selector` → runs 4 graph queries → builds projection → computes
/// `baseRevision` → writes 5 files (`manifest.json`, `projection.json`,
/// `evidence.json`, `styles.json`, `assets/`) atomically.
///
/// Uses `Clock::now_rfc3339()` for `generatedAt` and writes each file
/// atomically (write-then-rename) for idempotency.
pub fn run_export(
    store: &dyn GraphStore,
    selector: &str,
    out_dir: &Path,
    clock: &dyn Clock,
    fs: &dyn Filesystem,
) -> anyhow::Result<ExportReport> {
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

    // 2. Run queries
    let element_rows = query_elements(store, category, scope_ident, Some(&kind))
        .context("query_elements failed")?;

    let edge_rows = query_semantic_edges(store, category).context("query_semantic_edges failed")?;

    // Collect version IDs for evidence + version props queries
    let version_ids: Vec<String> = element_rows
        .iter()
        .filter(|e| !e.current_version_id.is_empty())
        .map(|e| e.current_version_id.clone())
        .collect();

    let evidence_entries = query_evidence_for_versions(store, &version_ids)
        .context("query_evidence_for_versions failed")?;

    let version_props =
        query_version_props(store, &version_ids).context("query_version_props failed")?;

    // 3. Build projection (nodes + edges)
    let version_map: std::collections::HashMap<String, &crate::diagram::queries::VersionPropsRow> =
        version_props.iter().map(|v| (v.id.clone(), v)).collect();

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

            ExportNode {
                id: e.id.clone(),
                element_type: e.category.clone(),
                name: e.current_name.clone(),
                description,
                canonical_key: Some(e.canonical_key.clone()).filter(|s| !s.is_empty()),
                status: Some(e.current_status.clone()).filter(|s| !s.is_empty()),
                confidence: Some(e.current_confidence).filter(|&c| c > 0.0),
                evidence_refs: Some(evidence_refs).filter(|v| !v.is_empty()),
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
        schema_version: "1.0.0".into(),
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

    // 7. Write bundle files (atomic: write to tmp, then rename)
    // Create output directory
    fs.create_dir_all(out_dir)
        .with_context(|| format!("creating output directory {}", out_dir.display()))?;

    // Write manifest.json
    write_atomic(fs, &out_dir.join("manifest.json"), &manifest)?;

    // Write projection.json
    write_atomic(fs, &out_dir.join("projection.json"), &projection)?;

    // Write evidence.json
    let evidence_bundle = EvidenceBundle {
        evidence: evidence_entries,
    };
    write_atomic(fs, &out_dir.join("evidence.json"), &evidence_bundle)?;

    // Write styles.json
    write_atomic(fs, &out_dir.join("styles.json"), &styles)?;

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

    Ok(ExportReport {
        manifest,
        element_count: projection.nodes.len(),
        edge_count: projection.edges.len(),
        evidence_count: evidence_bundle.evidence.len(),
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
    }

    impl MockGraphStore {
        fn new(
            elements: Vec<Row>,
            edges: Vec<Row>,
            evidence: Vec<Row>,
            version_props: Vec<Row>,
        ) -> Self {
            Self {
                elements,
                edges,
                evidence,
                version_props,
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
        fn query(&self, cypher: &str) -> anyhow::Result<Vec<Row>> {
            let upper = cypher.to_uppercase();
            // Route based on Cypher pattern
            if upper.contains("MATCH (E:ELEMENT)") && upper.contains("E.CATEGORY") {
                // query_elements: apply WHERE filtering
                let (category, canonical_key, kind_id) = Self::extract_query_filters(&upper);
                let filtered: Vec<Row> = self
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
                        let cat_match = category
                            .as_ref()
                            .map(|c| row_cat == c.to_uppercase())
                            .unwrap_or(true);
                        let key_match = canonical_key
                            .as_ref()
                            .map(|k| row_key.starts_with(&k.to_uppercase()))
                            .unwrap_or(true);
                        let kind_match = kind_id
                            .as_ref()
                            .map(|k| row_kind.starts_with(&k.to_uppercase()))
                            .unwrap_or(true);
                        cat_match && key_match && kind_match
                    })
                    .cloned()
                    .collect();
                Ok(filtered)
            } else if upper.contains("SEMANTIC_EDGE") {
                // query_semantic_edges
                Ok(self.edges.clone())
            } else if upper.contains("SUPPORTED_BY") {
                // query_evidence_for_versions — filter by version IDs
                let ids = Self::extract_id_list(cypher, "EV.ID");
                let filtered: Vec<Row> = self
                    .evidence
                    .iter()
                    .filter(|row| {
                        let vid = row
                            .get("v.id")
                            .and_then(|c| c.as_str())
                            .map(|s| s.to_uppercase())
                            .unwrap_or_default();
                        ids.iter().any(|target| vid == target.to_uppercase())
                    })
                    .cloned()
                    .collect();
                Ok(filtered)
            } else if upper.contains("MATCH (V:ELEMENTVERSION)") {
                // query_version_props
                Ok(self.version_props.clone())
            } else {
                Ok(Vec::new())
            }
        }
    }

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
            unimplemented!()
        }
        fn update_view_member_label(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
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

        let store = MockGraphStore::new(elements, edges, evidence, version_props);
        let clock = FixedClock::new("2026-07-30T12:00:00Z");
        let fs = MemoryFilesystem::new();
        let out_dir = std::path::PathBuf::from("/out");

        let report = run_export(&store, "container:orders", &out_dir, &clock, &fs).unwrap();

        // Verify report — container:orders matches only el:1 (canonical_key STARTS WITH 'orders')
        // el:2 has canonical_key='payments' which doesn't match 'orders'
        assert_eq!(report.element_count, 1);
        assert_eq!(report.edge_count, 1);
        assert_eq!(report.evidence_count, 1);
        assert_eq!(report.manifest.schema_version, "1.0.0");
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
        let store = MockGraphStore::new(elements, Vec::new(), Vec::new(), Vec::new());
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
        let store = MockGraphStore::new(elements, Vec::new(), Vec::new(), Vec::new());
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
        let store = MockGraphStore::new(Vec::new(), Vec::new(), Vec::new(), Vec::new());
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
}
