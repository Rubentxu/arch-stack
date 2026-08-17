//! Integration tests for `archctl architecture explain`.
//!
//! These tests exercise the full CLI command with a real (in-memory) store.
//! Relation/RelationVersion data is seeded via `execute_raw_cypher_for_test`
//! since no high-level writer exists for those tables (F2: relations live on
//! SEMANTIC_EDGE REL TABLE — the reified SemanticRelation node table is
//! reserved for future use per ADR-009).

use archctl::store::{GraphStore, LbugStore};
use tempfile::TempDir;

/// Helper: open an in-memory store for testing.
/// Returns both the store and the temp dir to ensure the directory
/// stays alive while the store is in use.
fn test_store() -> (LbugStore, TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    let mut store = LbugStore::open(&project).unwrap();
    store.init().unwrap();
    (store, tmp)
}

/// Seed an element with evidence for explain tests.
fn seed_element_with_evidence(store: &mut LbugStore, id: &str, version_id: &str) {
    let category = if id.starts_with("c4:") {
        "c4".to_string()
    } else if id.starts_with("uml") {
        "uml".to_string()
    } else if id.starts_with("behavior:") {
        "behavior".to_string()
    } else {
        "c4".to_string()
    };
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:Element {{id: '{id}', kind_id: 'container', category: '{category}', canonical_key: '{id}', current_name: 'TestService', current_status: 'active', current_confidence: 0.9, current_version_id: '{version_id}'}})"
        ))
        .expect("seed element");
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:ElementVersion {{id: '{version_id}', element_id: '{id}', name: 'TestService', status: 'active', origin: 'ast-grep', confidence: 0.9}})"
        ))
        .expect("seed element version");
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:Evidence {{id: 'ev:{version_id}', kind: 'structural', claim: 'test evidence', path: 'src/lib.rs', start_line: 10, end_line: 15, tool_name: 'ast-grep', tool_version: '0.1', rule_id: 'test:rule', props: '{{\"status\":\"accepted\"}}', content_hash: 'sha256:abc', observed_at: timestamp('2026-08-01T00:00:00Z')}})"
        ))
        .expect("seed evidence");
    store
        .execute_raw_cypher_for_test(&format!(
            "MATCH (v:ElementVersion {{id: '{version_id}'}}), (e:Evidence {{id: 'ev:{version_id}'}}) CREATE (v)-[:SUPPORTED_BY]->(e)"
        ))
        .expect("link version to evidence");
}

/// Seed a relation with evidence for explain tests.
fn seed_relation_with_evidence(store: &mut LbugStore, id: &str, version_id: &str, label: &str) {
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:SemanticRelation {{id: '{id}', predicate_id: 'calls', source_id: 'el:1', target_id: 'el:2', canonical_key: '{id}', current_version_id: '{version_id}', current_label: '{label}', current_status: 'active', current_origin: 'ast-grep', current_confidence: 0.9}})"
        ))
        .expect("seed semantic relation");
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:RelationVersion {{id: '{version_id}', relation_id: '{id}', label: '{label}', status: 'active', origin: 'ast-grep', confidence: 0.9}})"
        ))
        .expect("seed relation version");
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:Evidence {{id: 'ev:{version_id}', kind: 'structural', claim: 'test evidence', path: 'src/lib.rs', start_line: 20, end_line: 25, tool_name: 'ast-grep', tool_version: '0.1', rule_id: 'test:rule', props: '{{\"status\":\"accepted\"}}', content_hash: 'sha256:def', observed_at: timestamp('2026-08-01T00:00:00Z')}})"
        ))
        .expect("seed evidence");
    store
        .execute_raw_cypher_for_test(&format!(
            "MATCH (rv:RelationVersion {{id: '{version_id}'}}), (e:Evidence {{id: 'ev:{version_id}'}}) CREATE (rv)-[:SUPPORTED_BY]->(e)"
        ))
        .expect("link relation version to evidence");
}

// ---------------------------------------------------------------------------
// Happy path tests
// ---------------------------------------------------------------------------

#[test]
fn explain_element_json_happy_path() {
    let (mut store, _tmp) = test_store();
    seed_element_with_evidence(&mut store, "c4:container:orders", "v:orders:1");

    let report = archctl::architecture::explain(&store, "c4:container:orders").unwrap();

    assert_eq!(report.schema_version, "1.0");
    assert_eq!(report.capability, "architecture-explain-mvp");
    assert_eq!(report.subject.kind, "element");
    assert_eq!(report.subject.id, "c4:container:orders");
    assert_eq!(report.subject.version_id, Some("v:orders:1".to_string()));
    assert!(!report.provenance.unsubstantiated);
    assert_eq!(report.provenance.evidence.len(), 1);
    assert_eq!(report.provenance.evidence[0].path, "src/lib.rs");
    assert_eq!(report.provenance.evidence[0].start_line, 10);
    assert!(report.warnings.is_empty());
}

#[test]
fn explain_relation_json_happy_path() {
    let (mut store, _tmp) = test_store();
    seed_relation_with_evidence(
        &mut store,
        "rel:orders-payment",
        "rv:orders-payment:1",
        "calls",
    );

    let report = archctl::architecture::explain(&store, "rel:orders-payment").unwrap();

    assert_eq!(report.schema_version, "1.0");
    assert_eq!(report.capability, "architecture-explain-mvp");
    assert_eq!(report.subject.kind, "relation");
    assert_eq!(report.subject.id, "rel:orders-payment");
    assert_eq!(
        report.subject.version_id,
        Some("rv:orders-payment:1".to_string())
    );
    assert_eq!(report.subject.statement, "calls");
    assert!(!report.provenance.unsubstantiated);
    assert_eq!(report.provenance.evidence.len(), 1);
    assert_eq!(report.provenance.evidence[0].path, "src/lib.rs");
    assert_eq!(report.provenance.evidence[0].start_line, 20);
}

// ---------------------------------------------------------------------------
// Honesty tests (no evidence)
// ---------------------------------------------------------------------------

#[test]
fn explain_element_no_evidence_is_unsubstantiated() {
    let (mut store, _tmp) = test_store();
    // Seed element WITHOUT evidence
    store
        .execute_raw_cypher_for_test(
            "CREATE (:Element {id: 'c4:container:orders', kind_id: 'container', category: 'c4', canonical_key: 'orders', current_name: 'OrderService', current_status: 'active', current_confidence: 0.9, current_version_id: 'v:orders:1'})",
        )
        .expect("seed element");
    store
        .execute_raw_cypher_for_test(
            "CREATE (:ElementVersion {id: 'v:orders:1', element_id: 'c4:container:orders', name: 'OrderService', status: 'active', origin: 'ast-grep', confidence: 0.9})",
        )
        .expect("seed element version");
    // Note: NO evidence, NO SUPPORTED_BY link

    let report = archctl::architecture::explain(&store, "c4:container:orders").unwrap();

    assert!(report.provenance.unsubstantiated);
    assert!(report.provenance.evidence.is_empty());
    assert!(!report.warnings.is_empty());
}

#[test]
fn explain_relation_no_evidence_is_unsubstantiated() {
    let (mut store, _tmp) = test_store();
    // Seed relation WITHOUT evidence
    store
        .execute_raw_cypher_for_test(
            "CREATE (:SemanticRelation {id: 'rel:orders-payment', predicate_id: 'calls', source_id: 'el:1', target_id: 'el:2', canonical_key: 'orders-payment', current_version_id: 'rv:1', current_label: 'calls', current_status: 'active', current_origin: 'ast-grep', current_confidence: 0.9})",
        )
        .expect("seed relation");
    store
        .execute_raw_cypher_for_test(
            "CREATE (:RelationVersion {id: 'rv:1', relation_id: 'rel:orders-payment', label: 'calls', status: 'active', origin: 'ast-grep', confidence: 0.9})",
        )
        .expect("seed relation version");
    // Note: NO evidence, NO SUPPORTED_BY link

    let report = archctl::architecture::explain(&store, "rel:orders-payment").unwrap();

    assert!(report.provenance.unsubstantiated);
    assert!(report.provenance.evidence.is_empty());
    assert!(!report.warnings.is_empty());
}

// ---------------------------------------------------------------------------
// Error path tests
// ---------------------------------------------------------------------------

#[test]
fn explain_unknown_element_id_returns_error() {
    let (store, _tmp) = test_store();

    let result = archctl::architecture::explain(&store, "c4:container:nonexistent");
    assert!(matches!(
        result,
        Err(archctl::architecture::explain::ExplainError::SubjectNotFound(_))
    ));
}

#[test]
fn explain_unknown_relation_id_returns_error() {
    let (store, _tmp) = test_store();

    let result = archctl::architecture::explain(&store, "rel:nonexistent");
    assert!(matches!(
        result,
        Err(archctl::architecture::explain::ExplainError::RelationNotFound(_))
    ));
}

// ---------------------------------------------------------------------------
// Schema and routing tests
// ---------------------------------------------------------------------------

#[test]
fn explain_uml_element_routes_to_element_path() {
    let (mut store, _tmp) = test_store();
    seed_element_with_evidence(&mut store, "uml:class:OrderService", "v:uml:1");

    let report = archctl::architecture::explain(&store, "uml:class:OrderService").unwrap();

    assert_eq!(report.subject.kind, "element");
    assert_eq!(report.subject.id, "uml:class:OrderService");
}

#[test]
fn explain_behavior_element_routes_to_element_path() {
    let (mut store, _tmp) = test_store();
    seed_element_with_evidence(&mut store, "behavior:user:login", "v:behavior:1");

    let report = archctl::architecture::explain(&store, "behavior:user:login").unwrap();

    assert_eq!(report.subject.kind, "element");
    assert_eq!(report.subject.id, "behavior:user:login");
}

#[test]
fn explain_null_version_id_yields_unsubstantiated_with_warning() {
    let (mut store, _tmp) = test_store();
    // Seed element with empty current_version_id
    store
        .execute_raw_cypher_for_test(
            "CREATE (:Element {id: 'c4:container:orphan', kind_id: 'container', category: 'c4', canonical_key: 'orphan', current_name: 'OrphanService', current_status: 'active', current_confidence: 0.9, current_version_id: ''})",
        )
        .expect("seed element with null version");

    let report = archctl::architecture::explain(&store, "c4:container:orphan").unwrap();

    assert!(report.provenance.unsubstantiated);
    assert!(report.subject.version_id.is_none());
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("no current version"))
    );
}
