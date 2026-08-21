//! Integration tests for `archctl architecture coverage`.
//!
//! These tests exercise the full CLI command with a real (in-memory) store.

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

/// Seed an element with given confidence and version id.
fn seed_element(
    store: &mut LbugStore,
    id: &str,
    version_id: &str,
    confidence: f64,
    category: &str,
) {
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:Element {{id: '{id}', kind_id: 'container', category: '{category}', canonical_key: '{id}', current_name: 'TestService', current_status: 'active', current_confidence: {confidence}, current_version_id: '{version_id}'}})"
        ))
        .expect("seed element");
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:ElementVersion {{id: '{version_id}', element_id: '{id}', name: 'TestService', status: 'active', origin: 'ast-grep', confidence: {confidence}}})"
        ))
        .expect("seed element version");
}

/// Seed evidence with given status and observed_at timestamp.
fn seed_evidence(store: &mut LbugStore, version_id: &str, status: &str, observed_at: &str) {
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:Evidence {{id: 'ev:{version_id}:{status}', kind: 'structural', claim: 'test evidence', path: 'src/lib.rs', start_line: 10, end_line: 15, tool_name: 'ast-grep', tool_version: '0.1', rule_id: 'test:rule', props: '{{\"status\":\"{status}\"}}', content_hash: 'sha256:abc', observed_at: timestamp('{observed_at}')}})"
        ))
        .expect("seed evidence");
    store
        .execute_raw_cypher_for_test(&format!(
            "MATCH (v:ElementVersion {{id: '{version_id}'}}), (e:Evidence {{id: 'ev:{version_id}:{status}'}}) CREATE (v)-[:SUPPORTED_BY]->(e)"
        ))
        .expect("link version to evidence");
}

// ---------------------------------------------------------------------------
// CLI integration tests
// ---------------------------------------------------------------------------

#[test]
fn coverage_cli_no_elements_empty_report() {
    let (store, _tmp) = test_store();
    let report = archctl::architecture::coverage(&store, &archctl::clock::SystemClock)
        .expect("coverage should succeed");

    assert_eq!(report.total_elements, 0);
    assert_eq!(report.total_relations, 0);
    assert_eq!(report.by_confidence.high, 0);
    assert_eq!(report.by_confidence.unknown, 0);
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("CONTRADICTED_BY"))
    );
}

#[test]
fn coverage_mixed_confidence_buckets() {
    let (mut store, _tmp) = test_store();
    // high: >= 0.9
    seed_element(&mut store, "c4:container:a", "v:1", 0.95, "c4");
    // medium: >= 0.7
    seed_element(&mut store, "c4:container:b", "v:2", 0.80, "c4");
    // low: >= 0.5
    seed_element(&mut store, "c4:container:c", "v:3", 0.60, "c4");
    // unknown: < 0.5
    seed_element(&mut store, "c4:container:d", "v:4", 0.40, "c4");

    let report = archctl::architecture::coverage(&store, &archctl::clock::SystemClock)
        .expect("coverage should succeed");

    assert_eq!(report.total_elements, 4);
    assert_eq!(report.by_confidence.high, 1);
    assert_eq!(report.by_confidence.medium, 1);
    assert_eq!(report.by_confidence.low, 1);
    assert_eq!(report.by_confidence.unknown, 1);
}

#[test]
fn coverage_evidence_status_buckets() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", "v:1", 0.95, "c4");
    seed_evidence(&mut store, "v:1", "accepted", "2026-08-01T00:00:00Z");

    seed_element(&mut store, "c4:container:b", "v:2", 0.90, "c4");
    seed_evidence(&mut store, "v:2", "drafted", "2026-08-01T00:00:00Z");
    seed_evidence(&mut store, "v:2", "drafted2", "2026-08-01T00:00:00Z");

    seed_element(&mut store, "c4:container:c", "v:3", 0.85, "c4");
    seed_evidence(&mut store, "v:3", "superseded", "2026-08-01T00:00:00Z");

    let report = archctl::architecture::coverage(&store, &archctl::clock::SystemClock)
        .expect("coverage should succeed");

    assert_eq!(report.by_evidence_status.accepted, 1);
    assert_eq!(report.by_evidence_status.drafted, 2);
    assert_eq!(report.by_evidence_status.superseded, 1);
}

#[test]
fn coverage_conflict_warning_always_present() {
    let (store, _tmp) = test_store();
    let report = archctl::architecture::coverage(&store, &archctl::clock::SystemClock)
        .expect("coverage should succeed");

    // CONTRADICTED_BY is always 0 — warning must be present
    assert_eq!(report.by_conflict.conflicted, 0);
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("CONTRADICTED_BY"))
    );
}

#[test]
fn coverage_schema_version_invariant() {
    let (store, _tmp) = test_store();
    let report = archctl::architecture::coverage(&store, &archctl::clock::SystemClock)
        .expect("coverage should succeed");

    assert_eq!(report.schema_version, "1.1");
    assert_eq!(report.capability, "architecture-coverage-mvp");
}

// ---------------------------------------------------------------------------
// Fused claims buckets (v6, Wave 3 Item 27 follow-ups)
// ---------------------------------------------------------------------------

#[test]
fn coverage_fused_claim_buckets() {
    use archctl::architecture::fusion::FusedClaim;
    use archctl::store::DiagramRepository;

    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", "v:1", 0.95, "c4");

    // Claim A: multi-support (2), fresh.
    let a = FusedClaim {
        id: "clm:fused:aaaa".to_string(),
        kind: "structural".to_string(),
        statement: "foo exists".to_string(),
        confidence: 1.0,
        observation_ids: vec!["obs:ev:a1".to_string(), "obs:ev:a2".to_string()],
        derived_from: vec!["ev:a1".to_string(), "ev:a2".to_string()],
        supports: 2,
        conflicts_with: vec![],
        status: "accepted".to_string(),
        stale: false,
        warnings: vec![],
        evidence_origin: "user_workspace".to_string(),
    };
    // Claim B: single-support (1), stale.
    let b = FusedClaim {
        id: "clm:fused:bbbb".to_string(),
        kind: "structural".to_string(),
        statement: "bar exists".to_string(),
        confidence: 0.5,
        observation_ids: vec!["obs:ev:b1".to_string()],
        derived_from: vec!["ev:b1".to_string()],
        supports: 1,
        conflicts_with: vec![],
        status: "accepted".to_string(),
        stale: true,
        warnings: vec![],
        evidence_origin: "user_workspace".to_string(),
    };
    store
        .put_fused_claims("v:1", &[a, b], "2026-08-01T00:00:00Z")
        .expect("persist fused claims");

    let report = archctl::architecture::coverage(&store, &archctl::clock::SystemClock)
        .expect("coverage should succeed");

    assert_eq!(report.schema_version, "1.1");
    assert_eq!(report.by_fused_claims.total, 2);
    assert_eq!(report.by_fused_claims.by_supports_single, 1);
    assert_eq!(report.by_fused_claims.by_supports_multi, 1);
    assert_eq!(report.by_fused_claims.by_conflicts, 0);
    assert_eq!(report.by_fused_claims.by_staleness_fresh, 1);
    assert_eq!(report.by_fused_claims.by_staleness_stale, 1);
    // Invariant: total == single + multi == fresh + stale.
    assert_eq!(
        report.by_fused_claims.total,
        report.by_fused_claims.by_supports_single + report.by_fused_claims.by_supports_multi
    );
    assert_eq!(
        report.by_fused_claims.total,
        report.by_fused_claims.by_staleness_fresh + report.by_fused_claims.by_staleness_stale
    );
}

#[test]
fn coverage_fused_claims_zero_when_none_persisted() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", "v:1", 0.95, "c4");

    let report = archctl::architecture::coverage(&store, &archctl::clock::SystemClock)
        .expect("coverage should succeed");

    assert_eq!(report.schema_version, "1.1");
    assert_eq!(report.by_fused_claims.total, 0);
    assert_eq!(report.by_fused_claims.by_supports_single, 0);
    assert_eq!(report.by_fused_claims.by_supports_multi, 0);
    assert_eq!(report.by_fused_claims.by_conflicts, 0);
    assert_eq!(report.by_fused_claims.by_staleness_fresh, 0);
    assert_eq!(report.by_fused_claims.by_staleness_stale, 0);
}
