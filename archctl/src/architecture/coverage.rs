//! Architecture coverage use case — aggregate evidence quality metrics.
//!
//! Read-only scan of the live architecture graph that produces a
//! `CoverageReport` across four axes: confidence, evidence status,
//! conflict, and staleness. No graph-store writes are performed.
//!
//! ## Public surface
//!
//! - `coverage` — the main use case function
//! - `CoverageReport` — the JSON-serializable output carrier
//! - `CoverageError` — domain errors
//!
//! ## MVP defaults (documented as invariants)
//!
//! - Confidence bucket thresholds: `high ≥ 0.9`, `medium ≥ 0.7`,
//!   `low ≥ 0.5`, `unknown < 0.5` or missing.
//! - Staleness cutoff: 90 days before the current clock time.

use serde::{Deserialize, Serialize};

use crate::clock::Clock;
use crate::diagram::export_types::EvidenceEntry;
use crate::store::DiagramRepository;

/// Confidence bucket for `byConfidence`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConfidenceBuckets {
    /// Elements/relations with confidence ≥ 0.9.
    pub high: usize,
    /// Elements/relations with confidence ≥ 0.7 and < 0.9.
    pub medium: usize,
    /// Elements/relations with confidence ≥ 0.5 and < 0.7.
    pub low: usize,
    /// Elements/relations with confidence < 0.5 or not set.
    pub unknown: usize,
}

/// Evidence-status buckets for `byEvidenceStatus`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EvidenceStatusBuckets {
    /// Evidence rows with `Accepted` status.
    pub accepted: usize,
    /// Evidence rows with `Drafted` status.
    pub drafted: usize,
    /// Evidence rows with `Superseded` status.
    pub superseded: usize,
}

/// Conflict buckets for `byConflict`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConflictBuckets {
    /// Subjects involved in a `CONTRADICTED_BY` edge.
    /// **Always 0 in MVP** — no extractor populates `CONTRADICTED_BY`.
    pub conflicted: usize,
}

/// Staleness buckets for `byStaleness`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StalenessBuckets {
    /// Subjects with evidence observed within 90 days.
    pub fresh: usize,
    /// Subjects with evidence observed more than 90 days ago.
    pub stale: usize,
}

/// Fused-claim buckets for `byFusedClaims` (v6 persisted claims).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FusedClaimBuckets {
    /// Total persisted fused claims across all version-linked subjects.
    pub total: usize,
    /// Claims supported by exactly one observation.
    pub by_supports_single: usize,
    /// Claims supported by two or more observations.
    pub by_supports_multi: usize,
    /// Claims cross-linked through `CONTRADICTS` (conflicts_with non-empty).
    pub by_conflicts: usize,
    /// Claims flagged stale at persist time (90-day cutoff).
    pub by_staleness_fresh: usize,
    /// Claims NOT flagged stale at persist time.
    pub by_staleness_stale: usize,
}

/// The coverage-report/1 carrier — the output of the `coverage` use case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageReport {
    /// Schema version of this report format.
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,

    /// Capability that produced this report.
    pub capability: String,

    /// Total number of elements in the graph.
    #[serde(rename = "totalElements")]
    pub total_elements: usize,

    /// Total number of relations in the graph.
    #[serde(rename = "totalRelations")]
    pub total_relations: usize,

    /// Element/relation counts bucketed by confidence threshold.
    /// Bucket thresholds (MVP defaults): high ≥ 0.9, medium ≥ 0.7,
    /// low ≥ 0.5, unknown < 0.5 or missing.
    #[serde(rename = "byConfidence")]
    pub by_confidence: ConfidenceBuckets,

    /// Evidence rows bucketed by lifecycle status.
    #[serde(rename = "byEvidenceStatus")]
    pub by_evidence_status: EvidenceStatusBuckets,

    /// Subjects bucketed by conflict status.
    #[serde(rename = "byConflict")]
    pub by_conflict: ConflictBuckets,

    /// Subjects bucketed by evidence staleness.
    /// Staleness cutoff (MVP default): 90 days before `clock.now_rfc3339()`.
    #[serde(rename = "byStaleness")]
    pub by_staleness: StalenessBuckets,

    /// Persisted fused claims (v6) bucketed by support, conflict and
    /// staleness. All-zero when no fused claims are persisted.
    #[serde(rename = "byFusedClaims")]
    pub by_fused_claims: FusedClaimBuckets,

    /// Elements with no version link and no evidence.
    #[serde(rename = "unsubstantiatedCount")]
    pub unsubstantiated_count: usize,

    /// Warnings generated during coverage computation.
    pub warnings: Vec<String>,
}

/// Errors specific to coverage operations.
#[derive(Debug, Clone)]
pub enum CoverageError {
    /// The store returned an error.
    Store(String),
}

impl std::fmt::Display for CoverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoverageError::Store(msg) => write!(f, "coverage error: {msg}"),
        }
    }
}

impl std::error::Error for CoverageError {}

impl From<anyhow::Error> for CoverageError {
    fn from(e: anyhow::Error) -> Self {
        CoverageError::Store(e.to_string())
    }
}

/// Confidence bucket for a given confidence value.
///
/// Bucket thresholds (MVP defaults):
/// - `high`: confidence ≥ 0.9
/// - `medium`: confidence ≥ 0.7
/// - `low`: confidence ≥ 0.5
/// - `unknown`: confidence < 0.5 or NaN
fn bucket_for_confidence(confidence: f64) -> &'static str {
    if confidence >= 0.9 {
        "high"
    } else if confidence >= 0.7 {
        "medium"
    } else if confidence >= 0.5 {
        "low"
    } else {
        "unknown"
    }
}

/// Returns true when `observed_at` is strictly before `cutoff` in
/// RFC3339 lexicographic (ASCII) ordering.
///
/// RFC3339 timestamps sort lexicographically as strings, so a simple
/// string comparison gives the correct temporal ordering.
fn is_stale(observed_at: &str, cutoff: &str) -> bool {
    observed_at < cutoff
}

/// Compute the staleness cutoff timestamp: 90 days before `clock.now_rfc3339()`.
///
/// Returns an RFC3339 timestamp string representing the point in time
/// that is 90 days before the current clock reading.
fn cutoff_90d(clock: &dyn Clock) -> String {
    use chrono::{DateTime, Duration, Utc};

    let now_str = clock.now_rfc3339();
    // Parse the RFC3339 timestamp
    let now = DateTime::parse_from_rfc3339(&now_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let cutoff = now - Duration::days(90);
    cutoff.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Compute coverage metrics over the live architecture graph.
///
/// Iterates all elements across `c4`, `uml`, and `behavior` categories,
/// all semantic edges (relations), and all evidence entries. Buckets are
/// accumulated across both element and relation axes where applicable.
///
/// No graph-store writes are performed.
pub fn coverage(
    repo: &dyn DiagramRepository,
    clock: &dyn Clock,
) -> Result<CoverageReport, CoverageError> {
    let mut total_elements: usize = 0;
    let mut total_relations: usize = 0;

    let mut by_confidence = ConfidenceBuckets::default();
    let mut by_evidence_status = EvidenceStatusBuckets::default();
    let by_conflict = ConflictBuckets::default();
    let mut by_staleness = StalenessBuckets::default();
    let mut unsubstantiated_count: usize = 0;

    let cutoff = cutoff_90d(clock);

    // Collect all version ids for evidence lookup
    let mut all_version_ids: Vec<String> = Vec::new();

    // Iterate elements across all categories
    for category in &["c4", "uml", "behavior", "code"] {
        let elements = repo.list_elements(category, None, None)?;
        for element in elements {
            total_elements += 1;
            let confidence = element.current_confidence;
            match bucket_for_confidence(confidence) {
                "high" => by_confidence.high += 1,
                "medium" => by_confidence.medium += 1,
                "low" => by_confidence.low += 1,
                _ => by_confidence.unknown += 1,
            }

            if element.current_version_id.is_empty() {
                // No version link = unsubstantiated
                unsubstantiated_count += 1;
            } else {
                all_version_ids.push(element.current_version_id.clone());
            }
        }
    }

    // Iterate semantic edges (relations)
    for category in &["c4", "uml", "behavior", "code"] {
        let edges = repo.list_semantic_edges(category)?;
        total_relations += edges.len();
        // SemanticEdgeRow doesn't have confidence — relations in MVP
        // don't carry confidence on the edge itself. Skip bucketing
        // by confidence for relations.
    }

    // Fetch element evidence
    let mut all_evidence: Vec<EvidenceEntry> = Vec::new();
    if !all_version_ids.is_empty() {
        let elem_evidence = repo.list_evidence_for_versions(&all_version_ids)?;
        all_evidence.extend(elem_evidence);
    }

    // Also collect relation version ids and fetch relation evidence
    let mut all_rel_version_ids: Vec<String> = Vec::new();
    for category in &["c4", "uml", "behavior", "code"] {
        let edges = repo.list_semantic_edges(category)?;
        for edge in edges {
            if !edge.relation_id.is_empty()
                && let Ok(Some(rel)) = repo.read_relation_by_id(&edge.relation_id)
                && !rel.current_version_id.is_empty()
            {
                all_rel_version_ids.push(rel.current_version_id.clone());
            }
        }
    }

    if !all_rel_version_ids.is_empty() {
        let rel_evidence = repo.list_evidence_for_relation_versions(&all_rel_version_ids)?;
        all_evidence.extend(rel_evidence);
    }

    // Deduplicate evidence by id (same evidence can support multiple versions)
    let mut seen_evidence_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for ev in &all_evidence {
        if seen_evidence_ids.contains(&ev.id) {
            continue;
        }
        seen_evidence_ids.insert(ev.id.clone());

        // Count by evidence status
        match ev.status.as_deref() {
            Some("accepted") => by_evidence_status.accepted += 1,
            Some("drafted") => by_evidence_status.drafted += 1,
            Some("superseded") => by_evidence_status.superseded += 1,
            Some(_) | None => {
                // Unknown status treated as drafted for safety
                by_evidence_status.drafted += 1;
            }
        }

        // Check staleness
        if is_stale(&ev.observed_at, &cutoff) {
            by_staleness.stale += 1;
        } else {
            by_staleness.fresh += 1;
        }
    }

    // Always emit the CONTRADICTED_BY warning
    let warnings = vec![String::from(
        "CONTRADICTED_BY edges not populated by any extractor — conflicted count always 0",
    )];

    // Fused claims (v6): count persisted claims across every
    // version-linked subject. Zero buckets when nothing is persisted
    // or the store predates the v6 tables (Ok(None)).
    let mut by_fused_claims = FusedClaimBuckets::default();
    {
        let mut version_ids: Vec<String> = all_version_ids.clone();
        version_ids.extend(all_rel_version_ids);
        version_ids.sort();
        version_ids.dedup();
        if let Some(rows) = repo
            .read_fused_claim_rows(&version_ids)
            .map_err(CoverageError::from)?
        {
            let claim_ids: Vec<String> = rows
                .iter()
                .filter_map(|r| r.get("f.id").and_then(|c| c.as_str()).map(String::from))
                .collect();
            let edges = repo
                .list_fused_conflict_edges(&claim_ids)
                .map_err(CoverageError::from)?;
            let claims = crate::architecture::fusion::fused_claims_from_rows(&rows, &edges);
            by_fused_claims.total = claims.len();
            for claim in &claims {
                if claim.supports > 1 {
                    by_fused_claims.by_supports_multi += 1;
                } else {
                    by_fused_claims.by_supports_single += 1;
                }
                if !claim.conflicts_with.is_empty() {
                    by_fused_claims.by_conflicts += 1;
                }
                if claim.stale {
                    by_fused_claims.by_staleness_stale += 1;
                } else {
                    by_fused_claims.by_staleness_fresh += 1;
                }
            }
        }
    }

    Ok(CoverageReport {
        schema_version: "1.1".to_string(),
        capability: "architecture-coverage-mvp".to_string(),
        total_elements,
        total_relations,
        by_confidence,
        by_evidence_status,
        by_conflict,
        by_staleness,
        by_fused_claims,
        unsubstantiated_count,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FixedClock;
    use crate::diagram::export_types::EvidenceEntry;
    use crate::store::{ElementRepository, EvidenceRepository, GraphStore, LbugStore};

    /// Builder-style seeder that persists the test fixture into a
    /// real `LbugStore` opened in a TempDir. Mirrors the previous
    /// FakeRepo builder ergonomics. `.build()` returns the
    /// `LbugStore` ready to pass to functions taking `&dyn
    /// DiagramRepository`.
    struct SeededStore {
        project_dir: std::path::PathBuf,
        elements: Vec<(String, String, f64)>, // id, version_id, confidence
        element_evidence: Vec<(String, EvidenceEntry)>,
    }

    impl SeededStore {
        fn new(project_dir: &std::path::Path) -> Self {
            Self {
                project_dir: project_dir.to_path_buf(),
                elements: vec![],
                element_evidence: vec![],
            }
        }
        fn with_element(mut self, id: &str, version_id: &str, confidence: f64) -> Self {
            self.elements
                .push((id.to_string(), version_id.to_string(), confidence));
            self
        }
        fn with_element_evidence(mut self, version_id: &str, evidence: EvidenceEntry) -> Self {
            self.element_evidence
                .push((version_id.to_string(), evidence));
            self
        }
        fn build(self) -> LbugStore {
            let mut store = LbugStore::open(&self.project_dir).expect("LbugStore::open");
            store.init().expect("LbugStore::init");
            for (id, version_id, confidence) in &self.elements {
                let category = if id.starts_with("c4:") {
                    "c4".to_string()
                } else if id.starts_with("uml") {
                    "uml".to_string()
                } else if id.starts_with("behavior:") {
                    "behavior".to_string()
                } else {
                    "c4".to_string()
                };
                let v = crate::graph::ElementVersion {
                    id: version_id.clone(),
                    element_id: id.clone(),
                    name: "TestElement".to_string(),
                    status: "accepted".to_string(),
                    origin: "test".to_string(),
                    confidence: *confidence,
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
                    category: category.clone(),
                    canonical_key: id.clone(),
                    current_name: "TestElement".to_string(),
                    current_status: "active".to_string(),
                    current_confidence: *confidence,
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
                // SUPPORTED_BY: ElementVersion → Evidence
                let cypher = format!(
                    "MATCH (ev:ElementVersion {{id: '{}'}}), (e:Evidence {{id: '{}'}}) \
                     CREATE (ev)-[:SUPPORTED_BY]->(e);",
                    version_id, evidence.id
                );
                store
                    .execute_raw_cypher_for_test(&cypher)
                    .expect("link supported_by");
            }
            store
        }
    }

    fn make_evidence(id: &str, status: &str, observed_at: &str) -> EvidenceEntry {
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
            observed_at: observed_at.to_string(),
            status: Some(status.to_string()),
        }
    }

    // -------------------------------------------------------------------------
    // S2: Empty graph → all zeros + warning
    // -------------------------------------------------------------------------

    #[test]
    fn coverage_empty_graph_yields_zeros_and_warning() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path()).build();
        let clock = FixedClock::new("2026-08-17T00:00:00Z");
        let result = coverage(&repo, &clock).unwrap();

        assert_eq!(result.total_elements, 0);
        assert_eq!(result.total_relations, 0);
        assert_eq!(result.by_confidence.high, 0);
        assert_eq!(result.by_confidence.medium, 0);
        assert_eq!(result.by_confidence.low, 0);
        assert_eq!(result.by_confidence.unknown, 0);
        assert_eq!(result.by_evidence_status.accepted, 0);
        assert_eq!(result.by_evidence_status.drafted, 0);
        assert_eq!(result.by_evidence_status.superseded, 0);
        assert_eq!(result.by_conflict.conflicted, 0);
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("CONTRADICTED_BY"))
        );
    }

    // -------------------------------------------------------------------------
    // S1: Mixed confidence levels → correct bucket counts
    // -------------------------------------------------------------------------

    #[test]
    fn coverage_mixed_confidence_buckets() {
        // The "no-version" element (id "c4:container:e", version "")
        // exercises a defensive bucket in coverage(). Production's
        // validate_identifier rejects empty version ids so the
        // seeder can't reach that state through the normal write
        // ports; we seed the four real elements via the builder and
        // then patch the fifth in via raw Cypher with a comment
        // explaining why.
        let tmp = tempfile::TempDir::new().unwrap();
        let mut repo = SeededStore::new(tmp.path())
            .with_element("c4:container:a", "v:1", 0.95) // high
            .with_element("c4:container:b", "v:2", 0.80) // medium
            .with_element("c4:container:c", "v:3", 0.60) // low
            .with_element("c4:container:d", "v:4", 0.40) // unknown
            .build();
        repo.execute_raw_cypher_for_test(
            "CREATE (:Element {id: 'c4:container:e', kind_id: 'container', \
             category: 'c4', canonical_key: 'c4:container:e', \
             current_name: 'TestElement', current_status: 'active', \
             current_confidence: 0.30, current_version_id: ''});",
        )
        .expect("seed no-version element");

        let clock = FixedClock::new("2026-08-17T00:00:00Z");
        let result = coverage(&repo, &clock).unwrap();

        assert_eq!(result.total_elements, 5);
        assert_eq!(result.by_confidence.high, 1);
        assert_eq!(result.by_confidence.medium, 1);
        assert_eq!(result.by_confidence.low, 1);
        assert_eq!(result.by_confidence.unknown, 2); // 0.40 + no-version
    }

    // -------------------------------------------------------------------------
    // S3: All high-confidence accepted → all in high/accepted
    // -------------------------------------------------------------------------

    #[test]
    fn coverage_all_high_accepted() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path())
            .with_element("c4:container:a", "v:1", 0.95)
            .with_element("c4:container:b", "v:2", 0.92)
            .with_element("c4:container:c", "v:3", 0.99)
            .with_element("c4:container:d", "v:4", 0.91)
            .with_element("c4:container:e", "v:5", 0.90)
            .with_element_evidence(
                "v:1",
                make_evidence("ev:1", "accepted", "2026-08-01T00:00:00Z"),
            )
            .with_element_evidence(
                "v:2",
                make_evidence("ev:2", "accepted", "2026-08-01T00:00:00Z"),
            )
            .with_element_evidence(
                "v:3",
                make_evidence("ev:3", "accepted", "2026-08-01T00:00:00Z"),
            )
            .with_element_evidence(
                "v:4",
                make_evidence("ev:4", "accepted", "2026-08-01T00:00:00Z"),
            )
            .with_element_evidence(
                "v:5",
                make_evidence("ev:5", "accepted", "2026-08-01T00:00:00Z"),
            )
            .build();
        let clock = FixedClock::new("2026-08-17T00:00:00Z");
        let result = coverage(&repo, &clock).unwrap();

        assert_eq!(result.by_confidence.high, 5);
        assert_eq!(result.by_confidence.medium, 0);
        assert_eq!(result.by_confidence.low, 0);
        assert_eq!(result.by_confidence.unknown, 0);
        assert_eq!(result.by_evidence_status.accepted, 5);
        assert_eq!(result.by_evidence_status.drafted, 0);
        assert_eq!(result.by_evidence_status.superseded, 0);
    }

    // -------------------------------------------------------------------------
    // S4: Drafted evidence surfaces in byEvidenceStatus
    // -------------------------------------------------------------------------

    #[test]
    fn coverage_drafted_evidence_counted() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path())
            .with_element("c4:container:a", "v:1", 0.95)
            .with_element_evidence(
                "v:1",
                make_evidence("ev:1", "drafted", "2026-08-01T00:00:00Z"),
            )
            .with_element_evidence(
                "v:1",
                make_evidence("ev:2", "drafted", "2026-08-01T00:00:00Z"),
            )
            .with_element_evidence(
                "v:1",
                make_evidence("ev:3", "accepted", "2026-08-01T00:00:00Z"),
            )
            .build();
        let clock = FixedClock::new("2026-08-17T00:00:00Z");
        let result = coverage(&repo, &clock).unwrap();

        assert_eq!(result.by_evidence_status.drafted, 2);
        assert_eq!(result.by_evidence_status.accepted, 1);
    }

    // -------------------------------------------------------------------------
    // Invariant tests
    // -------------------------------------------------------------------------

    #[test]
    fn coverage_schema_version_invariant() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path()).build();
        let clock = FixedClock::new("2026-08-17T00:00:00Z");
        let result = coverage(&repo, &clock).unwrap();
        assert_eq!(result.schema_version, "1.1");
    }

    #[test]
    fn coverage_capability_invariant() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path()).build();
        let clock = FixedClock::new("2026-08-17T00:00:00Z");
        let result = coverage(&repo, &clock).unwrap();
        assert_eq!(result.capability, "architecture-coverage-mvp");
    }

    #[test]
    fn coverage_warning_always_present() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path()).build();
        let clock = FixedClock::new("2026-08-17T00:00:00Z");
        let result = coverage(&repo, &clock).unwrap();
        assert!(!result.warnings.is_empty());
    }
}
