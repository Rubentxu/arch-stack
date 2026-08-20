//! Integration tests for UAT-06: false agent claim cannot become canonical fact.
//!
//! ADR-063. Verifies that `SourceOrigin::ModelInference` evidence cannot be
//! promoted to canonical via `accept_evidence` — the trust guard chokepoint
//! must block it.
//!
//! The scenario (examples/uat-06-false-agent-claim.yaml step 11):
//!   - Agent makes a false claim (Orders->Stripe)
//!   - Human rejects it (feedback verdict: reject)
//!   - critical_gate: false_canonical_promotions: 0
//!
//! Strategy: since lbug sessions don't share uncommitted data across
//! `LbugStore::open` calls, tests use a pre-opened store shared
//! between seed and assert.

use tempfile::TempDir;

use archctl::clock::FixedClock;
use archctl::evidence::{Evidence, EvidenceKind, EvidenceStatus, SourceOrigin};
use archctl::store::{EvidenceOps, GraphStore, LbugStore, RawGraphQuery};

// ──────────────────────────────────────────────────────────────────────────────
// Helper functions (mirror tests/diagram_apply.rs:1-5 pattern)
// ──────────────────────────────────────────────────────────────────────────────

/// Open and init a LbugStore in a temp directory.
fn open_store(tmp: &TempDir) -> LbugStore {
    let project = tmp.path().join("proj");
    let mut store = LbugStore::open(&project).unwrap();
    store.init().unwrap();
    store
}

/// Fixed clock for deterministic timestamps.
fn fixed_clock() -> FixedClock {
    FixedClock::new("2026-08-20T12:00:00Z")
}

/// Seed a minimal container diagram for bundle and element-reference checks.
/// This is a NOP for the critical gate test (which only tests accept_evidence),
/// but needed for the bundle projection checks in step 19.
#[allow(dead_code)]
fn seed_orders_stripe_fixture(_store: &mut LbugStore) {
    // Elements and diagrams are seeded via changeset apply in the full UAT-06
    // pipeline. The critical gate test only tests the trust guard at
    // accept_evidence, not the bundle projection. Bundle projection is
    // tested in step 19 (#[ignore]d pending TRUST-005 + spec-35).
}

/// Mint an Evidence row in the store. Used by both critical gate and negative control.
fn mint_evidence_row(
    store: &mut LbugStore,
    id: &str,
    claim: &str,
    source_origin: SourceOrigin,
    tool_name: &str,
    status: EvidenceStatus,
) {
    let evidence = Evidence {
        id: id.to_string(),
        kind: EvidenceKind::Structural,
        claim: claim.to_string(),
        path: "src/orders.rs".to_string(),
        start_line: 1,
        end_line: 1,
        start_byte: Some(0),
        end_byte: Some(4),
        tool_name: tool_name.to_string(),
        tool_version: "test".to_string(),
        rule_id: "test:rule".to_string(),
        language: "rust".to_string(),
        observed_at: "2026-08-20T12:00:00Z".to_string(),
        source_origin,
        content_hash: Some("sha256:abc123".to_string()),
        text_preview: Some("fn orders()".to_string()),
        props: {
            let mut p = serde_json::Map::new();
            p.insert(
                "status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
            p.insert(
                "source_origin".to_string(),
                serde_json::Value::String(source_origin.as_str().to_string()),
            );
            p
        },
        status,
    };
    store.put_evidence(std::slice::from_ref(&evidence)).unwrap();
}

// ──────────────────────────────────────────────────────────────────────────────
// UAT-06 step 11: critical gate
// ──────────────────────────────────────────────────────────────────────────────

/// UAT-06 step 11 critical gate: a ModelInference claim MUST NOT be
/// promotable to canonical. The trust guard at accept_evidence blocks it
/// with a TrustViolation error.
///
/// Metric: false_canonical_promotions == 0 (the guard blocks the promotion)
#[test]
fn uat_06_step_11_false_canonical_promotions_equals_zero_after_llm_claim() {
    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);
    let clock: &dyn archctl::clock::Clock = &fixed_clock();

    // Seed fixture
    seed_orders_stripe_fixture(&mut store);

    // Mint an LLM claim (Orders calls Stripe directly) — this is the false agent claim
    mint_evidence_row(
        &mut store,
        "ev:llm:orders-stripe",
        "Orders calls Stripe directly",
        SourceOrigin::ModelInference,
        "llm_analyst",
        EvidenceStatus::Drafted,
    );

    // Step 11: attempt to accept the LLM claim — the trust guard MUST block it
    let result = store.accept_evidence("ev:llm:orders-stripe", clock);

    // GUARD MUST DENY: ModelInference cannot be promoted to canonical
    let err = result.expect_err("accept_evidence on ModelInference must be denied by trust guard");
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("canonical write denied"),
        "error must mention trust guard denial, got: {err_msg}"
    );

    // Verify: no Evaluation was created (the accept failed before attestation)
    let eval_rows = store
        .query("MATCH (e:Evaluation) RETURN count(e) AS n;")
        .unwrap();
    let eval_count = eval_rows
        .first()
        .and_then(|r| r.get("n"))
        .and_then(|c| c.as_i64())
        .unwrap_or(0);
    assert_eq!(
        eval_count, 0,
        "no Evaluation should be created when guard denies"
    );

    // Critical gate metric: false_canonical_promotions == 0
    // The LLM claim (ev:llm:orders-stripe) must NOT appear in Accepted list.
    // We check by ID rather than parsing props (Cell is private).
    let accepted = store
        .list_evidence_by_status(EvidenceStatus::Accepted, None)
        .unwrap();
    let accepted_ids: Vec<_> = accepted
        .iter()
        .filter_map(|r| r.get("e.id").and_then(|c| c.as_str()))
        .collect();
    assert!(
        !accepted_ids.contains(&"ev:llm:orders-stripe"),
        "critical gate: ModelInference claim must NOT appear as Accepted"
    );
    // Self-contained sanity: the LLM claim is the only row minted in this test,
    // so Accepted list MUST be empty (no false promotion through the chokepoint).
    assert!(
        accepted_ids.is_empty(),
        "critical gate: no ModelInference claim may reach Accepted; got {:?}",
        accepted_ids
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// UAT-06 step 11: negative control
// ──────────────────────────────────────────────────────────────────────────────

/// UAT-06 step 11 negative control: a pure deterministic (UserWorkspace +
/// tree-sitter) claim CAN be promoted to canonical. This proves the guard
/// discriminates on authority/stamp, not on claim content.
#[test]
fn uat_06_step_11_negative_control_pure_deterministic_observes_accepts() {
    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);
    let clock: &dyn archctl::clock::Clock = &fixed_clock();

    // Seed fixture
    seed_orders_stripe_fixture(&mut store);

    // Mint a deterministic claim with identical text but UserWorkspace origin
    mint_evidence_row(
        &mut store,
        "ev:ws:orders-stripe",
        "Orders calls Stripe directly",
        SourceOrigin::UserWorkspace,
        "tree-sitter",
        EvidenceStatus::Drafted,
    );

    // Negative control: accept must SUCCEED (UserWorkspace is always allowed)
    let result = store.accept_evidence("ev:ws:orders-stripe", clock);
    result.expect("accept_evidence on UserWorkspace must succeed");

    // Verify: Evaluation was created
    let eval_rows = store
        .query("MATCH (e:Evaluation) RETURN count(e) AS n;")
        .unwrap();
    let eval_count = eval_rows
        .first()
        .and_then(|r| r.get("n"))
        .and_then(|c| c.as_i64())
        .unwrap_or(0);
    assert_eq!(
        eval_count, 1,
        "Evaluation must be created for accepted evidence"
    );

    // Verify: the UserWorkspace claim IS accepted
    let accepted = store
        .list_evidence_by_status(EvidenceStatus::Accepted, None)
        .unwrap();
    let accepted_ids: Vec<_> = accepted
        .iter()
        .filter_map(|r| r.get("e.id").and_then(|c| c.as_str()))
        .collect();
    assert!(
        accepted_ids.contains(&"ev:ws:orders-stripe"),
        "UserWorkspace claim must be accepted"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// UAT-06 steps 7/9/13/14/15/16/17/19/20: #[ignore]d skeletons
// ──────────────────────────────────────────────────────────────────────────────

/// UAT-06 step 7: invoke-agent — blocked on TRUST-005 (FusedClaim persistence).
#[test]
#[ignore]
fn uat_06_step_07_invoke_agent() {
    // TRUST-005: FusedClaim persistence — agent claim must survive across sessions
    // as a Drafted FusedClaim, not as a raw Evidence row.
    todo!("step 07 blocked on TRUST-005: implement FusedClaim entity and persistence")
}

/// UAT-06 step 9: assert candidate-visible — blocked on TRUST-005.
#[test]
#[ignore]
fn uat_06_step_09_assert_candidate_visible() {
    // After step 7 (invoke-agent), the LLM claim must appear in the candidate set.
    // TRUST-005: FusedClaim must be queryable as candidate-visible.
    todo!("step 09 blocked on TRUST-005")
}

/// UAT-06 step 13: human-feedback verdict: reject — blocked on spec-35 (FEEDBACK-AND-RECONCILIATION).
#[test]
#[ignore]
fn uat_06_step_13_human_feedback_reject() {
    // spec-35: human feedback verdict must be recordable and must mark
    // the FusedClaim as rejected, not accepted.
    todo!("step 13 blocked on spec-35: FEEDBACK-AND-RECONCILIATION")
}

/// UAT-06 step 14: replacement claim — blocked on spec-35.
#[test]
#[ignore]
fn uat_06_step_14_replacement_claim() {
    // spec-35: the replacement "Orders uses PaymentProvider" must appear
    // as the new canonical fact after rejection.
    todo!("step 14 blocked on spec-35")
}

/// UAT-06 step 15: restart-workbench — blocked on spec-35.
#[test]
#[ignore]
fn uat_06_step_15_restart_workbench() {
    // spec-35: after restart, the rejection must persist (FusedClaim state
    // survived the workbench restart).
    todo!("step 15 blocked on spec-35")
}

/// UAT-06 step 16: invoke-agent re-evaluation — blocked on spec-35 + spec-39 (SEARCH-CONTEXT-BUNDLE).
#[test]
#[ignore]
fn uat_06_step_16_reinvoke_agent() {
    // spec-35 + spec-39: when the agent re-evaluates Orders payment dependency,
    // it must see the prior rejection in context (SEARCH-CONTEXT-BUNDLE) and
    // NOT repeat the false claim.
    todo!("step 16 blocked on spec-35 + spec-39: SEARCH-CONTEXT-BUNDLE")
}

/// UAT-06 step 17: assert prior-rejection-in-context — blocked on spec-39.
#[test]
#[ignore]
fn uat_06_step_17_assert_prior_rejection_in_context() {
    // spec-39: the SEARCH-CONTEXT-BUNDLE must include the rejected FusedClaim
    // in the agent's context for step 16.
    todo!("step 17 blocked on spec-39: SEARCH-CONTEXT-BUNDLE")
}

/// UAT-06 step 19: verify bundle projection — blocked on TRUST-005 + spec-35.
#[test]
#[ignore]
fn uat_06_step_19_verify_bundle_no_false_canonical() {
    // After all previous steps, the bundle must NOT contain the false
    // Orders->Stripe canonical fact. This is the end-to-end verification
    // of the full pipeline.
    todo!("step 19 blocked on TRUST-005 + spec-35")
}

/// UAT-06 step 20: verify replacement canonical — blocked on spec-35.
#[test]
#[ignore]
fn uat_06_step_20_verify_replacement_canonical() {
    // The replacement "Orders uses PaymentProvider" must appear as a
    // canonical fact in the bundle after step 14.
    todo!("step 20 blocked on spec-35")
}
