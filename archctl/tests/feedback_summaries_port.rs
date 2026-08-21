//! Regression tests for `FeedbackRepository::summaries_for_claims` (REQ-T06-003, REQ-T07-006).
//!
//! Verifies:
//! - Empty input short-circuits to `Ok(vec![])` without a query.
//! - Ordering is deterministic: `(c.id ASC, f.revision ASC, f.timestamp ASC, f.id ASC)`.
//! - Feedback rows for non-requested claim ids are excluded from results.

use archctl::DiagramRepository;
use archctl::feedback::{Feedback, FeedbackSummary, FeedbackVerdict};
use archctl::store::{FeedbackRepository, GraphStore, LbugStore};
use tempfile::TempDir;

// ─────────────────────────────────────────────────────────────────────────────
// Helper functions (mirrors tests/uat_06_false_agent_claim.rs:30-40 pattern)
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

/// Seed a Feedback row on a given claim.
fn seed_feedback(store: &mut LbugStore, claim_id: &str, revision: &str, verdict: FeedbackVerdict) {
    let fb = Feedback {
        id: format!("fdbk:{claim_id}:{revision}"),
        target: claim_id.into(),
        verdict,
        replacement: None,
        actor: "tester".into(),
        revision: revision.into(),
        timestamp: "2026-08-20T12:00:00Z".to_string(),
        evidence: None,
        correlation_id: None,
    };
    store.put_feedback(&fb).expect("put_feedback");
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

/// SCN-T07-006a / SCN-T07-001a: empty claim id list returns Ok(vec![]) without dispatching a query.
#[test]
fn summaries_for_claims_empty_when_no_claims() {
    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);

    let result = store.summaries_for_claims(&[]);

    assert!(result.is_ok(), "empty input must not error");
    assert!(
        result.unwrap().is_empty(),
        "empty input must return empty vec"
    );
}

/// SCN-T07-006b / SCN-T07-001b: out-of-order insertion yields revision-ASC ordering per claim.
#[test]
fn summaries_for_claims_deterministic_ordering() {
    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);

    // Persist claims and capture their actual store-assigned IDs.
    let cid_a = persist_claim(&mut store, "claim-a-rev");
    let cid_b = persist_claim(&mut store, "claim-b-rev");

    // Insert in order rev3, rev1, rev2 for cid_a
    seed_feedback(&mut store, &cid_a, "rev3", FeedbackVerdict::Reject);
    seed_feedback(&mut store, &cid_a, "rev1", FeedbackVerdict::Accept);
    seed_feedback(&mut store, &cid_a, "rev2", FeedbackVerdict::Uncertain);

    // Insert in order rev2, rev3, rev1 for cid_b
    seed_feedback(&mut store, &cid_b, "rev2", FeedbackVerdict::Correct);
    seed_feedback(&mut store, &cid_b, "rev3", FeedbackVerdict::Reject);
    seed_feedback(&mut store, &cid_b, "rev1", FeedbackVerdict::Accept);

    let result = store
        .summaries_for_claims(&[&cid_a, &cid_b])
        .expect("query must succeed");

    // Must return 6 rows total
    assert_eq!(
        result.len(),
        6,
        "expected 6 rows for 2 claims × 3 revisions"
    );

    // Per-claim revision ASC: cid_a rows must be in rev1, rev2, rev3 order
    let a_rows: Vec<&FeedbackSummary> = result.iter().filter(|r| r.target == cid_a).collect();
    assert_eq!(
        a_rows
            .iter()
            .map(|r| r.revision.as_str())
            .collect::<Vec<_>>(),
        vec!["rev1", "rev2", "rev3"],
        "cid_a rows must be ordered by revision ASC"
    );

    // Per-claim revision ASC: cid_b rows must be in rev1, rev2, rev3 order
    let b_rows: Vec<&FeedbackSummary> = result.iter().filter(|r| r.target == cid_b).collect();
    assert_eq!(
        b_rows
            .iter()
            .map(|r| r.revision.as_str())
            .collect::<Vec<_>>(),
        vec!["rev1", "rev2", "rev3"],
        "cid_b rows must be ordered by revision ASC"
    );

    // Cross-claim contiguity: rows for each claim must appear in a single block.
    // GROUP BY c.id is guaranteed by ORDER BY c.id ASC.
    let a_indices: Vec<usize> = result
        .iter()
        .enumerate()
        .filter(|(_, r)| r.target == cid_a)
        .map(|(i, _)| i)
        .collect();
    let b_indices: Vec<usize> = result
        .iter()
        .enumerate()
        .filter(|(_, r)| r.target == cid_b)
        .map(|(i, _)| i)
        .collect();

    // All a indices should be contiguous (no gaps = no interleaving with b)
    let a_max = a_indices.iter().max().copied().unwrap_or(0);
    let a_min = a_indices.iter().min().copied().unwrap_or(0);
    assert_eq!(
        a_indices.len(),
        a_max - a_min + 1,
        "cid_a rows must be contiguous (no interleaving with cid_b)"
    );

    // All b indices should be contiguous
    let b_max = b_indices.iter().max().copied().unwrap_or(0);
    let b_min = b_indices.iter().min().copied().unwrap_or(0);
    assert_eq!(
        b_indices.len(),
        b_max - b_min + 1,
        "cid_b rows must be contiguous (no interleaving)"
    );
}

/// SCN-T07-006d / SCN-T07-001d: feedback rows for non-requested claim ids are excluded.
#[test]
fn summaries_for_claims_excludes_non_persisted_feedback() {
    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);

    // Persist claims and capture their actual store-assigned IDs.
    let cid_targeted = persist_claim(&mut store, "claim-targeted");
    let cid_other = persist_claim(&mut store, "claim-other");

    // Seed feedback on both claims
    seed_feedback(&mut store, &cid_targeted, "rev1", FeedbackVerdict::Reject);
    seed_feedback(&mut store, &cid_other, "rev1", FeedbackVerdict::Accept);

    // Query only the targeted claim
    let result = store
        .summaries_for_claims(&[&cid_targeted])
        .expect("query must succeed");

    assert_eq!(result.len(), 1, "must return exactly 1 row");
    assert_eq!(
        result[0].target, cid_targeted,
        "returned row must belong to the requested claim"
    );
}

/// SCN-T07-002b: invalid identifier in `claim_ids` returns `Err` with
/// `"summaries_for_claims"` and `"validation"` in the context chain; no
/// Cypher query is dispatched. Regression for the `let _ = …` validation
/// bug flagged in TRUST-007 verify-report (lens: spec-compliance).
#[test]
fn summaries_for_claims_rejects_invalid_identifier() {
    let tmp = TempDir::new().unwrap();
    let mut store = open_store(&tmp);

    // Identifier with a disallowed char (space). `validate_identifier`
    // rejects anything outside `[A-Za-z0-9_.:-]+`.
    let bad = "clm bad id";
    let result = store.summaries_for_claims(&[bad]);

    let err = result.expect_err("invalid identifier must surface Err");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("summaries_for_claims") && msg.contains("validation"),
        "error context must reference both `summaries_for_claims` and `validation`; got: {msg}"
    );
}
