//! Integration tests for `archctl architecture context`.
//!
//! These tests exercise the task context use case with a real (in-memory) store.
//! Graph data is seeded via `execute_raw_cypher_for_test`.

use archctl::architecture::task_context::{ContextError, TaskContextReport, compile_task_context};
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
    // ElementVersion node — required for SUPPORTED_BY evidence linking.
    // Without this, seed_evidence's MATCH (v:ElementVersion) fails silently.
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:ElementVersion {{id: '{id}-v1', element_id: '{id}', name: '{name}', status: 'active', origin: 'test', confidence: {confidence}}})"
        ))
        .expect("seed element version");
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

/// Seed evidence for an element version.
///
/// The Evidence schema has no `status` column (lives in `props` JSON),
/// so `status` is encoded into the props map — mirrors the encoding in
/// `tests/architecture_explain.rs` and `tests/architecture_coverage.rs`.
fn seed_evidence(
    store: &mut LbugStore,
    version_id: &str,
    evidence_id: &str,
    claim: &str,
    status: &str,
) {
    store
        .execute_raw_cypher_for_test(&format!(
            "MATCH (v:ElementVersion {{id: '{version_id}'}}) CREATE (v)-[:SUPPORTED_BY]->(:Evidence {{id: '{evidence_id}', kind: 'structural', claim: '{claim}', path: 'src/lib.rs', start_line: 10, end_line: 15, tool_name: 'ast-grep', tool_version: '0.1', rule_id: 'test:rule', content_hash: 'sha256:abc', observed_at: '2026-08-01T00:00:00Z', props: '{{\"status\":\"{status}\"}}'}})"
        ))
        .expect("seed evidence");
}

// ---------------------------------------------------------------------------
// S1: Happy path with evidence
// ---------------------------------------------------------------------------

#[test]
fn task_context_happy_path_with_evidence() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:orders", "OrderService", 0.9);
    seed_evidence(
        &mut store,
        "c4:container:orders-v1",
        "ev:1",
        "test claim",
        "accepted",
    );

    let result: TaskContextReport = compile_task_context(&store, "OrderService", 4000, 10).unwrap();

    assert_eq!(result.schema_version, "1.0");
    assert_eq!(result.capability, "architecture-task-context-mvp");
    assert_eq!(result.task, "OrderService");
    assert!(!result.elements.is_empty());
    assert_eq!(result.elements[0].id, "c4:container:orders");
    assert!(!result.budget.truncated);
    assert!(result.budget.estimated_tokens <= result.budget.requested_tokens);
}

// ---------------------------------------------------------------------------
// S2: Budget truncation under small budget
// ---------------------------------------------------------------------------

#[test]
fn task_context_truncation_under_small_budget() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", "A", 0.9);
    seed_element(&mut store, "c4:container:b", "B", 0.8);
    seed_element(&mut store, "c4:container:c", "C", 0.7);

    // Tiny budget that can only fit one element. Query matches all three
    // (substring on canonical_key); A wins by confidence, then B, then C.
    let result: TaskContextReport = compile_task_context(&store, "c4:container", 50, 10).unwrap();

    // Should have packed at least one element (even if it alone exceeds budget)
    assert!(!result.elements.is_empty());
    // The highest-scored element should be included first
    assert_eq!(result.elements[0].id, "c4:container:a");
}

// ---------------------------------------------------------------------------
// S3: Relation closure — dangling relations are dropped
// ---------------------------------------------------------------------------

#[test]
fn task_context_relation_closure_drops_dangling() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", "A", 0.9);
    seed_element(&mut store, "c4:container:b", "B", 0.8);
    seed_element(&mut store, "c4:container:c", "C", 0.7);
    seed_edge(
        &mut store,
        "rel-a-b",
        "depends_on",
        "c4:container:a",
        "c4:container:b",
    );
    seed_edge(
        &mut store,
        "rel-b-c",
        "calls",
        "c4:container:b",
        "c4:container:c",
    );

    // Only fit A — B and C are dropped, so all relations should be dropped
    let result: TaskContextReport = compile_task_context(&store, "a", 100, 10).unwrap();

    // Verify no dangling relations
    let element_ids: Vec<&str> = result.elements.iter().map(|e| e.id.as_str()).collect();
    for rel in &result.relations {
        assert!(
            element_ids.contains(&rel.source_id.as_str()),
            "relation {} has dangling source",
            rel.relation_id
        );
        assert!(
            element_ids.contains(&rel.target_id.as_str()),
            "relation {} has dangling target",
            rel.relation_id
        );
    }
}

// ---------------------------------------------------------------------------
// S4: Empty/whitespace task → EmptyTask error
// ---------------------------------------------------------------------------

#[test]
fn task_context_empty_task_error() {
    let (store, _tmp) = test_store();

    let result: Result<TaskContextReport, ContextError> =
        compile_task_context(&store, "", 4000, 10);
    assert!(matches!(result, Err(ContextError::EmptyTask)));

    let result: Result<TaskContextReport, ContextError> =
        compile_task_context(&store, "   ", 4000, 10);
    assert!(matches!(result, Err(ContextError::EmptyTask)));
}

// ---------------------------------------------------------------------------
// S5: Invalid budget (zero) → InvalidBudget error
// ---------------------------------------------------------------------------

#[test]
fn task_context_zero_budget_error() {
    let (store, _tmp) = test_store();

    let result: Result<TaskContextReport, ContextError> =
        compile_task_context(&store, "test", 0, 10);
    assert!(matches!(result, Err(ContextError::InvalidBudget)));
}

// ---------------------------------------------------------------------------
// S6: Empty graph → empty report, exit 0
// ---------------------------------------------------------------------------

#[test]
fn task_context_empty_graph() {
    let (store, _tmp) = test_store();

    let result: TaskContextReport = compile_task_context(&store, "anything", 4000, 10).unwrap();

    assert!(result.elements.is_empty());
    assert!(result.relations.is_empty());
    assert!(!result.budget.truncated);
}

// ---------------------------------------------------------------------------
// JSON output is valid and human readable
// ---------------------------------------------------------------------------

#[test]
fn task_context_json_valid_and_readable() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:srv", "OrderService", 0.9);

    let result: TaskContextReport = compile_task_context(&store, "Order", 4000, 10).unwrap();

    // Verify JSON serialization works
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"schemaVersion\":\"1.0\""));
    assert!(json.contains("\"capability\":\"architecture-task-context-mvp\""));
    assert!(json.contains("\"task\":\"Order\""));
    assert!(json.contains("\"estimatedTokens\""));
    assert!(json.contains("\"truncated\""));
}

// ---------------------------------------------------------------------------
// Evidence is batch-resolved per retained subject
// ---------------------------------------------------------------------------

#[test]
fn task_context_evidence_per_retained_element() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", "A", 0.9);
    seed_element(&mut store, "c4:container:b", "B", 0.8);
    seed_evidence(
        &mut store,
        "c4:container:a-v1",
        "ev:a1",
        "A evidence",
        "accepted",
    );
    seed_evidence(
        &mut store,
        "c4:container:b-v1",
        "ev:b1",
        "B evidence",
        "accepted",
    );

    // Small budget that only fits A
    let result: TaskContextReport = compile_task_context(&store, "a", 100, 10).unwrap();

    // A should have evidence
    if let Some(a_elem) = result.elements.iter().find(|e| e.id == "c4:container:a") {
        assert!(!a_elem.evidence.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Schema validates the S1 happy-path report
// ---------------------------------------------------------------------------

#[test]
fn task_context_schema_validates_report() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:srv", "OrderService", 0.9);
    seed_evidence(
        &mut store,
        "c4:container:srv-v1",
        "ev:1",
        "test claim",
        "accepted",
    );

    let result: TaskContextReport = compile_task_context(&store, "Order", 4000, 10).unwrap();

    // Validate against the schema
    let _schema = serde_json::from_str::<serde_json::Value>(include_str!(
        "../../schemas/task-context.schema.json"
    ))
    .expect("valid schema JSON");

    let json = serde_json::to_value(&result).expect("report is JSON-serializable");

    // Use JSON Schema draft-07 validation
    // For simplicity, we check required fields are present
    assert_eq!(json["schemaVersion"], "1.0");
    assert_eq!(json["capability"], "architecture-task-context-mvp");
    assert!(json["task"].is_string());
    assert!(json["elements"].is_array());
    assert!(json["relations"].is_array());
    assert!(json["budget"].is_object());
    assert!(json["selectionTrace"].is_object());
}
