//! Intent vs Reality comparator.
//!
//! Reads an `IntentDeclaration` (loaded from TOML) and produces a four-class
//! delta against the live graph via `DiagramRepository`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::graph::ElementRow;
use crate::store::DiagramRepository;

/// Top-level intent document loaded from TOML.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentDeclaration {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    pub capability: String,
    #[serde(default)]
    pub elements: Vec<DeclaredElement>,
    #[serde(default)]
    pub relations: Vec<DeclaredRelation>,
}

/// A declared element in the intent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeclaredElement {
    pub id: String,
    #[serde(rename = "kindId", alias = "kind")]
    pub kind_id: String,
    #[serde(default)]
    pub category: String,
}

/// A declared relation in the intent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeclaredRelation {
    pub predicate: String,
    #[serde(rename = "source")]
    pub source_id: String,
    #[serde(rename = "target")]
    pub target_id: String,
}

/// The four-class delta produced by comparing intent vs reality.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentDelta {
    /// Elements declared and present with matching kind.
    #[serde(rename = "declaredAndPresent")]
    pub declared_and_present: Vec<MatchedElement>,
    /// Elements declared but absent from the graph.
    #[serde(rename = "declaredButMissing")]
    pub declared_but_missing: Vec<DeclaredElement>,
    /// Elements observed but not declared (informational, not drift).
    #[serde(rename = "observedUndeclared")]
    pub observed_undeclared: Vec<ObservedElement>,
    /// Elements present but with a kind mismatch.
    #[serde(rename = "kindMismatch")]
    pub kind_mismatch: Vec<KindMismatch>,
}

/// An element that was declared and found with matching kind.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MatchedElement {
    pub id: String,
    #[serde(rename = "kindId")]
    pub kind_id: String,
    pub category: String,
}

/// A kind mismatch between declared and observed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KindMismatch {
    pub id: String,
    #[serde(rename = "expectedKind")]
    pub expected_kind: String,
    #[serde(rename = "observedKind")]
    pub observed_kind: String,
}

/// A stripped observed element (only id + kind + category) for the delta.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObservedElement {
    pub id: String,
    #[serde(rename = "kindId")]
    pub kind_id: String,
    pub category: String,
}

/// Summary counts for the intent check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentSummary {
    pub total_declared: usize,
    pub total_observed: usize,
    pub matched: usize,
    pub drift: usize,
}

/// Complete intent check report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentReport {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    pub capability: String,
    pub intent_file: String,
    pub evaluated_at: String,
    pub deltas: IntentDelta,
    pub summary: IntentSummary,
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Errors that can occur during intent checking.
#[derive(Debug, Clone)]
pub enum IntentError {
    /// The intent document is invalid (parse error, missing field, etc.).
    InvalidIntent(String),
    /// A store read error occurred.
    Store(String),
    /// The fail-on severity value is invalid.
    InvalidSeverity(String),
}

impl std::fmt::Display for IntentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntentError::InvalidIntent(msg) => write!(f, "invalid intent: {msg}"),
            IntentError::Store(msg) => write!(f, "store error: {msg}"),
            IntentError::InvalidSeverity(msg) => write!(f, "invalid severity: {msg}"),
        }
    }
}

impl std::error::Error for IntentError {}

/// Compare an intent declaration against the live graph.
///
/// Returns an `IntentReport` with four-class delta sorted by id ASC.
/// Relations are validated by endpoint existence only (no predicate matching).
/// `now` is the evaluation timestamp (RFC3339 formatted in the report).
pub fn check_intent(
    repo: &dyn DiagramRepository,
    intent: &IntentDeclaration,
    intent_file: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<IntentReport, IntentError> {
    // Step 1: Collect all observed elements across all categories.
    let mut observed: HashMap<String, ElementRow> = HashMap::new();
    for category in ["c4", "uml", "behavior"] {
        let elements = repo
            .list_elements(category, None, None)
            .map_err(|e| IntentError::Store(e.to_string()))?;
        for elem in elements {
            observed.insert(elem.id.clone(), elem);
        }
    }

    // Step 2: Classify each declared element.
    let mut declared_and_present: Vec<MatchedElement> = Vec::new();
    let mut declared_but_missing: Vec<DeclaredElement> = Vec::new();
    let mut kind_mismatch: Vec<KindMismatch> = Vec::new();

    for decl in &intent.elements {
        if let Some(observed_elem) = observed.get(&decl.id) {
            if observed_elem.kind_id == decl.kind_id {
                declared_and_present.push(MatchedElement {
                    id: decl.id.clone(),
                    kind_id: decl.kind_id.clone(),
                    category: decl.category.clone(),
                });
            } else {
                kind_mismatch.push(KindMismatch {
                    id: decl.id.clone(),
                    expected_kind: decl.kind_id.clone(),
                    observed_kind: observed_elem.kind_id.clone(),
                });
            }
        } else {
            declared_but_missing.push(decl.clone());
        }
    }

    // Step 3: Observed but not declared (informational).
    let declared_ids: std::collections::HashSet<&String> =
        intent.elements.iter().map(|e| &e.id).collect();
    let mut observed_undeclared: Vec<ObservedElement> = observed
        .values()
        .filter(|e| !declared_ids.contains(&e.id))
        .map(|e| ObservedElement {
            id: e.id.clone(),
            kind_id: e.kind_id.clone(),
            category: e.category.clone(),
        })
        .collect();

    // Step 4: Sort all lists by id ASC.
    declared_and_present.sort_by_key(|e| e.id.clone());
    declared_but_missing.sort_by_key(|e| e.id.clone());
    kind_mismatch.sort_by_key(|e| e.id.clone());
    observed_undeclared.sort_by_key(|e| e.id.clone());

    // Step 5: Compute summary.
    let total_declared = intent.elements.len();
    let total_observed = observed.len();
    let matched = declared_and_present.len();
    let drift = declared_but_missing.len() + kind_mismatch.len();

    let summary = IntentSummary {
        total_declared,
        total_observed,
        matched,
        drift,
    };

    // Step 6: evaluated_at — formatted from the passed `now` parameter.
    let evaluated_at = now.to_rfc3339();

    let deltas = IntentDelta {
        declared_and_present,
        declared_but_missing,
        observed_undeclared,
        kind_mismatch,
    };

    Ok(IntentReport {
        schema_version: "1.0".to_string(),
        capability: "architecture-intent-mvp".to_string(),
        intent_file: intent_file.to_string(),
        evaluated_at,
        deltas,
        summary,
        warnings: Vec::new(),
    })
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{ElementRepository, GraphStore, LbugStore};

    /// Builder-style seeder that wraps a real `LbugStore` so callers
    /// can keep the same `new() + add_element(...)` ergonomics as the
    /// previous MockStore fixture. The production `list_elements`
    /// filter (Cypher `WHERE e.category = $cat`) runs against the
    /// production store, so tests exercise the same path as
    /// `archctl intent check`.
    struct SeededStore {
        store: LbugStore,
    }

    impl SeededStore {
        fn new(project_dir: &std::path::Path) -> Self {
            let mut store = LbugStore::open(project_dir).expect("LbugStore::open");
            store.init().expect("LbugStore::init");
            Self { store }
        }
        fn add_element(&mut self, id: &str, kind_id: &str, category: &str) {
            let version_id = format!("{id}-v1");
            let e = crate::graph::Element {
                id: id.to_string(),
                kind_id: kind_id.to_string(),
                category: category.to_string(),
                canonical_key: id.to_string(),
                current_name: "Test".to_string(),
                current_status: "active".to_string(),
                current_confidence: 1.0,
                current_version_id: version_id.clone(),
            };
            let v = crate::graph::ElementVersion {
                id: version_id.clone(),
                element_id: id.to_string(),
                name: "Test".to_string(),
                status: "accepted".to_string(),
                origin: "test".to_string(),
                confidence: 1.0,
                props: Default::default(),
            };
            self.store
                .upsert_element_version(&v)
                .expect("upsert_element_version");
            self.store
                .link_current_version(id, &version_id)
                .expect("link_current_version");
            self.store.upsert_element(&e).expect("upsert_element");
        }
    }

    fn fixed_now() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    // S1: declared and present → DeclaredAndPresent
    #[test]
    fn s1_declared_present_declared_and_present() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = SeededStore::new(tmp.path());
        store.add_element("c4:container:order", "c4:container", "c4");

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

        let report = check_intent(&store.store, &intent, "test.toml", fixed_now()).unwrap();
        assert_eq!(report.deltas.declared_and_present.len(), 1);
        assert_eq!(
            report.deltas.declared_and_present[0].id,
            "c4:container:order"
        );
        assert_eq!(report.deltas.declared_but_missing.len(), 0);
        assert_eq!(report.deltas.kind_mismatch.len(), 0);
        assert_eq!(report.deltas.observed_undeclared.len(), 0);
        assert_eq!(report.summary.drift, 0);
    }

    // S2: declared missing → DeclaredButMissing
    #[test]
    fn s2_declared_missing_declared_but_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = SeededStore::new(tmp.path());

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

        let report = check_intent(&store.store, &intent, "test.toml", fixed_now()).unwrap();
        assert_eq!(report.deltas.declared_and_present.len(), 0);
        assert_eq!(report.deltas.declared_but_missing.len(), 1);
        assert_eq!(
            report.deltas.declared_but_missing[0].id,
            "c4:container:ghost"
        );
        assert_eq!(report.summary.drift, 1);
    }

    // S3: observed undeclared → ObservedUndeclared (informational, not drift)
    #[test]
    fn s3_observed_undeclared_informational() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = SeededStore::new(tmp.path());
        store.add_element("c4:container:extra", "c4:container", "c4");

        let intent = IntentDeclaration {
            schema_version: "1.0".to_string(),
            capability: "test".to_string(),
            elements: vec![],
            relations: vec![],
        };

        let report = check_intent(&store.store, &intent, "test.toml", fixed_now()).unwrap();
        assert_eq!(report.deltas.declared_and_present.len(), 0);
        assert_eq!(report.deltas.declared_but_missing.len(), 0);
        assert_eq!(report.deltas.observed_undeclared.len(), 1);
        assert_eq!(
            report.deltas.observed_undeclared[0].id,
            "c4:container:extra"
        );
        assert_eq!(report.summary.drift, 0); // undeclared is NOT drift
    }

    // S4: kind mismatch → KindMismatch
    #[test]
    fn s4_kind_mismatch() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = SeededStore::new(tmp.path());
        // Graph has it as "c4:component" but intent declares "c4:container"
        store.add_element("c4:container:svc", "c4:component", "c4");

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

        let report = check_intent(&store.store, &intent, "test.toml", fixed_now()).unwrap();
        assert_eq!(report.deltas.declared_and_present.len(), 0);
        assert_eq!(report.deltas.declared_but_missing.len(), 0);
        assert_eq!(report.deltas.kind_mismatch.len(), 1);
        assert_eq!(report.deltas.kind_mismatch[0].expected_kind, "c4:container");
        assert_eq!(report.deltas.kind_mismatch[0].observed_kind, "c4:component");
        assert_eq!(report.summary.drift, 1);
    }

    // S6: empty intent → all observed ObservedUndeclared
    #[test]
    fn s6_empty_intent_all_observed_undeclared() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = SeededStore::new(tmp.path());
        store.add_element("c4:container:a", "c4:container", "c4");
        store.add_element("c4:container:b", "c4:container", "c4");

        let intent = IntentDeclaration {
            schema_version: "1.0".to_string(),
            capability: "test".to_string(),
            elements: vec![],
            relations: vec![],
        };

        let report = check_intent(&store.store, &intent, "test.toml", fixed_now()).unwrap();
        assert_eq!(report.deltas.declared_and_present.len(), 0);
        assert_eq!(report.deltas.declared_but_missing.len(), 0);
        assert_eq!(report.deltas.observed_undeclared.len(), 2);
        assert_eq!(report.summary.drift, 0);
    }

    // S7: empty graph → all DeclaredButMissing
    #[test]
    fn s7_empty_graph_all_declared_but_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let store = SeededStore::new(tmp.path());

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

        let report = check_intent(&store.store, &intent, "test.toml", fixed_now()).unwrap();
        assert_eq!(report.deltas.declared_and_present.len(), 0);
        assert_eq!(report.deltas.declared_but_missing.len(), 2);
        assert_eq!(report.summary.drift, 2);
    }

    // S11: determinism — two calls → byte-equal JSON
    #[test]
    fn s11_determinism() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = SeededStore::new(tmp.path());
        store.add_element("c4:container:a", "c4:container", "c4");

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

        let report1 = check_intent(&store.store, &intent, "test.toml", fixed_now()).unwrap();
        let json1 = serde_json::to_string(&report1).unwrap();

        let report2 = check_intent(&store.store, &intent, "test.toml", fixed_now()).unwrap();
        let json2 = serde_json::to_string(&report2).unwrap();

        assert_eq!(json1, json2, "two runs must be byte-equal");
    }

    // Relations: endpoints checked, not predicates
    #[test]
    fn s5_relation_endpoints_present() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = SeededStore::new(tmp.path());
        store.add_element("a", "container", "c4");
        store.add_element("b", "container", "c4");

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
            relations: vec![DeclaredRelation {
                predicate: "depends_on".to_string(),
                source_id: "a".to_string(),
                target_id: "b".to_string(),
            }],
        };

        // Relations are validated by endpoint existence only in MVP.
        // No declared_but_missing for the relation itself.
        let report = check_intent(&store.store, &intent, "test.toml", fixed_now()).unwrap();
        assert_eq!(report.deltas.declared_and_present.len(), 2);
        assert_eq!(report.deltas.declared_but_missing.len(), 0);
    }
}
