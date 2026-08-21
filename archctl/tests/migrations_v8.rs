//! REQ-T08-003 v8 migration coverage.
//!
//! Mirrors the v7 migration test pattern. Uses TempDir + LbugStore::open.
//! See: sddk/p-38e02210a9f14317/trust-008-m30-bridge-promotion/specification.md REQ-T08-003.

use std::fs;

use tempfile::TempDir;

use archctl::filesystem::SystemFilesystem;
use archctl::graph::init as graph_init;
use archctl::migrations::{
    SCHEMA_MARKER_FILENAME, apply_pending, backfill_adjudication_event_diagnostics, current_version,
};
use archctl::store::{GraphStore, LbugStore, RawGraphQuery};

fn system_fs() -> SystemFilesystem {
    SystemFilesystem
}

/// SCN-T08-003a: empty tempdir; schema migrate; marker reads v8.
#[test]
fn v8_clean_install_advances_marker() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("proj");
    let fs = system_fs();
    graph_init(&project, &fs).unwrap();

    let marker = project.join(SCHEMA_MARKER_FILENAME);
    let version = current_version(&marker, &fs).unwrap().unwrap();
    // Fresh graph: marker must reach the latest migration (v9-fused-claim-evidence-origin).
    assert_eq!(
        version, "v9-fused-claim-evidence-origin",
        "fresh graph must advance to latest migration version"
    );
}

/// SCN-T08-003b: graph at v7 seeded with 3 offenders; schema migrate;
/// marker advances; tables created; pre-existing FusedClaim rows
/// unchanged. The v8 rust_hook emits tracing::warn! for offenders
/// but does NOT mutate.
#[test]
fn v7_to_v8_upgrade_logs_offenders_without_mutating() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("proj");
    let fs = system_fs();

    // First init: brings graph to v9.
    graph_init(&project, &fs).unwrap();

    // Simulate a v7-upgraded graph by resetting the marker to v7.
    // The DDL for v8 (Adjudication node table + ADJUDICATES edge)
    // will be applied by apply_pending.
    let marker = project.join(SCHEMA_MARKER_FILENAME);
    fs::write(&marker, "v7-observation-status").unwrap();

    // Verify pre-v8 FusedClaim rows exist with pending_adjudication_event=true.
    // The v8 hook should NOT modify these.
    let mut store = LbugStore::open(&project).unwrap();
    store.init().unwrap();
    store
        .execute_raw_cypher_for_test(
            "MERGE (f:FusedClaim {id: 'offender:1'}) \
             SET f.pending_adjudication_event = true, f.status = 'drafted';",
        )
        .unwrap();
    store
        .execute_raw_cypher_for_test(
            "MERGE (f:FusedClaim {id: 'offender:2'}) \
             SET f.pending_adjudication_event = true, f.status = 'drafted';",
        )
        .unwrap();
    store
        .execute_raw_cypher_for_test(
            "MERGE (f:FusedClaim {id: 'offender:3'}) \
             SET f.pending_adjudication_event = true, f.status = 'drafted';",
        )
        .unwrap();

    // Run apply_pending — should advance from v7 to v9 (v8 + v9 applied).
    let applied = apply_pending(store.session_for_migrations(), &fs, &marker).unwrap();
    assert!(
        applied.iter().any(|v| v == "v8-adjudication-event-store"),
        "v8 migration must be applied; got {applied:?}"
    );

    // Marker must now be at latest.
    let final_version = current_version(&marker, &fs).unwrap().unwrap();
    assert_eq!(
        final_version, "v9-fused-claim-evidence-origin",
        "marker must advance to latest after applying v8 + v9"
    );

    // Adjudication node table must exist (v8 created it).
    let adj_count: i64 = store
        .query("MATCH (a:Adjudication) RETURN count(a) AS n;")
        .unwrap()
        .first()
        .and_then(|r| r.get("n").and_then(|c| c.as_i64()))
        .unwrap_or(0);
    assert_eq!(
        adj_count, 0,
        "Adjudication table must exist but have 0 rows post-migration"
    );

    // FusedClaim rows must be unchanged (hook is non-mutating).
    let offenders: Vec<String> = store
        .query(
            "MATCH (f:FusedClaim) \
             WHERE f.pending_adjudication_event = true \
             RETURN f.id;",
        )
        .unwrap()
        .iter()
        .filter_map(|r| {
            r.get("f.id")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    assert_eq!(
        offenders.len(),
        3,
        "all 3 offenders must remain; v8 hook is non-mutating; got {offenders:?}"
    );

    // v8 hook can be called independently (idempotent check).
    let hook_result = backfill_adjudication_event_diagnostics(&mut store);
    assert!(
        hook_result.is_ok(),
        "v8 hook must succeed on graph with offenders; got {hook_result:?}"
    );
}

/// SCN-T08-003c: graph already at v8; second migrate; marker unchanged;
/// IF NOT EXISTS guards skip DDL.
#[test]
fn v8_re_run_is_noop() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("proj");
    let fs = system_fs();

    // First init.
    graph_init(&project, &fs).unwrap();
    let marker = project.join(SCHEMA_MARKER_FILENAME);

    // Reset marker to v8 to simulate an already-at-v8 graph.
    fs::write(&marker, "v8-adjudication-event-store").unwrap();

    let mut store = LbugStore::open(&project).unwrap();
    store.init().unwrap();

    // apply_pending should find v8 >= current and skip all.
    let applied = apply_pending(store.session_for_migrations(), &fs, &marker).unwrap();
    assert!(
        applied.is_empty(),
        "re-run on v8 graph must be no-op; got applied {applied:?}"
    );

    // Marker must be unchanged.
    let version = current_version(&marker, &fs).unwrap().unwrap();
    assert_eq!(
        version, "v8-adjudication-event-store",
        "marker must stay at v8 after noop re-run"
    );

    // Adjudication table must still be accessible (not dropped).
    let adj_count: i64 = store
        .query("MATCH (a:Adjudication) RETURN count(a) AS n;")
        .unwrap()
        .first()
        .and_then(|r| r.get("n").and_then(|c| c.as_i64()))
        .unwrap_or(0);
    assert_eq!(
        adj_count, 0,
        "Adjudication table must still be queryable after noop re-run"
    );
}
