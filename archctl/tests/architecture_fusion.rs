//! Integration tests for `archctl architecture fuse` (Wave 3 Item 27).
//!
//! Exercises the in-process API (`architecture::fusion::fuse_observations`)
//! through the same seeded-store pattern used by `observation_claim.rs`
//! integration tests.

use archctl::architecture::fusion::fuse_observations;
use archctl::observation_claim::observations_and_claims_for_version;
use archctl::store::{DiagramRepository, GraphStore, LbugStore, RawGraphQuery};
use tempfile::TempDir;

fn test_store() -> (LbugStore, TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    let mut store = LbugStore::open(&project).unwrap();
    store.init().unwrap();
    (store, tmp)
}

/// Seed one Evidence row linked to an ElementVersion (raw CREATE —
/// the dual-write seam is not needed for the pure fusion test).
fn seed_evidence_for_version(
    store: &mut LbugStore,
    ev_id: &str,
    version_id: &str,
    claim: &str,
    path: &str,
    kind: &str,
    status: &str,
) {
    store
        .execute_raw_cypher_for_test(&format!(
            "MERGE (v:ElementVersion {{id: '{version_id}'}}) ON CREATE SET v.element_id = 'el:{version_id}', v.name = 'TestEl', v.status = 'active', v.origin = 'ast-grep', v.confidence = 0.9"
        ))
        .expect("seed element version");
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:Evidence {{id: '{ev_id}', kind: '{kind}', claim: '{claim}', path: '{path}', start_line: 10, end_line: 20, tool_name: 'ast-grep', tool_version: '0.1', rule_id: 'test:rule', props: '{{\"status\":\"{status}\"}}', content_hash: 'sha256:{ev_id}', observed_at: timestamp('2026-08-01T00:00:00Z')}})"
        ))
        .expect("seed evidence");
    store
        .execute_raw_cypher_for_test(&format!(
            "MATCH (v:ElementVersion {{id: '{version_id}'}}), (e:Evidence {{id: '{ev_id}'}}) CREATE (v)-[:SUPPORTED_BY]->(e)"
        ))
        .expect("link version to evidence");
}

// ---------------------------------------------------------------------------
// F1 — two observations of the same statement fuse into one claim.
// ---------------------------------------------------------------------------

#[test]
fn fuse_two_supports_into_one_claim() {
    let (mut store, _tmp) = test_store();
    seed_evidence_for_version(
        &mut store,
        "ev:fuse:1",
        "vid-fuse",
        "foo returns int",
        "src/lib.rs",
        "structural",
        "accepted",
    );
    seed_evidence_for_version(
        &mut store,
        "ev:fuse:2",
        "vid-fuse",
        "foo returns int",
        "src/lib.rs",
        "structural",
        "accepted",
    );

    let (observations, _claims) =
        observations_and_claims_for_version(&store, "vid-fuse").expect("read");
    assert_eq!(observations.len(), 2, "two observations expected");

    let fused = fuse_observations(&observations);
    assert_eq!(fused.len(), 1, "same statement must fuse into one claim");
    assert_eq!(fused[0].supports, 2);
    assert_eq!(fused[0].observation_ids.len(), 2, "0 provenance loss");
    assert_eq!(fused[0].derived_from.len(), 2);
    assert!(fused[0].id.starts_with("clm:fused:"));
    assert!(fused[0].conflicts_with.is_empty());
}

// ---------------------------------------------------------------------------
// F2 — contradicting statements produce two cross-linked claims.
// ---------------------------------------------------------------------------

#[test]
fn fuse_contradictions_cross_link() {
    let (mut store, _tmp) = test_store();
    seed_evidence_for_version(
        &mut store,
        "ev:con:1",
        "vid-con",
        "foo returns int",
        "src/api.rs",
        "structural",
        "accepted",
    );
    seed_evidence_for_version(
        &mut store,
        "ev:con:2",
        "vid-con",
        "foo returns string",
        "src/api.rs",
        "structural",
        "accepted",
    );

    let (observations, _claims) =
        observations_and_claims_for_version(&store, "vid-con").expect("read");
    let fused = fuse_observations(&observations);
    assert_eq!(fused.len(), 2, "different statements stay separate");
    assert_eq!(fused[0].conflicts_with.len(), 1);
    assert_eq!(fused[1].conflicts_with.len(), 1);
    assert_eq!(fused[0].conflicts_with[0], fused[1].id);
    assert_eq!(fused[1].conflicts_with[0], fused[0].id);
    assert!(!fused[0].warnings.is_empty());
}

// ---------------------------------------------------------------------------
// F3 — empty version yields no fused claims (exit-0 semantics).
// ---------------------------------------------------------------------------

#[test]
fn fuse_empty_version_yields_no_claims() {
    let (store, _tmp) = test_store();
    let (observations, _claims) =
        observations_and_claims_for_version(&store, "vid-empty").expect("read");
    assert!(observations.is_empty());
    let fused = fuse_observations(&observations);
    assert!(fused.is_empty());
}

// ---------------------------------------------------------------------------
// F4 — JSON shape of a fused claim.
// ---------------------------------------------------------------------------

#[test]
fn fused_claim_json_shape() {
    let (mut store, _tmp) = test_store();
    seed_evidence_for_version(
        &mut store,
        "ev:jsonf:1",
        "vid-jsonf",
        "foo exists",
        "src/lib.rs",
        "structural",
        "accepted",
    );

    let (observations, _claims) =
        observations_and_claims_for_version(&store, "vid-jsonf").expect("read");
    let fused = fuse_observations(&observations);
    assert_eq!(fused.len(), 1);
    let json = serde_json::to_string(&fused[0]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["id"].as_str().unwrap(), fused[0].id);
    assert_eq!(parsed["supports"], 1);
    assert_eq!(parsed["status"], "accepted");
    assert!(parsed.get("observation_ids").unwrap().is_array());
    assert!(parsed.get("conflicts_with").unwrap().is_array());
}

// ---------------------------------------------------------------------------
// Fusion persistence (Wave 3 Item 27 follow-ups, v6 migration).
// ---------------------------------------------------------------------------

/// Seed one Evidence row + its 1:1 `:Observation` row (so FUSED_FROM
/// edges resolve), linked to an ElementVersion.
fn seed_evidence_with_observation(
    store: &mut LbugStore,
    ev_id: &str,
    version_id: &str,
    claim: &str,
    path: &str,
    kind: &str,
    status: &str,
) {
    seed_evidence_for_version(store, ev_id, version_id, claim, path, kind, status);
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:Observation {{id: 'obs:{ev_id}', kind: '{kind}', claim: '{claim}', \
             path: '{path}', start_line: 10, end_line: 20, tool_name: 'ast-grep', \
             tool_version: '0.1', rule_id: 'test:rule', content_hash: 'sha256:{ev_id}', \
             observed_at: '2026-08-01T00:00:00Z'}})"
        ))
        .expect("seed observation row");
}

#[test]
fn fused_claims_persist_and_round_trip() {
    use archctl::architecture::fusion::{fuse_observations, fused_claims_from_rows};

    let (mut store, _tmp) = test_store();
    seed_evidence_with_observation(
        &mut store,
        "ev:persist:1",
        "vid-persist",
        "foo returns int",
        "src/lib.rs",
        "structural",
        "accepted",
    );
    seed_evidence_with_observation(
        &mut store,
        "ev:persist:2",
        "vid-persist",
        "foo returns int",
        "src/lib.rs",
        "structural",
        "accepted",
    );

    let (observations, _claims) =
        observations_and_claims_for_version(&store, "vid-persist").expect("read");
    let fused = fuse_observations(&observations);
    assert_eq!(fused.len(), 1);

    store
        .put_fused_claims("vid-persist", &fused, "2026-08-01T00:00:00Z")
        .expect("persist fused claims");

    let rows = store
        .read_fused_claim_rows(&["vid-persist".to_string()])
        .expect("read fused claim rows")
        .expect("v6 tables present");
    assert_eq!(rows.len(), 1, "one row per fused claim");
    let edges = store
        .list_fused_conflict_edges(&[fused[0].id.clone()])
        .expect("read conflict edges");
    let read_back = fused_claims_from_rows(&rows, &edges);
    assert_eq!(read_back.len(), 1);
    let claim = &read_back[0];
    assert_eq!(claim.id, fused[0].id);
    assert_eq!(claim.kind, fused[0].kind);
    assert_eq!(claim.statement, fused[0].statement);
    assert_eq!(claim.confidence, fused[0].confidence);
    assert_eq!(claim.supports, fused[0].supports);
    assert_eq!(claim.status, fused[0].status);
    assert_eq!(claim.stale, fused[0].stale);
    assert_eq!(claim.observation_ids, fused[0].observation_ids);
    assert_eq!(claim.derived_from, fused[0].derived_from);
    assert_eq!(claim.conflicts_with, fused[0].conflicts_with);

    // FUSED_FROM edges exist for both members.
    let edge_rows = <LbugStore as RawGraphQuery>::query(
        &store,
        "MATCH (f:FusedClaim)-[:FUSED_FROM]->(o:Observation) RETURN f.id AS fid, o.id AS oid",
    )
    .expect("query fused_from edges");
    assert_eq!(edge_rows.len(), 2, "one FUSED_FROM edge per member");
}

#[test]
fn fused_claims_persist_idempotently() {
    use archctl::architecture::fusion::fuse_observations;

    let (mut store, _tmp) = test_store();
    seed_evidence_with_observation(
        &mut store,
        "ev:idem:1",
        "vid-idem",
        "foo exists",
        "src/lib.rs",
        "structural",
        "accepted",
    );

    let (observations, _claims) =
        observations_and_claims_for_version(&store, "vid-idem").expect("read");
    let fused = fuse_observations(&observations);

    store
        .put_fused_claims("vid-idem", &fused, "2026-08-01T00:00:00Z")
        .expect("first persist");
    store
        .put_fused_claims("vid-idem", &fused, "2026-08-01T00:00:00Z")
        .expect("second persist");

    let count =
        <LbugStore as RawGraphQuery>::query(&store, "MATCH (f:FusedClaim) RETURN count(f) AS n")
            .expect("count fused claims");
    let n = count[0].get("n").and_then(|c| c.as_i64()).unwrap_or(-1);
    assert_eq!(n, 1, "MERGE semantics: no duplicates on re-persist");
}

#[test]
fn fused_claims_persist_conflicts_both_directions() {
    use archctl::architecture::fusion::fuse_observations;

    let (mut store, _tmp) = test_store();
    seed_evidence_with_observation(
        &mut store,
        "ev:conf:1",
        "vid-conf",
        "foo returns int",
        "src/a.rs",
        "structural",
        "accepted",
    );
    seed_evidence_with_observation(
        &mut store,
        "ev:conf:2",
        "vid-conf",
        "foo returns string",
        "src/a.rs",
        "structural",
        "accepted",
    );

    let (observations, _claims) =
        observations_and_claims_for_version(&store, "vid-conf").expect("read");
    let fused = fuse_observations(&observations);
    assert_eq!(fused.len(), 2, "contradicting statements stay separate");
    assert_eq!(fused[0].conflicts_with.len(), 1);
    assert_eq!(fused[1].conflicts_with.len(), 1);

    store
        .put_fused_claims("vid-conf", &fused, "2026-08-01T00:00:00Z")
        .expect("persist conflicting claims");

    let ids: Vec<String> = fused.iter().map(|c| c.id.clone()).collect();
    let edges = store
        .list_fused_conflict_edges(&ids)
        .expect("conflict edges");
    assert_eq!(edges.len(), 2, "both directions persisted");
    assert!(edges.contains(&(fused[0].id.clone(), fused[1].id.clone())));
    assert!(edges.contains(&(fused[1].id.clone(), fused[0].id.clone())));

    // Round-trip reconstructs the cross-links + warning.
    let rows = store
        .read_fused_claim_rows(&["vid-conf".to_string()])
        .expect("read")
        .expect("tables");
    let read_back = archctl::architecture::fusion::fused_claims_from_rows(&rows, &edges);
    assert_eq!(read_back.len(), 2);
    for claim in &read_back {
        assert_eq!(claim.conflicts_with.len(), 1, "conflicts reconstructed");
        assert!(!claim.warnings.is_empty(), "warning reconstructed");
    }
}

#[test]
fn fused_claims_read_absent_version_is_empty() {
    let (mut store, _tmp) = test_store();
    let rows = store
        .read_fused_claim_rows(&["vid-never".to_string()])
        .expect("read");
    assert!(
        rows.is_some_and(|r| r.is_empty()),
        "absent version → empty rows"
    );
}
