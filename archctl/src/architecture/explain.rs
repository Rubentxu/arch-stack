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
    use crate::graph::{ElementRow, RelationRow};
    use crate::store::DiagramRepository;

    /// A minimal DiagramRepository stub for unit tests.
    struct FakeRepo {
        elements: Vec<ElementRow>,
        relations: Vec<RelationRow>,
        element_evidence: Vec<(String, Vec<EvidenceEntry>)>, // (version_id, evidence)
        relation_evidence: Vec<(String, Vec<EvidenceEntry>)>, // (version_id, evidence)
    }

    impl FakeRepo {
        fn new() -> Self {
            Self {
                elements: vec![],
                relations: vec![],
                element_evidence: vec![],
                relation_evidence: vec![],
            }
        }
        fn with_element(mut self, id: &str, version_id: &str, name: &str) -> Self {
            let category = if id.starts_with("c4:") {
                "c4".to_string()
            } else if id.starts_with("uml") {
                "uml".to_string()
            } else if id.starts_with("behavior:") {
                "behavior".to_string()
            } else {
                "c4".to_string()
            };
            self.elements.push(ElementRow {
                id: id.to_string(),
                kind_id: "container".to_string(),
                category,
                canonical_key: id.to_string(),
                current_name: name.to_string(),
                current_status: "active".to_string(),
                current_confidence: 0.9,
                current_version_id: version_id.to_string(),
            });
            self
        }
        fn with_element_evidence(mut self, version_id: &str, evidence: EvidenceEntry) -> Self {
            self.element_evidence
                .push((version_id.to_string(), vec![evidence]));
            self
        }
        fn with_relation(mut self, id: &str, version_id: &str, label: &str) -> Self {
            self.relations.push(RelationRow {
                id: id.to_string(),
                current_version_id: version_id.to_string(),
                current_label: label.to_string(),
            });
            self
        }
        fn with_relation_evidence(mut self, version_id: &str, evidence: EvidenceEntry) -> Self {
            self.relation_evidence
                .push((version_id.to_string(), vec![evidence]));
            self
        }
    }

    impl DiagramRepository for FakeRepo {
        fn list_elements(
            &self,
            category: &str,
            _scope: Option<&str>,
            _kind: Option<&str>,
        ) -> anyhow::Result<Vec<ElementRow>> {
            Ok(self
                .elements
                .iter()
                .filter(|e| e.category == category)
                .cloned()
                .collect())
        }

        fn list_semantic_edges(
            &self,
            _category: &str,
        ) -> anyhow::Result<Vec<crate::graph::SemanticEdgeRow>> {
            Ok(vec![])
        }

        fn list_evidence_for_versions(
            &self,
            version_ids: &[String],
        ) -> anyhow::Result<Vec<EvidenceEntry>> {
            Ok(version_ids
                .iter()
                .filter_map(|vid| {
                    self.element_evidence
                        .iter()
                        .find(|(id, _)| id == vid)
                        .map(|(_, ev)| ev.clone())
                })
                .flatten()
                .collect())
        }

        fn list_version_props(
            &self,
            _version_ids: &[String],
        ) -> anyhow::Result<Vec<crate::graph::VersionPropsRow>> {
            Ok(vec![])
        }

        fn read_relation_by_id(&self, id: &str) -> anyhow::Result<Option<RelationRow>> {
            Ok(self.relations.iter().find(|r| r.id == id).cloned())
        }

        fn list_evidence_for_relation_versions(
            &self,
            version_ids: &[String],
        ) -> anyhow::Result<Vec<EvidenceEntry>> {
            Ok(version_ids
                .iter()
                .filter_map(|vid| {
                    self.relation_evidence
                        .iter()
                        .find(|(id, _)| id == vid)
                        .map(|(_, ev)| ev.clone())
                })
                .flatten()
                .collect())
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
        let repo = FakeRepo::new()
            .with_element("c4:container:orders", "v:1", "OrderService")
            .with_element_evidence("v:1", make_evidence("ev:1"));

        let result = explain(&repo, "c4:container:orders").unwrap();
        assert_eq!(result.subject.kind, "element");
        assert_eq!(result.subject.id, "c4:container:orders");
        assert_eq!(result.provenance.evidence.len(), 1);
        assert!(!result.provenance.unsubstantiated);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn explain_element_without_evidence_returns_unsubstantiated() {
        let repo = FakeRepo::new().with_element("c4:container:orders", "v:1", "OrderService");
        // No evidence added

        let result = explain(&repo, "c4:container:orders").unwrap();
        assert_eq!(result.subject.kind, "element");
        assert!(result.provenance.unsubstantiated);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn explain_element_unknown_id_returns_error() {
        let repo = FakeRepo::new();
        let result = explain(&repo, "c4:container:unknown");
        assert!(matches!(result, Err(ExplainError::SubjectNotFound(_))));
    }

    #[test]
    fn explain_element_no_version_id_returns_unsubstantiated_with_warning() {
        let repo = FakeRepo::new().with_element("c4:container:orders", "", "OrderService");

        let result = explain(&repo, "c4:container:orders").unwrap();
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
        let repo = FakeRepo::new()
            .with_relation("rel:orders-payment", "rv:1", "calls")
            .with_relation_evidence("rv:1", make_evidence("ev:rel:1"));

        let result = explain(&repo, "rel:orders-payment").unwrap();
        assert_eq!(result.subject.kind, "relation");
        assert_eq!(result.subject.id, "rel:orders-payment");
        assert_eq!(result.provenance.evidence.len(), 1);
        assert!(!result.provenance.unsubstantiated);
    }

    #[test]
    fn explain_relation_unknown_id_returns_error() {
        let repo = FakeRepo::new();
        let result = explain(&repo, "rel:nonexistent");
        assert!(matches!(result, Err(ExplainError::RelationNotFound(_))));
    }

    #[test]
    fn explain_relation_without_evidence_returns_unsubstantiated() {
        let repo = FakeRepo::new().with_relation("rel:orders-payment", "rv:1", "calls");
        // No evidence added

        let result = explain(&repo, "rel:orders-payment").unwrap();
        assert_eq!(result.subject.kind, "relation");
        assert!(result.provenance.unsubstantiated);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn explain_relation_no_version_id_returns_unsubstantiated_with_warning() {
        let repo = FakeRepo::new().with_relation("rel:orders-payment", "", "calls");

        let result = explain(&repo, "rel:orders-payment").unwrap();
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
        let repo = FakeRepo::new()
            .with_element("c4:container:orders", "v:1", "OrderService")
            .with_element_evidence("v:1", make_evidence("ev:1"));

        let result = explain(&repo, "c4:container:orders").unwrap();
        assert_eq!(result.schema_version, "1.1");
        assert_eq!(result.capability, "architecture-explain-mvp");
    }

    // -------------------------------------------------------------------------
    // Routing tests
    // -------------------------------------------------------------------------

    #[test]
    fn explain_routes_uml_id_to_element_path() {
        let repo = FakeRepo::new()
            .with_element("uml:class:OrderService", "v:2", "OrderService")
            .with_element_evidence("v:2", make_evidence("ev:2"));

        let result = explain(&repo, "uml:class:OrderService").unwrap();
        assert_eq!(result.subject.kind, "element");
    }

    #[test]
    fn explain_routes_behavior_id_to_element_path() {
        let repo = FakeRepo::new()
            .with_element("behavior:user:login", "v:3", "login")
            .with_element_evidence("v:3", make_evidence("ev:3"));

        let result = explain(&repo, "behavior:user:login").unwrap();
        assert_eq!(result.subject.kind, "element");
    }
}
