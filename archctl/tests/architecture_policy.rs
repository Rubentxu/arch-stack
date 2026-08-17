//! Integration tests for `archctl architecture policy check`.
//!
//! These tests exercise the policy use case with a real (in-memory) store.
//! Graph data is seeded via `execute_raw_cypher_for_test`.

use archctl::architecture::policy::{
    PolicyParams, PolicyReport, PolicyRule, Severity, Waiver, check_policy,
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
/// Edges are REL rows `(src:Element)-[:SEMANTIC_EDGE {active: true}]->(tgt:Element)`
/// — the same shape `list_semantic_edges` reads in the real store.
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
// S1: Policy with no violations → all passed
// ---------------------------------------------------------------------------

#[test]
fn policy_check_no_violations_all_passed() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", 0.9);
    seed_element(&mut store, "c4:container:b", 0.9);
    seed_edge(&mut store, "c4:container:a", "c4:container:b");

    let policy = vec![PolicyRule::ForbidDependency {
        selector: "c4:container:other*".to_string(),
        severity: Severity::Error,
        params: PolicyParams::Dependency {
            target: "c4:container:b".to_string(),
        },
    }];

    let report: PolicyReport = check_policy(&policy, &[], &store, "error", now()).unwrap();
    assert_eq!(report.summary.total, 1);
    assert_eq!(report.summary.passed, 1);
    assert_eq!(report.summary.failed, 0);
    assert_eq!(report.schema_version, "1.0");
    assert_eq!(report.capability, "architecture-policy-mvp");
}

// ---------------------------------------------------------------------------
// S2: Violation triggers failed count
// ---------------------------------------------------------------------------

#[test]
fn policy_check_forbid_dependency_violation() {
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

    let report = check_policy(&policy, &[], &store, "error", now()).unwrap();
    assert_eq!(report.summary.failed, 1);
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].rule, "forbid_dependency");
    assert_eq!(report.violations[0].subject.id, "c4:container:a");
}

// ---------------------------------------------------------------------------
// S3: Active waiver suppresses a violation
// ---------------------------------------------------------------------------

#[test]
fn policy_check_waiver_suppresses_violation() {
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
    let waivers = vec![Waiver {
        rule: "forbid_dependency".to_string(),
        subject_id: "c4:container:a".to_string(),
        reason: "intentional shared kernel".to_string(),
        expires_at: now() + chrono::Duration::days(30),
        expired: false,
    }];

    let report = check_policy(&policy, &waivers, &store, "error", now()).unwrap();
    assert_eq!(report.summary.failed, 0);
    assert_eq!(report.summary.waived, 1);
    assert_eq!(report.waivers.len(), 1);
    assert!(!report.waivers[0].expired);
}

// ---------------------------------------------------------------------------
// S4: Expired waiver keeps the violation
// ---------------------------------------------------------------------------

#[test]
fn policy_check_expired_waiver_keeps_violation() {
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
    let waivers = vec![Waiver {
        rule: "forbid_dependency".to_string(),
        subject_id: "c4:container:a".to_string(),
        reason: "intentional shared kernel".to_string(),
        expires_at: now() - chrono::Duration::days(1),
        expired: false,
    }];

    let report = check_policy(&policy, &waivers, &store, "error", now()).unwrap();
    assert_eq!(report.summary.failed, 1);
    assert_eq!(report.summary.waived, 0);
    assert_eq!(report.waivers.len(), 1);
    assert!(report.waivers[0].expired);
}

// ---------------------------------------------------------------------------
// S5: Malformed policy is rejected before graph access
// ---------------------------------------------------------------------------

#[test]
fn policy_check_malformed_rules_rejected_by_deserialization() {
    let raw = r#"{ "rules": [ { "rule": "not_a_rule", "selector": "x", "severity": "error" } ] }"#;
    let doc: serde_json::Value = serde_json::from_str(raw).unwrap();
    let result: Result<Vec<PolicyRule>, _> =
        serde_json::from_value(doc.get("rules").cloned().unwrap());
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Confidence min integration
// ---------------------------------------------------------------------------

#[test]
fn policy_check_confidence_min_violation() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:low", 0.3);
    seed_element(&mut store, "c4:container:high", 0.95);

    let policy = vec![PolicyRule::ConfidenceMin {
        selector: "c4:*".to_string(),
        severity: Severity::Warning,
        params: PolicyParams::ConfidenceMin { min: 0.7 },
    }];

    let report = check_policy(&policy, &[], &store, "warning", now()).unwrap();
    assert_eq!(report.summary.failed, 1);
    assert_eq!(report.violations[0].subject.id, "c4:container:low");
}
