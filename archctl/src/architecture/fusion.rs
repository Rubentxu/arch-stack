//! Fusion engine (Wave 3 Item 27 + follow-ups).
//!
//! Aggregates multiple `Observation`s into one `FusedClaim` per
//! distinct statement — deterministic, order-independent, with
//! non-opaque confidence and 0 provenance loss.
//!
//! Semantics (v1):
//! - Group key: `(kind, normalized_claim)`.
//! - `supports` = number of observations asserting the same
//!   statement.
//! - `conflicts_with` = fused claims sharing `(kind, path)` but
//!   asserting a different statement (cross-linked both ways).
//! - Confidence = `max` over member confidences (observation
//!   confidence: 1.0 when its evidence status is accepted, else
//!   0.0 — same membership rule as the compat claims).
//! - Id: `clm:fused:<blake3(sorted observation_ids)>` —
//!   content-addressed and order-independent.
//!
//! v2 (Item 27 follow-ups): the aggregation strategy is pluggable
//! via [`ClaimEvaluator`]. `MaxMemberEvaluator` preserves the v1
//! semantics exactly; `StalenessWeightedEvaluator` applies a 0.5
//! confidence factor when any member observation is older than the
//! 90-day staleness cutoff and flags the claim as `stale`.

use crate::observation_claim::Observation;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A claim produced by fusing multiple observations of the same
/// statement. Never loses provenance: `observation_ids` and
/// `derived_from` list every member.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FusedClaim {
    /// `clm:fused:<blake3(sorted observation_ids)>`.
    pub id: String,
    /// Group kind (copied from the member observations).
    pub kind: String,
    /// Normalized statement (group key).
    pub statement: String,
    /// Max member confidence (non-opaque aggregation).
    pub confidence: f64,
    /// Sorted ids of every fused observation (provenance, 0 loss).
    pub observation_ids: Vec<String>,
    /// Sorted evidence ids backing the fused observations.
    pub derived_from: Vec<String>,
    /// Number of supporting observations.
    pub supports: usize,
    /// Ids of fused claims contradicting this one (same kind+path,
    /// different statement).
    pub conflicts_with: Vec<String>,
    /// "accepted" when any member evidence is accepted.
    pub status: String,
    /// True when any member observation is stale (v2 semantics;
    /// always false under `MaxMemberEvaluator`).
    pub stale: bool,
    pub warnings: Vec<String>,
}

/// Fusion strategy for a group of observations asserting the same
/// statement. Implementations MUST be deterministic and
/// order-independent (the group members are collected in input order,
/// so strategies must not depend on that order).
pub trait ClaimEvaluator: Send + Sync {
    /// Stable strategy name, used by `architecture fuse --evaluator`.
    fn name(&self) -> &'static str;

    /// Aggregated confidence for the member observations.
    fn confidence(&self, members: &[&Observation], now: &str) -> f64;

    /// Whether the fused claim should be flagged stale.
    ///
    /// Defaults to `false` — v1 semantics: staleness is not part of
    /// the aggregation.
    fn stale(&self, members: &[&Observation], now: &str) -> bool {
        let _ = (members, now);
        false
    }
}

/// v1 strategy: confidence = max member confidence (accepted → 1.0,
/// else 0.0; the Observation carrier v1 default is 1.0). Never stale.
pub struct MaxMemberEvaluator;

impl ClaimEvaluator for MaxMemberEvaluator {
    fn name(&self) -> &'static str {
        "max-member"
    }

    fn confidence(&self, members: &[&Observation], _now: &str) -> f64 {
        members
            .iter()
            .map(|o| observation_confidence(o))
            .fold(0.0_f64, f64::max)
    }
}

/// v2 strategy: max-member confidence scaled by staleness. A claim is
/// stale when any member observation's `observed_at` is older than
/// [`STALENESS_CUTOFF_DAYS`] before `now`; stale claims get
/// confidence × 0.5. Deterministic, order-independent.
pub struct StalenessWeightedEvaluator;

/// Staleness cutoff in days — same invariant as `architecture
/// coverage` (`StalenessBuckets`): fresh ≤ 90 days, stale > 90 days.
pub const STALENESS_CUTOFF_DAYS: i64 = 90;

impl ClaimEvaluator for StalenessWeightedEvaluator {
    fn name(&self) -> &'static str {
        "staleness-weighted"
    }

    fn confidence(&self, members: &[&Observation], now: &str) -> f64 {
        let base = members
            .iter()
            .map(|o| observation_confidence(o))
            .fold(0.0_f64, f64::max);
        if self.stale(members, now) {
            base * 0.5
        } else {
            base
        }
    }

    fn stale(&self, members: &[&Observation], now: &str) -> bool {
        members.iter().any(|o| is_stale_observation(o, now))
    }
}

/// True when the observation's `observed_at` is older than the
/// 90-day cutoff relative to `now` (RFC 3339).
///
/// - Empty `now` disables staleness (nothing to compare against).
/// - Unparseable `observed_at` is treated as stale (conservative:
///   if we cannot prove freshness, flag it).
fn is_stale_observation(obs: &Observation, now: &str) -> bool {
    if now.is_empty() {
        return false;
    }
    let Ok(now_dt) = chrono::DateTime::parse_from_rfc3339(now) else {
        // Unparseable `now` — same conservative rule.
        return true;
    };
    let Some(obs_dt) = parse_observed_at(&obs.observed_at) else {
        return true;
    };
    let age = now_dt.with_timezone(&chrono::Utc) - obs_dt;
    age > chrono::Duration::days(STALENESS_CUTOFF_DAYS)
}

/// Parse an observed-at timestamp into UTC. Accepts RFC 3339 and the
/// readback format LadybugDB produces for `timestamp()` columns:
/// `"2026-08-15 0:00:00.0 +00:00:00"` (space separator, unpadded hour,
/// fractional seconds, offset with seconds). Parsing the lbug format
/// is required — without it every persisted observation looks stale
/// (conservative fallback) and the staleness-weighted evaluator is
/// useless on real data (Item 27 residual discovery).
///
/// `pub(crate)` so migration backfill tests can pin the written_at
/// contract (P2-09b residual).
pub(crate) fn parse_observed_at(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&chrono::Utc));
    }
    // Normalize the lbug readback format into RFC 3339:
    //   "2026-08-15 0:00:00.0 +00:00:00" → "2026-08-15T00:00:00.0+00:00"
    let mut parts = s.trim().splitn(3, ' ');
    let date = parts.next()?;
    let time = parts.next()?;
    let offset = parts.next()?;
    // Pad a single-digit hour: "0:00:00.0" → "00:00:00.0".
    let time = {
        let mut t = time.split(':');
        let hour = t.next()?;
        let rest: Vec<&str> = t.collect();
        let padded = if hour.len() == 1 {
            format!("0{hour}")
        } else {
            hour.to_string()
        };
        format!("{padded}:{}", rest.join(":"))
    };
    // Offset "+00:00:00" → "+00:00" (drop the seconds component).
    let offset = if offset.len() == 9 && offset.starts_with('+') && offset.ends_with(":00") {
        offset[..6].to_string()
    } else {
        offset.to_string()
    };
    let rfc = format!("{date}T{time}{offset}");
    chrono::DateTime::parse_from_rfc3339(&rfc)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .ok()
}

/// Normalize a claim text for grouping: trim, collapse whitespace,
/// lowercase. Deterministic and locale-independent.
fn normalize_claim(claim: &str) -> String {
    claim
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Member confidence rule (matches the compat claim rule):
/// accepted → 1.0, anything else → 0.0.
///
/// The Observation carrier itself does not carry a status; the
/// compat confidence convention maps to 1.0 for accepted evidence.
/// We use the same convention here: an observation whose evidence
/// is accepted (observed via its status in the canonical row path)
/// contributes 1.0, otherwise 0.0. Since the Observation struct
/// lacks the status field, we conservatively derive it from the
/// observation id namespace + claim provenance; v1 uses the
/// observation confidence default documented below.
fn observation_confidence(_obs: &Observation) -> f64 {
    // P2-09a/b compatibility: the Observation carrier does not
    // expose an evidence status directly. The canonical row path
    // stores `confidence` on the Observation node; for the pure
    // in-memory projection we default to 1.0 (the common case for
    // accepted evidence) and document this as the v1 rule.
    // A future cycle can thread status/confidence through the
    // Observation carrier itself.
    1.0
}

/// Extract the evidence id from an observation id (`obs:<evid>`).
fn evidence_id_from_observation(obs_id: &str) -> Option<String> {
    obs_id.strip_prefix("obs:").map(String::from)
}

/// Hash helper: blake3 of the sorted observation ids.
fn fused_id(observation_ids: &[String]) -> String {
    let mut sorted = observation_ids.to_vec();
    sorted.sort();
    let joined = sorted.join("|");
    let digest = blake3::hash(joined.as_bytes());
    format!("clm:fused:{}", digest.to_hex())
}

/// Aggregate observations into fused claims using the v1
/// (`MaxMemberEvaluator`) strategy.
///
/// Deterministic: input order does not matter (groups are
/// BTreeMap-ordered; ids are sorted before hashing). Empty input
/// yields an empty vector.
pub fn fuse_observations(observations: &[Observation]) -> Vec<FusedClaim> {
    fuse_observations_with(observations, &MaxMemberEvaluator, "")
}

/// Aggregate observations into fused claims with an explicit
/// evaluation strategy.
///
/// Deterministic: input order does not matter (groups are
/// BTreeMap-ordered; ids are sorted before hashing). Empty input
/// yields an empty vector. `now` is an RFC 3339 timestamp consumed by
/// staleness-aware evaluators (ignored by `MaxMemberEvaluator`).
pub fn fuse_observations_with(
    observations: &[Observation],
    evaluator: &dyn ClaimEvaluator,
    now: &str,
) -> Vec<FusedClaim> {
    if observations.is_empty() {
        return vec![];
    }

    // Group by (kind, normalized_claim) — BTreeMap keeps output
    // order deterministic.
    let mut groups: BTreeMap<(String, String), Vec<&Observation>> = BTreeMap::new();
    for obs in observations {
        groups
            .entry((obs.kind.clone(), normalize_claim(&obs.claim)))
            .or_default()
            .push(obs);
    }

    // Build the fused claims.
    let mut claims: Vec<FusedClaim> = Vec::with_capacity(groups.len());
    for ((kind, statement), members) in groups {
        let mut observation_ids: Vec<String> = members.iter().map(|o| o.id.clone()).collect();
        observation_ids.sort();
        observation_ids.dedup();

        let mut derived_from: Vec<String> = observation_ids
            .iter()
            .filter_map(|id| evidence_id_from_observation(id))
            .collect();
        derived_from.sort();
        derived_from.dedup();

        let member_refs: Vec<&Observation> = members.to_vec();
        let confidence = evaluator.confidence(&member_refs, now);
        let stale = evaluator.stale(&member_refs, now);

        // Status: accepted if any member observation's evidence is
        // accepted (v1: always true per the confidence rule).
        let status = if confidence > 0.0 {
            "accepted"
        } else {
            "drafted"
        };

        claims.push(FusedClaim {
            id: fused_id(&observation_ids),
            kind: kind.clone(),
            statement: statement.clone(),
            confidence,
            observation_ids,
            derived_from,
            supports: members.len(),
            conflicts_with: Vec::new(),
            status: status.to_string(),
            stale,
            warnings: Vec::new(),
        });
    }

    // Contradiction pass: groups sharing (kind, path) with
    // different statements cross-link each other.
    // Path is taken from the first member's observation path; a
    // group "occupies" a path when any member does.
    let mut path_groups: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
    for (idx, claim) in claims.iter().enumerate() {
        // We need the member paths; rebuild a small index from the
        // original observations to avoid storing paths on the claim.
        // (Observation path is not part of FusedClaim — keep it lean.)
        for obs in observations
            .iter()
            .filter(|o| o.claim.trim().eq_ignore_ascii_case(&claim.statement))
        {
            path_groups
                .entry((claim.kind.clone(), obs.path.clone()))
                .or_default()
                .push(idx);
        }
    }
    for (_key, members) in path_groups {
        if members.len() > 1 {
            // Clone ids first: we mutate `claims` while reading
            // other entries.
            let ids: Vec<String> = members.iter().map(|&i| claims[i].id.clone()).collect();
            for &a in &members {
                for id in &ids {
                    if claims[a].id != *id && !claims[a].conflicts_with.contains(id) {
                        claims[a].conflicts_with.push(id.clone());
                    }
                }
            }
        }
    }
    for claim in &mut claims {
        claim.conflicts_with.sort();
        claim.conflicts_with.dedup();
        if !claim.conflicts_with.is_empty() {
            claim.warnings.push(format!(
                "conflict_with: {}",
                claim.conflicts_with.join(", ")
            ));
        }
    }

    claims
}

/// Reconstruct `FusedClaim`s from raw rows produced by
/// `DiagramRepository::read_fused_claim_rows` plus the conflict edges
/// produced by `DiagramRepository::list_fused_conflict_edges`.
///
/// Column names: `f.id, f.kind, f.statement, f.confidence,
/// f.supports, f.status, f.stale, f.observation_ids,
/// f.derived_from, f.version_id`.
pub fn fused_claims_from_rows(
    rows: &[crate::row::Row],
    conflict_edges: &[(String, String)],
) -> Vec<FusedClaim> {
    let mut claims: Vec<FusedClaim> = Vec::with_capacity(rows.len());
    for row in rows {
        let str_col = |k: &str| {
            row.get(k)
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string()
        };
        let id = str_col("f.id");
        if id.is_empty() {
            continue;
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
        let mut conflicts: Vec<String> = conflict_edges
            .iter()
            .filter(|(from, _)| *from == id)
            .map(|(_, to)| to.clone())
            .collect();
        conflicts.sort();
        conflicts.dedup();
        let mut warnings = Vec::new();
        if !conflicts.is_empty() {
            warnings.push(format!("conflict_with: {}", conflicts.join(", ")));
        }
        claims.push(FusedClaim {
            id,
            kind: str_col("f.kind"),
            statement: str_col("f.statement"),
            confidence: row
                .get("f.confidence")
                .and_then(|c| c.as_f64())
                .unwrap_or(0.0),
            observation_ids: list_col("f.observation_ids"),
            derived_from: list_col("f.derived_from"),
            supports: row
                .get("f.supports")
                .and_then(|c| c.as_i64())
                .unwrap_or(0)
                .max(0) as usize,
            conflicts_with: conflicts,
            status: str_col("f.status"),
            stale: row
                .get("f.stale")
                .and_then(|c| c.as_bool())
                .unwrap_or(false),
            warnings,
        });
    }
    claims
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(id: &str, kind: &str, claim: &str, path: &str) -> Observation {
        Observation {
            id: id.to_string(),
            kind: kind.to_string(),
            claim: claim.to_string(),
            path: path.to_string(),
            start_line: 1,
            end_line: 2,
            tool_name: "ast-grep".to_string(),
            tool_version: "0.1".to_string(),
            rule_id: "test:rule".to_string(),
            content_hash: String::new(),
            observed_at: "2026-08-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn empty_input_yields_no_claims() {
        assert!(fuse_observations(&[]).is_empty());
    }

    #[test]
    fn single_observation_yields_one_fused_claim() {
        let obs = obs(
            "obs:ev:1",
            "structural",
            "function foo exists",
            "src/lib.rs",
        );
        let claims = fuse_observations(&[obs]);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].supports, 1);
        assert_eq!(claims[0].observation_ids, vec!["obs:ev:1".to_string()]);
        assert_eq!(claims[0].derived_from, vec!["ev:1".to_string()]);
        assert!(claims[0].conflicts_with.is_empty());
        assert!(claims[0].id.starts_with("clm:fused:"));
    }

    #[test]
    fn two_supports_fuse_into_one_claim_with_provenance() {
        let a = obs(
            "obs:ev:1",
            "structural",
            "function foo exists",
            "src/lib.rs",
        );
        let b = obs(
            "obs:ev:2",
            "structural",
            "function foo exists",
            "src/lib.rs",
        );
        let claims = fuse_observations(&[a, b]);
        assert_eq!(claims.len(), 1, "same statement must fuse");
        assert_eq!(claims[0].supports, 2);
        assert_eq!(claims[0].observation_ids.len(), 2, "0 provenance loss");
        assert_eq!(claims[0].derived_from.len(), 2);
    }

    #[test]
    fn order_independence() {
        let a = obs("obs:ev:1", "structural", "foo exists", "src/a.rs");
        let b = obs("obs:ev:2", "structural", "bar exists", "src/b.rs");
        let c = obs("obs:ev:3", "structural", "foo exists", "src/a.rs");
        let forward = fuse_observations(&[a.clone(), b.clone(), c.clone()]);
        let backward = fuse_observations(&[c, b, a]);
        assert_eq!(forward, backward, "output must be order-independent");
    }

    #[test]
    fn determinism_byte_equal() {
        let input = vec![
            obs("obs:ev:1", "structural", "foo exists", "src/a.rs"),
            obs("obs:ev:2", "lexical", "bar", "src/b.rs"),
        ];
        let r1 = fuse_observations(&input);
        let r2 = fuse_observations(&input);
        assert_eq!(
            serde_json::to_string(&r1).unwrap(),
            serde_json::to_string(&r2).unwrap()
        );
    }

    #[test]
    fn conflicting_statements_cross_link() {
        let a = obs("obs:ev:1", "structural", "foo returns int", "src/a.rs");
        let b = obs("obs:ev:2", "structural", "foo returns string", "src/a.rs");
        let claims = fuse_observations(&[a, b]);
        assert_eq!(claims.len(), 2, "different statements stay separate");
        let (c1, c2) = (&claims[0], &claims[1]);
        assert_eq!(c1.conflicts_with, vec![c2.id.clone()]);
        assert_eq!(c2.conflicts_with, vec![c1.id.clone()]);
        assert!(!c1.warnings.is_empty());
        assert!(!c2.warnings.is_empty());
    }

    #[test]
    fn same_statement_different_paths_not_conflicts() {
        let a = obs("obs:ev:1", "structural", "foo exists", "src/a.rs");
        let b = obs("obs:ev:2", "structural", "foo exists", "src/b.rs");
        let claims = fuse_observations(&[a, b]);
        assert_eq!(claims.len(), 1, "same statement fuses regardless of path");
        assert!(claims[0].conflicts_with.is_empty());
    }

    #[test]
    fn id_is_content_addressed_and_stable() {
        let a = obs("obs:ev:1", "structural", "foo exists", "src/a.rs");
        let b = obs("obs:ev:2", "structural", "foo exists", "src/a.rs");
        let r1 = fuse_observations(&[a.clone(), b.clone()]);
        let r2 = fuse_observations(&[b, a]);
        assert_eq!(r1[0].id, r2[0].id, "id must be order-independent");
        assert!(r1[0].id.starts_with("clm:fused:"));
    }

    #[test]
    fn fuse_observations_matches_max_member_evaluator() {
        // v1 delegation: fuse_observations == fuse_observations_with(MaxMember).
        let input = vec![
            obs("obs:ev:1", "structural", "foo exists", "src/a.rs"),
            obs("obs:ev:2", "lexical", "bar", "src/b.rs"),
        ];
        let via_v1 = fuse_observations(&input);
        let via_trait = fuse_observations_with(&input, &MaxMemberEvaluator, "");
        assert_eq!(via_v1, via_trait, "v1 semantics must be preserved");
        assert!(via_v1.iter().all(|c| !c.stale), "v1 never stale");
        assert_eq!(MaxMemberEvaluator.name(), "max-member");
    }

    #[test]
    fn staleness_cutoff_boundary_fresh_at_exactly_90_days() {
        let now = "2026-08-01T00:00:00Z";
        // observed_at exactly 90 days before now → fresh.
        let mut o = obs("obs:ev:1", "structural", "foo exists", "src/a.rs");
        o.observed_at = "2026-05-03T00:00:00Z".to_string();
        let stale = is_stale_observation(&o, now);
        assert!(!stale, "exactly 90 days must be fresh");
    }

    #[test]
    fn staleness_cutoff_boundary_stale_after_90_days() {
        let now = "2026-08-01T00:00:00Z";
        // observed_at 90 days + 1 second before now → stale.
        let mut o = obs("obs:ev:1", "structural", "foo exists", "src/a.rs");
        o.observed_at = "2026-05-02T23:59:59Z".to_string();
        let stale = is_stale_observation(&o, now);
        assert!(stale, "90 days + 1s must be stale");
    }

    #[test]
    fn parses_lbug_timestamp_readback_format() {
        // LadybugDB returns timestamp() columns as
        // "2026-08-15 0:00:00.0 +00:00:00" — NOT RFC 3339. Without
        // normalization every persisted observation looks stale
        // (Item 27 residual discovery: staleness-weighted evaluator
        // was broken on real data).
        let parsed = parse_observed_at("2026-08-15 0:00:00.0 +00:00:00");
        assert!(parsed.is_some(), "lbug readback format must parse");
        assert_eq!(parsed.unwrap().to_rfc3339(), "2026-08-15T00:00:00+00:00");

        // Midday unpadded + nonzero offset with seconds.
        let parsed = parse_observed_at("2026-08-15 18:30:00.5 +02:00:00");
        assert!(parsed.is_some(), "offset with seconds must parse");
        assert_eq!(
            parsed.unwrap().to_rfc3339(),
            "2026-08-15T16:30:00.500+00:00"
        );

        // RFC 3339 passes through unchanged.
        let parsed = parse_observed_at("2026-08-15T12:00:00Z");
        assert!(parsed.is_some(), "RFC 3339 must still parse");
        assert_eq!(parsed.unwrap().to_rfc3339(), "2026-08-15T12:00:00+00:00");

        // Garbage → None (conservative stale path upstream).
        assert!(parse_observed_at("not-a-date").is_none());
    }

    #[test]
    fn staleness_weighted_halves_confidence_on_stale_member() {
        let now = "2026-08-01T00:00:00Z";
        let mut fresh = obs("obs:ev:1", "structural", "foo exists", "src/a.rs");
        fresh.observed_at = "2026-07-01T00:00:00Z".to_string();
        let mut old = obs("obs:ev:2", "structural", "foo exists", "src/a.rs");
        old.observed_at = "2025-01-01T00:00:00Z".to_string();

        let claims = fuse_observations_with(&[fresh, old], &StalenessWeightedEvaluator, now);
        assert_eq!(claims.len(), 1, "same statement still fuses");
        assert!(claims[0].stale, "mixed members → stale claim");
        // MaxMember base is 1.0; stale → × 0.5.
        assert!((claims[0].confidence - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn staleness_weighted_all_fresh_keeps_full_confidence() {
        let now = "2026-08-01T00:00:00Z";
        let mut a = obs("obs:ev:1", "structural", "foo exists", "src/a.rs");
        a.observed_at = "2026-07-01T00:00:00Z".to_string();
        let mut b = obs("obs:ev:2", "structural", "foo exists", "src/a.rs");
        b.observed_at = "2026-06-01T00:00:00Z".to_string();

        let claims = fuse_observations_with(&[a, b], &StalenessWeightedEvaluator, now);
        assert!(!claims[0].stale);
        assert!((claims[0].confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn unparseable_observed_at_is_conservatively_stale() {
        let now = "2026-08-01T00:00:00Z";
        let mut o = obs("obs:ev:1", "structural", "foo exists", "src/a.rs");
        o.observed_at = "not-a-timestamp".to_string();
        assert!(is_stale_observation(&o, now));
    }

    #[test]
    fn empty_now_disables_staleness() {
        let mut o = obs("obs:ev:1", "structural", "foo exists", "src/a.rs");
        o.observed_at = "2000-01-01T00:00:00Z".to_string();
        assert!(!is_stale_observation(&o, ""), "no now → not stale");
    }

    #[test]
    fn evaluator_order_independence_with_staleness() {
        let now = "2026-08-01T00:00:00Z";
        let mut a = obs("obs:ev:1", "structural", "foo exists", "src/a.rs");
        a.observed_at = "2026-07-01T00:00:00Z".to_string();
        let mut b = obs("obs:ev:2", "structural", "foo exists", "src/a.rs");
        b.observed_at = "2025-01-01T00:00:00Z".to_string();
        let forward =
            fuse_observations_with(&[a.clone(), b.clone()], &StalenessWeightedEvaluator, now);
        let backward = fuse_observations_with(&[b, a], &StalenessWeightedEvaluator, now);
        assert_eq!(forward, backward, "v2 must be order-independent");
    }
}
