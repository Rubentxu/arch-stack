//! Integration tests for `archctl architecture observe`.
//!
//! Exercises the in-process API (`observation_claim::observations_and_claims_for_version`)
//! against a real store seeded via `execute_raw_cypher_for_test`.
//!
//! The earlier `Command::new("cargo")`-based tests were removed: the CLI
//! computes its own `project_dir` from the cwd's source identity, so the
//! spawned process opens a DIFFERENT store than the test's TempDir.
//! In-process calls share the same store (pattern used by every P2 test
//! in this repo).

use archctl::observation_claim::{
    Observation, ObservationError, compat_claim_from_evidence, observation_from_evidence,
    observations_and_claims_for_version,
};
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

/// Seed an EvidenceEntry linked to an ElementVersion.
///
/// Uses `MERGE` for the element/version nodes (idempotent across
/// multiple calls with the same `version_id`) so a test can register
/// several evidence rows against one version without panicking.
fn seed_evidence_for_version(store: &mut LbugStore, ev_id: &str, version_id: &str, status: &str) {
    store
        .execute_raw_cypher_for_test(&format!(
            "MERGE (e:Element {{id: 'el:{version_id}'}}) ON CREATE SET e.kind_id = 'container', e.category = 'c4', e.canonical_key = 'el:{version_id}', e.current_name = 'TestEl', e.current_status = 'active', e.current_confidence = 0.9, e.current_version_id = '{version_id}'"
        ))
        .expect("seed element");
    store
        .execute_raw_cypher_for_test(&format!(
            "MERGE (v:ElementVersion {{id: '{version_id}'}}) ON CREATE SET v.element_id = 'el:{version_id}', v.name = 'TestEl', v.status = 'active', v.origin = 'ast-grep', v.confidence = 0.9"
        ))
        .expect("seed element version");
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:Evidence {{id: '{ev_id}', kind: 'structural', claim: 'test evidence for {ev_id}', path: 'src/lib.rs', start_line: 10, end_line: 20, tool_name: 'ast-grep', tool_version: '0.1', rule_id: 'test:rule', props: '{{\"status\":\"{status}\"}}', content_hash: 'sha256:{ev_id}', observed_at: timestamp('2026-08-01T00:00:00Z')}})"
        ))
        .expect("seed evidence");
    store
        .execute_raw_cypher_for_test(&format!(
            "MATCH (v:ElementVersion {{id: '{version_id}'}}), (e:Evidence {{id: '{ev_id}'}}) CREATE (v)-[:SUPPORTED_BY]->(e)"
        ))
        .expect("link version to evidence");
}

// ---------------------------------------------------------------------------
// S1 — happy path: 2 evidence rows on one version_id
// ---------------------------------------------------------------------------

#[test]
fn observe_happy_path_two_observations_one_version() {
    let (mut store, _tmp) = test_store();
    seed_evidence_for_version(&mut store, "ev:json:1", "vid-json", "accepted");
    seed_evidence_for_version(&mut store, "ev:json:2", "vid-json", "drafted");

    let (observations, claims) = observations_and_claims_for_version(&store, "vid-json").unwrap();

    assert_eq!(observations.len(), 2);
    assert_eq!(claims.len(), 2);

    assert_eq!(observations[0].id, "obs:ev:json:1");
    assert_eq!(observations[1].id, "obs:ev:json:2");
    assert_eq!(claims[0].id, "clm:compat:ev:json:1");
    assert_eq!(claims[1].id, "clm:compat:ev:json:2");

    // Parallel arrays + fused=false literal
    assert!(!claims[0].fused, "claim fused must be false");
    assert!(!claims[1].fused, "claim fused must be false");

    // Status mirrors evidence.status
    assert_eq!(claims[0].status, "accepted");
    assert_eq!(claims[1].status, "drafted");

    // Confidence defaulted per status
    assert!((claims[0].confidence - 1.0).abs() < f64::EPSILON);
    assert!((claims[1].confidence - 0.0).abs() < f64::EPSILON);

    // Observation ids parse cleanly as JSON too
    let json = serde_json::to_string(&observations[0]).unwrap();
    assert!(json.contains("\"id\":\"obs:ev:json:1\""));
}

// ---------------------------------------------------------------------------
// S2 — invalid version_id: returns ObservationError::InvalidVersionId
// ---------------------------------------------------------------------------

#[test]
fn observe_invalid_version_id_errors() {
    let (store, _tmp) = test_store();
    let result = observations_and_claims_for_version(&store, "bad;id");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ObservationError::InvalidVersionId(_)));
}

// ---------------------------------------------------------------------------
// S3 — empty version: returns empty arrays without error
// ---------------------------------------------------------------------------

#[test]
fn observe_empty_version_returns_empty() {
    let (mut store, _tmp) = test_store();
    seed_evidence_for_version(&mut store, "ev:other:1", "v:other", "accepted");

    let (observations, claims) = observations_and_claims_for_version(&store, "vid-empty").unwrap();

    assert!(observations.is_empty());
    assert!(claims.is_empty());
}

// ---------------------------------------------------------------------------
// S4 — JSON roundtrip: Observation + Claim serialise/deserialise losslessly
// ---------------------------------------------------------------------------

#[test]
fn json_roundtrip() {
    use archctl::diagram::export_types::EvidenceEntry;

    let ev = EvidenceEntry {
        id: "ev:round".to_string(),
        kind: "structural".to_string(),
        claim: "roundtrip test".to_string(),
        path: "src/lib.rs".to_string(),
        start_line: 5,
        end_line: 15,
        tool_name: "ast-grep".to_string(),
        tool_version: "0.1".to_string(),
        rule_id: "test:round".to_string(),
        content_hash: "sha256:round".to_string(),
        observed_at: "2026-08-01T00:00:00Z".to_string(),
        status: Some("accepted".to_string()),
    };

    let obs: Observation = observation_from_evidence(&ev);
    let claim = compat_claim_from_evidence(&ev);

    let obs_json = serde_json::to_string(&obs).unwrap();
    let claim_json = serde_json::to_string(&claim).unwrap();

    let obs_parsed: serde_json::Value = serde_json::from_str(&obs_json).unwrap();
    let claim_parsed: serde_json::Value = serde_json::from_str(&claim_json).unwrap();

    let obs_json2 = serde_json::to_string(&obs_parsed).unwrap();
    let claim_json2 = serde_json::to_string(&claim_parsed).unwrap();

    assert_eq!(obs_json, obs_json2, "Observation must roundtrip losslessly");
    assert_eq!(claim_json, claim_json2, "Claim must roundtrip losslessly");
}
