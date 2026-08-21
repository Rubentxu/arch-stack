//! Reconciliation bounded context.
//!
//! ADR-064 — Fusion Bounded Context: Trust-Gated FusedClaim Recompute +
//! Feedback/Reconciliation as First-Class Types.
//!
//! Per spec-35 v1.1 §3: Reconciliation is a graph-native record deriving
//! the `computed_status` of a target `FusedClaim` from the union of
//! (a) its underlying `Evidence` set and (b) the `Feedback` history
//! targeting it. Persisted as `(:Reconciliation)` node with typed edge
//! `(:Reconciliation)-[:RECONCILES]->(:FusedClaim)`.
//!
//! Single-responsibility: this module owns the pure `Reconciliation::compute()`
//! function. The consumer-facing API only ever sees a `Reconciliation` row
//! written to disk. The split from `feedback.rs` makes the
//! connascence-of-algorithm smell visible and lets `fusion_bridge.rs`
//! depend on `compute()` directly without depending on `feedback.rs`.

use crate::feedback::{Feedback, FeedbackVerdict};
#[allow(unused_imports)]
use crate::trust::{
    AuthorityClass, ExecutionClass, TrustClassification, canonical_promotion_allowed,
};
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Id types
// ─────────────────────────────────────────────────────────────────────────────

/// Namespaced id: `recon:<blake3(assertion_id + revision)>`.
pub type ReconciliationId = String;

/// Namespaced id for the assertion this reconciliation resolves.
pub type AssertionId = String;

/// Subject of the asserted fact.
pub type SubjectId = String;

/// Predicate kind (e.g. "uses", "calls", "depends_on").
pub type PredicateKind = String;

/// Object of the asserted fact.
pub type ObjectId = String;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Forward-compat for spec-35 v1.2: multi-plane reconciliation.
/// v1.1 ships with `planes.len() == 1` always (single static-analysis plane).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaneEvidence {
    /// Plane identifier (e.g. `"static-analysis"`).
    pub plane_id: String,
    /// Evidence ids in this plane.
    pub evidence_refs: Vec<String>,
}

/// A graph-native record computing the `computed_status` of a target `FusedClaim`.
///
/// Persisted as `(:Reconciliation)` node with typed edge
/// `(:Reconciliation)-[:RECONCILES]->(:FusedClaim)`.
///
/// The `computed_status` is derived by [`Reconciliation::compute`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reconciliation {
    /// Namespaced id: `recon:<blake3(assertion_id + revision)>`.
    pub id: ReconciliationId,

    /// The assertion this reconciliation resolves.
    pub assertion_id: AssertionId,

    /// Subject of the asserted fact.
    pub subject: SubjectId,

    /// Predicate kind.
    pub predicate: PredicateKind,

    /// Object of the asserted fact.
    pub object: ObjectId,

    /// Flat list of evidence ids (v1.1; `planes` reserved for v1.2).
    pub evidence_set: Vec<String>,

    /// Reserved for v1.2 multi-plane. Always `len() == 1` in v1.1.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub planes: Vec<PlaneEvidence>,

    /// Derived status: `"drafted"` | `"accepted"` | `"rejected"` | `"superseded"` | `"pending_adjudication"`.
    pub computed_status: String,

    /// Human-readable rationale citing trust classification + most-recent Feedback.
    pub rationale: String,

    /// Graph revision at time of computation.
    pub revision: GraphRevision,
}

/// Graph revision id (mirrors feedback.rs for consistency).
pub type GraphRevision = String;

/// Derive a computed_status string from a TrustClassification.
/// Used by the reconciliation compute path and by fusion_bridge.
/// The name is pinned in architecture.toml.
pub fn reconciliation_status(trust: TrustClassification) -> &'static str {
    use crate::trust::ExecutionClass;
    if canonical_promotion_allowed(trust.execution, trust.authority).is_ok() {
        "accepted"
    } else {
        match trust.execution {
            ExecutionClass::ModelInference => "pending_adjudication",
            _ => "drafted",
        }
    }
}

impl Reconciliation {
    /// Pure function: given identical inputs, returns identical output.
    ///
    /// Sort order: `feedback.id ASC`, then `feedback.revision ASC`,
    /// then `feedback.timestamp ASC`. Empty Feedback history produces a
    /// derived-only result based on `trust::canonical_promotion_allowed(exec, authority)`.
    ///
    /// # Computed status priority rule (spec-35 v1.1 §6)
    ///
    /// 1. **Trust gate first**: classify via `trust::classify(origin, tool_name)`;
    ///    if `canonical_promotion_allowed(exec, authority)` is denied, `computed_status`
    ///    derives from the trust classification, NOT from Feedback.
    /// 2. **For ModelInference × Suggested**: `computed_status = "drafted"`. If a
    ///    `Feedback.verdict=accept` has arrived, `computed_status = "pending_adjudication"`
    ///    (m30 bridge not yet wired; rationale cites the bridge).
    /// 3. **For green cells with Feedback history**: most-recent Feedback verdict wins.
    ///    - `Accept → "accepted"`
    ///    - `Reject → "rejected"`
    ///    - `Supersede/Correct → "superseded"` (carries replacement in rationale)
    ///    - `Uncertain → trust-gated default`
    ///
    /// # Arguments
    ///
    /// * `assertion_id` — assertion this reconciliation resolves
    /// * `subject`, `predicate`, `object` — the SPOC triple
    /// * `evidence_set` — flat list of evidence ids (v1.1)
    /// * `feedback_history` — sorted by (id, revision, timestamp) ASC
    /// * `revision` — graph revision at computation time
    /// * `trust` — pre-classified trust tuple for the target FusedClaim
    ///
    /// # Returns
    ///
    /// A `Reconciliation` with deterministic `computed_status` and `rationale`.
    #[allow(clippy::manual_flatten)]
    #[allow(clippy::too_many_arguments)]
    pub fn compute(
        assertion_id: AssertionId,
        subject: SubjectId,
        predicate: PredicateKind,
        object: ObjectId,
        evidence_set: Vec<String>,
        feedback_history: &[Feedback],
        revision: GraphRevision,
        trust: TrustClassification,
    ) -> Reconciliation {
        // Sort: (feedback.id ASC, revision ASC, timestamp ASC).
        // The sort is done on a copy so the input slice is not mutated.
        let mut sorted: Vec<&Feedback> = feedback_history.iter().collect();
        sorted.sort_by_key(|f| (&f.id, &f.revision, &f.timestamp));

        let computed_status;
        let rationale;

        // Rule 1: trust gate — if canonical promotion is denied, derive from trust
        if canonical_promotion_allowed(trust.execution, trust.authority).is_err() {
            let status_for_denied = match trust.execution {
                ExecutionClass::ModelInference => {
                    // Check if there's a Feedback.accept in history
                    let has_accept = sorted.iter().any(|f| f.verdict == FeedbackVerdict::Accept);
                    if has_accept {
                        "pending_adjudication".to_string()
                    } else {
                        "drafted".to_string()
                    }
                }
                _ => "drafted".to_string(),
            };
            computed_status = status_for_denied;
            rationale = format!(
                "trust gate denied ({:?} × {:?}): {}",
                trust.execution, trust.authority, computed_status
            );
        } else if sorted.is_empty() {
            // Rule 2a: green cell, no feedback → accepted
            computed_status = "accepted".to_string();
            rationale = format!(
                "green cell ({:?} × {:?}), no feedback history",
                trust.execution, trust.authority
            );
        } else {
            // Rule 2b: green cell with feedback history — most-recent verdict wins
            let last = sorted.last().expect("non-empty history");
            match last.verdict {
                FeedbackVerdict::Accept => {
                    computed_status = "accepted".to_string();
                    rationale = format!(
                        "Feedback.accept from {:?} at {}",
                        last.actor, last.timestamp
                    );
                }
                FeedbackVerdict::Reject => {
                    computed_status = "rejected".to_string();
                    let replacement_note = last
                        .replacement
                        .as_ref()
                        .map(|r| format!("; replacement: {}", r))
                        .unwrap_or_default();
                    rationale = format!(
                        "Feedback.reject from {:?} at {}{}",
                        last.actor, last.timestamp, replacement_note
                    );
                }
                FeedbackVerdict::Supersede => {
                    computed_status = "superseded".to_string();
                    let replacement_note = last
                        .replacement
                        .as_ref()
                        .map(|r| format!("; replacement: {}", r))
                        .unwrap_or_default();
                    rationale = format!(
                        "Feedback.supersede from {:?} at {}{}",
                        last.actor, last.timestamp, replacement_note
                    );
                }
                FeedbackVerdict::Correct => {
                    computed_status = "superseded".to_string();
                    rationale = format!(
                        "Feedback.correct from {:?} at {}",
                        last.actor, last.timestamp
                    );
                }
                FeedbackVerdict::Uncertain => {
                    // Uncertain: fall back to trust-gated default
                    computed_status = "drafted".to_string();
                    rationale = format!(
                        "Feedback.uncertain from {:?} at {}; trust-gated default applied",
                        last.actor, last.timestamp
                    );
                }
            }
        }

        Reconciliation {
            id: format!(
                "recon:{}",
                blake3::hash(format!("{}+{}", assertion_id, revision).as_bytes()).to_hex()
            ),
            assertion_id,
            subject,
            predicate,
            object,
            evidence_set,
            planes: Vec::new(), // v1.1: always empty
            computed_status,
            rationale,
            revision,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feedback::FeedbackVerdict;

    fn make_feedback(verdict: FeedbackVerdict, actor: &str, revision: &str) -> Feedback {
        Feedback {
            id: format!("fdbk:{}", actor),
            target: "clm:fused:abc".to_string(),
            verdict,
            replacement: None,
            actor: actor.to_string(),
            revision: revision.to_string(),
            timestamp: "2026-08-21T00:00:00Z".to_string(),
            evidence: None,
            correlation_id: None,
        }
    }

    fn green_trust() -> TrustClassification {
        TrustClassification {
            execution: ExecutionClass::HumanDecision,
            authority: AuthorityClass::Normative,
        }
    }

    fn model_inference_trust() -> TrustClassification {
        TrustClassification {
            execution: ExecutionClass::ModelInference,
            authority: AuthorityClass::Suggested,
        }
    }

    #[test]
    fn reconciliation_is_deterministic_for_identical_inputs() {
        let trust = green_trust();
        let fb = make_feedback(FeedbackVerdict::Accept, "alice", "rev1");

        let r1 = Reconciliation::compute(
            "a1".to_string(),
            "Subject".to_string(),
            "uses".to_string(),
            "Object".to_string(),
            vec!["e1".to_string()],
            std::slice::from_ref(&fb),
            "rev1".to_string(),
            trust,
        );
        let r2 = Reconciliation::compute(
            "a1".to_string(),
            "Subject".to_string(),
            "uses".to_string(),
            "Object".to_string(),
            vec!["e1".to_string()],
            std::slice::from_ref(&fb),
            "rev1".to_string(),
            trust,
        );

        assert_eq!(r1.computed_status, r2.computed_status);
        assert_eq!(r1.rationale, r2.rationale);
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn reconciliation_order_independent_on_feedback_history() {
        let trust = green_trust();
        // Two feedbacks in different order should produce the same result
        // because we always take the last one by (id, revision, timestamp)
        let fb_a = make_feedback(FeedbackVerdict::Reject, "alice", "rev1");
        let fb_b = make_feedback(FeedbackVerdict::Accept, "bob", "rev1");

        let r_forward = Reconciliation::compute(
            "a1".to_string(),
            "Subject".to_string(),
            "uses".to_string(),
            "Object".to_string(),
            vec!["e1".to_string()],
            &[fb_a.clone(), fb_b.clone()],
            "rev1".to_string(),
            trust,
        );

        // Same feedbacks, reversed order — last (bob's Accept) should still win
        let r_reversed = Reconciliation::compute(
            "a1".to_string(),
            "Subject".to_string(),
            "uses".to_string(),
            "Object".to_string(),
            vec!["e1".to_string()],
            &[fb_b, fb_a],
            "rev1".to_string(),
            trust,
        );

        assert_eq!(r_forward.computed_status, r_reversed.computed_status);
        assert_eq!(r_forward.computed_status, "accepted"); // bob's Accept wins
    }

    #[test]
    fn reconciliation_computed_status_respects_trust_gate() {
        // ModelInference × Suggested: even with Accept feedback, should be pending_adjudication
        let trust = model_inference_trust();
        let fb = make_feedback(FeedbackVerdict::Accept, "alice", "rev1");

        let r = Reconciliation::compute(
            "a1".to_string(),
            "Subject".to_string(),
            "uses".to_string(),
            "Object".to_string(),
            vec!["e1".to_string()],
            &[fb],
            "rev1".to_string(),
            trust,
        );

        assert_eq!(r.computed_status, "pending_adjudication");
        assert!(r.rationale.contains("trust gate denied"));
    }
}
