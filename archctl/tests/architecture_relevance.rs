//! Integration tests for `archctl architecture relevance`.
//!
//! These tests exercise the relevance use case with a real (in-memory) store.
//! Graph data is seeded via `execute_raw_cypher_for_test`.

use archctl::architecture::relevance::{RelevanceOptions, RelevanceReport};
use archctl::store::{GraphStore, LbugStore};
use tempfile::TempDir;

/// Helper: open an in-memory store for testing.
fn test_store() -> (LbugStore, TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    let mut store = LbugStore::open(&project).unwrap();
    store.init().unwrap();
    (store, tmp)
}

/// Seed an element with the given id, name, and confidence.
fn seed_element(store: &mut LbugStore, id: &str, name: &str, confidence: f64) {
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:Element {{id: '{id}', kind_id: 'container', category: 'c4', canonical_key: '{id}', current_name: '{name}', current_status: 'active', current_confidence: {confidence}, current_version_id: '{id}-v1'}})"
        ))
        .expect("seed element");
}

/// Seed a semantic edge from source to target.
fn seed_edge(
    store: &mut LbugStore,
    relation_id: &str,
    predicate: &str,
    source: &str,
    target: &str,
) {
    store
        .execute_raw_cypher_for_test(&format!(
            "MATCH (s:Element {{id: '{source}'}}), (t:Element {{id: '{target}'}}) CREATE (s)-[:SEMANTIC_EDGE {{relation_id: '{relation_id}', predicate_id: '{predicate}', active: true, order_key: '0'}}]->(t)"
        ))
        .expect("seed edge");
}

// ---------------------------------------------------------------------------
// S1: Exact-id seed score 1.0
// ---------------------------------------------------------------------------

#[test]
fn relevance_exact_id_seed() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:orders", "OrderService", 0.9);

    let report: RelevanceReport = archctl::architecture::relevance(
        &store,
        "c4:container:orders",
        &RelevanceOptions::default(),
    )
    .unwrap();

    assert_eq!(report.schema_version, "1.0");
    assert_eq!(report.capability, "architecture-relevance-mvp");
    assert_eq!(report.elements.len(), 1);
    let elem = &report.elements[0];
    assert_eq!(elem.id, "c4:container:orders");
    assert!((elem.score - 0.9).abs() < 1e-9);
    assert_eq!(elem.match_type, "exact-id");
    assert_eq!(elem.hop_distance, 0);
}

// ---------------------------------------------------------------------------
// S2: Free-text name match sorted (score DESC, id ASC)
// ---------------------------------------------------------------------------

#[test]
fn relevance_name_match_ranking() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:orders", "OrderService", 0.9);
    seed_element(&mut store, "c4:container:queue", "OrderQueue", 0.8);

    let report: RelevanceReport =
        archctl::architecture::relevance(&store, "Order", &RelevanceOptions::default()).unwrap();

    assert_eq!(report.elements.len(), 2);
    // OrderService (0.9) before OrderQueue (0.8)
    assert_eq!(report.elements[0].id, "c4:container:orders");
    assert!((report.elements[0].score - 0.72).abs() < 1e-9); // 0.8 * 0.9
    assert_eq!(report.elements[1].id, "c4:container:queue");
    assert!((report.elements[1].score - 0.64).abs() < 1e-9); // 0.8 * 0.8
}

// ---------------------------------------------------------------------------
// S3: 1-hop expansion at 0.5x
// ---------------------------------------------------------------------------

#[test]
fn relevance_expansion_0_5x() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", "A", 0.9);
    seed_element(&mut store, "c4:container:b", "B", 0.8);
    seed_edge(
        &mut store,
        "rel-a-b",
        "depends_on",
        "c4:container:a",
        "c4:container:b",
    );

    let result = archctl::architecture::relevance(
        &store,
        "a",
        &RelevanceOptions {
            top: 10,
            max_hops: 1,
        },
    )
    .unwrap();

    assert_eq!(result.elements.len(), 2);
    // B should appear via expansion with 0.5 * 0.8 = 0.4
    let b = result.elements.iter().find(|e| e.id == "c4:container:b");
    assert!(b.is_some());
    let b = b.unwrap();
    assert!((b.score - 0.4).abs() < 1e-9);
    assert_eq!(b.match_type, "expansion");
    assert_eq!(b.hop_distance, 1);
    assert!(result.selection_trace.expansion_edges_followed >= 1);
}

// ---------------------------------------------------------------------------
// S4: max_hops=0 disables expansion
// ---------------------------------------------------------------------------

#[test]
fn relevance_max_hops_zero_no_expansion() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", "A", 0.9);
    seed_element(&mut store, "c4:container:b", "B", 0.8);
    seed_edge(
        &mut store,
        "rel-a-b",
        "depends_on",
        "c4:container:a",
        "c4:container:b",
    );

    let result = archctl::architecture::relevance(
        &store,
        "a",
        &RelevanceOptions {
            top: 10,
            max_hops: 0,
        },
    )
    .unwrap();

    assert_eq!(result.elements.len(), 1);
    assert_eq!(result.elements[0].id, "c4:container:a");
    assert_eq!(result.selection_trace.expansion_edges_followed, 0);
}

// ---------------------------------------------------------------------------
// S6: Empty graph → empty arrays, exit 0 (no error)
// ---------------------------------------------------------------------------

#[test]
fn relevance_empty_graph() {
    let (store, _tmp) = test_store();

    let result =
        archctl::architecture::relevance(&store, "anything", &RelevanceOptions::default()).unwrap();

    assert!(result.elements.is_empty());
    assert!(result.relations.is_empty());
    assert_eq!(result.selection_trace.seeds_matched, 0);
    assert_eq!(result.selection_trace.candidates_scanned, 0);
}

// ---------------------------------------------------------------------------
// S7: --top N caps shortlist independently
// ---------------------------------------------------------------------------

#[test]
fn relevance_top_caps_shortlist() {
    let (mut store, _tmp) = test_store();
    for i in 0..12 {
        seed_element(
            &mut store,
            &format!("c4:container:e{}", i),
            &format!("Element{}", i),
            0.9,
        );
    }

    let result = archctl::architecture::relevance(
        &store,
        "e",
        &RelevanceOptions {
            top: 5,
            max_hops: 0,
        },
    )
    .unwrap();

    assert_eq!(result.elements.len(), 5);
}

// ---------------------------------------------------------------------------
// S9: ASCII-fold match (MañanasService ↔ mananas)
// ---------------------------------------------------------------------------

#[test]
fn relevance_ascii_fold_match() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:srv", "MañanasService", 0.9);

    let result =
        archctl::architecture::relevance(&store, "mananas", &RelevanceOptions::default()).unwrap();

    assert!(!result.elements.is_empty());
    assert_eq!(result.elements[0].id, "c4:container:srv");
}

// ---------------------------------------------------------------------------
// S5: Determinism — two calls produce byte-equal JSON
// ---------------------------------------------------------------------------

#[test]
fn relevance_determinism() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:srv", "OrderService", 0.9);
    seed_element(&mut store, "c4:container:api", "OrderApi", 0.8);

    let opts = RelevanceOptions::default();
    let json1 =
        serde_json::to_string(&archctl::architecture::relevance(&store, "Order", &opts).unwrap())
            .unwrap();
    let json2 =
        serde_json::to_string(&archctl::architecture::relevance(&store, "Order", &opts).unwrap())
            .unwrap();

    assert_eq!(json1, json2);
}

// ---------------------------------------------------------------------------
// S8: Relations scored when source or target is in shortlist
// ---------------------------------------------------------------------------

#[test]
fn relevance_relation_scored() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", "A", 0.9);
    seed_element(&mut store, "c4:container:b", "B", 0.8);
    seed_edge(
        &mut store,
        "rel-a-b",
        "depends_on",
        "c4:container:a",
        "c4:container:b",
    );

    // Query for "depends_on" should surface the relation
    let result =
        archctl::architecture::relevance(&store, "depends_on", &RelevanceOptions::default())
            .unwrap();

    // The edge should be present because source is in shortlist
    assert!(!result.relations.is_empty() || !result.elements.is_empty());
}
