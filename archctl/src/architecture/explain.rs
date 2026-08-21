//! Architecture explain/provenance use case.
//!
//! Read-only lookup that returns the evidence chain backing any graph subject
//! (Element or SemanticRelation) via SUPPORTED_BY edges.
//!
//! ## Public surface
//!
//! - `explain` — router that dispatches to element or relation path based on id shape
//! - `ExplainReport` — the JSON-serializable output carrier
//! - `ExplainError` — domain errors

use serde::{Deserialize, Serialize};

use crate::architecture::fusion::{FusedClaim, fused_claims_from_rows};
use crate::diagram::export_types::EvidenceEntry;
use crate::graph::RelationRow;
use crate::store::DiagramRepository;

/// The explain-report/1 carrier — the output of the `explain` use case.
///
/// NOTE: intentionally does not derive `Eq` — the report embeds
/// `FusedClaim` (which carries an `f64` confidence).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplainReport {
    /// Schema version of this report format.
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    /// Capability that produced this report.
    pub capability: String,
    /// The subject that was explained.
    pub subject: ExplainSubject,
    /// Provenance chain: evidence entries and substantiation status.
    pub provenance: ExplainProvenance,
    /// Fused claims backing this subject (v6 persisted claims whose
    /// derived evidence intersects the subject's evidence). Absent
    /// when the subject has no version link, when no fused claims are
    /// persisted, or when none intersect.
    #[serde(rename = "fusedClaims")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fused_claims: Option<Vec<FusedClaim>>,
    /// Warnings generated during the explain (e.g., missing version link).
    pub warnings: Vec<String>,
}

/// The explained subject — either an Element or a Relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplainSubject {
    /// Kind of subject: "element" or "relation".
    pub kind: String,
    /// Canonical subject id.
    pub id: String,
    /// The current version id (null if not set).
    #[serde(rename = "versionId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    /// Human-readable statement derived from the subject's label/name.
    pub statement: String,
}

/// Provenance block: evidence list and substantiation flag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplainProvenance {
    /// List of evidence entries backing this subject.
    pub evidence: Vec<EvidenceEntry>,
    /// True when no evidence backs this subject (unsubstantiated).
    #[serde(rename = "unsubstantiated")]
    pub unsubstantiated: bool,
}

/// Errors specific to explain operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplainError {
    /// The element id does not exist in the graph.
    SubjectNotFound(String),
    /// The relation id does not exist in the graph.
    RelationNotFound(String),
    /// The store returned an error.
    Store(String),
}

impl std::fmt::Display for ExplainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExplainError::SubjectNotFound(id) => {
                write!(
                    f,
                    "element not found: {} — run `archctl architecture list` to see available elements",
                    id
                )
            }
            ExplainError::RelationNotFound(id) => {
                write!(
                    f,
                    "relation not found: {} — run `archctl architecture list` to see available relations",
                    id
                )
            }
            ExplainError::Store(msg) => {
                write!(f, "store error: {msg}")
            }
        }
    }
}

impl std::error::Error for ExplainError {}

/// Routing prefix for relation ids.
const REL_PREFIX: &str = "rel:";

impl From<anyhow::Error> for ExplainError {
    fn from(e: anyhow::Error) -> Self {
        ExplainError::Store(e.to_string())
    }
}

/// Explain a graph subject by its canonical id.
///
/// Routes to the element path if `id` starts with `c4:`, `uml`, or `behavior:`.
/// Routes to the relation path if `id` starts with `rel:`.
///
/// Returns `ExplainReport` with provenance evidence or an error.
pub fn explain(repo: &dyn DiagramRepository, id: &str) -> Result<ExplainReport, ExplainError> {
    if id.starts_with(REL_PREFIX) {
        explain_relation(repo, id)
    } else {
        explain_element(repo, id)
    }
}

/// Explain an element subject.
fn explain_element(repo: &dyn DiagramRepository, id: &str) -> Result<ExplainReport, ExplainError> {
    // Determine category from id prefix
    let category = if id.starts_with("c4:") {
        "c4"
    } else if id.starts_with("cg:") || id.starts_with("cd:") {
        // Code elements (call-graph / class-diagram). UAT smoke
        // 2026-08-19: explain failed for code elements because the
        // fallback list omitted "code".
        "code"
    } else if id.starts_with("uml") {
        "uml"
    } else if id.starts_with("behavior:") {
        "behavior"
    } else {
        // Try all categories
        "c4"
    };

    // List elements and filter by id
    let elements = repo
        .list_elements(category, None, None)
        .map_err(ExplainError::from)?;

    let element_row = elements.into_iter().find(|e| e.id == id);

    let element_row = match element_row {
        Some(e) => e,
        None => {
            // Try other categories
            for cat in &["uml", "behavior", "code"] {
                let elements = repo
                    .list_elements(cat, None, None)
                    .map_err(ExplainError::from)?;
                if let Some(e) = elements.into_iter().find(|e| e.id == id) {
                    return build_element_report(repo, e);
                }
            }
            return Err(ExplainError::SubjectNotFound(id.to_string()));
        }
    };

    build_element_report(repo, element_row)
}

/// Build an ExplainReport for an element row.
fn build_element_report(
    repo: &dyn DiagramRepository,
    element: crate::graph::ElementRow,
) -> Result<ExplainReport, ExplainError> {
    let version_id = if element.current_version_id.is_empty() {
        None
    } else {
        Some(element.current_version_id.clone())
    };

    let evidence = version_id.as_ref().map_or_else(
        || Ok(vec![]),
        #[allow(clippy::cloned_ref_to_slice_refs)]
        |vid| {
            repo.list_evidence_for_versions(&[vid.clone()])
                .map_err(ExplainError::from)
        },
    )?;

    let unsubstantiated = evidence.is_empty();
    let warnings = if unsubstantiated {
        if version_id.is_none() {
            vec!["element has no current version link".to_string()]
        } else {
            vec!["element version has no supporting evidence".to_string()]
        }
    } else {
        vec![]
    };

    let fused_claims = fused_claims_for_subject(repo, version_id.as_deref(), &evidence)?;

    Ok(ExplainReport {
        schema_version: "1.1".to_string(),
        capability: "architecture-explain-mvp".to_string(),
        subject: ExplainSubject {
            kind: "element".to_string(),
            id: element.id,
            version_id,
            statement: element.current_name,
        },
        provenance: ExplainProvenance {
            evidence,
            unsubstantiated,
        },
        fused_claims,
        warnings,
    })
}

/// Explain a relation subject.
fn explain_relation(repo: &dyn DiagramRepository, id: &str) -> Result<ExplainReport, ExplainError> {
    let rel_row = repo.read_relation_by_id(id).map_err(ExplainError::from)?;

    let rel_row = match rel_row {
        Some(r) => r,
        None => return Err(ExplainError::RelationNotFound(id.to_string())),
    };

    build_relation_report(repo, id, &rel_row)
}

/// Build an ExplainReport for a relation row.
fn build_relation_report(
    repo: &dyn DiagramRepository,
    id: &str,
    rel: &RelationRow,
) -> Result<ExplainReport, ExplainError> {
    let version_id = if rel.current_version_id.is_empty() {
        None
    } else {
        Some(rel.current_version_id.clone())
    };

    #[allow(clippy::cloned_ref_to_slice_refs)]
    #[allow(clippy::cloned_ref_to_slice_refs)]
    let evidence = version_id.as_ref().map_or_else(
        || Ok(vec![]),
        |vid| {
            repo.list_evidence_for_relation_versions(&[vid.clone()])
                .map_err(ExplainError::from)
        },
    )?;

    let unsubstantiated = evidence.is_empty();
    let warnings = if unsubstantiated {
        if version_id.is_none() {
            vec!["relation has no current version link".to_string()]
        } else {
            vec!["relation version has no supporting evidence".to_string()]
        }
    } else {
        vec![]
    };

    let fused_claims = fused_claims_for_subject(repo, version_id.as_deref(), &evidence)?;

    Ok(ExplainReport {
        schema_version: "1.1".to_string(),
        capability: "architecture-explain-mvp".to_string(),
        subject: ExplainSubject {
            kind: "relation".to_string(),
            id: id.to_string(),
            version_id,
            statement: rel.current_label.clone(),
        },
        provenance: ExplainProvenance {
            evidence,
            unsubstantiated,
        },
        fused_claims,
        warnings,
    })
}

/// Surface persisted fused claims (v6) that back the given subject
/// evidence.
///
/// Returns `None` when the subject has no version link, when the
/// store predates the v6 tables, or when no fused claim's
/// `derived_from` intersects the subject's evidence ids.
fn fused_claims_for_subject(
    repo: &dyn DiagramRepository,
    version_id: Option<&str>,
    evidence: &[EvidenceEntry],
) -> Result<Option<Vec<FusedClaim>>, ExplainError> {
    let Some(vid) = version_id else {
        return Ok(None);
    };
    if evidence.is_empty() {
        return Ok(None);
    }
    let Some(rows) = repo
        .read_fused_claim_rows(std::slice::from_ref(&vid.to_string()))
        .map_err(ExplainError::from)?
    else {
        // Pre-v6 store: no persisted fused claims.
        return Ok(None);
    };
    if rows.is_empty() {
        return Ok(None);
    }
    let claim_ids: Vec<String> = rows
        .iter()
        .filter_map(|r| r.get("f.id").and_then(|c| c.as_str()).map(String::from))
        .collect();
    let edges = repo
        .list_fused_conflict_edges(&claim_ids)
        .map_err(ExplainError::from)?;
    let all = fused_claims_from_rows(&rows, &edges);
    let evidence_ids: std::collections::HashSet<String> =
        evidence.iter().map(|e| e.id.clone()).collect();
    let mut backing: Vec<FusedClaim> = all
        .into_iter()
        .filter(|c| c.derived_from.iter().any(|eid| evidence_ids.contains(eid)))
        .collect();
    if backing.is_empty() {
        return Ok(None);
    }
    backing.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(Some(backing))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::export_types::EvidenceEntry;
    use crate::store::{ElementRepository, EvidenceRepository, GraphStore, LbugStore};

    /// Builder-style seeder that persists the test fixture into a
    /// real `LbugStore` opened in a TempDir. Mirrors the previous
    /// FakeRepo builder ergonomics. Element relations use raw Cypher
    /// because there is no high-level writer for the
    /// `(:SemanticRelation)` table (see ADR-022 / store.rs:5354).
    struct SeededStore {
        project_dir: std::path::PathBuf,
        elements: Vec<(String, String, String)>, // id, version_id, name
        element_evidence: Vec<(String, EvidenceEntry)>,
        relations: Vec<(String, String, String)>, // id, version_id, label
        relation_evidence: Vec<(String, EvidenceEntry)>,
    }

    impl SeededStore {
        fn new(project_dir: &std::path::Path) -> Self {
            Self {
                project_dir: project_dir.to_path_buf(),
                elements: vec![],
                element_evidence: vec![],
                relations: vec![],
                relation_evidence: vec![],
            }
        }
        fn with_element(mut self, id: &str, version_id: &str, name: &str) -> Self {
            self.elements
                .push((id.to_string(), version_id.to_string(), name.to_string()));
            self
        }
        fn with_element_evidence(mut self, version_id: &str, evidence: EvidenceEntry) -> Self {
            self.element_evidence
                .push((version_id.to_string(), evidence));
            self
        }
        fn with_relation(mut self, id: &str, version_id: &str, label: &str) -> Self {
            self.relations
                .push((id.to_string(), version_id.to_string(), label.to_string()));
            self
        }
        fn with_relation_evidence(mut self, version_id: &str, evidence: EvidenceEntry) -> Self {
            self.relation_evidence
                .push((version_id.to_string(), evidence));
            self
        }
        fn build(self) -> LbugStore {
            let mut store = LbugStore::open(&self.project_dir).expect("LbugStore::open");
            store.init().expect("LbugStore::init");

            for (id, version_id, name) in &self.elements {
                let category = if id.starts_with("c4:") {
                    "c4"
                } else if id.starts_with("uml") {
                    "uml"
                } else if id.starts_with("behavior:") {
                    "behavior"
                } else {
                    "c4"
                };
                let v = crate::graph::ElementVersion {
                    id: version_id.clone(),
                    element_id: id.clone(),
                    name: name.clone(),
                    status: "accepted".to_string(),
                    origin: "test".to_string(),
                    confidence: 0.9,
                    props: Default::default(),
                };
                store
                    .upsert_element_version(&v)
                    .expect("upsert_element_version");
                store
                    .link_current_version(id, version_id)
                    .expect("link_current_version");
                let e = crate::graph::Element {
                    id: id.clone(),
                    kind_id: "container".to_string(),
                    category: category.to_string(),
                    canonical_key: id.clone(),
                    current_name: name.clone(),
                    current_status: "active".to_string(),
                    current_confidence: 0.9,
                    current_version_id: version_id.clone(),
                };
                store.upsert_element(&e).expect("upsert_element");
            }

            for (version_id, evidence) in &self.element_evidence {
                let mut props = serde_json::Map::new();
                if let Some(s) = &evidence.status {
                    props.insert("status".into(), serde_json::Value::String(s.clone()));
                }
                let ev = crate::graph::StructuralEvidence {
                    id: evidence.id.clone(),
                    kind: evidence.kind.clone(),
                    claim: evidence.claim.clone(),
                    file: evidence.path.clone(),
                    line: evidence.start_line,
                    confidence: 0.9,
                    rule_id: evidence.rule_id.clone(),
                    props,
                };
                store
                    .put_structural_evidence(&ev)
                    .expect("put_structural_evidence");
                store
                    .link_supported_by(version_id, &evidence.id)
                    .expect("link_supported_by");
            }

            for (id, version_id, label) in &self.relations {
                // No high-level writer for SemanticRelation — raw Cypher.
                let cypher = format!(
                    "CREATE (:SemanticRelation {{id: '{}', current_version_id: '{}', current_label: '{}'}});",
                    id, version_id, label
                );
                store
                    .execute_raw_cypher_for_test(&cypher)
                    .expect("create SemanticRelation");
                // And a RelationVersion node for SUPPORTED_BY traversal.
                let cypher = format!(
                    "CREATE (:RelationVersion {{id: '{}', relation_id: '{}'}});",
                    version_id, id
                );
                store
                    .execute_raw_cypher_for_test(&cypher)
                    .expect("create RelationVersion");
            }

            for (version_id, evidence) in &self.relation_evidence {
                let mut props = serde_json::Map::new();
                if let Some(s) = &evidence.status {
                    props.insert("status".into(), serde_json::Value::String(s.clone()));
                }
                let ev = crate::graph::StructuralEvidence {
                    id: evidence.id.clone(),
                    kind: evidence.kind.clone(),
                    claim: evidence.claim.clone(),
                    file: evidence.path.clone(),
                    line: evidence.start_line,
                    confidence: 0.9,
                    rule_id: evidence.rule_id.clone(),
                    props,
                };
                store
                    .put_structural_evidence(&ev)
                    .expect("put_structural_evidence");
                // SUPPORTED_BY from RelationVersion → Evidence
                let cypher = format!(
                    "MATCH (rv:RelationVersion {{id: '{}'}}), (e:Evidence {{id: '{}'}}) \
                     CREATE (rv)-[:SUPPORTED_BY]->(e);",
                    version_id, evidence.id
                );
                store
                    .execute_raw_cypher_for_test(&cypher)
                    .expect("link relation_supported_by");
            }

            store
        }
    }

    fn make_evidence(id: &str) -> EvidenceEntry {
        EvidenceEntry {
            id: id.to_string(),
            kind: "structural".to_string(),
            claim: "test claim".to_string(),
            path: "src/lib.rs".to_string(),
            start_line: 10,
            end_line: 15,
            tool_name: "ast-grep".to_string(),
            tool_version: "0.1".to_string(),
            rule_id: "test:rule".to_string(),
            content_hash: "sha256:abc".to_string(),
            observed_at: "2026-08-01T00:00:00Z".to_string(),
            status: Some("accepted".to_string()),
        }
    }

    // -------------------------------------------------------------------------
    // Element path tests
    // -------------------------------------------------------------------------

    #[test]
    fn explain_element_with_evidence_returns_evidence_list() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path())
            .with_element("c4:container:orders", "v:1", "OrderService")
            .with_element_evidence("v:1", make_evidence("ev:1"))
            .build();

        let result = explain(&repo, "c4:container:orders").unwrap();
        assert_eq!(result.subject.kind, "element");
        assert_eq!(result.subject.id, "c4:container:orders");
        assert_eq!(result.provenance.evidence.len(), 1);
        assert!(!result.provenance.unsubstantiated);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn explain_element_without_evidence_returns_unsubstantiated() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path())
            .with_element("c4:container:orders", "v:1", "OrderService")
            .build();
        // No evidence added

        let result = explain(&repo, "c4:container:orders").unwrap();
        assert_eq!(result.subject.kind, "element");
        assert!(result.provenance.unsubstantiated);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn explain_element_unknown_id_returns_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path()).build();
        let result = explain(&repo, "c4:container:unknown");
        assert!(matches!(result, Err(ExplainError::SubjectNotFound(_))));
    }

    #[test]
    fn explain_element_no_version_id_returns_unsubstantiated_with_warning() {
        // Defensive path: `build_element_report` treats an empty
        // `current_version_id` as "no version". Production's
        // `validate_identifier` rejects empty ids so this scenario is
        // unreachable through the normal write ports; raw Cypher is
        // used to seed the unreachable state and exercise the guard.
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = LbugStore::open(tmp.path()).expect("LbugStore::open");
        store.init().expect("LbugStore::init");
        store
            .execute_raw_cypher_for_test(
                "CREATE (:Element {id: 'c4:container:orders', kind_id: 'container', \
                 category: 'c4', canonical_key: 'c4:container:orders', \
                 current_name: 'OrderService', current_status: 'active', \
                 current_confidence: 0.9, current_version_id: ''});",
            )
            .expect("seed element with empty version");

        let result = explain(&store, "c4:container:orders").unwrap();
        assert!(result.provenance.unsubstantiated);
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("no current version"))
        );
    }

    // -------------------------------------------------------------------------
    // Relation path tests
    // -------------------------------------------------------------------------

    #[test]
    fn explain_relation_with_evidence_returns_evidence_list() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path())
            .with_relation("rel:orders-payment", "rv:1", "calls")
            .with_relation_evidence("rv:1", make_evidence("ev:rel:1"))
            .build();

        let result = explain(&repo, "rel:orders-payment").unwrap();
        assert_eq!(result.subject.kind, "relation");
        assert_eq!(result.subject.id, "rel:orders-payment");
        assert_eq!(result.provenance.evidence.len(), 1);
        assert!(!result.provenance.unsubstantiated);
    }

    #[test]
    fn explain_relation_unknown_id_returns_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path()).build();
        let result = explain(&repo, "rel:nonexistent");
        assert!(matches!(result, Err(ExplainError::RelationNotFound(_))));
    }

    #[test]
    fn explain_relation_without_evidence_returns_unsubstantiated() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path())
            .with_relation("rel:orders-payment", "rv:1", "calls")
            .build();
        // No evidence added

        let result = explain(&repo, "rel:orders-payment").unwrap();
        assert_eq!(result.subject.kind, "relation");
        assert!(result.provenance.unsubstantiated);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn explain_relation_no_version_id_returns_unsubstantiated_with_warning() {
        // Defensive path: production's validate_identifier rejects
        // empty version ids, so this scenario is unreachable through
        // the normal write ports. Raw Cypher seeds the unreachable
        // state to exercise the guard.
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = LbugStore::open(tmp.path()).expect("LbugStore::open");
        store.init().expect("LbugStore::init");
        store
            .execute_raw_cypher_for_test(
                "CREATE (:SemanticRelation {id: 'rel:orders-payment', \
                 current_version_id: '', current_label: 'calls'});",
            )
            .expect("seed relation with empty version");

        let result = explain(&store, "rel:orders-payment").unwrap();
        assert!(result.provenance.unsubstantiated);
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("no current version"))
        );
    }

    // -------------------------------------------------------------------------
    // Schema and capability
    // -------------------------------------------------------------------------

    #[test]
    fn explain_report_has_correct_schema_version() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path())
            .with_element("c4:container:orders", "v:1", "OrderService")
            .with_element_evidence("v:1", make_evidence("ev:1"))
            .build();

        let result = explain(&repo, "c4:container:orders").unwrap();
        assert_eq!(result.schema_version, "1.1");
        assert_eq!(result.capability, "architecture-explain-mvp");
    }

    // -------------------------------------------------------------------------
    // Routing tests
    // -------------------------------------------------------------------------

    #[test]
    fn explain_routes_uml_id_to_element_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path())
            .with_element("uml:class:OrderService", "v:2", "OrderService")
            .with_element_evidence("v:2", make_evidence("ev:2"))
            .build();

        let result = explain(&repo, "uml:class:OrderService").unwrap();
        assert_eq!(result.subject.kind, "element");
    }

    #[test]
    fn explain_routes_behavior_id_to_element_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path())
            .with_element("behavior:user:login", "v:3", "login")
            .with_element_evidence("v:3", make_evidence("ev:3"))
            .build();

        let result = explain(&repo, "behavior:user:login").unwrap();
        assert_eq!(result.subject.kind, "element");
    }
}
