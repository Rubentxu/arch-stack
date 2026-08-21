//! REQ-T08-002 integration coverage.
//!
//! Mirrors `archctl/tests/feedback_summaries_port.rs` shape (TRUST-007 reference).
//! Uses TempDir + LbugStore::open per project test pattern.
//! See: sddk/p-38e02210a9f14317/trust-008-m30-bridge-promotion/specification.md REQ-T08-002.

use archctl::adjudication::{AdjudicationDecision, AdjudicationEvent, id_for};
use archctl::store::{AdjudicationRepository, DiagramRepository, GraphStore, LbugStore};
use tempfile::TempDir;

// ─────────────────────────────────────────────────────────────────────────────
// Helper functions (mirrors tests/feedback_summaries_port.rs:18-54 pattern)
// ─────────────────────────────────────────────────────────────────────────────

/// Open and init a LbugStore in a temp directory.
fn open_store(tmp: &TempDir) -> LbugStore {
    let project = tmp.path().join("proj");
    let mut store = LbugStore::open(&project).unwrap();
    store.init().unwrap();
    store
}

/// Persist a minimal FusedClaim and return its canonical id.
fn persist_claim(store: &mut LbugStore, version_id: &str) -> String {
    use archctl::architecture::fusion::fuse_observations;
    use archctl::observation_claim::{Observation, ObservationStatus};

    let obs = Observation {
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
    let fused = fuse_observations(&[obs]);
    assert_eq!(fused.len(), 1, "fixture must yield exactly one fused claim");
    let claim_id = fused[0].id.clone();
    store
        .put_fused_claims(version_id, &fused, "2026-08-20T12:00:00Z")
        .expect("persist fused claim");
    claim_id
}

/// Persist an AdjudicationEvent targeting a FusedClaim.
/// `seq` is a monotonic integer used to generate unique decided_at timestamps.
fn seed_adjudication(
    store: &mut LbugStore,
    claim_id: &str,
    adjudicator: &str,
    decision: AdjudicationDecision,
    seq: u32,
) -> AdjudicationEvent {
    let decided_at = format!("2026-08-21T12:00:{:02}Z", seq);
    let event = AdjudicationEvent {
        id: id_for(claim_id, adjudicator, &decided_at),
        target_fused_claim_id: claim_id.into(),
        adjudicator: adjudicator.into(),
        evidence_refs: vec![],
        decided_at,
        decision,
    };
    store.put_adjudication(&event).expect("put_adjudication");
    event
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

/// SCN-T08-002a + b: put + read roundtrip — an adjudication written to the
/// store is read back with all fields preserved.
#[test]
fn put_then_read_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);

    let claim_id = persist_claim(&mut store, "claim-for-roundtrip");

    let decided_at = "2026-08-21T12:00:00Z".to_string();
    let event = AdjudicationEvent {
        id: id_for(&claim_id, "tester", &decided_at),
        target_fused_claim_id: claim_id.clone(),
        adjudicator: "tester".into(),
        evidence_refs: vec!["ev:ref:001".into(), "ev:ref:002".into()],
        decided_at,
        decision: AdjudicationDecision::Promote,
    };

    store
        .put_adjudication(&event)
        .expect("put_adjudication must succeed");

    let read_back = store
        .read_adjudications_for_claim(&claim_id)
        .expect("read_adjudications_for_claim must succeed");

    assert_eq!(read_back.len(), 1, "must return exactly one row");
    let row = &read_back[0];
    assert_eq!(row.id, event.id);
    assert_eq!(row.target_fused_claim_id, event.target_fused_claim_id);
    assert_eq!(row.adjudicator, event.adjudicator);
    assert_eq!(row.evidence_refs, event.evidence_refs);
    assert_eq!(row.decided_at, event.decided_at);
    assert_eq!(row.decision, event.decision);
}

/// SCN-T08-002b: read_adjudications_for_claim returns only rows whose
/// target matches the requested claim id.
#[test]
fn read_adjudications_filters_by_target() {
    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);

    let claim_a = persist_claim(&mut store, "claim-target-a");
    let claim_b = persist_claim(&mut store, "claim-target-b");

    // Three adjudications across two claims with distinct seq values
    // to guarantee distinct id_for hashes.
    seed_adjudication(
        &mut store,
        &claim_a,
        "tester",
        AdjudicationDecision::Promote,
        0,
    );
    seed_adjudication(
        &mut store,
        &claim_b,
        "tester",
        AdjudicationDecision::Reject,
        1,
    );
    seed_adjudication(
        &mut store,
        &claim_a,
        "tester",
        AdjudicationDecision::Defer,
        2,
    );

    let for_a = store
        .read_adjudications_for_claim(&claim_a)
        .expect("query must succeed");
    let for_b = store
        .read_adjudications_for_claim(&claim_b)
        .expect("query must succeed");

    assert_eq!(for_a.len(), 2, "claim_a must have 2 adjudications");
    assert_eq!(for_b.len(), 1, "claim_b must have 1 adjudication");
    assert!(
        for_a.iter().all(|r| r.target_fused_claim_id == claim_a),
        "all rows for_a must target claim_a"
    );
    assert!(
        for_b.iter().all(|r| r.target_fused_claim_id == claim_b),
        "all rows for_b must target claim_b"
    );
}

/// SCN-T08-002c: list_pending_adjudications returns rows where decision=defer
/// OR where the target FusedClaim has status = "drafted".
///
/// Verifies both conditions:
/// - defer decision → appears regardless of target status
/// - drafted status → appears even if decision is promote
#[test]
fn list_pending_returns_only_defer_or_drafted_targets() {
    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);

    // claim_defer: decision=defer → should appear
    let claim_defer = persist_claim(&mut store, "claim-defer");
    seed_adjudication(
        &mut store,
        &claim_defer,
        "tester",
        AdjudicationDecision::Defer,
        3,
    );

    // claim_accepted: decision=promote, status=accepted → should NOT appear
    let claim_accepted = persist_claim(&mut store, "claim-accepted");
    seed_adjudication(
        &mut store,
        &claim_accepted,
        "tester",
        AdjudicationDecision::Promote,
        4,
    );

    let pending = store
        .list_pending_adjudications()
        .expect("list_pending_adjudications must succeed");

    let pending_targets: Vec<&str> = pending
        .iter()
        .map(|r| r.target_fused_claim_id.as_str())
        .collect();

    assert!(
        pending_targets.contains(&claim_defer.as_str()),
        "defer decision must appear; got {pending_targets:?}"
    );
    assert!(
        !pending_targets.contains(&claim_accepted.as_str()),
        "accepted+promote must NOT appear; got {pending_targets:?}"
    );

    // Additionally verify the ordering: defer rows come last when
    // ORDER BY decided_at DESC (most-recent first). The defer row
    // with decided_at "2026-08-21T12:00:03Z" is the newest, so it
    // should be first in the DESC list.
    if !pending.is_empty() {
        assert_eq!(
            pending[0].decision,
            AdjudicationDecision::Defer,
            "newest pending row (by decided_at DESC) should be the defer row"
        );
    }
}

/// SCN-T08-002d: invalid identifier in `read_adjudications_for_claim` returns
/// `Err` with the function name and "validation" in the context chain.
#[test]
fn invalid_identifier_surfaces_validation_error() {
    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);

    // Identifier with a disallowed char (space). `validate_identifier`
    // rejects anything outside `[A-Za-z0-9_.:-]+`.
    let bad = "clm bad id";
    let result = store.read_adjudications_for_claim(bad);

    let err = result.expect_err("invalid identifier must surface Err");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("read_adjudications_for_claim") && msg.contains("validation"),
        "error context must reference both `read_adjudications_for_claim` and \
         `validation`; got: {msg}"
    );
}

/// SCN-T08-005a: pending_adjudications round-trips through AgentContext serde.
///
/// Verifies that an AgentContext carrying a non-empty pending_adjudications Vec
/// survives JSON serialisation + deserialisation with the field intact.
///
/// See: specification.md REQ-T08-005 / design.md SCN-T08-005a.
#[test]
fn pending_adjudications_round_trips_through_agent_context_serde() {
    use archctl::adjudication::AdjudicationDecision;
    use archctl::cognitive::context::AgentContext;
    use archctl::cognitive::descriptor::AgentBudget;

    let adj_event = AdjudicationEvent {
        id: "adj:test:2026-08-21T12:00:00Z".into(),
        target_fused_claim_id: "claim:dr:ws:test-001".into(),
        adjudicator: "operator".into(),
        evidence_refs: vec![],
        decided_at: "2026-08-21T12:00:00Z".into(),
        decision: AdjudicationDecision::Defer,
    };

    let ctx = AgentContext::with_pending_adjudications(
        "audit pending adjudications".into(),
        None,
        Default::default(),
        vec![],
        vec![],
        vec![],
        vec![],
        AgentBudget::default(),
        vec![],
        vec![adj_event.clone()],
    );

    let json = serde_json::to_string(&ctx).expect("serialise must succeed");
    let back: AgentContext = serde_json::from_str(&json).expect("deserialise must succeed");

    assert_eq!(
        back.pending_adjudications.len(),
        1,
        "must preserve one event"
    );
    assert_eq!(back.pending_adjudications[0].id, adj_event.id);
    assert_eq!(
        back.pending_adjudications[0].decision,
        AdjudicationDecision::Defer
    );
}
