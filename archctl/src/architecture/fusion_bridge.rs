//! Fusion bridge: trust-gated FusedClaim recompute seam.
//!
//! ADR-064 — Fusion Bounded Context: Trust-Gated FusedClaim Recompute +
//! Feedback/Reconciliation as First-Class Types.
//!
//! This module is the **single source of truth** for the FusedClaim.status
//! derivation. It is consumed by:
//!   - `architecture/fusion.rs::fuse_observations_with` (the recompute path)
//!   - `store.rs::FeedbackRepository::put_feedback` (the m30 bridge path)
//!
//! One function, two callers — eliminates connascence-of-algorithm smell
//! between FusedClaim recompute and Reconciliation derivation.

use crate::evidence::SourceOrigin as EvSourceOrigin;
use crate::observation_claim::Observation;
use crate::trust::{
    AuthorityClass, ExecutionClass, TrustClassification, canonical_promotion_allowed, classify,
};

/// Trust-gated FusedClaim.status derivation.
///
/// Consults `trust::canonical_promotion_allowed` before stamping
/// `FusedClaim.status`. The function is pure (no I/O).
///
/// # Rules (priority order, per spec-35 v1.1 §6)
///
/// 1. **Trust gate first**: classify via `trust::classify(origin, tool_name)`;
///    if `canonical_promotion_allowed(exec, authority)` is denied, status
///    derives from the trust verdict (never from Feedback).
/// 2. **For ModelInference × Suggested**: status = `"drafted"` (never silently
///    promoted to `"accepted"`). `pending_adjudication_event = false` until
///    a `Feedback.verdict=accept` arrives (then it's set to true and the
///    m30 bridge emits `tracing::warn!`).
/// 3. **For all other green cells**: status = `"accepted"`.
///
/// # Arguments
///
/// * `group` — slice of Observations for one fused claim group
/// * `source_origin` — SourceOrigin of the first Observation (sufficient;
///   trust gate is per-claim, not per-group)
///
/// # Returns
///
/// `(&str, TrustClassification)` — the status string and the trust classification
/// for use by callers that need the classification (e.g. m30 bridge).
pub fn recompute_status(
    group: &[&Observation],
    source_origin: EvSourceOrigin,
) -> (String, TrustClassification) {
    // Classify from the first observation's origin. All observations in a
    // fused group share the same provenance, so one classification is sufficient.
    let tool_name = group.first().map(|o| o.tool_name.as_str());
    let trust = classify(source_origin, tool_name);

    // Trust gate: if canonical promotion is denied, status = "drafted"
    if canonical_promotion_allowed(trust.execution, trust.authority).is_err() {
        let status = match trust.execution {
            ExecutionClass::ModelInference => "drafted",
            _ => "drafted",
        };
        return (status.to_string(), trust);
    }

    // Green cell: status = "accepted"
    ("accepted".to_string(), trust)
}

/// Pending-adjudication flag check: returns true if a Feedback.verdict=accept
/// has landed on a ModelInference FusedClaim but the m30 event store is
/// not yet wired.
///
/// # Arguments
///
/// * `trust` — the pre-computed TrustClassification of the target FusedClaim
/// * `feedback_verdict` — the verdict of the newly-arrived Feedback
///
/// # Returns
///
/// `true` if the m30 bridge should emit `tracing::warn!` instead of
/// silently promoting the FusedClaim.
#[deprecated(
    since = "1.87.0",
    note = "m30 bridge is now a hard fail (TRUST-008 / REQ-M25-006); use \
            `promotion_requires_adjudication_event` which returns \
            `Result<(), TrustViolation>` and is consulted by \
            `FeedbackRepository::put_feedback`."
)]
pub fn should_warn_pending_adjudication(
    trust: TrustClassification,
    feedback_verdict: crate::feedback::FeedbackVerdict,
) -> bool {
    use crate::feedback::FeedbackVerdict;
    trust.execution == ExecutionClass::ModelInference
        && trust.authority == AuthorityClass::Suggested
        && feedback_verdict == FeedbackVerdict::Accept
}

/// TRUST-008 (REQ-M25-006 closure): the m30 bridge predicate. Returns
/// `Ok(())` for combinations that do NOT require an Adjudication event,
/// and `Err(TrustViolation::ModelInferenceWithoutAdjudicationEvent)` for
/// the one combination that does (`ModelInference × Suggested + Accept`).
///
/// The chokepoint at `FeedbackRepository::put_feedback` consults the
/// `AdjudicationRepository` to decide whether to swallow the `Err`
/// (Promote event present ⇒ swallow) or propagate it (no Promote ⇒ `Err`).
///
/// Pure function (no I/O). Mirrors the shape of `canonical_promotion_allowed`.
pub fn promotion_requires_adjudication_event(
    trust: TrustClassification,
    feedback_verdict: crate::feedback::FeedbackVerdict,
) -> Result<(), crate::trust::TrustViolation> {
    use crate::feedback::FeedbackVerdict;
    if trust.execution == ExecutionClass::ModelInference
        && trust.authority == AuthorityClass::Suggested
        && feedback_verdict == FeedbackVerdict::Accept
    {
        return Err(crate::trust::TrustViolation::ModelInferenceWithoutAdjudicationEvent);
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::SourceOrigin;
    use crate::observation_claim::Observation;

    fn make_obs(origin: SourceOrigin, tool: &str) -> Observation {
        Observation {
            id: "obs:test".to_string(),
            kind: "structural".to_string(),
            claim: "test claim".to_string(),
            path: "src/lib.rs".to_string(),
            start_line: 1,
            end_line: 1,
            tool_name: tool.to_string(),
            tool_version: "0.1".to_string(),
            rule_id: "test:rule".to_string(),
            content_hash: "sha256:test".to_string(),
            observed_at: "2026-08-21T00:00:00Z".to_string(),
            evidence_origin: origin.as_str().to_string(),
            confidence: 0.0,
            status: crate::observation_claim::ObservationStatus::Drafted,
            written_via_backfill: false,
        }
    }

    #[test]
    fn recompute_status_trust_gated_drafted_for_model_inference() {
        let obs = make_obs(SourceOrigin::ModelInference, "test-tool");
        let (status, trust) = recompute_status(&[&obs], SourceOrigin::ModelInference);
        assert_eq!(status, "drafted");
        assert_eq!(trust.execution, ExecutionClass::ModelInference);
    }

    #[test]
    fn recompute_status_trust_gated_accepted_for_human_decision() {
        // UserWorkspace with an unknown tool defaults to PureDeterministic (green cell).
        // The status should be "accepted" since canonical_promotion_allowed passes.
        let obs = make_obs(SourceOrigin::UserWorkspace, "test-tool");
        let (status, trust) = recompute_status(&[&obs], SourceOrigin::UserWorkspace);
        assert_eq!(status, "accepted");
        assert_eq!(trust.execution, ExecutionClass::PureDeterministic);
    }

    /// TRUST-005 PR3a: green cell explicit check (PureDeterministic × Observed → "accepted").
    #[test]
    fn recompute_status_trust_gated_accepted_for_pure_deterministic_observed() {
        // UserWorkspace + unknown tool = PureDeterministic × Observed (green cell).
        // canonical_promotion_allowed(PureDeterministic, Observed) = Ok(()).
        let obs = make_obs(SourceOrigin::UserWorkspace, "unknown-tool");
        let (status, trust) = recompute_status(&[&obs], SourceOrigin::UserWorkspace);
        assert_eq!(status, "accepted");
        assert_eq!(trust.execution, ExecutionClass::PureDeterministic);
        assert_eq!(trust.authority, AuthorityClass::Observed);
    }

    /// TRUST-005 PR3a: ModelInference execution is blocked by the trust gate.
    #[test]
    fn recompute_status_blocks_model_inference_to_observed_with_trust_violation() {
        let obs = make_obs(SourceOrigin::ModelInference, "some-other-tool");
        let (status, trust) = recompute_status(&[&obs], SourceOrigin::ModelInference);
        assert_eq!(status, "drafted");
        assert_eq!(trust.execution, ExecutionClass::ModelInference);
    }

    /// TRUST-005 PR3a: m30 bridge — ModelInference × Suggested + Accept must warn.
    #[allow(deprecated)]
    #[test]
    fn recompute_status_sets_pending_adjudication_event_on_feedback_accept_for_model_inference() {
        let trust = TrustClassification {
            execution: ExecutionClass::ModelInference,
            authority: AuthorityClass::Suggested,
        };
        let verdict = crate::feedback::FeedbackVerdict::Accept;
        assert!(
            should_warn_pending_adjudication(trust, verdict),
            "ModelInference × Suggested + Accept must trigger m30 bridge warning"
        );
        let human_trust = TrustClassification {
            execution: ExecutionClass::HumanDecision,
            authority: AuthorityClass::Normative,
        };
        assert!(
            !should_warn_pending_adjudication(human_trust, verdict),
            "HumanDecision must NOT trigger pending adjudication"
        );
        assert!(
            !should_warn_pending_adjudication(trust, crate::feedback::FeedbackVerdict::Reject),
            "ModelInference + Reject must NOT trigger pending adjudication"
        );
    }

    /// SCN-T08-008b: the one dangerous combo — ModelInference × Suggested + Accept
    /// returns Err.
    #[test]
    fn promotion_requires_adjudication_event_returns_err_for_model_inference_accept() {
        let trust = TrustClassification {
            execution: ExecutionClass::ModelInference,
            authority: AuthorityClass::Suggested,
        };
        let result =
            promotion_requires_adjudication_event(trust, crate::feedback::FeedbackVerdict::Accept);
        assert!(matches!(
            result,
            Err(crate::trust::TrustViolation::ModelInferenceWithoutAdjudicationEvent)
        ));
    }

    /// SCN-T08-008c: all non-promoting combos return Ok.
    #[test]
    fn promotion_requires_adjudication_event_returns_ok_for_non_promoting_combos() {
        let human = TrustClassification {
            execution: ExecutionClass::HumanDecision,
            authority: AuthorityClass::Normative,
        };
        let deterministic = TrustClassification {
            execution: ExecutionClass::PureDeterministic,
            authority: AuthorityClass::Observed,
        };
        let model = TrustClassification {
            execution: ExecutionClass::ModelInference,
            authority: AuthorityClass::Suggested,
        };
        assert!(
            promotion_requires_adjudication_event(human, crate::feedback::FeedbackVerdict::Accept)
                .is_ok(),
            "HumanDecision + Accept must be Ok"
        );
        assert!(
            promotion_requires_adjudication_event(
                deterministic,
                crate::feedback::FeedbackVerdict::Accept
            )
            .is_ok(),
            "PureDeterministic + Accept must be Ok"
        );
        assert!(
            promotion_requires_adjudication_event(model, crate::feedback::FeedbackVerdict::Reject)
                .is_ok(),
            "ModelInference + Reject must be Ok"
        );
    }
}
