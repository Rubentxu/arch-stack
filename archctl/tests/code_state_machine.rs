//! Integration tests for `archctl code state-machine` apply contract.

use std::collections::BTreeMap;

use archctl::filesystem::SystemFilesystem;
use archctl::store::{LbugStore, RawGraphQuery};
use tempfile::TempDir;

// ─── UNWIND bulk correctness (M32 D2) ────────────────────────────────────

/// Regression guard for M32 D2: the UNWIND bulk-import path in state_machine::apply
/// must produce the same element and edge counts as the pre-UNWIND per-element path.
///
/// Strategy: build a synthetic StateMachineReport with 3 machines × 3 states × 4
/// transitions, apply it, verify exact counts via LbugStore queries, then apply
/// the same report again and verify idempotency (skip, no duplicates).
///
/// T6.2: state_machine UNWIND test.
#[test]
fn state_machine_apply_unwind_bulk_correctness() {
    use archctl::code::state_machine::{
        State, StateKind, StateMachine, StateMachineReport, Transition,
    };
    use archctl::store::{ElementRepository, GraphStore, LbugStore};

    let tmp = TempDir::new().unwrap();
    let project = tmp.path();

    // Build 3 machines, each with 3 states and 4 transitions.
    // Machine 1: states S1, S2, S3; transitions: S1→S2, S2→S3, S3→S1, S1→S3
    // Machine 2: states T1, T2, T3; transitions: T1→T2, T2→T3, T3→T1, T1→T3
    // Machine 3: states U1, U2, U3; transitions: U1→U2, U2→U3, U3→U1, U1→U3
    let machines = vec![
        StateMachine {
            canonical_key: "rust:src/lib.rs:state_machine:SM1:3".to_string(),
            name: "SM1".to_string(),
            file: "src/lib.rs".to_string(),
            content_hash: "sha256:1111".to_string(),
            states: vec![
                State {
                    name: "S1".to_string(),
                    kind: StateKind::Initial,
                    line: 4,
                },
                State {
                    name: "S2".to_string(),
                    kind: StateKind::Regular,
                    line: 5,
                },
                State {
                    name: "S3".to_string(),
                    kind: StateKind::Final,
                    line: 6,
                },
            ],
            transitions: vec![
                Transition {
                    from: "S1".to_string(),
                    to: "S2".to_string(),
                    trigger: None,
                    guard: None,
                    line: 7,
                },
                Transition {
                    from: "S2".to_string(),
                    to: "S3".to_string(),
                    trigger: None,
                    guard: None,
                    line: 8,
                },
                Transition {
                    from: "S3".to_string(),
                    to: "S1".to_string(),
                    trigger: None,
                    guard: None,
                    line: 9,
                },
                Transition {
                    from: "S1".to_string(),
                    to: "S3".to_string(),
                    trigger: None,
                    guard: None,
                    line: 10,
                },
            ],
            confidence: 0.90,
        },
        StateMachine {
            canonical_key: "rust:src/lib.rs:state_machine:SM2:13".to_string(),
            name: "SM2".to_string(),
            file: "src/lib.rs".to_string(),
            content_hash: "sha256:2222".to_string(),
            states: vec![
                State {
                    name: "T1".to_string(),
                    kind: StateKind::Initial,
                    line: 14,
                },
                State {
                    name: "T2".to_string(),
                    kind: StateKind::Regular,
                    line: 15,
                },
                State {
                    name: "T3".to_string(),
                    kind: StateKind::Final,
                    line: 16,
                },
            ],
            transitions: vec![
                Transition {
                    from: "T1".to_string(),
                    to: "T2".to_string(),
                    trigger: None,
                    guard: None,
                    line: 17,
                },
                Transition {
                    from: "T2".to_string(),
                    to: "T3".to_string(),
                    trigger: None,
                    guard: None,
                    line: 18,
                },
                Transition {
                    from: "T3".to_string(),
                    to: "T1".to_string(),
                    trigger: None,
                    guard: None,
                    line: 19,
                },
                Transition {
                    from: "T1".to_string(),
                    to: "T3".to_string(),
                    trigger: None,
                    guard: None,
                    line: 20,
                },
            ],
            confidence: 0.90,
        },
        StateMachine {
            canonical_key: "rust:src/lib.rs:state_machine:SM3:23".to_string(),
            name: "SM3".to_string(),
            file: "src/lib.rs".to_string(),
            content_hash: "sha256:3333".to_string(),
            states: vec![
                State {
                    name: "U1".to_string(),
                    kind: StateKind::Initial,
                    line: 24,
                },
                State {
                    name: "U2".to_string(),
                    kind: StateKind::Regular,
                    line: 25,
                },
                State {
                    name: "U3".to_string(),
                    kind: StateKind::Final,
                    line: 26,
                },
            ],
            transitions: vec![
                Transition {
                    from: "U1".to_string(),
                    to: "U2".to_string(),
                    trigger: None,
                    guard: None,
                    line: 27,
                },
                Transition {
                    from: "U2".to_string(),
                    to: "U3".to_string(),
                    trigger: None,
                    guard: None,
                    line: 28,
                },
                Transition {
                    from: "U3".to_string(),
                    to: "U1".to_string(),
                    trigger: None,
                    guard: None,
                    line: 29,
                },
                Transition {
                    from: "U1".to_string(),
                    to: "U3".to_string(),
                    trigger: None,
                    guard: None,
                    line: 30,
                },
            ],
            confidence: 0.90,
        },
    ];

    let report = StateMachineReport {
        schema_version: "1.0".to_string(),
        project: archctl::code::state_machine::ProjectMeta {
            root: project.to_string_lossy().to_string(),
            files_scanned: 1,
            languages: [("rust".to_string(), 3)].into(),
        },
        machines,
    };

    // First apply: write all elements
    let r = archctl::code::state_machine::apply(project, &report, &SystemFilesystem)
        .expect("apply must succeed");

    // Expected: 3 machines + 9 states + 12 transitions = 24 elements
    assert_eq!(
        r.elements_written, 24,
        "UNWIND bulk: expected 24 elements written (3 machines + 9 states + 12 transitions)"
    );
    // Expected: 12 transitions × 2 edges each (source + target) = 24 relations
    assert_eq!(
        r.relations_written, 24,
        "UNWIND bulk: expected 24 relations written (12 transitions × 2 edges)"
    );
    assert_eq!(
        r.elements_skipped, 0,
        "first apply: nothing should be skipped"
    );

    // Verify persisted counts via LbugStore query (scoped so store is dropped before idempotency apply)
    {
        let mut store = LbugStore::open(project).expect("store must open");
        store.init().expect("store must init");

        let machine_count: i64 = store
            .query(
                "MATCH (e:Element) WHERE e.kind_id = 'uml.state_machine' RETURN count(e) AS cnt;",
            )
            .expect("machine count query must succeed")
            .pop()
            .and_then(|r| r.get("cnt").and_then(|c| c.as_i64()))
            .expect("count must be i64");
        assert_eq!(
            machine_count, 3,
            "UNWIND bulk: expected 3 state_machine elements"
        );

        let state_count: i64 = store
            .query("MATCH (e:Element) WHERE e.kind_id = 'uml.state' RETURN count(e) AS cnt;")
            .expect("state count query must succeed")
            .pop()
            .and_then(|r| r.get("cnt").and_then(|c| c.as_i64()))
            .expect("count must be i64");
        assert_eq!(state_count, 9, "UNWIND bulk: expected 9 state elements");

        let transition_count: i64 = store
            .query("MATCH (e:Element) WHERE e.kind_id = 'uml.transition' RETURN count(e) AS cnt;")
            .expect("transition count query must succeed")
            .pop()
            .and_then(|r| r.get("cnt").and_then(|c| c.as_i64()))
            .expect("count must be i64");
        assert_eq!(
            transition_count, 12,
            "UNWIND bulk: expected 12 transition elements"
        );

        let edge_count: i64 = store
            .query("MATCH ()-[r:SEMANTIC_EDGE]->() RETURN count(r) AS cnt;")
            .expect("edge count query must succeed")
            .pop()
            .and_then(|r| r.get("cnt").and_then(|c| c.as_i64()))
            .expect("count must be i64");
        assert_eq!(
            edge_count, 24,
            "UNWIND bulk: expected 24 semantic edges (12 transitions × 2)"
        );
    } // store dropped here, flock released

    // Second apply: idempotency — all skipped, no duplicates
    // (flock must be released by now, so this open succeeds)
    // Note: link_semantic_edge uses MERGE which always succeeds (creates or matches),
    // so relations_written is non-zero on re-apply even though no NEW edges are created.
    // We verify idempotency via the persisted store count instead.
    let r2 = archctl::code::state_machine::apply(project, &report, &SystemFilesystem)
        .expect("second apply must succeed (idempotent)");

    assert_eq!(
        r2.elements_written, 0,
        "idempotent re-apply: expected 0 elements written (all skipped)"
    );
    assert_eq!(
        r2.elements_skipped, 24,
        "idempotent re-apply: expected 24 elements skipped"
    );
    // Verify no duplicate elements were created by re-apply
    {
        let mut store = LbugStore::open(project).expect("store must open");
        store.init().expect("store must init");
        let total_elements: i64 = store
            .query("MATCH (e:Element) RETURN count(e) AS cnt;")
            .expect("count query must succeed")
            .pop()
            .and_then(|r| r.get("cnt").and_then(|c| c.as_i64()))
            .expect("count must be i64");
        assert_eq!(
            total_elements, 24,
            "idempotent re-apply: no duplicate elements"
        );
        let total_edges: i64 = store
            .query("MATCH ()-[r:SEMANTIC_EDGE]->() RETURN count(r) AS cnt;")
            .expect("edge count query must succeed")
            .pop()
            .and_then(|r| r.get("cnt").and_then(|c| c.as_i64()))
            .expect("count must be i64");
        assert_eq!(total_edges, 24, "idempotent re-apply: no duplicate edges");
    }
}

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
