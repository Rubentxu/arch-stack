//! Evaluation domain type.
//!
//! One evaluation of one evidence row against a criterion. Evaluation is
//! **optional** in B1 (D3) — `put_evidence` does NOT require one; the
//! adapter accepts `Option<&Evaluation>`. Future cycles add threshold gates
//! and user-acceptance workflows that create Evaluation rows.

use blake3::Hasher;
use serde::{Deserialize, Serialize};

use crate::clock::Clock;

/// One evaluation of one evidence row against a criterion.
///
/// Maps to the `Evaluation` node table in
/// `docs/schema/002_source_evaluation.cypher`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    /// `"eval:" + blake3(criterion + target_evidence_id + evaluated_at)[..16]`
    pub id: String,
    /// The evidence row this evaluation belongs to.
    pub target_evidence_id: String,
    /// The criterion name: `"min_occurrence"` | `"user_accepted"` | …
    pub criterion: String,
    /// `true` for accept, `false` for reject.
    pub passed: bool,
    /// Who/what performed the evaluation.
    /// `"archctl:threshold_v1"` | `"human:<id>"` | …
    pub evaluator: String,
    /// RFC3339 timestamp from the injected [`Clock`].
    pub evaluated_at: String,
    /// Extra fields. `criterion_params`, `observed_value`, and `notes`
    /// are set by the caller via `props`.
    pub props: serde_json::Map<String, serde_json::Value>,
}

impl Evaluation {
    /// Create a passing evaluation.
    ///
    /// `id` is derived from `blake3(criterion + target_evidence_id + evaluated_at)`.
    /// `evaluated_at` is sourced from `clock.now_rfc3339()`.
    pub fn accept(
        target_evidence_id: &str,
        criterion: &str,
        evaluator: &str,
        clock: &dyn Clock,
    ) -> Self {
        Self::new(target_evidence_id, criterion, true, evaluator, clock)
    }

    /// Create a failing evaluation.
    ///
    /// `id` is derived from `blake3(criterion + target_evidence_id + evaluated_at)`.
    /// `evaluated_at` is sourced from `clock.now_rfc3339()`.
    pub fn reject(
        target_evidence_id: &str,
        criterion: &str,
        evaluator: &str,
        clock: &dyn Clock,
    ) -> Self {
        Self::new(target_evidence_id, criterion, false, evaluator, clock)
    }

    fn new(
        target_evidence_id: &str,
        criterion: &str,
        passed: bool,
        evaluator: &str,
        clock: &dyn Clock,
    ) -> Self {
        let evaluated_at = clock.now_rfc3339();
        let id = Self::id_for(criterion, target_evidence_id, &evaluated_at);
        Self {
            id,
            target_evidence_id: target_evidence_id.to_string(),
            criterion: criterion.to_string(),
            passed,
            evaluator: evaluator.to_string(),
            evaluated_at,
            props: serde_json::Map::new(),
        }
    }

    /// Derive the stable id from criterion + evidence_id + timestamp.
    /// Even for the same inputs, two calls at different times produce
    /// different ids (because `evaluated_at` differs).
    pub fn id_for(criterion: &str, target_evidence_id: &str, evaluated_at: &str) -> String {
        let mut h = Hasher::new();
        h.update(criterion.as_bytes());
        h.update(target_evidence_id.as_bytes());
        h.update(evaluated_at.as_bytes());
        format!("eval:{}", hex::encode(&h.finalize().as_bytes()[..16]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::FixedClock;

    #[test]
    fn evaluation_accept_sets_passed_true_and_stamps_clock() {
        let clock = FixedClock::new("2026-07-30T12:00:00Z");
        let eval = Evaluation::accept(
            "ev:abcd1234",
            "min_occurrence",
            "archctl:threshold_v1",
            &clock,
        );
        assert!(eval.passed);
        assert_eq!(eval.criterion, "min_occurrence");
        assert_eq!(eval.target_evidence_id, "ev:abcd1234");
        assert_eq!(eval.evaluator, "archctl:threshold_v1");
        assert_eq!(eval.evaluated_at, "2026-07-30T12:00:00Z");
        assert!(
            eval.id.starts_with("eval:"),
            "id must use eval: prefix"
        );
    }

    #[test]
    fn evaluation_reject_sets_passed_false() {
        let clock = FixedClock::new("2026-07-30T12:00:00Z");
        let eval = Evaluation::reject(
            "ev:abcd1234",
            "min_confidence",
            "human:alice",
            &clock,
        );
        assert!(!eval.passed);
        assert_eq!(eval.criterion, "min_confidence");
    }

    #[test]
    fn evaluation_id_is_deterministic() {
        let id = Evaluation::id_for(
            "min_occurrence",
            "ev:abcd1234",
            "2026-07-30T12:00:00Z",
        );
        let id2 = Evaluation::id_for(
            "min_occurrence",
            "ev:abcd1234",
            "2026-07-30T12:00:00Z",
        );
        assert_eq!(id, id2, "same inputs must produce same id");
        assert_ne!(
            Evaluation::id_for("min_confidence", "ev:abcd1234", "2026-07-30T12:00:00Z"),
            id,
            "different criterion must produce different id"
        );
    }
}
