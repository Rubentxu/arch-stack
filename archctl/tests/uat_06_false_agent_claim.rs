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
use archctl::diagram::export_types::EvidenceEntry;
use archctl::evidence::{Evidence, EvidenceKind, EvidenceStatus, SourceOrigin};
use archctl::store::{
    ElementRepository, EvidenceOps, EvidenceRepository, GraphStore, LbugStore, RawGraphQuery,
};

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

/// Seed the Orders / Stripe / PaymentProvider fixture for UAT-06.
///
/// Creates Evidence rows (the dual-write creates Observations via
/// `observation_from_evidence`). The fusion path uses `fuse_observations`
/// directly so the test doesn't depend on Element/ElementVersion scaffolding.
///
/// Creates:
/// - 1 Evidence row for Orders→PaymentProvider with UserWorkspace/Drafted
/// - 1 Evidence row for Orders→Stripe (false claim) with ModelInference/Drafted
///
/// Per spec REQ-T05-006 and design.md §7 PR-3a.
fn seed_orders_stripe_fixture(store: &mut LbugStore) {
    use archctl::evidence::{Evidence, EvidenceKind, EvidenceStatus, SourceOrigin};

    // 1 Evidence row: Orders → PaymentProvider (UserWorkspace, Drafted).
    // The trust guard will promote UserWorkspace evidence to Accepted when
    // accept_evidence is called (tested by the negative control).
    let evidence_wp = Evidence {
        id: "ev:ws:orders-payment".into(),
        kind: EvidenceKind::Structural,
        claim: "Orders uses PaymentProvider for checkout".into(),
        path: "src/orders.rs".into(),
        start_line: 1,
        end_line: 10,
        start_byte: None,
        end_byte: None,
        tool_name: "tree-sitter".into(),
        tool_version: "0.1".into(),
        rule_id: "struct:dependency".into(),
        language: "rust".into(),
        observed_at: "2026-08-20T12:00:00Z".into(),
        source_origin: SourceOrigin::UserWorkspace,
        content_hash: Some("sha256:ws001".into()),
        text_preview: Some("fn orders()".into()),
        props: {
            let mut p = serde_json::Map::new();
            p.insert(
                "status".to_string(),
                serde_json::Value::String("drafted".into()),
            );
            p.insert(
                "source_origin".to_string(),
                serde_json::Value::String(SourceOrigin::UserWorkspace.as_str().into()),
            );
            p
        },
        status: EvidenceStatus::Drafted,
    };
    store
        .put_evidence(std::slice::from_ref(&evidence_wp))
        .unwrap();

    // 1 Evidence row: Orders → Stripe (ModelInference, Drafted — the false claim)
    let evidence_stripe = Evidence {
        id: "ev:llm:orders-stripe".into(),
        kind: EvidenceKind::Structural,
        claim: "Orders calls Stripe directly".into(),
        path: "src/orders.rs".into(),
        start_line: 1,
        end_line: 10,
        start_byte: None,
        end_byte: None,
        tool_name: "llm_analyst".into(),
        tool_version: "0.1".into(),
        rule_id: "struct:dependency".into(),
        language: "rust".into(),
        observed_at: "2026-08-20T12:00:00Z".into(),
        source_origin: SourceOrigin::ModelInference,
        content_hash: Some("sha256:llm001".into()),
        text_preview: Some("fn orders()".into()),
        props: {
            let mut p = serde_json::Map::new();
            p.insert(
                "status".to_string(),
                serde_json::Value::String("drafted".into()),
            );
            p.insert(
                "source_origin".to_string(),
                serde_json::Value::String(SourceOrigin::ModelInference.as_str().into()),
            );
            p
        },
        status: EvidenceStatus::Drafted,
    };
    store
        .put_evidence(std::slice::from_ref(&evidence_stripe))
        .unwrap();
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

/// UAT-06 step 7: invoke-agent — trust gate blocks ModelInference claims
/// from becoming "accepted" FusedClaims.
///
/// Per spec REQ-T05-007. Uses `fuse_observations_with` directly so the
/// test bypasses Element/ElementVersion scaffolding requirement.
#[test]
fn uat_06_step_07_invoke_agent() {
    use archctl::architecture::fusion::MaxMemberEvaluator;
    use archctl::architecture::fusion::fuse_observations_with;
    use archctl::observation_claim::{Observation, ObservationStatus};

    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);

    // Seed fixture: creates Evidence rows (evidence_wp and evidence_stripe)
    seed_orders_stripe_fixture(&mut store);

    // Manually create Observations with correct evidence_origin field.
    // observation_from_evidence would set evidence_origin="", which would
    // default to UserWorkspace and produce "accepted" status (wrong for llm).
    let obs_wp = Observation {
        id: "obs:ev:ws:orders-payment".into(),
        kind: "structural".into(),
        claim: "Orders uses PaymentProvider for checkout".into(),
        path: "src/orders.rs".into(),
        start_line: 1,
        end_line: 10,
        tool_name: "tree-sitter".into(),
        tool_version: "0.1".into(),
        rule_id: "struct:dependency".into(),
        content_hash: "sha256:ws001".into(),
        observed_at: "2026-08-20T12:00:00Z".into(),
        evidence_origin: "UserWorkspace".into(), // correct origin for ws
        confidence: 1.0,
        status: ObservationStatus::Accepted,
        written_via_backfill: false,
    };
    let obs_llm = Observation {
        id: "obs:ev:llm:orders-stripe".into(),
        kind: "structural".into(),
        claim: "Orders calls Stripe directly".into(),
        path: "src/orders.rs".into(),
        start_line: 1,
        end_line: 10,
        tool_name: "llm_analyst".into(),
        tool_version: "0.1".into(),
        rule_id: "struct:dependency".into(),
        content_hash: "sha256:llm001".into(),
        observed_at: "2026-08-20T12:00:00Z".into(),
        evidence_origin: "ModelInference".into(), // correct origin for llm
        confidence: 0.0,
        status: ObservationStatus::Drafted,
        written_via_backfill: false,
    };

    // Call fusion directly (bypasses Element/ElementVersion scaffolding)
    let claims = fuse_observations_with(
        &[obs_wp.clone(), obs_llm.clone()],
        &MaxMemberEvaluator,
        "2026-08-20T12:00:00Z",
    );

    // Find the llm claim FusedClaim
    // NOTE: FusedClaim.statement stores the NORMALIZED (lowercased) claim text
    let llm_claim = claims
        .iter()
        .find(|c| c.statement == "orders calls stripe directly")
        .expect("llm FusedClaim must exist after fusion");

    // Verify trust gate: ModelInference claim must have status="drafted"
    assert_eq!(
        llm_claim.status, "drafted",
        "ModelInference FusedClaim must have status=drafted, got {}",
        llm_claim.status
    );
    assert_eq!(
        llm_claim.confidence, 0.0,
        "ModelInference confidence must be 0.0, got {}",
        llm_claim.confidence
    );

    // Verify ws claim has status="accepted"
    // NOTE: FusedClaim.statement stores the NORMALIZED (lowercased) claim text
    let ws_claim = claims
        .iter()
        .find(|c| c.statement == "orders uses paymentprovider for checkout")
        .expect("ws FusedClaim must exist after fusion");
    assert_eq!(
        ws_claim.status, "accepted",
        "UserWorkspace FusedClaim must have status=accepted, got {}",
        ws_claim.status
    );
}

/// UAT-06 step 9: assert candidate-visible — the LLM FusedClaim
/// is queryable as status="drafted" but NOT promoted to canonical.
///
/// Per spec REQ-T05-007 step 9.
#[test]
fn uat_06_step_09_assert_candidate_visible() {
    use archctl::architecture::fusion::MaxMemberEvaluator;
    use archctl::architecture::fusion::fuse_observations_with;
    use archctl::observation_claim::{Observation, ObservationStatus};

    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);

    // Seed fixture
    seed_orders_stripe_fixture(&mut store);

    // Create Observations with correct evidence_origin (same as step 7)
    let obs_wp = Observation {
        id: "obs:ev:ws:orders-payment".into(),
        kind: "structural".into(),
        claim: "Orders uses PaymentProvider for checkout".into(),
        path: "src/orders.rs".into(),
        start_line: 1,
        end_line: 10,
        tool_name: "tree-sitter".into(),
        tool_version: "0.1".into(),
        rule_id: "struct:dependency".into(),
        content_hash: "sha256:ws001".into(),
        observed_at: "2026-08-20T12:00:00Z".into(),
        evidence_origin: "UserWorkspace".into(),
        confidence: 1.0,
        status: ObservationStatus::Accepted,
        written_via_backfill: false,
    };
    let obs_llm = Observation {
        id: "obs:ev:llm:orders-stripe".into(),
        kind: "structural".into(),
        claim: "Orders calls Stripe directly".into(),
        path: "src/orders.rs".into(),
        start_line: 1,
        end_line: 10,
        tool_name: "llm_analyst".into(),
        tool_version: "0.1".into(),
        rule_id: "struct:dependency".into(),
        content_hash: "sha256:llm001".into(),
        observed_at: "2026-08-20T12:00:00Z".into(),
        evidence_origin: "ModelInference".into(),
        confidence: 0.0,
        status: ObservationStatus::Drafted,
        written_via_backfill: false,
    };

    // Run fusion
    let claims = fuse_observations_with(
        &[obs_wp, obs_llm],
        &MaxMemberEvaluator,
        "2026-08-20T12:00:00Z",
    );

    // Find llm FusedClaim
    // NOTE: FusedClaim.statement stores the NORMALIZED (lowercased) claim text
    let llm_claim = claims
        .iter()
        .find(|c| c.statement == "orders calls stripe directly")
        .expect("llm FusedClaim must exist");

    // Candidate-visible means status="drafted" (not promoted to canonical)
    assert_eq!(
        llm_claim.status, "drafted",
        "candidate-visible FusedClaim must have status=drafted, got {}",
        llm_claim.status
    );

    // Verify it is NOT in "accepted" status (would mean it was promoted)
    assert_ne!(
        llm_claim.status, "accepted",
        "candidate-visible FusedClaim must NOT have status=accepted"
    );
}

/// Persist a single fused claim derived from two inline Observations,
/// returning its canonical id. Mirrors the step_7 pattern but persists
/// the claim so `FeedbackRepository` can link a `:Feedback` edge to it.
fn persist_one_fused_claim(store: &mut LbugStore, version_id: &str) -> String {
    use archctl::DiagramRepository;
    use archctl::architecture::fusion::fuse_observations;
    use archctl::observation_claim::{Observation, ObservationStatus};

    // Two Observations: one UserWorkspace (canonical) + one ModelInference
    // (rejected false claim). Fused into a single FusedClaim.
    let obs_wp = Observation {
        id: format!("obs:ev:ws:{version_id}"),
        kind: "structural".into(),
        claim: "Orders uses PaymentProvider for checkout".into(),
        path: "src/orders.rs".into(),
        start_line: 1,
        end_line: 10,
        tool_name: "tree-sitter".into(),
        tool_version: "0.1".into(),
        rule_id: "struct:dependency".into(),
        content_hash: "sha256:ws001".into(),
        observed_at: "2026-08-20T12:00:00Z".into(),
        evidence_origin: "UserWorkspace".into(),
        confidence: 1.0,
        status: ObservationStatus::Accepted,
        written_via_backfill: false,
    };
    let obs_llm = Observation {
        id: format!("obs:ev:llm:{version_id}"),
        kind: "structural".into(),
        claim: "Orders uses PaymentProvider for checkout".into(),
        path: "src/orders.rs".into(),
        start_line: 1,
        end_line: 10,
        tool_name: "llm_analyst".into(),
        tool_version: "0.1".into(),
        rule_id: "struct:dependency".into(),
        content_hash: "sha256:llm001".into(),
        observed_at: "2026-08-20T12:00:00Z".into(),
        evidence_origin: "ModelInference".into(),
        confidence: 0.0,
        status: ObservationStatus::Drafted,
        written_via_backfill: false,
    };
    let fused = fuse_observations(&[obs_wp, obs_llm]);
    assert_eq!(fused.len(), 1, "fixture must yield exactly one fused claim");
    let claim_id = fused[0].id.clone();
    store
        .put_fused_claims(version_id, &fused, "2026-08-20T12:00:00Z")
        .expect("persist fused claim");
    claim_id
}

/// UAT-06 step 13: human-feedback verdict: reject.
#[test]
fn uat_06_step_13_human_feedback_reject() {
    use archctl::feedback::{Feedback, FeedbackVerdict};
    use archctl::store::FeedbackRepository;
    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);
    let claim_id = persist_one_fused_claim(&mut store, "v:orders-stripe-step13");

    let fb = Feedback {
        id: "fdbk:test:reject".into(),
        target: claim_id.clone(),
        verdict: FeedbackVerdict::Reject,
        replacement: Some("Orders uses PaymentProvider for checkout".into()),
        actor: "alice".into(),
        revision: "rev1".into(),
        timestamp: "2026-08-20T12:00:00Z".into(),
        evidence: None,
        correlation_id: None,
    };
    store.put_feedback(&fb).unwrap();
    let read = store.read_feedback_for_claim(&claim_id).unwrap();
    assert_eq!(read.len(), 1, "exactly one feedback row recorded");
    assert_eq!(read[0].verdict, FeedbackVerdict::Reject);
    assert_eq!(
        read[0].replacement.as_deref(),
        Some("Orders uses PaymentProvider for checkout")
    );
}

/// UAT-06 step 14: replacement claim is the new canonical fact after rejection.
#[test]
fn uat_06_step_14_replacement_claim() {
    use archctl::feedback::{Feedback, FeedbackVerdict};
    use archctl::reconciliation::Reconciliation;
    use archctl::store::FeedbackRepository;
    use archctl::trust::{AuthorityClass, ExecutionClass, TrustClassification};

    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);
    let claim_id = persist_one_fused_claim(&mut store, "v:orders-stripe-step14");

    // HumanDecision × Normative — canonical trust tier; the bridge allows promotion.
    let trust = TrustClassification {
        execution: ExecutionClass::HumanDecision,
        authority: AuthorityClass::Normative,
    };
    let fb = Feedback {
        id: "fdbk:test:reject-replace".into(),
        target: claim_id,
        verdict: FeedbackVerdict::Reject,
        replacement: Some("Orders uses PaymentProvider for checkout".into()),
        actor: "alice".into(),
        revision: "rev1".into(),
        timestamp: "2026-08-20T12:00:00Z".into(),
        evidence: None,
        correlation_id: None,
    };
    store.put_feedback(&fb).unwrap();

    // Compute reconciliation — replacement must drive the derived assertion.
    let r = Reconciliation::compute(
        "a1".into(),
        "Orders".into(),
        "uses".into(),
        "PaymentProvider".into(),
        vec!["ev:ws:orders-payment".into()],
        std::slice::from_ref(&fb),
        "rev1".into(),
        trust,
    );
    assert_eq!(
        r.computed_status, "rejected",
        "Reject + replacement → computed_status=rejected"
    );
    assert!(
        r.rationale.contains("replacement") || r.rationale.contains("reject"),
        "rationale must mention rejection basis: {}",
        r.rationale
    );
}

/// UAT-06 step 15: rejection persists across workbench restart.
#[test]
fn uat_06_step_15_restart_workbench() {
    use archctl::feedback::{Feedback, FeedbackVerdict};
    use archctl::store::FeedbackRepository;
    let tmp = TempDir::new().unwrap();

    // Session 1: open + record reject feedback.
    let claim_id = {
        let mut store = open_store(&tmp);
        let cid = persist_one_fused_claim(&mut store, "v:orders-stripe-step15");
        let fb = Feedback {
            id: "fdbk:test:reject-persist".into(),
            target: cid.clone(),
            verdict: FeedbackVerdict::Reject,
            replacement: Some("Orders uses PaymentProvider for checkout".into()),
            actor: "alice".into(),
            revision: "rev1".into(),
            timestamp: "2026-08-20T12:00:00Z".into(),
            evidence: None,
            correlation_id: None,
        };
        store.put_feedback(&fb).unwrap();
        cid
    };

    // Session 2: reopen same store and verify the feedback row is still there.
    let project = tmp.path().join("proj");
    let mut store = LbugStore::open(&project).unwrap();
    let read = store.read_feedback_for_claim(&claim_id).unwrap();
    assert_eq!(read.len(), 1, "feedback row must survive store restart");
    assert_eq!(read[0].verdict, FeedbackVerdict::Reject);
    assert_eq!(read[0].actor, "alice");
    assert_eq!(read[0].revision, "rev1");
}

/// UAT-06 step 16: invoke-agent re-evaluation — un-ignored in TRUST-006-b.
#[test]
fn uat_06_step_16_reinvoke_agent() {
    // spec-35 + spec-39: when the agent re-evaluates Orders payment dependency,
    // it must see the prior rejection in context (SEARCH-CONTEXT-BUNDLE) and
    // NOT repeat the false claim.
    //
    // TRUST-006-b: implements by populating `AgentContext.feedback_history`
    // with a `FeedbackSummary` derived from a stored Reject, and asserting
    // the agent respects it.
    //
    // Approach: avoid modifying production agents (out of TRUST-006 scope).
    // Use a test-only `FeedbackAwareMockAgent` that emits `NoAction` if its
    // `feedback_history` contains a Reject for the target claim id, and
    // `FindingCandidate(claim_id)` otherwise. The mock exercises the same
    // feedback plumbing that production agents will use (REQ-T06-002).
    use archctl::cognitive::context::AgentContext;
    use archctl::cognitive::test_support::{FeedbackAwareMockAgent, MockOutcome};
    use archctl::feedback::{Feedback, FeedbackSummary, FeedbackVerdict};

    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);
    let _clock: &dyn archctl::clock::Clock = &fixed_clock();

    let false_claim_id = persist_one_fused_claim(&mut store, "v:orders-stripe-step16");

    use archctl::store::FeedbackRepository;

    // Save a Feedback(Reject) on the false claim (simulating step 13 outcome).
    let fb = Feedback {
        id: "fdbk:step16:reject".into(),
        target: false_claim_id.clone(),
        verdict: FeedbackVerdict::Reject,
        replacement: Some("Orders uses PaymentProvider for checkout".into()),
        actor: "alice".into(),
        revision: "rev1".into(),
        timestamp: "2026-08-20T12:00:00Z".into(),
        evidence: None,
        correlation_id: None,
    };
    store.put_feedback(&fb).unwrap();

    // Build AgentContext with feedback_history populated.
    let summaries = vec![FeedbackSummary::from(&fb)];
    let ctx = AgentContext {
        goal: "find architectural patterns".into(),
        triggering_event: None,
        graph_view: archctl::cognitive::context::GraphView::default(),
        source_fragments: vec![],
        evidence: vec![],
        applicable_rules: vec![],
        available_tools: vec![],
        budget: archctl::cognitive::descriptor::AgentBudget::default(),
        feedback_history: summaries,
        pending_adjudications: vec![],
        recent_events: vec![],
    };

    // Re-invoke agent with feedback_history populated.
    let agent = FeedbackAwareMockAgent;
    let outcome = agent.invoke(&ctx, &false_claim_id);

    // Assertion: agent must NOT propose the false claim as FindingCandidate.
    match outcome {
        MockOutcome::NoAction => {
            // expected: agent respects feedback history
        }
        MockOutcome::FindingCandidate(f) => {
            panic!(
                "agent must NOT propose rejected false claim {} as FindingCandidate; got FindingCandidate({:?})",
                false_claim_id, f
            );
        }
    }
}

/// UAT-06 step 17: assert prior-rejection-in-context — un-ignored in TRUST-006-b.
#[test]
fn uat_06_step_17_assert_prior_rejection_in_context() {
    // spec-39: the SEARCH-CONTEXT-BUNDLE must include the rejected FusedClaim
    // in the agent's context for step 16.
    //
    // TRUST-006-b: FeedbackSummary::from(&Feedback) round-trips the relevant
    // fields; AgentContext.feedback_history carries them to the agent.
    use archctl::feedback::{Feedback, FeedbackSummary, FeedbackVerdict};

    let fb = Feedback {
        id: "fdbk:step17:test".into(),
        target: "clm:fused:step17-target".into(),
        verdict: FeedbackVerdict::Reject,
        replacement: Some("Orders uses PaymentProvider".into()),
        actor: "alice".into(),
        revision: "rev1".into(),
        timestamp: "2026-08-20T12:00:00Z".into(),
        evidence: None,
        correlation_id: None,
    };

    let summary = FeedbackSummary::from(&fb);

    assert_eq!(summary.id, "fdbk:step17:test");
    assert_eq!(summary.target, "clm:fused:step17-target");
    assert_eq!(summary.verdict, FeedbackVerdict::Reject);
    assert_eq!(
        summary.replacement.as_deref(),
        Some("Orders uses PaymentProvider")
    );
    assert_eq!(summary.actor, "alice");
    assert_eq!(summary.revision, "rev1");

    // FeedbackSummary must NOT carry evidence or correlation_id (per
    // REQ-T06-001 — slim view, pipeline-internal fields excluded).
    let json = serde_json::to_string(&summary).unwrap();
    assert!(
        !json.contains("evidence"),
        "FeedbackSummary JSON must not contain 'evidence' field: {json}"
    );
    assert!(
        !json.contains("correlation_id"),
        "FeedbackSummary JSON must not contain 'correlation_id' field: {json}"
    );
}

/// Seed an Element + ElementVersion + SUPPORTED_BY chain so the bundle
/// export query (`(ev:ElementVersion)-[r:SUPPORTED_BY]->(e:Evidence)`) can
/// reach the Evidence rows. TRUST-006-a step 19/20 fixture.
fn seed_bundle_fixture(store: &mut LbugStore) {
    use archctl::Element;
    use archctl::ElementVersion;

    // Element rows: Orders, Stripe (false), PaymentProvider (replacement).
    let elements = vec![
        Element {
            id: "el:orders".into(),
            kind_id: "mt.container".into(),
            category: "c4".into(),
            canonical_key: "orders".into(),
            current_name: "Orders".into(),
            current_status: "active".into(),
            current_confidence: 1.0,
            current_version_id: "ev:orders:v1".into(),
        },
        Element {
            id: "el:stripe".into(),
            kind_id: "mt.container".into(),
            category: "c4".into(),
            canonical_key: "stripe".into(),
            current_name: "Stripe".into(),
            current_status: "active".into(),
            current_confidence: 0.0,
            current_version_id: "ev:stripe:v1".into(),
        },
        Element {
            id: "el:payment".into(),
            kind_id: "mt.container".into(),
            category: "c4".into(),
            canonical_key: "payment".into(),
            current_name: "PaymentProvider".into(),
            current_status: "active".into(),
            current_confidence: 1.0,
            current_version_id: "ev:payment:v1".into(),
        },
    ];
    store.batch_upsert_elements(&elements).unwrap();

    // ElementVersion rows.
    let versions = vec![
        ElementVersion {
            id: "ev:orders:v1".into(),
            element_id: "el:orders".into(),
            name: "Orders".into(),
            status: "active".into(),
            origin: "UserWorkspace".into(),
            confidence: 1.0,
            props: serde_json::Map::new(),
        },
        ElementVersion {
            id: "ev:stripe:v1".into(),
            element_id: "el:stripe".into(),
            name: "Stripe".into(),
            status: "active".into(),
            origin: "ModelInference".into(),
            confidence: 0.0,
            props: serde_json::Map::new(),
        },
        ElementVersion {
            id: "ev:payment:v1".into(),
            element_id: "el:payment".into(),
            name: "PaymentProvider".into(),
            status: "active".into(),
            origin: "UserWorkspace".into(),
            confidence: 1.0,
            props: serde_json::Map::new(),
        },
    ];
    store.batch_upsert_element_versions(&versions).unwrap();

    // SUPPORTED_BY edges: link each ElementVersion to its Evidence row.
    store
        .link_supported_by("ev:payment:v1", "ev:ws:orders-payment")
        .unwrap();
    store
        .link_supported_by("ev:stripe:v1", "ev:llm:orders-stripe")
        .unwrap();
}

/// Assert the bundle does NOT contain a canonical fact whose text
/// contains the given needle. Checks the bundle's evidence section
/// (filtered to status=accepted by the export layer).
fn assert_no_canonical_fact_in_bundle(
    bundle: &archctl::diagram::BundleEnvelope,
    needle: &str,
    bundle_json: &str,
) {
    // The evidence section is filtered to accepted entries (ADR-005).
    let in_evidence = bundle
        .evidence
        .evidence
        .iter()
        .any(|e: &EvidenceEntry| e.claim.contains(needle));
    assert!(
        !in_evidence,
        "bundle MUST NOT contain canonical fact containing '{needle}' in its evidence section; bundle has {} entries",
        bundle.evidence.evidence.len()
    );
    // Belt-and-suspenders: also assert the literal string is absent from the
    // serialized bundle. This catches regressions where claim text leaks via
    // an unexpected path.
    assert!(
        !bundle_json.contains(needle),
        "bundle JSON MUST NOT contain literal '{needle}' (any leak path); bundle_json length = {}",
        bundle_json.len()
    );
}

/// Assert the bundle DOES contain a canonical fact whose text contains
/// the given needle. Checks the bundle's evidence section.
fn assert_has_canonical_fact_in_bundle(
    bundle: &archctl::diagram::BundleEnvelope,
    needle: &str,
    bundle_json: &str,
) {
    let in_evidence = bundle
        .evidence
        .evidence
        .iter()
        .any(|e: &EvidenceEntry| e.claim.contains(needle));
    assert!(
        in_evidence,
        "bundle MUST contain canonical fact containing '{needle}' in its evidence section; bundle has {} entries",
        bundle.evidence.evidence.len()
    );
    assert!(
        bundle_json.contains(needle),
        "bundle JSON MUST contain literal '{needle}'"
    );
}

/// UAT-06 step 19: verify bundle projection — un-ignored in TRUST-006-a.
#[test]
fn uat_06_step_19_verify_bundle_no_false_canonical() {
    // After all previous steps, the bundle must NOT contain the false
    // Orders->Stripe canonical fact. This is the end-to-end verification
    // of the full pipeline.
    //
    // TRUST-006-a: build a bundle via `build_bundle` and assert the bundle's
    // evidence section does NOT include the rejected false claim.
    //
    // We build a minimal fixture (Element + ElementVersion + Evidence +
    // SUPPORTED_BY edge) that lets the bundle export query traverse the
    // canonical-promotion chain. The ModelInference claim stays Drafted
    // (trust guard rejects accept_evidence); the bundle's evidence filter
    // (ADR-005: only canonical evidence in projections) excludes it.
    use archctl::diagram::export::build_bundle;

    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);
    let clock: &dyn archctl::clock::Clock = &fixed_clock();

    seed_orders_stripe_fixture(&mut store);

    // Seed Element + ElementVersion + SUPPORTED_BY for both claims so the
    // bundle's evidence query (ElementVersion -> SUPPORTED_BY -> Evidence)
    // can traverse to the Evidence rows.
    seed_bundle_fixture(&mut store);

    // Try to accept ModelInference evidence (false). Trust guard rejects.
    let accept_result = store.accept_evidence("ev:llm:orders-stripe", clock);
    assert!(
        accept_result.is_err(),
        "ModelInference accept MUST be rejected by trust guard; got {:?}",
        accept_result
    );

    // Build bundle and verify the false claim is excluded.
    let bundle = build_bundle(&store, "container:*", clock).expect("build_bundle");
    let bundle_json = serde_json::to_string(&bundle.evidence).expect("evidence serialization");

    assert_no_canonical_fact_in_bundle(&bundle, "Orders calls Stripe", &bundle_json);
}

/// UAT-06 step 20: verify replacement canonical — un-ignored in TRUST-006-a.
#[test]
fn uat_06_step_20_verify_replacement_canonical() {
    // The replacement "Orders uses PaymentProvider" must appear as a
    // canonical fact in the bundle after step 14.
    //
    // TRUST-006-a: build a bundle and assert the bundle's evidence section
    // DOES include the accepted replacement claim text.
    use archctl::diagram::export::build_bundle;

    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);
    let clock: &dyn archctl::clock::Clock = &fixed_clock();

    seed_orders_stripe_fixture(&mut store);
    seed_bundle_fixture(&mut store);

    // Accept the UserWorkspace evidence (the replacement canonical fact).
    store
        .accept_evidence("ev:ws:orders-payment", clock)
        .expect("UserWorkspace accept must succeed");

    let bundle = build_bundle(&store, "container:*", clock).expect("build_bundle");
    let bundle_json = serde_json::to_string(&bundle.evidence).expect("evidence serialization");

    assert_has_canonical_fact_in_bundle(&bundle, "Orders uses PaymentProvider", &bundle_json);
}
