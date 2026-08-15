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
    use archctl::store::{GraphStore, LbugStore, RawGraphQuery};

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();
    let mut store = LbugStore::open(project).expect("store must open");
    store.init().expect("store must init");

    store.begin_transaction().expect("begin must succeed");
    store
        .query("MERGE (e:Element {id: 'state_mach:good'}) SET e.kind_id = 'k';")
        .expect("good write inside tx must succeed");

    // Trigger a binder error: SUPPORTED_BY is declared FROM
    // ElementVersion TO Evidence — so (Element)-[SUPPORTED_BY]->(Evidence)
    // violates the direction constraint.
    let bad = store.query(
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
