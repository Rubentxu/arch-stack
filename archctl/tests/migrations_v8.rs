//! REQ-T08-003 v8 migration coverage.
//!
//! Mirrors the v7 migration test pattern. Uses TempDir + LbugStore::open.
//! See: sddk/p-38e02210a9f14317/trust-008-m30-bridge-promotion/specification.md REQ-T08-003.

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

/// SCN-T08-003b: v8 hook is non-mutating and idempotent.
///
/// This test seeds a fresh graph with 3 FusedClaim rows that have
/// `pending_adjudication_event = true` and no backing `(:Adjudication)`
/// event (the pre-v8 offender shape), then runs the v8 rust hook
/// directly. The hook must succeed without raising an error and must
/// leave the FusedClaim rows unchanged (HITL preserved).
#[test]
fn v8_hook_is_non_mutating_on_pre_v8_offenders() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("proj");
    let fs = system_fs();

    // First init: brings graph to v9 (latest).
    graph_init(&project, &fs).unwrap();

    let mut store = LbugStore::open(&project).unwrap();
    store.init().unwrap();

    // Seed 3 offenders (pre-v8 shape: pending_adjudication_event=true, status=drafted).
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

    // Run the v8 hook directly. It must succeed (non-mutating) and
    // surface the 3 offenders via tracing::warn! (capture is best-effort;
    // we assert success + post-state instead of capturing log output).
    let hook_result = backfill_adjudication_event_diagnostics(&mut store);
    assert!(
        hook_result.is_ok(),
        "v8 hook must succeed on graph with offenders; got {hook_result:?}"
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

    // Adjudication node table must exist (v8 migration created it).
    let adj_count: i64 = store
        .query("MATCH (a:Adjudication) RETURN count(a) AS n;")
        .unwrap()
        .first()
        .and_then(|r| r.get("n").and_then(|c| c.as_i64()))
        .unwrap_or(0);
    assert_eq!(
        adj_count, 0,
        "Adjudication table must exist with 0 rows (no Promote events)"
    );

    // Idempotent re-call: must succeed without error.
    let hook_result2 = backfill_adjudication_event_diagnostics(&mut store);
    assert!(
        hook_result2.is_ok(),
        "v8 hook must be idempotent; second call must succeed; got {hook_result2:?}"
    );
}

/// SCN-T08-003c: graph already at latest (v9); second migrate; marker unchanged;
/// IF NOT EXISTS guards skip DDL.
#[test]
fn v8_re_run_is_noop() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("proj");
    let fs = system_fs();

    // First init: brings graph to v9 (latest in the chain).
    graph_init(&project, &fs).unwrap();
    let marker = project.join(SCHEMA_MARKER_FILENAME);

    let mut store = LbugStore::open(&project).unwrap();
    store.init().unwrap();

    // apply_pending on a fresh graph should advance marker to v9 (one pass
    // through all pending migrations). The DDL is idempotent (IF NOT EXISTS),
    // so the second call must be a no-op.
    let _ = apply_pending(store.session_for_migrations(), &fs, &marker).unwrap();

    let version_before = current_version(&marker, &fs).unwrap().unwrap();
    let applied = apply_pending(store.session_for_migrations(), &fs, &marker).unwrap();
    assert!(
        applied.is_empty(),
        "re-run on graph already at latest must be no-op; got applied {applied:?}"
    );

    // Marker must be unchanged after second pass.
    let version = current_version(&marker, &fs).unwrap().unwrap();
    assert_eq!(
        version, version_before,
        "marker must stay unchanged after noop re-run"
    );
    assert_eq!(
        version, "v9-fused-claim-evidence-origin",
        "fresh graph must reach v9 (latest in the chain)"
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
