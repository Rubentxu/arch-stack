//! Test-only mocks for cognitive agents.
//!
//! Exposed via the `test-fixtures` Cargo feature so integration tests can
//! construct deterministic agents without touching production logic.
//!
//! Used by:
//! - `archctl/tests/uat_06_false_agent_claim.rs` (TRUST-006 step 16)
//!
//! Spec: REQ-T06-005, REQ-T06-006 (TRUST-006 design §3.2).

#![cfg(any(test, feature = "test-fixtures"))]

use crate::cognitive::context::AgentContext;
use crate::feedback::FeedbackVerdict;

/// Result of a [`FeedbackAwareMockAgent::invoke`] call.
///
/// In production this maps to [`crate::cognitive::output::AgentOutput`],
/// but the mock uses a slim local enum so test code does not depend on
/// the full AgentOutput surface.
#[derive(Debug, Clone, PartialEq)]
pub enum MockOutcome {
    NoAction,
    FindingCandidate(String),
}

/// Deterministic agent that emits a `FindingCandidate` for the given
/// claim id UNLESS its `feedback_history` contains a `Reject` for that
/// claim. This mirrors the trust-first invariant: agents respect prior
/// rejections and do not re-propose them.
///
/// Construction: unit struct — no state.
///
/// Invocation: pass an `AgentContext` with `feedback_history` populated
/// and the `claim_id` the agent is asked to evaluate.
#[derive(Debug, Clone, Copy, Default)]
pub struct FeedbackAwareMockAgent;

impl FeedbackAwareMockAgent {
    /// Invoke the agent. Returns `NoAction` if `feedback_history` contains
    /// a `Reject` for `claim_id`; otherwise returns `FindingCandidate(claim_id)`.
    pub fn invoke(&self, ctx: &AgentContext, claim_id: &str) -> MockOutcome {
        let rejected = ctx
            .feedback_history
            .iter()
            .any(|fb| fb.target == claim_id && fb.verdict == FeedbackVerdict::Reject);
        if rejected {
            MockOutcome::NoAction
        } else {
            MockOutcome::FindingCandidate(claim_id.to_string())
        }
    }
}
