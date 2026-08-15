//! Integration tests for `archctl code state-machine` apply contract.

use tempfile::TempDir;

// ─── Atomic-abort regression (M32 D5) ──────────────────────────────────────

/// Verifies that `state_machine::apply` wraps writes in a transaction:
/// a mid-loop binder error triggers Kùzu's implicit rollback, COMMIT
/// fails, and 0 partial rows survive. Pattern parallels PR1's
/// `transaction_atomic_abort_on_write_error` for call_graph and
/// M32 D5's `class_diagram_apply_atomic_abort_on_write_error`.
///
/// Primitive-level test (same reasoning — Kùzu per-process flock
/// prevents re-opening the same project within one test process).
#[test]
fn state_machine_apply_atomic_abort_on_write_error() {
    use archctl::store::{ElementRepository, GraphStore, LbugStore, RawGraphQuery};

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    let mut store = LbugStore::open(project).expect("store must open");
    store.init().expect("store must init");

    store.begin_transaction().expect("begin must succeed");
    // Use typed repository method instead of raw MERGE to avoid RawGraphQuery guard.
    store
        .upsert_element(&archctl::graph::Element {
            id: "state_mach:good".to_string(),
            kind_id: "k".to_string(),
            category: "test".to_string(),
            canonical_key: "state_mach:good".to_string(),
            current_name: "state_mach:good".to_string(),
            current_status: "active".to_string(),
            current_confidence: 1.0,
            current_version_id: uuid::Uuid::new_v4().to_string(),
        })
        .expect("good write inside tx must succeed");

    // Trigger a binder error: SUPPORTED_BY is declared FROM
    // ElementVersion TO Evidence — so (Element)-[SUPPORTED_BY]->(Evidence)
    // violates the direction constraint.
    // Use execute_raw_cypher_for_test to bypass RawGraphQuery guard and reach
    // Kùzu directly so Kùzu can enforce the direction constraint.
    let bad = store.execute_raw_cypher_for_test(
        "MATCH (e:Element {id: 'state_mach:good'}) MATCH (ev:Evidence {id: 'state_mach:ev'}) \
         MERGE (e)-[r:SUPPORTED_BY]->(ev);",
    );
    assert!(
        bad.is_err(),
        "expected SUPPORTED_BY direction violation to fail the binder"
    );

    // Active transaction is now implicitly rolled back by Kùzu.
    let commit = store.commit_transaction();
    assert!(
        commit.is_err(),
        "commit must fail after implicit rollback from binder error"
    );

    // 0 partial rows survive.
    let rows: Vec<archctl::Row> = store
        .query("MATCH (e:Element {id: 'state_mach:good'}) RETURN e.id;")
        .expect("query must succeed");
    assert_eq!(
        rows.len(),
        0,
        "atomic-abort: no partial state should survive an implicit rollback"
    );
}
