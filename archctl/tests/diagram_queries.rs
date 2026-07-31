// Integration tests for diagram query functions (SCN-050..053).
//
// These tests call the public query functions with a TinyGraphStore
// stub that routes based on Cypher pattern keywords. The stub implements
// GraphStore so the actual query function logic is exercised end-to-end.
//
// SCN-050: MATCH (e:Element) filtered by category → returns expected nodes.
// SCN-051: MATCH (e:Element)-[:SEMANTIC_EDGE]->(r) → returns expected edges.
// SCN-052: query_evidence_for_versions filters to Accepted status (in Rust).
// SCN-053: query_version_props returns version properties.

use archctl::diagram::queries::{
    query_elements, query_evidence_for_versions, query_semantic_edges, query_version_props,
};
use archctl::row::{Cell, Row};
use archctl::store::GraphStore;

/// Build a Row from a flat list of (column, value) pairs.
fn row_from_pairs(pairs: Vec<(&str, Cell)>) -> Row {
    let mut r = Row::new();
    for (k, v) in pairs {
        r.push(k, v);
    }
    r
}

fn evidence_props_cell(status: &str) -> Cell {
    Cell::Object(vec![(
        "status".to_string(),
        Cell::String(status.to_string()),
    )])
}

fn empty_props_cell() -> Cell {
    Cell::Object(Vec::new())
}

/// A minimal GraphStore stub that routes based on Cypher keyword presence.
/// This exercises the actual query function logic (serialization, error handling, etc.)
/// without needing a real lbug database.
struct TinyGraphStore {
    elements: Vec<Row>,
    edges: Vec<Row>,
    evidence: Vec<Row>,
    versions: Vec<Row>,
}

impl TinyGraphStore {
    fn new(
        elements: Vec<Row>,
        edges: Vec<Row>,
        evidence: Vec<Row>,
        versions: Vec<Row>,
    ) -> Self {
        Self {
            elements,
            edges,
            evidence,
            versions,
        }
    }

    /// Extract category and canonical_key filter from a query_elements cypher string.
    fn extract_element_filter_from_cypher(cypher: &str) -> (Option<String>, Option<String>) {
        let upper = cypher.to_uppercase();
        let category = Self::extract_quoted(&upper, "E.CATEGORY");
        let canonical_key = Self::extract_quoted(&upper, "E.CANONICAL_KEY");
        (category, canonical_key)
    }

    /// Extract category filter from a query_semantic_edges cypher string.
    fn extract_category_from_semantic_cypher(cypher: &str) -> String {
        Self::extract_quoted(cypher, "SRC.CATEGORY")
            .or_else(|| Self::extract_quoted(cypher, "SRC.CATEGORY"))
            .unwrap_or_else(|| "container".to_string())
    }

    /// Extract a quoted string value for a given key from a cypher string.
    fn extract_quoted(s: &str, key: &str) -> Option<String> {
        let pattern = format!("{} = '", key);
        let start = s.find(&pattern)?;
        let value_start = start + pattern.len();
        let value_end = s[value_start..].find('\'')?;
        Some(s[value_start..value_start + value_end].to_string())
    }

    /// Extract version IDs from a query_evidence_for_versions cypher WHERE clause.
    /// e.g., "WHERE ev.id IN [v:1, v:2]" -> ["v:1", "v:2"]
    fn extract_version_ids_from_cypher(cypher: &str) -> Vec<String> {
        let upper = cypher.to_uppercase();
        // Find "EV.ID IN [" pattern
        let pattern = "EV.ID IN [";
        let start = match upper.find(pattern) {
            Some(pos) => pos + pattern.len(),
            None => return Vec::new(),
        };
        let rest = &upper[start..];
        // Extract IDs inside brackets
        let mut ids = Vec::new();
        let mut current = rest;
        while !current.is_empty() && !current.starts_with(']') {
            // Skip whitespace and commas
            while !current.is_empty()
                && (current.starts_with(' ') || current.starts_with(','))
            {
                current = &current[1..];
            }
            if current.is_empty() || current.starts_with(']') {
                break;
            }
            // Extract ID (alphanumeric with colon and dash)
            let end = current
                .find(|c: char| c == ']' || c == ',' || c == ' ')
                .unwrap_or(current.len());
            let id = &current[..end];
            if !id.is_empty() {
                ids.push(id.to_string());
            }
            current = &current[end..];
        }
        ids
    }
}

impl GraphStore for TinyGraphStore {
    fn open(_: &std::path::Path) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        unimplemented!()
    }
    fn init(&mut self) -> anyhow::Result<()> {
        unimplemented!()
    }
    fn stat(&self) -> anyhow::Result<archctl::GraphStat> {
        unimplemented!()
    }
    fn query(&self, cypher: &str) -> anyhow::Result<Vec<Row>> {
        let upper = cypher.to_uppercase();
        // Route based on Cypher pattern keywords.
        // NOTE: The actual query functions handle WHERE clause filtering (category, version).
        // The mock data should already be "correct" (pre-filtered) per test.
        if upper.contains("SEMANTIC_EDGE") {
            // query_semantic_edges: return edges as-is (WHERE filtering done by database)
            Ok(self.edges.clone())
        } else if upper.contains("SUPPORTED_BY") {
            // query_evidence_for_versions: filter by version_id from WHERE clause
            // then actual code filters by status in Rust
            let version_ids = Self::extract_version_ids_from_cypher(cypher);
            let filtered: Vec<Row> = self
                .evidence
                .iter()
                .filter(|row| {
                    // Each evidence row has an associated version_id stored in a column
                    let ev_version = row
                        .get("ev.id")
                        .and_then(|c| c.as_str())
                        .unwrap_or("");
                    version_ids
                        .iter()
                        .any(|v| ev_version.to_uppercase() == v.to_uppercase())
                })
                .cloned()
                .collect();
            Ok(filtered)
        } else if upper.contains("ELEMENTVERSION") {
            Ok(self.versions.clone())
        } else if upper.contains("E.CATEGORY") && !upper.contains("SRC.CATEGORY") {
            // query_elements: filter by category and canonical_key from cypher WHERE clause
            let (category, canonical_key) = Self::extract_element_filter_from_cypher(cypher);
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
                    let cat_match = category
                        .as_ref()
                        .map(|c| row_cat == c.to_uppercase())
                        .unwrap_or(true);
                    let key_match = canonical_key
                        .as_ref()
                        .map(|k| row_key == k.to_uppercase())
                        .unwrap_or(true);
                    cat_match && key_match
                })
                .cloned()
                .collect();
            Ok(filtered)
        } else {
            Ok(Vec::new())
        }
    }

    fn put_evidence(&mut self, _: &[archctl::evidence::Evidence]) -> anyhow::Result<usize> {
        unimplemented!()
    }
    fn list_evidence(&self, _: Option<&str>) -> anyhow::Result<Vec<Row>> {
        unimplemented!()
    }
    fn put_source(&mut self, _: &archctl::source::SourceArtifact) -> anyhow::Result<()> {
        unimplemented!()
    }
    fn put_evaluation(&mut self, _: &archctl::evaluation::Evaluation) -> anyhow::Result<()> {
        unimplemented!()
    }
    fn link_extracted_from(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
        unimplemented!()
    }
    fn link_evaluates(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
        unimplemented!()
    }
    fn accept_evidence(&mut self, _: &str, _: &dyn archctl::clock::Clock) -> anyhow::Result<()> {
        unimplemented!()
    }
    fn supersede_evidence(&mut self, _: &str) -> anyhow::Result<()> {
        unimplemented!()
    }
    fn list_evidence_by_status(
        &self,
        _: archctl::evidence::EvidenceStatus,
        _: Option<&str>,
    ) -> anyhow::Result<Vec<Row>> {
        unimplemented!()
    }
    fn put_diagram(&mut self, _: &archctl::diagram::view_types::Diagram) -> anyhow::Result<()> {
        unimplemented!()
    }
    fn get_diagram(&self, _: &str) -> anyhow::Result<archctl::diagram::view_types::Diagram> {
        unimplemented!()
    }
    fn put_view_member(&mut self, _: &archctl::diagram::view_types::ViewMember) -> anyhow::Result<()> {
        unimplemented!()
    }
    fn link_member_of(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
        unimplemented!()
    }
    fn link_renders(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
        unimplemented!()
    }
    fn put_view_group(&mut self, _: &archctl::diagram::view_types::ViewGroup) -> anyhow::Result<()> {
        unimplemented!()
    }
    fn link_group_contains(&mut self, _: &str, _: &str) -> anyhow::Result<()> {
        unimplemented!()
    }
    fn get_view_members(&self, _: &str) -> anyhow::Result<Vec<archctl::diagram::view_types::ViewMember>> {
        unimplemented!()
    }
}

// SCN-050: query_elements filtered by category returns only matching nodes
#[test]
fn query_elements_filtered_by_category() {
    let elements = vec![
        row_from_pairs(vec![
            ("e.id", Cell::String("el:1".to_string())),
            ("e.kind_id", Cell::String("struct".to_string())),
            ("e.category", Cell::String("container".to_string())),
            ("e.canonical_key", Cell::String("orders".to_string())),
            ("e.current_name", Cell::String("OrderService".to_string())),
            ("e.current_status", Cell::String("accepted".to_string())),
            ("e.current_confidence", Cell::Float(0.9)),
            ("e.current_version_id", Cell::String("v:1".to_string())),
        ]),
        row_from_pairs(vec![
            ("e.id", Cell::String("el:2".to_string())),
            ("e.kind_id", Cell::String("struct".to_string())),
            ("e.category", Cell::String("container".to_string())),
            ("e.canonical_key", Cell::String("payments".to_string())),
            ("e.current_name", Cell::String("PaymentService".to_string())),
            ("e.current_status", Cell::String("accepted".to_string())),
            ("e.current_confidence", Cell::Float(0.85)),
            ("e.current_version_id", Cell::String("v:2".to_string())),
        ]),
        row_from_pairs(vec![
            ("e.id", Cell::String("el:3".to_string())),
            ("e.kind_id", Cell::String("struct".to_string())),
            ("e.category", Cell::String("component".to_string())),
            ("e.canonical_key", Cell::String("orders".to_string())),
            ("e.current_name", Cell::String("OrderComponent".to_string())),
            ("e.current_status", Cell::String("accepted".to_string())),
            ("e.current_confidence", Cell::Float(0.8)),
            ("e.current_version_id", Cell::String("v:3".to_string())),
        ]),
    ];

    let store = TinyGraphStore::new(elements, Vec::new(), Vec::new(), Vec::new());

    // SCN-050: query with container + orders scope
    // The actual query_function filters by category and scope in cypher
    let result = query_elements(&store, "container", Some("orders")).unwrap();
    // Result includes only el:1 (container + orders namespace)
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, "el:1");
    assert_eq!(result[0].current_name, "OrderService");

    // Query with component + orders scope
    let result2 = query_elements(&store, "component", Some("orders")).unwrap();
    assert_eq!(result2.len(), 1);
    assert_eq!(result2[0].id, "el:3");
}

// SCN-050: empty result when no matching elements
#[test]
fn query_elements_returns_empty_when_no_match() {
    let elements = vec![row_from_pairs(vec![
        ("e.id", Cell::String("el:1".to_string())),
        ("e.kind_id", Cell::String("struct".to_string())),
        ("e.category", Cell::String("container".to_string())),
        ("e.canonical_key", Cell::String("orders".to_string())),
        ("e.current_name", Cell::String("OrderService".to_string())),
        ("e.current_status", Cell::String("accepted".to_string())),
        ("e.current_confidence", Cell::Float(0.9)),
        ("e.current_version_id", Cell::String("v:1".to_string())),
    ])];

    let store = TinyGraphStore::new(elements, Vec::new(), Vec::new(), Vec::new());

    // No container elements in "payments" namespace → empty
    let result = query_elements(&store, "container", Some("payments")).unwrap();
    assert!(result.is_empty());
}

// SCN-051: query_semantic_edges returns intra-category edges
#[test]
fn query_semantic_edges_returns_intra_category_edges() {
    // Mock data should be what database would return AFTER category filtering.
    // Only the container→container edge (rel:1) is included.
    let edges = vec![
        // Edge between two containers (the only one that matches WHERE category = 'container')
        row_from_pairs(vec![
            ("edge.relation_id", Cell::String("rel:1".to_string())),
            ("edge.predicate_id", Cell::String("calls".to_string())),
            ("source_id", Cell::String("el:1".to_string())),
            ("target_id", Cell::String("el:2".to_string())),
            ("edge.order_key", Cell::String("1".to_string())),
            ("edge.props", empty_props_cell()),
        ]),
    ];

    let store = TinyGraphStore::new(Vec::new(), edges, Vec::new(), Vec::new());

    // The cypher for query_semantic_edges adds:
    //   WHERE src.category = 'container' AND tgt.category = 'container'
    // Mock data already represents the filtered result.
    let result = query_semantic_edges(&store, "container").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].relation_id, "rel:1");
    assert_eq!(result[0].source_id, "el:1");
    assert_eq!(result[0].target_id, "el:2");
}

// SCN-052: query_evidence_for_versions filters to Accepted status in Rust
// The cypher fetches all evidence for versions; Rust filters to "accepted"
#[test]
fn query_evidence_filters_to_accepted() {
    // Evidence rows returned by the mock.
    // The "ev.id" column represents the ElementVersion this evidence supports.
    // This is used by the mock to filter when the cypher has "WHERE ev.id IN [...]"
    let evidence = vec![
        // Evidence supporting v:1, accepted status
        row_from_pairs(vec![
            ("e.id", Cell::String("ev:1".to_string())),
            ("e.kind", Cell::String("structural".to_string())),
            ("e.claim", Cell::String("OrderService claim".to_string())),
            ("e.path", Cell::String("src/orders.rs".to_string())),
            ("e.start_line", Cell::Int(1)),
            ("e.end_line", Cell::Int(50)),
            ("e.tool_name", Cell::String("archctl".to_string())),
            ("e.tool_version", Cell::String("0.1.0".to_string())),
            ("e.rule_id", Cell::String("order-handler".to_string())),
            ("e.props", evidence_props_cell("accepted")),
            ("e.content_hash", Cell::String("sha256:abc".to_string())),
            ("e.observed_at", Cell::String("2026-07-30T10:00:00Z".to_string())),
            // Version association (used by mock for WHERE clause filtering)
            ("ev.id", Cell::String("v:1".to_string())),
        ]),
        // Evidence supporting v:2, drafted status — filtered out by status check
        row_from_pairs(vec![
            ("e.id", Cell::String("ev:2".to_string())),
            ("e.kind", Cell::String("structural".to_string())),
            ("e.claim", Cell::String("Drafted claim".to_string())),
            ("e.path", Cell::String("src/orders.rs".to_string())),
            ("e.start_line", Cell::Int(1)),
            ("e.end_line", Cell::Int(30)),
            ("e.tool_name", Cell::String("archctl".to_string())),
            ("e.tool_version", Cell::String("0.1.0".to_string())),
            ("e.rule_id", Cell::String("drafted-handler".to_string())),
            ("e.props", evidence_props_cell("drafted")),
            ("e.content_hash", Cell::String("sha256:def".to_string())),
            ("e.observed_at", Cell::String("2026-07-30T11:00:00Z".to_string())),
            // Version association
            ("ev.id", Cell::String("v:2".to_string())),
        ]),
    ];

    let store = TinyGraphStore::new(Vec::new(), Vec::new(), evidence, Vec::new());

    // SCN-052: Rust-side filter only returns "accepted" evidence
    let result = query_evidence_for_versions(&store, &["v:1".to_string()]).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, "ev:1");
    assert_eq!(result[0].claim, "OrderService claim");

    // v:2 has no accepted evidence (only drafted) → empty
    let result2 = query_evidence_for_versions(&store, &["v:2".to_string()]).unwrap();
    assert!(result2.is_empty());
}

// SCN-053: query_version_props returns version properties
#[test]
fn query_version_props_returns_props() {
    let versions = vec![
        row_from_pairs(vec![
            ("v.id", Cell::String("v:1".to_string())),
            ("v.name", Cell::String("OrderService".to_string())),
            ("v.description", Cell::String("Handles order processing".to_string())),
            ("v.props", evidence_props_cell("accepted")),
        ]),
        row_from_pairs(vec![
            ("v.id", Cell::String("v:2".to_string())),
            ("v.name", Cell::String("PaymentService".to_string())),
            ("v.description", Cell::String("Handles payment processing".to_string())),
            ("v.props", evidence_props_cell("drafted")),
        ]),
    ];

    let store = TinyGraphStore::new(Vec::new(), Vec::new(), Vec::new(), versions);

    let result = query_version_props(&store, &["v:1".to_string(), "v:2".to_string()]).unwrap();
    assert_eq!(result.len(), 2);

    let v1 = result.iter().find(|v| v.id == "v:1").unwrap();
    assert_eq!(v1.name, "OrderService");
    assert_eq!(v1.description, "Handles order processing");

    let v2 = result.iter().find(|v| v.id == "v:2").unwrap();
    assert_eq!(v2.name, "PaymentService");
}
