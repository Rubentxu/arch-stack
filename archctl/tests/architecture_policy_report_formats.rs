//! Integration tests for `archctl architecture policy check --format {json,sarif,junit}`.
//!
//! Exercises the end-to-end pipeline: policy use case + projectors + CLI output.

use archctl::architecture::policy::{
    PolicyParams, PolicyReport, PolicyRule, Severity, check_policy,
};
use archctl::store::{GraphStore, LbugStore};
use chrono::Utc;
use tempfile::TempDir;

/// Helper: open an in-memory store for testing.
fn test_store() -> (LbugStore, TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    let mut store = LbugStore::open(&project).unwrap();
    store.init().unwrap();
    (store, tmp)
}

/// Seed an element with the given id and confidence.
fn seed_element(store: &mut LbugStore, id: &str, confidence: f64) {
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:Element {{id: '{id}', kind_id: 'container', category: 'c4', canonical_key: '{id}', current_name: 'S', current_status: 'active', current_confidence: {confidence}, current_version_id: ''}})"
        ))
        .expect("seed element");
}

/// Seed a semantic edge from source to target.
fn seed_edge(store: &mut LbugStore, source: &str, target: &str) {
    store
        .execute_raw_cypher_for_test(&format!(
            "MATCH (s:Element {{id: '{source}'}}), (t:Element {{id: '{target}'}}) CREATE (s)-[:SEMANTIC_EDGE {{relation_id: 'rel-{source}-{target}', predicate_id: 'depends_on', active: true, order_key: '0'}}]->(t)"
        ))
        .expect("seed edge");
}

fn now() -> chrono::DateTime<Utc> {
    Utc::now()
}

// ---------------------------------------------------------------------------
// SARIF round-trip
// ---------------------------------------------------------------------------

#[test]
fn sarif_roundtrip_json_parse() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", 0.9);
    seed_element(&mut store, "c4:container:b", 0.9);
    seed_edge(&mut store, "c4:container:a", "c4:container:b");

    let policy = vec![PolicyRule::ForbidDependency {
        selector: "c4:container:a".to_string(),
        severity: Severity::Error,
        params: PolicyParams::Dependency {
            target: "c4:container:b".to_string(),
        },
    }];

    let report: PolicyReport = check_policy(&policy, &[], &store, "error", now()).unwrap();
    assert_eq!(report.summary.failed, 1);

    // Project to SARIF and serialise
    let sarif_log = archctl::architecture::to_sarif(&report);
    let json_str = serde_json::to_string(&sarif_log).unwrap();

    // Parse it back
    // Verify JSON is parseable and contains expected structure
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["version"], "2.1.0", "expected SARIF 2.1.0");
    assert!(
        parsed["runs"].is_array(),
        "expected runs array in SARIF output"
    );
    let results = parsed["runs"][0]["results"].as_array().unwrap();
    assert_eq!(results.len(), 1, "expected one result");
    assert_eq!(
        results[0]["ruleId"].as_str().unwrap(),
        "forbid_dependency",
        "expected forbid_dependency rule id"
    );
    assert_eq!(results[0]["level"].as_str().unwrap(), "error");
}

#[test]
fn sarif_result_contains_graph_uri() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", 0.9);
    seed_element(&mut store, "c4:container:b", 0.9);
    seed_edge(&mut store, "c4:container:a", "c4:container:b");

    let policy = vec![PolicyRule::ForbidDependency {
        selector: "c4:container:a".to_string(),
        severity: Severity::Error,
        params: PolicyParams::Dependency {
            target: "c4:container:b".to_string(),
        },
    }];

    let report: PolicyReport = check_policy(&policy, &[], &store, "error", now()).unwrap();
    let sarif_log = archctl::architecture::to_sarif(&report);
    let sarif_json = serde_json::to_string(&sarif_log).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&sarif_json).unwrap();
    let uri = parsed["runs"][0]["results"][0]["locations"][0]["physicalLocation"]
        ["artifactLocation"]["uri"]
        .as_str()
        .unwrap();
    assert!(
        uri.starts_with("archctl://graph/"),
        "expected archctl://graph/ URI prefix, got {uri}"
    );
    assert!(
        uri.contains("c4:container:a"),
        "expected subject id in URI: {uri}"
    );
}

// ---------------------------------------------------------------------------
// JUnit XML output
// ---------------------------------------------------------------------------

#[test]
fn junit_output_contains_violation() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", 0.9);
    seed_element(&mut store, "c4:container:b", 0.9);
    seed_edge(&mut store, "c4:container:a", "c4:container:b");

    let policy = vec![PolicyRule::ForbidDependency {
        selector: "c4:container:a".to_string(),
        severity: Severity::Error,
        params: PolicyParams::Dependency {
            target: "c4:container:b".to_string(),
        },
    }];

    let report: PolicyReport = check_policy(&policy, &[], &store, "error", now()).unwrap();
    let xml = archctl::architecture::to_junit_xml(&report);

    assert!(
        xml.contains("forbid_dependency"),
        "expected rule name in JUnit output: {xml}"
    );
    assert!(
        xml.contains("<failure"),
        "expected <failure> element for error severity: {xml}"
    );
}

#[test]
fn junit_output_error_violation_has_failure_tag() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", 0.9);
    seed_element(&mut store, "c4:container:b", 0.9);
    seed_edge(&mut store, "c4:container:a", "c4:container:b");

    let policy = vec![PolicyRule::ForbidDependency {
        selector: "c4:container:a".to_string(),
        severity: Severity::Error,
        params: PolicyParams::Dependency {
            target: "c4:container:b".to_string(),
        },
    }];

    let report: PolicyReport = check_policy(&policy, &[], &store, "error", now()).unwrap();
    let xml = archctl::architecture::to_junit_xml(&report);

    assert!(
        xml.contains(r#"type="error""#),
        "expected type=\"error\" in JUnit output: {xml}"
    );
    assert!(
        xml.contains("<testcase"),
        "expected <testcase> elements: {xml}"
    );
}

#[test]
fn junit_info_violation_becomes_skipped() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", 0.9);
    // No edge = evidence_required violation will be at Info severity
    let policy = vec![PolicyRule::EvidenceRequired {
        selector: "c4:container:a".to_string(),
        severity: Severity::Info,
    }];

    let report: PolicyReport = check_policy(&policy, &[], &store, "error", now()).unwrap();
    let xml = archctl::architecture::to_junit_xml(&report);

    assert!(
        xml.contains("<skipped/>"),
        "expected <skipped/> for info severity: {xml}"
    );
    assert!(
        !xml.contains("<failure"),
        "info violations must not be <failure>: {xml}"
    );
}

// ---------------------------------------------------------------------------
// Empty report
// ---------------------------------------------------------------------------

#[test]
fn junit_empty_report_has_zero_counts() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", 0.9);
    seed_element(&mut store, "c4:container:b", 0.9);
    // No violations possible with this selector
    let policy = vec![PolicyRule::ForbidDependency {
        selector: "c4:container:other*".to_string(),
        severity: Severity::Error,
        params: PolicyParams::Dependency {
            target: "c4:container:b".to_string(),
        },
    }];

    let report: PolicyReport = check_policy(&policy, &[], &store, "error", now()).unwrap();
    let xml = archctl::architecture::to_junit_xml(&report);

    assert!(
        xml.contains(r#"tests="0""#),
        "expected tests=\"0\" for empty report: {xml}"
    );
    assert!(
        xml.contains(r#"failures="0""#),
        "expected failures=\"0\" for empty report: {xml}"
    );
}

#[test]
fn sarif_empty_report_has_no_results() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", 0.9);

    let policy = vec![PolicyRule::ForbidDependency {
        selector: "c4:container:other*".to_string(),
        severity: Severity::Error,
        params: PolicyParams::Dependency {
            target: "c4:container:b".to_string(),
        },
    }];

    let report: PolicyReport = check_policy(&policy, &[], &store, "error", now()).unwrap();
    let log = archctl::architecture::to_sarif(&report);
    let log_json = serde_json::to_string(&log).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&log_json).unwrap();
    // results field is skip_serializing_if="Vec::is_empty" so it may not be present when empty
    let results = parsed["runs"][0]["results"].as_array();
    assert!(
        results.map(|a| a.is_empty()).unwrap_or(true),
        "expected empty results for clean graph"
    );
}
