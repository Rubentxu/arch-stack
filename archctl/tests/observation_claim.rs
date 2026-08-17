//! Integration tests for `archctl architecture observe`.
//!
//! Exercises the full CLI command with a real in-memory store seeded via
//! `execute_raw_cypher_for_test`.

use archctl::store::{GraphStore, LbugStore};
use std::process::Command;
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
fn seed_evidence_for_version(store: &mut LbugStore, ev_id: &str, version_id: &str, status: &str) {
    // Seed element + version
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:Element {{id: 'el:{version_id}', kind_id: 'container', category: 'c4', canonical_key: 'el:{version_id}', current_name: 'TestEl', current_status: 'active', current_confidence: 0.9, current_version_id: '{version_id}'}})"
        ))
        .expect("seed element");
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:ElementVersion {{id: '{version_id}', element_id: 'el:{version_id}', name: 'TestEl', status: 'active', origin: 'ast-grep', confidence: 0.9}})"
        ))
        .expect("seed element version");
    // Seed evidence with status in props JSON
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
// S8 — happy path with --json
// ---------------------------------------------------------------------------

#[test]
fn observe_json_happy_path() {
    let (mut store, tmp) = test_store();
    seed_evidence_for_version(&mut store, "ev:json:1", "vid-json", "accepted");
    seed_evidence_for_version(&mut store, "ev:json:2", "vid-json", "drafted");

    let archctl_bin = std::env::var("ARCHCTL_BIN")
        .unwrap_or_else(|_| "cargo run --quiet --bin archctl --".to_string());

    let output = if archctl_bin.starts_with("cargo") {
        Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "--bin",
                "archctl",
                "--",
                "architecture",
                "observe",
                "--version-id",
                "vid-json",
                "--json",
                "--cwd",
            ])
            .arg(tmp.path())
            .output()
            .expect("cargo run failed")
    } else {
        Command::new(&archctl_bin)
            .args([
                "architecture",
                "observe",
                "--version-id",
                "vid-json",
                "--json",
                "--cwd",
            ])
            .arg(tmp.path())
            .output()
            .expect("archctl run failed")
    };

    assert!(
        output.status.success(),
        "observe --json exited non-zero: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    let observations = parsed["observations"]
        .as_array()
        .expect("observations must be array");
    let claims = parsed["claims"].as_array().expect("claims must be array");

    assert_eq!(
        observations.len(),
        2,
        "must have 2 observations for vid-json"
    );
    assert_eq!(claims.len(), 2, "must have 2 claims for vid-json");

    // Parallel arrays: observation[i] maps to claim[i] via derived_from
    assert_eq!(observations[0]["id"], "obs:ev:json:1");
    assert_eq!(observations[1]["id"], "obs:ev:json:2");
    assert_eq!(claims[0]["id"], "clm:compat:ev:json:1");
    assert_eq!(claims[1]["id"], "clm:compat:ev:json:2");
    assert!(
        !claims[0]["fused"].as_bool().unwrap_or(true),
        "claim fused must be false"
    );
    assert!(
        !claims[1]["fused"].as_bool().unwrap_or(true),
        "claim fused must be false"
    );
    assert_eq!(claims[0]["status"], "accepted");
    assert_eq!(claims[1]["status"], "drafted");
    assert!((claims[0]["confidence"].as_f64().unwrap_or(0.0) - 1.0).abs() < f64::EPSILON);
    assert!((claims[1]["confidence"].as_f64().unwrap_or(1.0) - 0.0).abs() < f64::EPSILON);
}

// ---------------------------------------------------------------------------
// S8b — invalid version id exits non-zero
// ---------------------------------------------------------------------------

#[test]
fn observe_invalid_version_id_exits_nonzero() {
    let (mut store, tmp) = test_store();
    seed_evidence_for_version(&mut store, "ev:valid:1", "v:valid", "accepted");

    let archctl_bin = std::env::var("ARCHCTL_BIN")
        .unwrap_or_else(|_| "cargo run --quiet --bin archctl --".to_string());

    let output = if archctl_bin.starts_with("cargo") {
        Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "--bin",
                "archctl",
                "--",
                "architecture",
                "observe",
                "--version-id",
                "bad;id",
                "--json",
                "--cwd",
            ])
            .arg(tmp.path())
            .output()
            .expect("cargo run failed")
    } else {
        Command::new(&archctl_bin)
            .args([
                "architecture",
                "observe",
                "--version-id",
                "bad;id",
                "--json",
                "--cwd",
            ])
            .arg(tmp.path())
            .output()
            .expect("archctl run failed")
    };

    assert!(
        !output.status.success(),
        "observe with bad;id must exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid") || stderr.contains("error"),
        "stderr must mention invalid/error: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// Empty version returns empty arrays
// ---------------------------------------------------------------------------

#[test]
fn observe_empty_version_returns_empty() {
    let (mut store, tmp) = test_store();
    seed_evidence_for_version(&mut store, "ev:other:1", "v:other", "accepted");

    let archctl_bin = std::env::var("ARCHCTL_BIN")
        .unwrap_or_else(|_| "cargo run --quiet --bin archctl --".to_string());

    let output = if archctl_bin.starts_with("cargo") {
        Command::new("cargo")
            .args([
                "run",
                "--quiet",
                "--bin",
                "archctl",
                "--",
                "architecture",
                "observe",
                "--version-id",
                "vid-empty",
                "--json",
                "--cwd",
            ])
            .arg(tmp.path())
            .output()
            .expect("cargo run failed")
    } else {
        Command::new(&archctl_bin)
            .args([
                "architecture",
                "observe",
                "--version-id",
                "vid-empty",
                "--json",
                "--cwd",
            ])
            .arg(tmp.path())
            .output()
            .expect("archctl run failed")
    };

    assert!(
        output.status.success(),
        "observe on empty version must exit 0"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout must be valid JSON");

    assert_eq!(parsed["observations"].as_array().unwrap().len(), 0);
    assert_eq!(parsed["claims"].as_array().unwrap().len(), 0);
}

// ---------------------------------------------------------------------------
// JSON roundtrip: emit Observation + Claim JSON, parse back
// ---------------------------------------------------------------------------

#[test]
fn json_roundtrip() {
    use archctl::diagram::export_types::EvidenceEntry;
    use archctl::observation_claim::{compat_claim_from_evidence, observation_from_evidence};

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

    let obs = observation_from_evidence(&ev);
    let claim = compat_claim_from_evidence(&ev);

    // Serialize both
    let obs_json = serde_json::to_string(&obs).unwrap();
    let claim_json = serde_json::to_string(&claim).unwrap();

    // Deserialize back as serde_json::Value to confirm valid JSON
    let obs_parsed: serde_json::Value = serde_json::from_str(&obs_json).unwrap();
    let claim_parsed: serde_json::Value = serde_json::from_str(&claim_json).unwrap();

    // Re-serialize to confirm no data loss
    let obs_json2 = serde_json::to_string(&obs_parsed).unwrap();
    let claim_json2 = serde_json::to_string(&claim_parsed).unwrap();

    assert_eq!(obs_json, obs_json2, "Observation must roundtrip losslessly");
    assert_eq!(claim_json, claim_json2, "Claim must roundtrip losslessly");
}
