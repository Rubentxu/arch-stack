//! Integration tests for `archctl architecture fuse` (Wave 3 Item 27).
//!
//! Exercises the in-process API (`architecture::fusion::fuse_observations`)
//! through the same seeded-store pattern used by `observation_claim.rs`
//! integration tests.

use archctl::architecture::fusion::fuse_observations;
use archctl::observation_claim::observations_and_claims_for_version;
use archctl::store::{GraphStore, LbugStore};
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
