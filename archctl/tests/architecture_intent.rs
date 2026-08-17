//! Integration tests for `archctl architecture intent check`.
//!
//! These tests exercise the intent use case with a real (in-memory) store.
//! Graph data is seeded via `execute_raw_cypher_for_test`.

use archctl::architecture::intent::{DeclaredElement, IntentDeclaration, check_intent};
use archctl::architecture::load_intent;
use archctl::store::{GraphStore, LbugStore};
use chrono::Utc;
use std::io::Write;
use tempfile::NamedTempFile;

/// Helper: open an in-memory store for testing.
fn test_store() -> (LbugStore, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    let mut store = LbugStore::open(&project).unwrap();
    store.init().unwrap();
    (store, tmp)
}

/// Seed an element with the given id, kind, and category.
fn seed_element(store: &mut LbugStore, id: &str, kind_id: &str, category: &str) {
    store
        .execute_raw_cypher_for_test(&format!(
            "CREATE (:Element {{id: '{id}', kind_id: '{kind_id}', category: '{category}', canonical_key: '{id}', current_name: 'S', current_status: 'active', current_confidence: 1.0, current_version_id: ''}})"
        ))
        .expect("seed element");
}

fn fixed_now() -> chrono::DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc)
}

// ---------------------------------------------------------------------------
// S1: declared present → DeclaredAndPresent
// ---------------------------------------------------------------------------

#[test]
fn s1_declared_present() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:order", "c4:container", "c4");

    let intent = IntentDeclaration {
        schema_version: "1.0".to_string(),
        capability: "test".to_string(),
        elements: vec![DeclaredElement {
            id: "c4:container:order".to_string(),
            kind_id: "c4:container".to_string(),
            category: "c4".to_string(),
        }],
        relations: vec![],
    };

    let report = check_intent(&store, &intent, "test.toml", fixed_now()).unwrap();
    assert_eq!(report.deltas.declared_and_present.len(), 1);
    assert_eq!(report.deltas.declared_but_missing.len(), 0);
    assert_eq!(report.deltas.kind_mismatch.len(), 0);
    assert_eq!(report.deltas.observed_undeclared.len(), 0);
    assert_eq!(report.summary.drift, 0);
}

// ---------------------------------------------------------------------------
// S2: declared missing → DeclaredButMissing
// ---------------------------------------------------------------------------

#[test]
fn s2_declared_missing() {
    let (store, _tmp) = test_store();

    let intent = IntentDeclaration {
        schema_version: "1.0".to_string(),
        capability: "test".to_string(),
        elements: vec![DeclaredElement {
            id: "c4:container:ghost".to_string(),
            kind_id: "c4:container".to_string(),
            category: "c4".to_string(),
        }],
        relations: vec![],
    };

    let report = check_intent(&store, &intent, "test.toml", fixed_now()).unwrap();
    assert_eq!(report.deltas.declared_and_present.len(), 0);
    assert_eq!(report.deltas.declared_but_missing.len(), 1);
    assert_eq!(report.summary.drift, 1);
}

// ---------------------------------------------------------------------------
// S3: observed undeclared is informational (not drift)
// ---------------------------------------------------------------------------

#[test]
fn s3_observed_undeclared() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:extra", "c4:container", "c4");

    let intent = IntentDeclaration {
        schema_version: "1.0".to_string(),
        capability: "test".to_string(),
        elements: vec![],
        relations: vec![],
    };

    let report = check_intent(&store, &intent, "test.toml", fixed_now()).unwrap();
    assert_eq!(report.deltas.declared_and_present.len(), 0);
    assert_eq!(report.deltas.declared_but_missing.len(), 0);
    assert_eq!(report.deltas.observed_undeclared.len(), 1);
    assert_eq!(report.summary.drift, 0); // undeclared is NOT drift
}

// ---------------------------------------------------------------------------
// S4: kind mismatch → KindMismatch
// ---------------------------------------------------------------------------

#[test]
fn s4_kind_mismatch() {
    let (mut store, _tmp) = test_store();
    // Graph has kind "c4:component" but intent declares "c4:container"
    seed_element(&mut store, "c4:container:svc", "c4:component", "c4");

    let intent = IntentDeclaration {
        schema_version: "1.0".to_string(),
        capability: "test".to_string(),
        elements: vec![DeclaredElement {
            id: "c4:container:svc".to_string(),
            kind_id: "c4:container".to_string(),
            category: "c4".to_string(),
        }],
        relations: vec![],
    };

    let report = check_intent(&store, &intent, "test.toml", fixed_now()).unwrap();
    assert_eq!(report.deltas.declared_and_present.len(), 0);
    assert_eq!(report.deltas.declared_but_missing.len(), 0);
    assert_eq!(report.deltas.kind_mismatch.len(), 1);
    assert_eq!(report.deltas.kind_mismatch[0].expected_kind, "c4:container");
    assert_eq!(report.deltas.kind_mismatch[0].observed_kind, "c4:component");
    assert_eq!(report.summary.drift, 1);
}

// ---------------------------------------------------------------------------
// S5: relation endpoints present
// ---------------------------------------------------------------------------

#[test]
fn s5_relation_endpoints_present() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "a", "container", "c4");
    seed_element(&mut store, "b", "container", "c4");

    let intent = IntentDeclaration {
        schema_version: "1.0".to_string(),
        capability: "test".to_string(),
        elements: vec![
            DeclaredElement {
                id: "a".to_string(),
                kind_id: "container".to_string(),
                category: "c4".to_string(),
            },
            DeclaredElement {
                id: "b".to_string(),
                kind_id: "container".to_string(),
                category: "c4".to_string(),
            },
        ],
        relations: vec![archctl::architecture::intent::DeclaredRelation {
            predicate: "depends_on".to_string(),
            source_id: "a".to_string(),
            target_id: "b".to_string(),
        }],
    };

    let report = check_intent(&store, &intent, "test.toml", fixed_now()).unwrap();
    assert_eq!(report.deltas.declared_and_present.len(), 2);
    assert_eq!(report.deltas.declared_but_missing.len(), 0);
}

// ---------------------------------------------------------------------------
// S6: empty intent → all observed ObservedUndeclared
// ---------------------------------------------------------------------------

#[test]
fn s6_empty_intent() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", "c4:container", "c4");
    seed_element(&mut store, "c4:container:b", "c4:container", "c4");

    let intent = IntentDeclaration {
        schema_version: "1.0".to_string(),
        capability: "test".to_string(),
        elements: vec![],
        relations: vec![],
    };

    let report = check_intent(&store, &intent, "test.toml", fixed_now()).unwrap();
    assert_eq!(report.deltas.declared_and_present.len(), 0);
    assert_eq!(report.deltas.declared_but_missing.len(), 0);
    assert_eq!(report.deltas.observed_undeclared.len(), 2);
    assert_eq!(report.summary.drift, 0);
}

// ---------------------------------------------------------------------------
// S7: empty graph → all DeclaredButMissing
// ---------------------------------------------------------------------------

#[test]
fn s7_empty_graph() {
    let (store, _tmp) = test_store();

    let intent = IntentDeclaration {
        schema_version: "1.0".to_string(),
        capability: "test".to_string(),
        elements: vec![
            DeclaredElement {
                id: "c4:container:a".to_string(),
                kind_id: "c4:container".to_string(),
                category: "c4".to_string(),
            },
            DeclaredElement {
                id: "c4:container:b".to_string(),
                kind_id: "c4:container".to_string(),
                category: "c4".to_string(),
            },
        ],
        relations: vec![],
    };

    let report = check_intent(&store, &intent, "test.toml", fixed_now()).unwrap();
    assert_eq!(report.deltas.declared_and_present.len(), 0);
    assert_eq!(report.deltas.declared_but_missing.len(), 2);
    assert_eq!(report.summary.drift, 2);
}

// ---------------------------------------------------------------------------
// S8: invalid TOML → IntentError::InvalidIntent with path
// ---------------------------------------------------------------------------

#[test]
fn s8_invalid_toml_rejected() {
    let mut tmp = NamedTempFile::with_suffix(".toml").unwrap();
    writeln!(tmp, "this is not valid toml [[elements").unwrap();

    let result = load_intent(tmp.path());
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = err.to_string();
    assert!(err_str.contains("invalid intent") || err_str.contains("parse"));
}

// ---------------------------------------------------------------------------
// S11: determinism — two runs byte-equal JSON
// ---------------------------------------------------------------------------

#[test]
fn s11_determinism() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:a", "c4:container", "c4");

    let intent = IntentDeclaration {
        schema_version: "1.0".to_string(),
        capability: "test".to_string(),
        elements: vec![DeclaredElement {
            id: "c4:container:a".to_string(),
            kind_id: "c4:container".to_string(),
            category: "c4".to_string(),
        }],
        relations: vec![],
    };

    let report1 = check_intent(&store, &intent, "test.toml", fixed_now()).unwrap();
    let json1 = serde_json::to_string(&report1).unwrap();

    let report2 = check_intent(&store, &intent, "test.toml", fixed_now()).unwrap();
    let json2 = serde_json::to_string(&report2).unwrap();

    assert_eq!(json1, json2, "two runs must be byte-equal");
}

// ---------------------------------------------------------------------------
// Deltas sorted by id ASC
// ---------------------------------------------------------------------------

#[test]
fn deltas_sorted_by_id_asc() {
    let (mut store, _tmp) = test_store();
    seed_element(&mut store, "c4:container:c", "c4:container", "c4");
    seed_element(&mut store, "c4:container:a", "c4:container", "c4");
    seed_element(&mut store, "c4:container:b", "c4:container", "c4");

    let intent = IntentDeclaration {
        schema_version: "1.0".to_string(),
        capability: "test".to_string(),
        elements: vec![
            DeclaredElement {
                id: "c4:container:b".to_string(),
                kind_id: "c4:container".to_string(),
                category: "c4".to_string(),
            },
            DeclaredElement {
                id: "c4:container:a".to_string(),
                kind_id: "c4:container".to_string(),
                category: "c4".to_string(),
            },
            DeclaredElement {
                id: "c4:container:c".to_string(),
                kind_id: "c4:container".to_string(),
                category: "c4".to_string(),
            },
        ],
        relations: vec![],
    };

    let report = check_intent(&store, &intent, "test.toml", fixed_now()).unwrap();
    let ids: Vec<&str> = report
        .deltas
        .declared_and_present
        .iter()
        .map(|e| e.id.as_str())
        .collect();
    assert_eq!(
        ids,
        vec!["c4:container:a", "c4:container:b", "c4:container:c"]
    );
}
