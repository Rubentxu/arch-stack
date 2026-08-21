//! Adjudication bounded context.
//!
//! ADR-063 — Fusion Bounded Context: Trust-Gated FusedClaim Promotion.
//! REQ-M25-006 — AdjudicationEvent carrier for trust bridge promotion.
//!
//! The `AdjudicationRepository` trait lives in `store.rs` so the trait
//! boundary can convert `AdjudicationEventError` to `StoreError` without
//! adjudication.rs depending on store.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─────────────────────────────────────────────────────────────────────────────
// Id types
// ─────────────────────────────────────────────────────────────────────────────

/// Namespaced id type for Adjudication rows
/// (`adj:<blake3(target+adjudicator+decided_at)>`).
pub type AdjudicationId = String;

/// Namespaced id type for the target fused claim (`clm:fused:<hex>`).
pub type TargetClaimId = String;

/// Actor identity performing the adjudication
/// (e.g. `trust-gate`, `api:review-bot`).
pub type AdjudicatorId = String;

// ─────────────────────────────────────────────────────────────────────────────
// Enums
// ─────────────────────────────────────────────────────────────────────────────

/// Trust-gate adjudication decision on a fused claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjudicationDecision {
    Promote,
    Reject,
    Defer,
}

// ─────────────────────────────────────────────────────────────────────────────
// Error
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AdjudicationEventError {
    #[error("invalid id: {0}")]
    InvalidId(String),

    #[error("invalid target: {0}")]
    InvalidTarget(String),

    #[error("missing decision")]
    MissingDecision,
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Construct a deterministic [`AdjudicationId`] from its inputs.
///
/// Same inputs ⇒ same id (via blake3 hash).
pub fn id_for(target: &str, adjudicator: &str, decided_at: &str) -> AdjudicationId {
    format!(
        "adj:{}",
        blake3::hash(format!("{target}{adjudicator}{decided_at}").as_bytes()).to_hex()
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Carrier
// ─────────────────────────────────────────────────────────────────────────────

/// A record of an adjudication decision on a fused claim target.
///
/// Persisted as `(:Adjudication)` node with typed edge
/// `(:Adjudication)-[:ADJUDICATES]->(:FusedClaim)`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdjudicationEvent {
    /// Namespaced id: `adj:<blake3(target+adjudicator+decided_at)>`.
    pub id: AdjudicationId,

    /// The target FusedClaim id (`clm:fused:<hex>`).
    pub target_fused_claim_id: TargetClaimId,

    /// The adjudicator identity.
    pub adjudicator: AdjudicatorId,

    /// Evidence ids cited in the adjudication.
    pub evidence_refs: Vec<String>,

    /// RFC 3339 timestamp from `Clock::now_rfc3339`.
    pub decided_at: String,

    /// The adjudication decision.
    pub decision: AdjudicationDecision,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_adjudication(decision: AdjudicationDecision) -> AdjudicationEvent {
        AdjudicationEvent {
            id: "adj:test".to_string(),
            target_fused_claim_id: "clm:fused:abc123".to_string(),
            adjudicator: "tester".to_string(),
            evidence_refs: vec!["ev1".to_string()],
            decided_at: "2026-08-21T00:00:00Z".to_string(),
            decision,
        }
    }

    // SCN-T08-001a ───────────────────────────────────────────────────────────

    #[test]
    fn adjudication_event_has_six_top_level_fields() {
        let adj = make_adjudication(AdjudicationDecision::Promote);
        let json = serde_json::to_string(&adj).unwrap();
        let round = serde_json::from_str::<AdjudicationEvent>(&json).unwrap();
        // All 6 fields round-trip equal
        assert_eq!(adj.id, round.id);
        assert_eq!(adj.target_fused_claim_id, round.target_fused_claim_id);
        assert_eq!(adj.adjudicator, round.adjudicator);
        assert_eq!(adj.evidence_refs, round.evidence_refs);
        assert_eq!(adj.decided_at, round.decided_at);
        assert_eq!(adj.decision, round.decision);
        // Verify JSON has exactly the 6 expected keys
        let map: serde_json::Map<std::string::String, _> = serde_json::from_str(&json).unwrap();
        let keys: std::collections::HashSet<_> = map.keys().collect();
        assert_eq!(keys.len(), 6);
        assert!(keys.contains(&"id".to_string()));
        assert!(keys.contains(&"target_fused_claim_id".to_string()));
        assert!(keys.contains(&"adjudicator".to_string()));
        assert!(keys.contains(&"evidence_refs".to_string()));
        assert!(keys.contains(&"decided_at".to_string()));
        assert!(keys.contains(&"decision".to_string()));
    }

    // SCN-T08-001b ───────────────────────────────────────────────────────────

    #[test]
    fn adjudication_decision_serialises_to_snake_case_promote() {
        let json = serde_json::to_string(&AdjudicationDecision::Promote).unwrap();
        assert_eq!(json, "\"promote\"");
    }

    #[test]
    fn adjudication_decision_serialises_to_snake_case_reject() {
        let json = serde_json::to_string(&AdjudicationDecision::Reject).unwrap();
        assert_eq!(json, "\"reject\"");
    }

    #[test]
    fn adjudication_decision_serialises_to_snake_case_defer() {
        let json = serde_json::to_string(&AdjudicationDecision::Defer).unwrap();
        assert_eq!(json, "\"defer\"");
    }

    // SCN-T08-001c ───────────────────────────────────────────────────────────

    #[test]
    fn id_for_is_deterministic() {
        let id1 = id_for("clm:fused:abc", "tester", "2026-08-21T00:00:00Z");
        let id2 = id_for("clm:fused:abc", "tester", "2026-08-21T00:00:00Z");
        assert_eq!(id1, id2);
    }

    #[test]
    fn id_for_format_is_adj_prefix_and_64_hex_chars() {
        let id = id_for("clm:fused:abc", "tester", "2026-08-21T00:00:00Z");
        assert!(id.starts_with("adj:"));
        let hex = &id["adj:".len()..];
        assert_eq!(hex.len(), 64, "blake3 32-byte hex must be 64 chars");
        assert!(
            hex.chars().all(|c| c.is_ascii_hexdigit()),
            "hex chars must be lowercase ascii hex"
        );
    }
}
