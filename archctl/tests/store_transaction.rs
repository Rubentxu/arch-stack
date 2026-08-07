//! Integration tests for the `GraphStore` transaction primitives
//! added in M32 D1 (`begin_transaction`, `commit_transaction`,
//! `rollback_transaction`). These complement the existing
//! `code_call_graph::apply` happy-path test by exercising the
//! atomic-abort contract directly.

use tempfile::TempDir;

use archctl::Row;
use archctl::store::{GraphStore, LbugStore};

fn open_store(project: &std::path::Path) -> LbugStore {
    let mut store = LbugStore::open(project).expect("store must open");
    store.init().expect("store must init");
    store
}

#[test]
fn transaction_commit_persists_writes() {
    let tmp = TempDir::new().unwrap();
    let mut store = open_store(tmp.path());

    store.begin_transaction().expect("begin must succeed");
    store
        .query("MERGE (e:Element {id: 'tx_commit:test'}) SET e.kind_id = 'k';")
        .expect("write inside tx must succeed");
    store.commit_transaction().expect("commit must succeed");

    // Query in the SAME session (Kùzu holds an exclusive flock per
    // project, so we can't re-open within this test). The commit
    // already flushed to disk; subsequent queries in the same
    // session see it.
    let rows: Vec<Row> = store
        .query("MATCH (e:Element {id: 'tx_commit:test'}) RETURN e.id;")
        .expect("query must succeed");
    assert_eq!(
        rows.len(),
        1,
        "commit must persist the write within the same session"
    );
}

#[test]
fn transaction_explicit_rollback_clears_writes() {
    let tmp = TempDir::new().unwrap();
    let mut store = open_store(tmp.path());

    store.begin_transaction().expect("begin must succeed");
    store
        .query("MERGE (e:Element {id: 'tx_rollback:test'}) SET e.kind_id = 'k';")
        .expect("write inside tx must succeed");
    store.rollback_transaction().expect("rollback must succeed");

    // Verify nothing was persisted (within the same session — Kùzu
    // already cleared active state, COMMIT wasn't issued).
    let rows: Vec<Row> = store
        .query("MATCH (e:Element {id: 'tx_rollback:test'}) RETURN e.id;")
        .expect("query must succeed");
    assert_eq!(
        rows.len(),
        0,
        "explicit rollback must clear the write — same session sees 0 rows"
    );
}

#[test]
fn transaction_atomic_abort_on_write_error() {
    // Mirrors the SUPPORTED_BY scenario from M32 PR1 discovery:
    // a write that violates a schema constraint triggers an
    // implicit Kùzu rollback (lbug client_context.cpp L658), even
    // though we never explicitly call rollback_transaction. After
    // the failed write, COMMIT should fail because the active
    // transaction was cleared.
    let tmp = TempDir::new().unwrap();
    let mut store = open_store(tmp.path());

    store.begin_transaction().expect("begin must succeed");
    store
        .query("MERGE (e:Element {id: 'tx_abort:good'}) SET e.kind_id = 'k';")
        .expect("good write inside tx must succeed");

    // Now trigger a binder error: SUPPORTED_BY is declared
    // FROM ElementVersion TO Evidence, so (Element)-[SUPPORTED_BY]
    // ->(Evidence) violates the direction constraint.
    let bad = store.query(
        "MATCH (e:Element {id: 'tx_abort:good'}) MATCH (ev:Evidence {id: 'tx_abort:ev'}) \
         MERGE (e)-[r:SUPPORTED_BY]->(ev);",
    );
    assert!(
        bad.is_err(),
        "expected SUPPORTED_BY direction violation to fail the binder"
    );

    // The active transaction is now implicitly rolled back by Kùzu.
    // An explicit COMMIT must fail with 'No active transaction'.
    let commit = store.commit_transaction();
    assert!(
        commit.is_err(),
        "commit must fail after implicit rollback from binder error"
    );

    // Same-session check: nothing was persisted. Kùzu holds an
    // exclusive flock per project; we can't re-open in this test.
    let rows: Vec<Row> = store
        .query("MATCH (e:Element {id: 'tx_abort:good'}) RETURN e.id;")
        .expect("query must succeed");
    assert_eq!(
        rows.len(),
        0,
        "atomic-abort: no partial state should survive an implicit rollback"
    );
}
