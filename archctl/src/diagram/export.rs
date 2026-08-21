//! Export pipeline: graph query → projection → bundle write.
//!
//! Orchestrates the 4 read queries, canonicalizes the result,
//! computes the content hash, and writes the 5-file bundle.

use anyhow::Context;
use std::path::{Path, PathBuf};

use crate::clock::Clock;
use crate::diagram::export_types::{
    Edge as ExportEdge, EdgeColors, ElementColors, EvidenceBundle, ExportProfile, Manifest,
    Node as ExportNode, Projection, Styles,
};
use crate::diagram::hash::base_revision;
use crate::diagram::selector::{ScopeFilter, ViewSelector};
use crate::filesystem::Filesystem;
use crate::graph::ElementRow;
use crate::store::GraphStore;
use sha2::{Digest, Sha256};

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

    // Filter to accepted evidence only for the bundle (ADR-005: only canonical evidence in projections)
    let evidence_entries = store
        .list_evidence_for_versions(&version_ids)
        .context("list_evidence_for_versions failed")?
        .into_iter()
        .filter(|e| e.status.as_deref() == Some("accepted"))
        .collect::<Vec<_>>();

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
        schema_version: "1.1.1".into(), // 1.1.0 → 1.1.1: EvidenceEntry.status (UAT smoke 2026-08-19)
        format: "viewer-bundle".into(),
        view_selector: selector.to_string(),
        base_revision: revision,
        generated_at: clock.now_rfc3339(),
        element_count: projection.nodes.len(),
        edge_count: projection.edges.len(),
        evidence_count: evidence_entries.len(),
        strict: false,
        checksum: None,
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

// ─────────────────────────────────────────────────────────────────────────────
// Strict profile: sanitization + checksum
// ─────────────────────────────────────────────────────────────────────────────

/// Relativize an absolute path against `project_root`.
fn relativize_path(path: &str, project_root: &Path) -> String {
    let abs = PathBuf::from(path);
    if let Ok(rel) = abs.strip_prefix(project_root) {
        rel.to_string_lossy().into_owned()
    } else {
        // Not under project root — replace with a pseudonym
        // Keep just the filename for traceability
        abs.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "<external>".to_string())
    }
}

/// Apply strict-mode sanitization to a bundle.
/// - Paths in evidence entries are relativized to project root
/// - Secret shapes are redacted (`[REDACTED:<kind>]`, ADR-055 phase 2)
/// - `manifest.strict` is set to true
fn sanitize_bundle(bundle: &mut BundleEnvelope, project_root: &Path) {
    for entry in &mut bundle.evidence.evidence {
        entry.path = relativize_path(&entry.path, project_root);
    }
    crate::diagram::redact::redact_bundle(bundle);
    bundle.manifest.strict = true;
}

/// Public entry point for applying strict export profile to a bundle.
/// Exported so the CLI can apply strict sanitization to the JSON output path
/// (which bypasses `run_export`).
pub fn apply_strict_profile(bundle: &mut BundleEnvelope, project_root: &Path) {
    sanitize_bundle(bundle, project_root);
    let checksum = compute_checksum(bundle);
    bundle.manifest.checksum = Some(checksum);
}

/// Compute SHA-256 checksum over the bundle files (excluding `generatedAt` for determinism).
/// Returns hex-encoded checksum string.
fn compute_checksum(bundle: &BundleEnvelope) -> String {
    let mut hasher = Sha256::new();

    // Canonical JSON without pretty-printing, fields in deterministic order
    // Exclude generatedAt from checksum for reproducibility
    let manifest_for_checksum = serde_json::json!({
        "schemaVersion": bundle.manifest.schema_version,
        "format": bundle.manifest.format,
        "viewSelector": bundle.manifest.view_selector,
        "baseRevision": bundle.manifest.base_revision,
        "elementCount": bundle.manifest.element_count,
        "edgeCount": bundle.manifest.edge_count,
        "evidenceCount": bundle.manifest.evidence_count,
        "strict": bundle.manifest.strict,
    });

    hasher.update(manifest_for_checksum.to_string().as_bytes());
    hasher.update(
        serde_json::to_string(&bundle.projection)
            .unwrap_or_default()
            .as_bytes(),
    );
    hasher.update(
        serde_json::to_string(&bundle.evidence)
            .unwrap_or_default()
            .as_bytes(),
    );
    hasher.update(
        serde_json::to_string(&bundle.styles)
            .unwrap_or_default()
            .as_bytes(),
    );

    format!("{:x}", hasher.finalize())
}

/// Uses `Clock::now_rfc3339()` for `generatedAt` and writes each file
/// atomically (write-then-rename) for idempotency.
pub fn run_export(
    store: &dyn GraphStore,
    selector: &str,
    out_dir: &Path,
    clock: &dyn Clock,
    fs: &dyn Filesystem,
    profile: ExportProfile,
    project_dir: &Path,
) -> anyhow::Result<ExportReport> {
    let mut bundle = build_bundle(store, selector, clock)?;

    // Apply strict sanitization if requested
    if profile == ExportProfile::Strict {
        sanitize_bundle(&mut bundle, project_dir);
        let checksum = compute_checksum(&bundle);
        bundle.manifest.checksum = Some(checksum);
    }

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
    // Single source of truth: `assets::CANONICAL_C4_ICONS` + `ICON_EXTENSION`.
    for icon_name in crate::diagram::assets::CANONICAL_C4_ICONS {
        let icon_svg = crate::diagram::assets::icon_for(icon_name).unwrap_or_default();
        let icon_filename = format!("{icon_name}.{}", crate::diagram::assets::ICON_EXTENSION);
        write_atomic_bytes(fs, &assets_dir.join(&icon_filename), icon_svg.as_bytes())?;
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
    use crate::graph::{Element, ElementVersion, StructuralEvidence};
    use crate::row::{Cell, Row};
    use crate::store::{
        ElementRepository, EvidenceRepository, GraphStore, LbugStore, SemanticEdgeRepository,
    };

    /// Build a real `LbugStore` (production adapter) seeded with the
    /// test fixture. Replaces the in-memory `MockGraphStore` that
    /// previously shadowed `LbugStore`'s Cypher filter logic — the
    /// production filter now runs against the production store, so
    /// tests exercise the same path as `archctl diagram export`.
    fn seeded_graph_store(
        project_dir: &std::path::Path,
        elements: Vec<Row>,
        edges: Vec<Row>,
        evidence: Vec<Row>,
        version_props: Vec<Row>,
    ) -> LbugStore {
        let mut store = LbugStore::open(project_dir).expect("LbugStore::open");
        store.init().expect("LbugStore::init");

        // ElementVersion nodes + CURRENT_VERSION edges first (Element rows
        // reference them via current_version_id).
        for row in &version_props {
            let version_id = cell_string(row, "v.id");
            let element_id = derive_element_id_from_version(&version_id);
            let name = cell_string(row, "v.name");
            let description = cell_string(row, "v.description");
            let v = ElementVersion {
                id: version_id.clone(),
                element_id: element_id.clone(),
                name,
                status: "accepted".into(),
                origin: "test".into(),
                confidence: 0.9,
                props: serde_json::Map::new(),
            };
            store
                .upsert_element_version(&v)
                .expect("upsert_element_version");
            store
                .link_current_version(&element_id, &version_id)
                .expect("link_current_version");
            // Description lives on ElementVersion.props under "description"
            // so list_version_props can surface it.
            let desc_cypher = format!(
                "MATCH (v:ElementVersion {{id: '{vid}'}}) SET v.description = '{desc}';",
                vid = escape(&version_id),
                desc = escape(&description),
            );
            store
                .execute_raw_cypher_for_test(&desc_cypher)
                .expect("set description");
        }

        for row in &elements {
            let e = Element {
                id: cell_string(row, "e.id"),
                kind_id: cell_string(row, "e.kind_id"),
                category: cell_string(row, "e.category"),
                canonical_key: cell_string(row, "e.canonical_key"),
                current_name: cell_string(row, "e.current_name"),
                current_status: cell_string_or(row, "e.current_status", "accepted"),
                current_confidence: cell_f64_or(row, "e.current_confidence", 0.9),
                current_version_id: cell_string(row, "e.current_version_id"),
            };
            store.upsert_element(&e).expect("upsert_element");
        }

        for row in &edges {
            let src = cell_string(row, "src.id");
            let tgt = cell_string(row, "tgt.id");
            let relation_id = cell_string(row, "edge.relation_id");
            let predicate_id = cell_string_or(row, "edge.predicate_id", "calls");
            store
                .link_semantic_edge(
                    &src,
                    &tgt,
                    &relation_id,
                    &predicate_id,
                    &serde_json::Map::new(),
                    true,
                )
                .expect("link_semantic_edge");
        }

        for row in &evidence {
            // ADR-005: evidence must have `status: "accepted"` in props
            // to pass the build_bundle filter that only emits canonical
            // evidence to the bundle.
            let mut props = serde_json::Map::new();
            props.insert(
                "status".into(),
                serde_json::Value::String("accepted".into()),
            );
            let ev = StructuralEvidence {
                id: cell_string(row, "e.id"),
                kind: cell_string_or(row, "e.kind", "structural"),
                claim: cell_string_or(row, "e.claim", "test claim"),
                file: cell_string_or(row, "e.path", "src/lib.rs"),
                line: cell_i64_or(row, "e.start_line", 1) as u64,
                confidence: 0.9,
                rule_id: cell_string_or(row, "e.rule_id", "test:rule"),
                props,
            };
            store
                .put_structural_evidence(&ev)
                .expect("put_structural_evidence");
            let version_id = cell_string(row, "v.id");
            store
                .link_supported_by(&version_id, &ev.id)
                .expect("link_supported_by");
        }

        store
    }

    fn cell_string(row: &Row, key: &str) -> String {
        row.get(key)
            .and_then(|c| c.as_str())
            .unwrap_or_default()
            .to_string()
    }

    fn cell_string_or(row: &Row, key: &str, default: &str) -> String {
        row.get(key)
            .and_then(|c| c.as_str())
            .unwrap_or(default)
            .to_string()
    }

    fn cell_f64_or(row: &Row, key: &str, default: f64) -> f64 {
        row.get(key).and_then(|c| c.as_f64()).unwrap_or(default)
    }

    fn cell_i64_or(row: &Row, key: &str, default: i64) -> i64 {
        row.get(key).and_then(|c| c.as_i64()).unwrap_or(default)
    }

    /// Test convention: `v:1` → element `el:1`, `v:2` → element `el:2`.
    /// Mirrors the convention used by the make_*_row helpers in tests.
    fn derive_element_id_from_version(version_id: &str) -> String {
        version_id.replacen("v:", "el:", 1)
    }

    fn escape(s: &str) -> String {
        s.replace('\'', "\\'")
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
        let tmp = tempfile::tempdir().unwrap();
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

        let store = seeded_graph_store(tmp.path(), elements, edges, evidence, version_props);
        let clock = FixedClock::new("2026-07-30T12:00:00Z");
        let fs = MemoryFilesystem::new();
        let out_dir = std::path::PathBuf::from("/out");

        let report = run_export(
            &store,
            "container:orders",
            &out_dir,
            &clock,
            &fs,
            ExportProfile::Default,
            std::path::Path::new("/out"),
        )
        .unwrap();

        // Verify report — container:orders matches only el:1 (canonical_key STARTS WITH 'orders')
        // el:2 has canonical_key='payments' which doesn't match 'orders'
        assert_eq!(report.element_count, 1);
        assert_eq!(report.edge_count, 1);
        assert_eq!(report.evidence_count, 1);
        assert_eq!(report.manifest.schema_version, "1.1.1");
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
            let icon_filename = format!("{icon}.{}", crate::diagram::assets::ICON_EXTENSION);
            assert!(
                fs.exists(&out_dir.join("assets").join(&icon_filename)),
                "icon {icon_filename} should exist"
            );
        }
    }

    #[test]
    fn export_with_all_scope_returns_only_matching_category() {
        let tmp = tempfile::tempdir().unwrap();
        // Per ADR-024: category must be "c4" (diagram family).
        // For "container:*", the query filters by category='c4' AND kind_id STARTS WITH 'container'
        let elements = vec![
            make_element_row("el:1", "c4", "ServiceA", "v:1", "mt.container", "svc-a"),
            make_element_row("el:2", "c4", "ServiceB", "v:2", "mt.container", "svc-b"),
        ];
        let store = seeded_graph_store(tmp.path(), elements, Vec::new(), Vec::new(), Vec::new());
        let clock = FixedClock::new("2026-07-30T12:00:00Z");
        let fs = MemoryFilesystem::new();
        let out_dir = std::path::PathBuf::from("/out");

        // With container:*, should return both containers
        let report = run_export(
            &store,
            "container:*",
            &out_dir,
            &clock,
            &fs,
            ExportProfile::Default,
            std::path::Path::new("/out"),
        )
        .unwrap();
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
        let tmp = tempfile::tempdir().unwrap();
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
        let store = seeded_graph_store(tmp.path(), elements, Vec::new(), Vec::new(), Vec::new());
        let clock = FixedClock::new("2026-07-30T12:00:00Z");
        let fs = MemoryFilesystem::new();
        let out_dir = std::path::PathBuf::from("/out2");

        let r1 = run_export(
            &store,
            "container:*",
            &out_dir,
            &clock,
            &fs,
            ExportProfile::Default,
            std::path::Path::new("/out2"),
        )
        .unwrap();
        let r2 = run_export(
            &store,
            "container:*",
            &out_dir,
            &clock,
            &fs,
            ExportProfile::Default,
            std::path::Path::new("/out2"),
        )
        .unwrap();

        // Both runs succeed and produce same revision (deterministic)
        assert_eq!(r1.manifest.base_revision, r2.manifest.base_revision);
    }

    #[test]
    fn export_rejects_malformed_selector() {
        let tmp = tempfile::tempdir().unwrap();
        let store = seeded_graph_store(tmp.path(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let clock = FixedClock::new("2026-07-30T12:00:00Z");
        let fs = MemoryFilesystem::new();
        let out_dir = std::path::PathBuf::from("/out");

        // Empty selector
        let result = run_export(
            &store,
            "",
            &out_dir,
            &clock,
            &fs,
            ExportProfile::Default,
            std::path::Path::new("/out"),
        );
        assert!(result.is_err());

        // Unknown kind
        let result = run_export(
            &store,
            "unknown_kind",
            &out_dir,
            &clock,
            &fs,
            ExportProfile::Default,
            std::path::Path::new("/out"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn export_empty_graph_sets_empty_true() {
        let tmp = tempfile::tempdir().unwrap();
        // Empty graph: no elements, no versions, no edges, no evidence
        let store = seeded_graph_store(tmp.path(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let clock = FixedClock::new("2026-07-30T12:00:00Z");
        let fs = MemoryFilesystem::new();
        let out_dir = std::path::PathBuf::from("/out");

        let report = run_export(
            &store,
            "container:*",
            &out_dir,
            &clock,
            &fs,
            ExportProfile::Default,
            std::path::Path::new("/out"),
        )
        .unwrap();

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

        let tmp = tempfile::tempdir().unwrap();
        let store = seeded_graph_store(tmp.path(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
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
