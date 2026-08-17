//! Observation / Claim compatibility projection over EvidenceEntry.
//!
//! P2-09a only — additive read-only mapping. P2-09b (persistent
//! Observation + Claim tables, dual-write, fusion) is deferred.
//!
//! ID convention:
//! - obs:<evidence_id>   — Observation derived 1:1 from a single EvidenceEntry
//! - clm:compat:<evidence_id> — compatibility Claim, fused=false, status mirrors
//!
//! Confidence defaults are COMPATIBILITY ONLY (P2-09b replaces with
//! real recompute). Mappings:
//! - EvidenceStatus::Accepted → confidence 1.0
//! - EvidenceStatus::Drafted, Superseded → confidence 0.0

use crate::diagram::export_types::EvidenceEntry;
use serde::{Deserialize, Serialize};

/// Observation derived 1:1 from an EvidenceEntry.
///
/// **P2-09a**: `SourceOrigin` field is intentionally absent — EvidenceEntry
/// lacks that field. P2-09b will add it when the persistent Observation
/// table is introduced.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Observation {
    /// Namespaced id: `obs:<evidence_id>`.
    pub id: String,
    /// Kind, copied verbatim from EvidenceEntry.
    pub kind: String,
    /// Claim text, copied verbatim from EvidenceEntry.
    pub claim: String,
    /// Source file path.
    pub path: String,
    /// Start line of the evidence range.
    pub start_line: u64,
    /// End line of the evidence range.
    pub end_line: u64,
    /// Tool that produced this observation.
    pub tool_name: String,
    /// Tool version string.
    pub tool_version: String,
    /// Rule id that triggered this observation.
    pub rule_id: String,
    /// SHA-256 of the source content at observation time.
    pub content_hash: String,
    /// ISO-8601 timestamp of observation.
    pub observed_at: String,
}

/// A compatibility Claim derived from a single EvidenceEntry.
///
/// **P2-09a**: This is NOT a fused claim — it maps 1:1 from one
/// EvidenceEntry and is WARNED as compat-only. P2-09b will introduce
/// real fusion with `supports`/`contradicts` relations and per-claim
/// confidence recompute.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Claim {
    /// Namespaced id: `clm:compat:<evidence_id>`.
    pub id: String,
    /// Statement text, copied from EvidenceEntry.claim.
    pub statement: String,
    /// Default confidence: 1.0 if EvidenceEntry.status == "accepted", else 0.0.
    /// P2-09b replaces this with real recompute.
    pub confidence: f64,
    /// References the derived Observation id: `obs:<evidence_id>`.
    pub observation_ids: Vec<String>,
    /// Source evidence ids this claim is derived from.
    pub derived_from: Vec<String>,
    /// Always `false` in P2-09a. P2-09b sets this to `true` for fused claims.
    pub fused: bool,
    /// Mirrors EvidenceEntry.status verbatim.
    pub status: String,
    /// Warnings list; P2-09a always contains one compat-mode warning.
    pub warnings: Vec<String>,
}

/// Errors from observation/claim operations.
#[derive(Debug, Clone)]
pub enum ObservationError {
    /// The store returned an error.
    Store(String),
    /// The version id failed `graph::validate_identifier`.
    InvalidVersionId(String),
}

impl std::fmt::Display for ObservationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObservationError::Store(msg) => write!(f, "store error: {msg}"),
            ObservationError::InvalidVersionId(msg) => {
                write!(f, "invalid version id: {msg}")
            }
        }
    }
}

impl std::error::Error for ObservationError {}

/// Derive an Observation from an EvidenceEntry.
///
/// The id is namespaced as `obs:<evidence_id>`.
pub fn observation_from_evidence(ev: &EvidenceEntry) -> Observation {
    Observation {
        id: format!("obs:{}", ev.id),
        kind: ev.kind.clone(),
        claim: ev.claim.clone(),
        path: ev.path.clone(),
        start_line: ev.start_line,
        end_line: ev.end_line,
        tool_name: ev.tool_name.clone(),
        tool_version: ev.tool_version.clone(),
        rule_id: ev.rule_id.clone(),
        content_hash: ev.content_hash.clone(),
        observed_at: ev.observed_at.clone(),
    }
}

/// Derive a compatibility Claim from an EvidenceEntry.
///
/// The claim id is `clm:compat:<evidence_id>`, fused=false, and
/// confidence defaults per status (1.0 accepted, 0.0 others).
/// A warning is always included noting this is a compat-only mapping.
pub fn compat_claim_from_evidence(ev: &EvidenceEntry) -> Claim {
    let obs_id = format!("obs:{}", ev.id);
    let (confidence, status) = match ev.status.as_deref() {
        Some("accepted") => (1.0, "accepted"),
        Some("drafted") => (0.0, "drafted"),
        Some("superseded") => (0.0, "superseded"),
        // None or unknown treated as drafted for safety (P2-04 coverage behaviour)
        _ => (0.0, "drafted"),
    };
    Claim {
        id: format!("clm:compat:{}", ev.id),
        statement: ev.claim.clone(),
        confidence,
        observation_ids: vec![obs_id],
        derived_from: vec![ev.id.clone()],
        fused: false,
        status: status.to_string(),
        warnings: vec![format!(
            "compat-only Claim from EvidenceEntry id={}; P2-09b will replace",
            ev.id
        )],
    }
}

/// Project all observations and compatibility claims for a given version.
///
/// Validates `version_id` via `graph::validate_identifier`, then calls
/// `repo.list_evidence_for_versions([version_id])` and returns parallel
/// arrays (one Observation + one Claim per evidence row, in source order).
pub fn observations_and_claims_for_version(
    repo: &dyn crate::store::DiagramRepository,
    version_id: &str,
) -> Result<(Vec<Observation>, Vec<Claim>), ObservationError> {
    // Validate id (use crate::graph::validate_identifier)
    crate::graph::validate_identifier(version_id)
        .map_err(|e| ObservationError::InvalidVersionId(e.to_string()))?;
    let evidence = repo
        .list_evidence_for_versions(std::slice::from_ref(&version_id.to_string()))
        .map_err(|e| ObservationError::Store(e.to_string()))?;
    let observations: Vec<Observation> = evidence.iter().map(observation_from_evidence).collect();
    let claims: Vec<Claim> = evidence.iter().map(compat_claim_from_evidence).collect();
    Ok((observations, claims))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::export_types::EvidenceEntry;

    fn make_evidence(id: &str, status: Option<&str>) -> EvidenceEntry {
        EvidenceEntry {
            id: id.to_string(),
            kind: "structural".to_string(),
            claim: "test claim".to_string(),
            path: "src/lib.rs".to_string(),
            start_line: 10,
            end_line: 20,
            tool_name: "ast-grep".to_string(),
            tool_version: "0.1".to_string(),
            rule_id: "test:rule".to_string(),
            content_hash: "sha256:abc".to_string(),
            observed_at: "2026-08-01T00:00:00Z".to_string(),
            status: status.map(String::from),
        }
    }

    /// S1 — evidence id `ev:abc` projects to `obs:ev:abc`.
    #[test]
    fn obs_id_namespaced() {
        let ev = make_evidence("ev:abc", Some("accepted"));
        let obs = observation_from_evidence(&ev);
        assert_eq!(obs.id, "obs:ev:abc");
    }

    /// S2 — compat_claim never returns `fused: true`.
    #[test]
    fn compat_claim_fused_false() {
        let ev = make_evidence("ev:foo", Some("accepted"));
        let claim = compat_claim_from_evidence(&ev);
        assert!(!claim.fused, "fused must be false in P2-09a compat mode");
        assert_eq!(claim.id, "clm:compat:ev:foo");
    }

    /// S3 — drafted, accepted, superseded round-trip.
    #[test]
    fn status_mirrors() {
        for (status_str, expected) in [
            ("drafted", "drafted"),
            ("accepted", "accepted"),
            ("superseded", "superseded"),
        ] {
            let ev = make_evidence("ev:status", Some(status_str));
            let claim = compat_claim_from_evidence(&ev);
            assert_eq!(
                claim.status, expected,
                "status must round-trip for {status_str}"
            );
        }
    }

    /// S4 — accepted → 1.0, drafted/superseded → 0.0.
    #[test]
    fn confidence_default_per_status() {
        let cases = [
            ("accepted", 1.0),
            ("drafted", 0.0),
            ("superseded", 0.0),
            ("unknown_dummy", 0.0), // None maps to drafted → 0.0
        ];
        for (status_str, expected_conf) in cases {
            let ev = make_evidence("ev:conf", Some(status_str));
            let claim = compat_claim_from_evidence(&ev);
            assert!(
                (claim.confidence - expected_conf).abs() < f64::EPSILON,
                "confidence for {status_str} must be {expected_conf}, got {}",
                claim.confidence
            );
        }
        // None status
        let ev = make_evidence("ev:conf-none", None);
        let claim = compat_claim_from_evidence(&ev);
        assert!(
            (claim.confidence - 0.0).abs() < f64::EPSILON,
            "confidence for None status must be 0.0"
        );
    }

    /// S5 — mock repo with N entries returns N obs + N claims in source order.
    #[test]
    fn parallel_arrays_round_trip() {
        let ev1 = make_evidence("ev:1", Some("accepted"));
        let ev2 = make_evidence("ev:2", Some("drafted"));
        let ev3 = make_evidence("ev:3", Some("superseded"));
        let evidence = [ev1.clone(), ev2.clone(), ev3.clone()];

        let observations: Vec<Observation> =
            evidence.iter().map(observation_from_evidence).collect();
        let claims: Vec<Claim> = evidence.iter().map(compat_claim_from_evidence).collect();

        assert_eq!(observations.len(), 3);
        assert_eq!(claims.len(), 3);
        assert_eq!(observations[0].id, "obs:ev:1");
        assert_eq!(observations[1].id, "obs:ev:2");
        assert_eq!(observations[2].id, "obs:ev:3");
        assert_eq!(claims[0].id, "clm:compat:ev:1");
        assert_eq!(claims[1].id, "clm:compat:ev:2");
        assert_eq!(claims[2].id, "clm:compat:ev:3");
        assert_eq!(claims[0].observation_ids[0], "obs:ev:1");
        assert_eq!(claims[1].observation_ids[0], "obs:ev:2");
        assert_eq!(claims[2].observation_ids[0], "obs:ev:3");
    }

    /// S6 — unknown version returns empty arrays (no panic).
    #[test]
    fn empty_version_empty_arrays() {
        // Use a MockRepo that returns empty
        struct MockRepo;
        impl crate::store::DiagramRepository for MockRepo {
            fn list_elements(
                &self,
                _category: &str,
                _scope: Option<&str>,
                _kind: Option<&str>,
            ) -> anyhow::Result<Vec<crate::graph::ElementRow>> {
                Ok(vec![])
            }
            fn list_semantic_edges(
                &self,
                _category: &str,
            ) -> anyhow::Result<Vec<crate::graph::SemanticEdgeRow>> {
                Ok(vec![])
            }
            fn list_evidence_for_versions(
                &self,
                _version_ids: &[String],
            ) -> anyhow::Result<Vec<EvidenceEntry>> {
                Ok(vec![])
            }
            fn list_version_props(
                &self,
                _version_ids: &[String],
            ) -> anyhow::Result<Vec<crate::graph::VersionPropsRow>> {
                Ok(vec![])
            }
            fn read_relation_by_id(
                &self,
                _id: &str,
            ) -> anyhow::Result<Option<crate::graph::RelationRow>> {
                Ok(None)
            }
            fn list_evidence_for_relation_versions(
                &self,
                _version_ids: &[String],
            ) -> anyhow::Result<Vec<EvidenceEntry>> {
                Ok(vec![])
            }
        }
        let repo = MockRepo;
        let result = observations_and_claims_for_version(&repo, "v:empty").unwrap();
        assert_eq!(result.0.len(), 0);
        assert_eq!(result.1.len(), 0);
    }

    /// S7 — invalid version_id format is rejected.
    #[test]
    fn invalid_id_error() {
        struct MockRepo;
        impl crate::store::DiagramRepository for MockRepo {
            fn list_elements(
                &self,
                _category: &str,
                _scope: Option<&str>,
                _kind: Option<&str>,
            ) -> anyhow::Result<Vec<crate::graph::ElementRow>> {
                Ok(vec![])
            }
            fn list_semantic_edges(
                &self,
                _category: &str,
            ) -> anyhow::Result<Vec<crate::graph::SemanticEdgeRow>> {
                Ok(vec![])
            }
            fn list_evidence_for_versions(
                &self,
                _version_ids: &[String],
            ) -> anyhow::Result<Vec<EvidenceEntry>> {
                Ok(vec![])
            }
            fn list_version_props(
                &self,
                _version_ids: &[String],
            ) -> anyhow::Result<Vec<crate::graph::VersionPropsRow>> {
                Ok(vec![])
            }
            fn read_relation_by_id(
                &self,
                _id: &str,
            ) -> anyhow::Result<Option<crate::graph::RelationRow>> {
                Ok(None)
            }
            fn list_evidence_for_relation_versions(
                &self,
                _version_ids: &[String],
            ) -> anyhow::Result<Vec<EvidenceEntry>> {
                Ok(vec![])
            }
        }
        let repo = MockRepo;
        let result = observations_and_claims_for_version(&repo, "bad;id");
        assert!(matches!(result, Err(ObservationError::InvalidVersionId(_))));
    }

    /// S9 — Observation carries no Origin field + rustdoc mentions P2-09a and P2-09b.
    #[test]
    fn missing_source_origin_documented() {
        // Verify Observation struct has no field named "origin" (case-insensitive)
        let obs = Observation {
            id: "obs:test".into(),
            kind: "structural".into(),
            claim: "test".into(),
            path: "src/lib.rs".into(),
            start_line: 1,
            end_line: 10,
            tool_name: "ast-grep".into(),
            tool_version: "0.1".into(),
            rule_id: "test:rule".into(),
            content_hash: "sha256:abc".into(),
            observed_at: "2026-08-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&obs).unwrap();
        assert!(
            !json.contains("origin"),
            "Observation JSON must not contain 'origin' field: {json}"
        );
        // Verify rustdoc mentions P2-09a and P2-09b
        let src = include_str!("observation_claim.rs");
        assert!(
            src.contains("P2-09a") && src.contains("P2-09b"),
            "Source must document both P2-09a and P2-09b"
        );
    }
}
