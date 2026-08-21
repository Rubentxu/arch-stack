//! Trust: classification and canonical-write policy.
//!
//! ADR-063 + ADR-P02 + ADR-P03.
//!
//! This module is the SINGLE source of truth for the question
//! "may this evidence be promoted to canonical?". Two enums
//! ([`ExecutionClass`], [`AuthorityClass`]) span the axes that
//! ADR-P02 and ADR-P03 declared orthogonal. The producer
//! mapping in [`classify`] transcribes the table at
//! `docs/arch-stack-architecture-feedback-workbench-2026-08-20/architecture/12-TRUST-DETERMINISM-AND-AUTHORITY.md:16-24`.
//!
//! The canonical-write predicate [`canonical_write_allowed`] is the
//! single 2-input gate every transition to `EvidenceStatus::Accepted`
//! must pass. It is pure, exhaustive, and intentionally not
//! `const` (it may grow to consult ADR-022's per-agent determinism
//! catalog in a future cycle).
//!
//! **Independence from ADR-023**: `[`AuthorityClass::Adjudicated`]`
//! is intentionally distinct from ADR-023's `Approval`. The former
//! elevates a fact to canonical weight; the latter permits a side
//! effect on the world. They overlap in vocabulary but operate on
//! different objects.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// How a fact was produced. Cited from ADR-021 `Escalera de resolución`
/// L140-152 plus the rungs not present there (HumanDecision) added by
/// ADR-P03.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionClass {
    /// Byte-identical, deterministic: tree-sitter, SCIP, scc, blake3.
    /// ADR-021 step 1.
    PureDeterministic,
    /// Deterministic algorithm with a heuristic boundary (e.g. naming
    /// heuristics, SCC condensation). ADR-021 step 2.
    DeterministicHeuristic,
    /// Output of a model (Phi-3, Llama-3-8B, Claude, GPT). ADR-021
    /// steps 3+4. May NEVER mint canonical directly (invariant).
    ModelInference,
    /// Human provenance (UI click, ADR accept, HITL adjudication).
    /// ADR-021 step 5.
    HumanDecision,
}

/// How a fact earns its authority. Cited from ADR-P03.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityClass {
    /// The fact was observed directly (tree-sitter node, SCIP symbol).
    Observed,
    /// The fact was derived from observables (SCC condensation;
    /// call-graph edges from cross-reference).
    Derived,
    /// The fact is a suggestion from a heuristic or model. Not
    /// canonical until Promoted. The default for `ModelInference`.
    Suggested,
    /// The fact is normative (ADR accepted, lint rule). Always
    /// canonical; humans own it.
    Normative,
    /// The fact was adjudicated by an explicit human verdict. Distinct
    /// from ADR-023's `Approval` (see module doc).
    Adjudicated,
}

/// Returned by [`classify`]. Named struct (not 2-tuple) to eliminate
/// connascence of position (entropy lens: Position).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrustClassification {
    pub execution: ExecutionClass,
    pub authority: AuthorityClass,
}

/// Error returned by [`canonical_write_allowed`]. Variants enumerate
/// the *denied* (ExecutionClass, AuthorityClass) cell so the caller
/// can attribute the violation precisely.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TrustViolation {
    #[error("model inference cannot write canonical fact with authority {0:?}")]
    ModelInferenceCannotBe(AuthorityClass),

    #[error(
        "model inference produced authority Adjudicated but no Adjudication event accompanies the row"
    )]
    ModelInferenceWithoutAdjudicationEvent,

    #[error("deterministic producer cannot carry authority {0:?}")]
    DeterministicCannotBe(AuthorityClass),

    #[error("heuristic output is not an observation (must be Derived or Suggested)")]
    HeuristicWithoutObservation,

    #[error("evidence is already accepted; idempotent return")]
    AlreadyAccepted,

    #[error("evidence is Superseded; reinstate first")]
    SupersededStatus,
}

/// Classify a producer into a (Execution, Authority) pair.
///
/// Transcribes the 7-row table at `architecture/12-…:16-24` verbatim.
/// `tool_name` is the producer's declared name (e.g. `"tree-sitter"`,
/// `"scip"`, `"llm_analyst"`). Unknown `(origin, tool_name)` pairs
/// default to the same defaults as `EvidenceStatus::default_for_origin`
/// (UserWorkspace → PureDeterministic/Observed; others → HumanDecision/
/// Suggested) — but the caller is expected to surface this in tests.
pub fn classify(
    origin: crate::evidence::SourceOrigin,
    tool_name: Option<&str>,
) -> TrustClassification {
    use crate::evidence::SourceOrigin::*;
    let tool = tool_name.unwrap_or("");
    match (origin, tool) {
        // Tree-sitter / SCIP / SCC: deterministic extraction (workspace + tool)
        (UserWorkspace, "tree-sitter") | (ToolOutput, "scip") => TrustClassification {
            execution: ExecutionClass::PureDeterministic,
            authority: AuthorityClass::Observed,
        },
        (ToolOutput, "scc") => TrustClassification {
            execution: ExecutionClass::PureDeterministic,
            authority: AuthorityClass::Derived,
        },
        (ToolOutput, "naming_heuristic") | (ToolOutput, "tsg") => TrustClassification {
            execution: ExecutionClass::DeterministicHeuristic,
            authority: AuthorityClass::Suggested,
        },
        // LLM analyst (future model-backed writer)
        (ModelInference, _) | (_, "llm_analyst") => TrustClassification {
            execution: ExecutionClass::ModelInference,
            authority: AuthorityClass::Suggested,
        },
        // Human decision (ADR accepted, adjudication)
        (UserInput, "adr_accepted") => TrustClassification {
            execution: ExecutionClass::HumanDecision,
            authority: AuthorityClass::Normative,
        },
        (UserInput, "human_adjudication") => TrustClassification {
            execution: ExecutionClass::HumanDecision,
            authority: AuthorityClass::Adjudicated,
        },
        // Back-compat defaults (mirrors `Evaluation::default_for_origin`):
        // UserWorkspace without a known tool → assume deterministic observed.
        (UserWorkspace, _) => TrustClassification {
            execution: ExecutionClass::PureDeterministic,
            authority: AuthorityClass::Observed,
        },
        // Any other producer is treated as a human suggestion; safer than Normative.
        _ => TrustClassification {
            execution: ExecutionClass::HumanDecision,
            authority: AuthorityClass::Suggested,
        },
    }
}

/// The single authority gate. Returns `Ok(())` iff the `(exec, authority)`
/// cell is green in the 4×5 matrix below.
///
/// |                  | Observed | Derived | Suggested | Normative | Adjudicated |
/// |------------------|----------|---------|-----------|-----------|-------------|
/// | PureDeterministic| ✅       | ✅      | ❌        | ❌        | ❌          |
/// | DeterministicH…  | ❌       | ✅      | ✅        | ❌        | ❌          |
/// | ModelInference   | ❌       | ❌      | ✅        | ❌        | ❌*         |
/// | HumanDecision    | ✅       | ❌      | ✅        | ✅        | ✅          |
///
/// `*` `ModelInference × Adjudicated` is denied unless an explicit
/// Adjudication event accompanies the row (REQ-M25-006). This cycle
/// reserves the term; the event store is deferred.
pub fn canonical_write_allowed(
    exec: ExecutionClass,
    authority: AuthorityClass,
) -> Result<(), TrustViolation> {
    use AuthorityClass::*;
    use ExecutionClass::*;
    match (exec, authority) {
        // Green cells
        (PureDeterministic, Observed) | (PureDeterministic, Derived) => Ok(()),
        (DeterministicHeuristic, Derived) | (DeterministicHeuristic, Suggested) => Ok(()),
        (ModelInference, Suggested) => Ok(()),
        (HumanDecision, Observed)
        | (HumanDecision, Suggested)
        | (HumanDecision, Normative)
        | (HumanDecision, Adjudicated) => Ok(()),

        // ModelInference × Adjudicated: denied without event (future)
        (ModelInference, Adjudicated) => {
            Err(TrustViolation::ModelInferenceWithoutAdjudicationEvent)
        }

        // ModelInference × anything else
        (ModelInference, a) => Err(TrustViolation::ModelInferenceCannotBe(a)),

        // PureDeterministic × Suggested/Normative/Adjudicated
        (PureDeterministic, a) => Err(TrustViolation::DeterministicCannotBe(a)),

        // DeterministicHeuristic × Observed (not an observation)
        (DeterministicHeuristic, Observed) => Err(TrustViolation::HeuristicWithoutObservation),
        // DeterministicHeuristic × Normative/Adjudicated
        (DeterministicHeuristic, a) => Err(TrustViolation::DeterministicCannotBe(a)),

        // HumanDecision × Derived (humans don't derive; they state)
        (HumanDecision, Derived) => Err(TrustViolation::DeterministicCannotBe(Derived)),
    }
}

/// The **promotion gate**. Returns `Ok(())` iff `(exec, authority)` may
/// be promoted to `EvidenceStatus::Accepted` (= `CanonicalObservedFact`).
///
/// Stricter than [`canonical_write_allowed`]: while the matrix allows
/// `ModelInference × Suggested` for candidate visibility, this predicate
/// denies all `ModelInference × _` combinations because `ModelInference`
/// must never directly mint canonical. The only path for `ModelInference`
/// to reach canonical is `ModelInference × Adjudicated` with an explicit
/// Adjudication event (REQ-M25-006, deferred).
///
/// Single 2-input gate every transition to `EvidenceStatus::Accepted`
/// must pass at the chokepoint.
///
/// |                  | Observed | Derived | Suggested | Normative | Adjudicated |
/// |------------------|----------|---------|-----------|-----------|-------------|
/// | PureDeterministic| ✅       | ✅      | ❌        | ❌        | ❌          |
/// | DeterministicH…  | ❌       | ✅      | ✅        | ❌        | ❌          |
/// | ModelInference   | ❌       | ❌      | ❌        | ❌        | ❌*         |
/// | HumanDecision    | ✅       | ❌      | ✅        | ✅        | ✅          |
///
/// `*` `ModelInference × Adjudicated` is denied until REQ-M25-006 ships.
pub fn canonical_promotion_allowed(
    exec: ExecutionClass,
    authority: AuthorityClass,
) -> Result<(), TrustViolation> {
    if matches!(exec, ExecutionClass::ModelInference) {
        // The matrix entry for ModelInference × Suggested is for candidate
        // visibility only; promotion to Accepted is never direct.
        return Err(TrustViolation::ModelInferenceCannotBe(authority));
    }
    canonical_write_allowed(exec, authority)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::SourceOrigin;

    /// Mirrors `evidence.rs:541-546` — the wire strings are the contract
    /// the manifest gate probes. If these change, the producer table
    /// in `architecture/12-…:16-24` and ADR-063 must change in lockstep.
    #[test]
    fn execution_class_as_str_is_stable() {
        assert_eq!(
            serde_json::to_string(&ExecutionClass::PureDeterministic).unwrap(),
            "\"pure_deterministic\""
        );
        assert_eq!(
            serde_json::to_string(&ExecutionClass::DeterministicHeuristic).unwrap(),
            "\"deterministic_heuristic\""
        );
        assert_eq!(
            serde_json::to_string(&ExecutionClass::ModelInference).unwrap(),
            "\"model_inference\""
        );
        assert_eq!(
            serde_json::to_string(&ExecutionClass::HumanDecision).unwrap(),
            "\"human_decision\""
        );
    }

    #[test]
    fn authority_class_as_str_is_stable() {
        assert_eq!(
            serde_json::to_string(&AuthorityClass::Observed).unwrap(),
            "\"observed\""
        );
        assert_eq!(
            serde_json::to_string(&AuthorityClass::Derived).unwrap(),
            "\"derived\""
        );
        assert_eq!(
            serde_json::to_string(&AuthorityClass::Suggested).unwrap(),
            "\"suggested\""
        );
        assert_eq!(
            serde_json::to_string(&AuthorityClass::Normative).unwrap(),
            "\"normative\""
        );
        assert_eq!(
            serde_json::to_string(&AuthorityClass::Adjudicated).unwrap(),
            "\"adjudicated\""
        );
    }

    /// Table-driven transcription of `architecture/12-…:16-24`. If a
    /// row changes, this test fails — making doc drift a test failure.
    #[test]
    fn producer_mapping_matches_arch_doc() {
        let cases: &[(SourceOrigin, &str, ExecutionClass, AuthorityClass)] = &[
            (
                SourceOrigin::UserWorkspace,
                "tree-sitter",
                ExecutionClass::PureDeterministic,
                AuthorityClass::Observed,
            ),
            (
                SourceOrigin::ToolOutput,
                "scip",
                ExecutionClass::PureDeterministic,
                AuthorityClass::Observed,
            ),
            (
                SourceOrigin::ToolOutput,
                "scc",
                ExecutionClass::PureDeterministic,
                AuthorityClass::Derived,
            ),
            (
                SourceOrigin::ToolOutput,
                "naming_heuristic",
                ExecutionClass::DeterministicHeuristic,
                AuthorityClass::Suggested,
            ),
            (
                SourceOrigin::ModelInference,
                "llm_analyst",
                ExecutionClass::ModelInference,
                AuthorityClass::Suggested,
            ),
            (
                SourceOrigin::UserInput,
                "adr_accepted",
                ExecutionClass::HumanDecision,
                AuthorityClass::Normative,
            ),
            (
                SourceOrigin::UserInput,
                "human_adjudication",
                ExecutionClass::HumanDecision,
                AuthorityClass::Adjudicated,
            ),
        ];
        for (origin, tool, expected_exec, expected_auth) in cases {
            let got = classify(*origin, Some(tool));
            assert_eq!(
                got.execution, *expected_exec,
                "({origin:?}, {tool}): execution"
            );
            assert_eq!(
                got.authority, *expected_auth,
                "({origin:?}, {tool}): authority"
            );
        }
    }

    /// The invariant from `architecture/12-…:26-27`: ModelInference NEVER
    /// writes canonical directly. Matrix says only `Suggested` is allowed,
    /// and that's exactly what we want for *candidate* visibility (ADR-P02).
    #[test]
    fn model_inference_cannot_write_canonical() {
        for auth in [
            AuthorityClass::Observed,
            AuthorityClass::Derived,
            AuthorityClass::Normative,
        ] {
            assert!(
                matches!(
                    canonical_write_allowed(ExecutionClass::ModelInference, auth),
                    Err(TrustViolation::ModelInferenceCannotBe(_))
                ),
                "ModelInference × {auth:?} must be denied"
            );
        }
        // Suggested is the only green cell for ModelInference.
        assert!(
            canonical_write_allowed(ExecutionClass::ModelInference, AuthorityClass::Suggested)
                .is_ok()
        );
        // Adjudicated is denied without an event (deferred to REQ-M25-006).
        assert!(matches!(
            canonical_write_allowed(ExecutionClass::ModelInference, AuthorityClass::Adjudicated),
            Err(TrustViolation::ModelInferenceWithoutAdjudicationEvent)
        ));
    }

    /// Orthogonality proof required by ADR-P03 L16: deterministic ≠
    /// authoritative. A heuristic may carry a `Suggested` class even
    /// though it is not deterministic.
    #[test]
    fn deterministic_heuristic_can_be_only_suggested() {
        assert!(
            canonical_write_allowed(
                ExecutionClass::DeterministicHeuristic,
                AuthorityClass::Suggested
            )
            .is_ok()
        );
        assert!(
            canonical_write_allowed(
                ExecutionClass::DeterministicHeuristic,
                AuthorityClass::Derived
            )
            .is_ok()
        );
        assert!(matches!(
            canonical_write_allowed(
                ExecutionClass::DeterministicHeuristic,
                AuthorityClass::Observed
            ),
            Err(TrustViolation::HeuristicWithoutObservation)
        ));
    }

    /// Other half of orthogonality: human ≠ deterministic. A human
    /// can carry `Normative` (ADR accepted) without being deterministic.
    #[test]
    fn human_decision_can_be_normative_without_determinism() {
        assert!(
            canonical_write_allowed(ExecutionClass::HumanDecision, AuthorityClass::Normative)
                .is_ok()
        );
        assert!(
            canonical_write_allowed(ExecutionClass::HumanDecision, AuthorityClass::Adjudicated)
                .is_ok()
        );
        assert!(
            canonical_write_allowed(ExecutionClass::HumanDecision, AuthorityClass::Suggested)
                .is_ok()
        );
        assert!(
            canonical_write_allowed(ExecutionClass::HumanDecision, AuthorityClass::Observed)
                .is_ok()
        );
    }

    /// `parse_label` round-trip via serde_json (the manifest gate uses
    /// string literals, not enum names).
    #[test]
    fn parse_label_round_trips() {
        for e in [
            ExecutionClass::PureDeterministic,
            ExecutionClass::DeterministicHeuristic,
            ExecutionClass::ModelInference,
            ExecutionClass::HumanDecision,
        ] {
            let s = serde_json::to_string(&e).unwrap();
            let back: ExecutionClass = serde_json::from_str(&s).unwrap();
            assert_eq!(e, back);
        }
        for a in [
            AuthorityClass::Observed,
            AuthorityClass::Derived,
            AuthorityClass::Suggested,
            AuthorityClass::Normative,
            AuthorityClass::Adjudicated,
        ] {
            let s = serde_json::to_string(&a).unwrap();
            let back: AuthorityClass = serde_json::from_str(&s).unwrap();
            assert_eq!(a, back);
        }
    }

    /// Negative-control exhaustiveness: PureDeterministic × Suggested is
    /// the canonical example of "looks fine but is structurally wrong".
    /// This is the test that pins the matrix.
    #[test]
    fn canonical_write_allowed_for_pure_deterministic_cannot_be_suggested() {
        let cases = [
            (ExecutionClass::PureDeterministic, AuthorityClass::Suggested),
            (ExecutionClass::PureDeterministic, AuthorityClass::Normative),
            (
                ExecutionClass::PureDeterministic,
                AuthorityClass::Adjudicated,
            ),
        ];
        for (e, a) in cases {
            assert!(
                matches!(
                    canonical_write_allowed(e, a),
                    Err(TrustViolation::DeterministicCannotBe(_))
                ),
                "PureDeterministic × {a:?} must be denied"
            );
        }
    }

    /// The matrix allows `ModelInference × Suggested` (candidate visibility).
    /// But promotion to `Accepted` is never direct — the promotion gate denies
    /// all `ModelInference × _` combinations.
    #[test]
    fn model_inference_x_suggested_is_promotion_denied_even_though_matrix_allows() {
        // The matrix allows this combination (candidate visibility).
        assert_eq!(
            canonical_write_allowed(ExecutionClass::ModelInference, AuthorityClass::Suggested),
            Ok(())
        );
        // But promotion is denied (direct canonical write forbidden).
        assert_eq!(
            canonical_promotion_allowed(ExecutionClass::ModelInference, AuthorityClass::Suggested),
            Err(TrustViolation::ModelInferenceCannotBe(
                AuthorityClass::Suggested
            ))
        );
    }

    #[test]
    fn human_decision_x_normative_promotion_allowed() {
        assert_eq!(
            canonical_promotion_allowed(ExecutionClass::HumanDecision, AuthorityClass::Normative),
            Ok(())
        );
    }

    /// TRUST-008 REQ-T08-008b: ModelInference × Suggested + Accept
    /// requires an Adjudication event (the m30 bridge predicate returns
    /// Err for the one dangerous combination).
    #[test]
    fn model_inference_x_suggested_x_accept_requires_adjudication_event() {
        use crate::architecture::fusion_bridge::promotion_requires_adjudication_event;
        let trust = TrustClassification {
            execution: ExecutionClass::ModelInference,
            authority: AuthorityClass::Suggested,
        };
        let result =
            promotion_requires_adjudication_event(trust, crate::feedback::FeedbackVerdict::Accept);
        assert!(
            matches!(
                result,
                Err(TrustViolation::ModelInferenceWithoutAdjudicationEvent)
            ),
            "ModelInference × Suggested + Accept must require Adjudication event; got {result:?}"
        );
        // The promotion gate remains closed at the type system.
        assert!(
            canonical_promotion_allowed(ExecutionClass::ModelInference, AuthorityClass::Suggested)
                .is_err()
        );
    }

    /// TRUST-008 REQ-T08-008c: ModelInference × Suggested + Reject is NOT
    /// a promotion — the bridge predicate returns Ok(()).
    #[test]
    fn model_inference_x_suggested_x_reject_is_not_a_promotion() {
        use crate::architecture::fusion_bridge::promotion_requires_adjudication_event;
        let trust = TrustClassification {
            execution: ExecutionClass::ModelInference,
            authority: AuthorityClass::Suggested,
        };
        let result =
            promotion_requires_adjudication_event(trust, crate::feedback::FeedbackVerdict::Reject);
        assert!(
            result.is_ok(),
            "ModelInference × Suggested + Reject is not a promotion; bridge must allow; got {result:?}"
        );
    }
}
