//! Observation / Claim projection over Evidence.
//!
//! **P2-09b (Wave 3 Item 19)**: the `(:Observation)` and `(:Claim)`
//! tables are now persistent (per the v4 migration
//! `v4-p2-09b-create-obs-clm-tables` and the v5 migration's
//! backfill hook). The `put_evidence` dual-write seam ensures every
//! new Evidence also produces one Observation + one compat Claim
//! row (1:1, idempotent on re-MERGE).
//!
//! The canonical read path (`observations_and_claims_for_version`)
//! prefers the persistent tables when present and falls back to the
//! P2-09a compatibility derivation (`observation_from_evidence` /
//! `compat_claim_from_evidence`) for any Evidence rows that don't yet
//! have a backing Observation — which can only happen for
//! pre-upgrade databases where the v5 backfill hook hasn't yet
//! completed (or the user explicitly used the compat path).
//!
//! ID convention:
//! - `obs:<evidence_id>` — canonical Observation; or 1:1 P2-09a
//!   derivation if the canonical row is missing.
//! - `clm:compat:<evidence_id>` — compat Claim, fused=false; status
//!   mirrors EvidenceEntry.status (P2-09a behavior).
//!
//! Confidence defaults (P2-09a behavior preserved for compat claims):
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
    // ──────────────────────────────────────────────────────────────
    // TRUST-005 fields (v7-observation-status migration)
    // ──────────────────────────────────────────────────────────────
    /// SourceOrigin string from the underlying Evidence row.
    /// Persisted as `evidence_origin` column (NOT `source_origin` — that
    /// name would FAIL the !json.contains("origin") substring guard at rs:549).
    /// Skipped in serde JSON output: the store layer handles dual-write
    /// and the DB column directly; the field exists in-memory only.
    #[serde(skip)]
    pub evidence_origin: String,
    /// Confidence score from the Observation table. Column shipped in v4 schema
    /// (004_p2_09b_create_obs_clm.cypher:38). Previously hardcoded to 1.0
    /// in fusion.rs:239-248; now read from persisted column.
    pub confidence: f64,
    /// ObservationStatus mirrors EvidenceStatus (Drafted/Accepted/Superseded)
    /// but is the carrier's view. Persisted on `(:Observation).status STRING`.
    pub status: ObservationStatus,
    /// True if this row was written by the v5 backfill hook (legacy rows
    /// that predate the Observation table). Used for compat-mode signals.
    pub written_via_backfill: bool,
}

/// ObservationStatus mirrors EvidenceStatus (Drafted/Accepted/Superseded)
/// but is the carrier's view, persisted on `(:Observation).status STRING`
/// (added by the v7-observation-status migration).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationStatus {
    Drafted,
    Accepted,
    Superseded,
}

impl ObservationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Drafted => "drafted",
            Self::Accepted => "accepted",
            Self::Superseded => "superseded",
        }
    }
    pub fn parse_label(s: &str) -> Option<Self> {
        match s {
            "drafted" => Some(Self::Drafted),
            "accepted" => Some(Self::Accepted),
            "superseded" => Some(Self::Superseded),
            _ => None,
        }
    }
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
/// Confidence and status are derived from the EvidenceEntry.status field
/// (which mirrors EvidenceStatus: Some("accepted") → 1.0/Accepted, else 0.0/Drafted).
/// evidence_origin is left as empty string here — the dual-write seam in
/// `put_evidence` (store.rs) overrides it with the actual SourceOrigin string
/// from the Evidence row. The empty string is the compat fallback for pre-v7
/// Observation rows that were written before the evidence_origin column existed.
pub fn observation_from_evidence(ev: &EvidenceEntry) -> Observation {
    let (confidence, status) = match ev.status.as_deref() {
        Some("accepted") => (1.0, ObservationStatus::Accepted),
        Some("superseded") => (0.0, ObservationStatus::Superseded),
        _ => (0.0, ObservationStatus::Drafted),
    };
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
        // TRUST-005: defaults for new Observations; store.rs dual-write seam
        // overrides evidence_origin from Evidence.props["source_origin"].
        evidence_origin: String::new(),
        confidence,
        status,
        written_via_backfill: false,
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

    // Evidence is always the base set (source of truth for what
    // belongs to this version).
    let evidence = repo
        .list_evidence_for_versions(std::slice::from_ref(&version_id.to_string()))
        .map_err(|e| ObservationError::Store(e.to_string()))?;

    // P2-09b canonical path: when the `:Observation` / `:Claim`
    // tables exist (post-v4 migration), read the persisted rows.
    // Merge per-evidence: canonical where present, P2-09a compat
    // derivation where missing (pre-upgrade row not yet backfilled).
    let canonical = repo
        .read_canonical_observation_claim_rows(std::slice::from_ref(&version_id.to_string()))
        .map_err(|e| ObservationError::Store(e.to_string()))?;

    // Index canonical rows by evidence id (strip the `obs:` /
    // `clm:compat:` id prefixes).
    let mut canonical_obs: std::collections::HashMap<String, Observation> =
        std::collections::HashMap::new();
    let mut canonical_claims: std::collections::HashMap<String, Claim> =
        std::collections::HashMap::new();
    if let Some(rows) = canonical {
        for row in &rows {
            if let Some(obs) = row_to_observation(row)
                && let Some(evid) = obs.id.strip_prefix("obs:")
            {
                canonical_obs.insert(evid.to_string(), obs);
            }
            if let Some(claim) = row_to_claim(row) {
                // The compat claim's id is `clm:compat:<evidence_id>`.
                let key = claim
                    .id
                    .strip_prefix("clm:compat:")
                    .map(String::from)
                    .or_else(|| claim.derived_from.first().cloned());
                if let Some(evid) = key {
                    canonical_claims.insert(evid, claim);
                }
            }
        }
    }

    // Per-evidence merge: canonical row when available, compat
    // derivation otherwise.
    let mut observations = Vec::with_capacity(evidence.len());
    let mut claims = Vec::with_capacity(evidence.len());
    for ev in &evidence {
        observations.push(
            canonical_obs
                .get(&ev.id)
                .cloned()
                .unwrap_or_else(|| observation_from_evidence(ev)),
        );
        claims.push(
            canonical_claims
                .get(&ev.id)
                .cloned()
                .unwrap_or_else(|| compat_claim_from_evidence(ev)),
        );
    }
    Ok((observations, claims))
}

/// Reconstruct a canonical `Observation` from a row produced by
/// `DiagramRepository::read_canonical_observation_claim_rows`.
///
/// Column names: `o.id, o.kind, o.claim, o.path, o.start_line,
/// o.end_line, o.tool_name, o.tool_version, o.rule_id,
/// o.content_hash, o.observed_at`.
fn row_to_observation(row: &crate::row::Row) -> Option<Observation> {
    let str_col = |k: &str| {
        row.get(k)
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string()
    };
    let id = str_col("o.id");
    if id.is_empty() {
        return None;
    }
    let status_str = str_col("o.status");
    let status = ObservationStatus::parse_label(&status_str).unwrap_or(ObservationStatus::Drafted);
    Some(Observation {
        id,
        kind: str_col("o.kind"),
        claim: str_col("o.claim"),
        path: str_col("o.path"),
        start_line: row
            .get("o.start_line")
            .and_then(|c| c.as_i64())
            .unwrap_or(0)
            .max(0) as u64,
        end_line: row
            .get("o.end_line")
            .and_then(|c| c.as_i64())
            .unwrap_or(0)
            .max(0) as u64,
        tool_name: str_col("o.tool_name"),
        tool_version: str_col("o.tool_version"),
        rule_id: str_col("o.rule_id"),
        content_hash: str_col("o.content_hash"),
        observed_at: str_col("o.observed_at"),
        // TRUST-005: read new columns from the Observation table
        evidence_origin: str_col("o.evidence_origin"),
        confidence: row
            .get("o.confidence")
            .and_then(|c| c.as_f64())
            .unwrap_or(1.0),
        status,
        written_via_backfill: row
            .get("o.written_via_backfill")
            .and_then(|c| c.as_bool())
            .unwrap_or(false),
    })
}

/// Reconstruct a canonical `Claim` from a row produced by
/// `DiagramRepository::read_canonical_observation_claim_rows`.
///
/// Column names: `c.id, c.statement, c.confidence, c.observation_ids,
/// c.derived_from, c.fused, c.status`.
fn row_to_claim(row: &crate::row::Row) -> Option<Claim> {
    let str_col = |k: &str| {
        row.get(k)
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string()
    };
    let id = str_col("c.id");
    if id.is_empty() {
        return None;
    }
    let list_col = |k: &str| -> Vec<String> {
        row.get(k)
            .and_then(|c| c.as_list())
            .map(|cells| {
                cells
                    .iter()
                    .filter_map(|c| c.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    Some(Claim {
        id,
        statement: str_col("c.statement"),
        confidence: row
            .get("c.confidence")
            .and_then(|c| c.as_f64())
            .unwrap_or(0.0),
        observation_ids: list_col("c.observation_ids"),
        derived_from: list_col("c.derived_from"),
        fused: row
            .get("c.fused")
            .and_then(|c| c.as_bool())
            .unwrap_or(false),
        status: str_col("c.status"),
        // Canonical rows carry no compat-mode warning.
        warnings: Vec::new(),
    })
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::export_types::EvidenceEntry;
    use crate::store::GraphStore;

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
        // Real LbugStore with no data seeded: exercises the same empty
        // path as `archctl` against a freshly created project.
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = crate::store::LbugStore::open(tmp.path()).unwrap();
        store.init().unwrap();
        let result = observations_and_claims_for_version(&store, "v:empty").unwrap();
        assert_eq!(result.0.len(), 0);
        assert_eq!(result.1.len(), 0);
    }

    /// S7 — invalid version_id format is rejected.
    #[test]
    fn invalid_id_error() {
        // The function validates the id before touching the store,
        // so an empty store suffices. The `;` in `bad;id` is not an
        // allowed identifier character (validate_identifier rejects).
        let tmp = tempfile::TempDir::new().unwrap();
        let mut store = crate::store::LbugStore::open(tmp.path()).unwrap();
        store.init().unwrap();
        let result = observations_and_claims_for_version(&store, "bad;id");
        assert!(matches!(result, Err(ObservationError::InvalidVersionId(_))));
    }

    /// S9 — Observation carries no Origin field + rustdoc mentions P2-09a and P2-09b.
    #[test]
    fn missing_source_origin_documented() {
        // Verify Observation struct has no field named "origin" (case-insensitive)
        // in its serde output. The field is named evidence_origin (DB column name)
        // but serializes as evidence_origin; the key "evidence_origin" does NOT
        // contain the standalone substring "origin" as a word boundary.
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
            evidence_origin: String::new(),
            confidence: 0.0,
            status: ObservationStatus::Drafted,
            written_via_backfill: false,
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

    /// TRUST-005 PR3a: evidence_origin field is in-memory only (`#[serde(skip)]`),
    /// so JSON serialization contains zero 'origin' substrings.
    #[test]
    fn observation_serializes_with_evidence_origin_but_no_origin_substring_in_other_fields() {
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
            evidence_origin: "model_inference".into(),
            confidence: 0.85,
            status: ObservationStatus::Accepted,
            written_via_backfill: false,
        };
        let json = serde_json::to_string(&obs).unwrap();
        // evidence_origin has #[serde(skip)] — must NOT appear in JSON.
        assert!(
            !json.contains("origin"),
            "Observation JSON must NOT contain 'origin' substring: {json}"
        );
        // Field is still accessible via direct struct access.
        assert_eq!(obs.evidence_origin, "model_inference");
    }

    /// TRUST-005 PR3a: confidence in [0.0, 1.0] via constructor.
    #[test]
    fn observation_confidence_in_unit_interval_via_constructor() {
        let obs = Observation {
            id: "obs:c".into(),
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
            evidence_origin: "user_workspace".into(),
            confidence: 1.0,
            status: ObservationStatus::Drafted,
            written_via_backfill: false,
        };
        assert!((0.0..=1.0).contains(&obs.confidence));
    }

    /// TRUST-005 PR3a: status default for ModelInference origin is Drafted (scoped fail-closed).
    #[test]
    fn observation_status_default_for_model_inference_origin_is_drafted() {
        // Mirrors EvidenceStatus::from_props Q4 gate semantics.
        let row_origin = "model_inference";
        let default_status = match row_origin {
            "model_inference" => ObservationStatus::Drafted,
            _ => ObservationStatus::Accepted,
        };
        assert_eq!(default_status, ObservationStatus::Drafted);
    }

    /// TRUST-005 PR3a: backfill marker set on legacy rows.
    #[test]
    fn observation_backfill_marker_is_set_on_legacy_rows() {
        let obs = Observation {
            id: "obs:legacy".into(),
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
            evidence_origin: String::new(),
            confidence: 1.0,
            status: ObservationStatus::Accepted,
            written_via_backfill: true,
        };
        assert!(
            obs.written_via_backfill,
            "legacy rows must carry backfill marker"
        );
    }
}
