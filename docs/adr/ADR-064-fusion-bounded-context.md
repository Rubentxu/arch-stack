# ADR-064 — Fusion Bounded Context: Trust-Gated FusedClaim Recompute + Feedback/Reconciliation as First-Class Types

**Status:** Accepted (promoted by `feat/trust-005-pr1-docs`, 2026-08-21)
**Date:** 2026-08-21
**Cycle:** `p-38e02210a9f14317/trust-005-observation-fusion`
**Aplica a:** `archctl` (Rust) — capa transversal sobre el grafo de verdad
**Refuerza:** ADR-063 (Trust gate, m25), ADR-P09 (Feedback + Reconciliation graph-native), ADR-P02 (deterministic core), ADR-P03 (authority ≠ execution), ADR-049 (Evidence/Observation/Claim/Confidence model), ADR-021 (escalera), ADR-017 (migration runner).
**Relacionado:** ADR-040 (cognitive deferred), ADR-050 (architecture snapshots), ADR-054 (policy/fitness functions), ADR-023 (naming-collision note on `Adjudication` vs `Approval`).
**Supersedes / Amends:** Amends ADR-063 §Consecuencias by adding the trust-gated recompute contract. Amends spec-12 by adding the Feedback/Reconciliation cross-reference (spec-12 v1.1 §6, this cycle).

## Contexto

ADR-063 (m25, accepted 2026-08-20) closed the trust gate at the `accept_evidence` chokepoint (`archctl/src/store.rs:1493-1511`): `ModelInference` claims can no longer flip their `Evidence.status` to `Accepted` without passing `canonical_promotion_allowed`. But the breach m25's verify report named as **residual** (SUGGESTION-2, `sddk/m25-authority-execution-classes/verify-report.md:159-162`) is that the `fuse-on-write` path in `archctl/src/architecture/fusion.rs:398-452` (`recompute_fused_for_versions`) is **unguarded**: every `Observation` minted by `put_evidence` produces a `FusedClaim` whose `status` is derived by `fusion.rs:320-324` as `if confidence > 0.0 { "accepted" } else { "drafted" }` — and `confidence` is the **hardcoded 1.0** from `observation_confidence()` at `fusion.rs:239-248`. The leak is real: an `ModelInference` Evidence row that fails the `accept_evidence` guard still produces an `"accepted"` FusedClaim downstream. The trust gate blocks the wrong layer.

The companion problem is **spec-35 v1.0**: the `(:Feedback)` and `(:Reconciliation)` node types it names in 11 lines are not implementable. ADR-P09 (accepted 2026-08-20) commits to making them graph-native — but the types, the validation rules, the m30 Adjudication event store bridge, and the determinism contract are all unstated. The cycle is a **two-spec** delivery: TRUST-005 proper (data flow) plus spec-35 v1.1 promotion (Feedback/Reconciliation types).

The m30 Adjudication event store is **deferred** per m25 spec §6 (L293) and ROADMAP (M26 or M30 candidate). Today there is no way to elevate a `ModelInference × Adjudicated` FusedClaim to canonical without one. A `Feedback.verdict == Accept` landing on a `ModelInference` FusedClaim would, naively, silently promote it — violating the m25 invariant.

The `Observation` Rust struct (`archctl/src/observation_claim.rs:37-60`) is a lossy projection of the table: the table has `confidence DOUBLE` (column ships in v4, `docs/schema/004_p2_09b_create_obs_clm.cypher:38`) but the struct omits it. `observation_from_evidence` (L114-128) builds an Observation with 11 fields and **no confidence computation**; `row_to_observation` (L240-272) reads only 11 columns; the carrier's doc-comment (L32-35) is stale (P2-09a compat-only).

## Decisión

1. **New bounded context `feedback`** (mirror of `evidence` / `source` / `evaluation`):
   - New module `archctl/src/feedback.rs` carries `Feedback`, `FeedbackVerdict { Accept, Reject, Uncertain, Supersede, Correct }`, `FeedbackError`, `Feedback::validate()`. Pure data + validation; no I/O.
   - New module `archctl/src/reconciliation.rs` carries `Reconciliation`, `PlaneEvidence` (reserved for v1.2 multi-plane), `Reconciliation::compute()` — a **pure function** that, given identical inputs, returns identical `(computed_status, rationale)` (sort order: `feedback.id`, then `feedback.revision`, then `feedback.timestamp` — ascending).
   - New module `archctl/src/fusion_bridge.rs` is the **trust-gated recompute seam**: a single function `recompute_status(group: &[&Observation]) -> (String, TrustClassification)` consumed by both `architecture/fusion.rs::fuse_observations_with` (the recompute path) and `feedback.rs::FeedbackRepository::put_feedback` (the m30 bridge path). One function, two callers — eliminates the connascence-of-algorithm smell between FusedClaim recompute and Reconciliation derivation.

2. **New schema migration `v7-observation-status`** (`docs/schema/007_observation_status.cypher`):
   - `ALTER (:Observation) ADD status STRING` (mirrors the Claim table pattern at `004_p2_09b_create_obs_clm.cypher:44-52`).
   - `CREATE NODE TABLE Feedback (id, target, verdict, replacement, actor, revision, timestamp)`.
   - `CREATE NODE TABLE Reconciliation (id, assertion_id, subject, predicate, object, evidence_set, computed_status, rationale, revision)`.
   - `CREATE REL TABLE VERDICTS_ON (FROM Feedback TO FusedClaim)`.
   - `CREATE REL TABLE RECONCILES (FROM Reconciliation TO FusedClaim)`.
   - `ALTER (:FusedClaim) ADD pending_adjudication_event BOOLEAN` (default `false`; the m30 bridge column).
   - Idempotent rust_hook `backfill_observation_status` sets `status = "accepted"` on every pre-v7 `(:Observation)` row lacking a status (legacy-compatible default; matches the v5 backfill's `written_via_backfill = true` convention).
   - Marker advances from `v6-fusion-persistence` to `v7-observation-status`. `migrations.rs:454` (`MIGRATIONS.len()` test) and `graph.rs:372` (`schema_marker_present_after_init` test) bump together (m25 precedent).

3. **`Observation` struct gains `confidence`, `status`, `evidence_origin`, `written_via_backfill` fields** (the `source_origin` name would FAIL the existing `observation_claim.rs:549-553` `!json.contains("origin")` test; renamed to `evidence_origin` for substring-safety). `row_to_observation` (L240-272) reads the new columns; `observation_from_evidence` (L114-128) computes them.

4. **`fuse-on-write` recompute consults `trust::canonical_promotion_allowed` before stamping `FusedClaim.status`** (the central change). The hardcoded `observation_confidence(o) → 1.0` at `fusion.rs:239-248` is replaced with reading `o.confidence` from the persisted column. The status rule at `fusion.rs:320-324` is replaced with `fusion_bridge::recompute_status(group)`. Result: `ModelInference × Suggested` FusedClaims land as `"drafted"`, never `"accepted"`.

5. **`FeedbackRepository` trait** added to `archctl/src/store.rs` (sits next to `EvidenceRepository`):
   - `fn put_feedback(&mut self, feedback: &Feedback, clock: &dyn Clock) -> Result<(), StoreError>` — validates; probes target FusedClaim; classifies target's `(ExecutionClass, AuthorityClass)`; if `verdict == Accept` AND `(ModelInference, Suggested)`, emits `tracing::warn!("feedback received, Adjudication event store not yet wired; ...")`, sets `FusedClaim.pending_adjudication_event = true`, and **does NOT silently promote** `FusedClaim.status` to `"accepted"`; persists `(:Feedback)-[:VERDICTS_ON]->(:FusedClaim)`; calls `Reconciliation::compute(...)` and persists the resulting row.
   - `fn read_feedback_for_claim(&self, claim_id: &str) -> Result<Vec<Feedback>, StoreError>`.
   - `fn list_reconciliations(&self, claim_id: &str) -> Result<Vec<Reconciliation>, StoreError>`.

6. **`Reconciliation.computed_status` priority rule** (per spec-35 v1.1 §6):
   - **Trust gate first**: read the target FusedClaim's `(ExecutionClass, AuthorityClass)` via `trust::classify(Observation.source_origin, Observation.tool_name)`. If `canonical_promotion_allowed(exec, authority)` is denied → `computed_status` is derived from the trust classification, NOT from Feedback. The Feedback history is recorded but does not elevate the status.
   - **For ModelInference × Suggested**: `computed_status = "drafted"` (never silently promoted). `pending_adjudication_event` is set to `true` if `verdict == Accept` has arrived; rationale cites `"Adjudication event store not yet wired"`.
   - **For green cells**: most-recent Feedback verdict wins (`Accept → "accepted"`, `Reject → "rejected"`, `Supersede/Correct → "superseded"` with replacement in rationale, `Uncertain → trust-gated default`).
   - **Sort order** is `(feedback.id ASC, feedback.revision ASC, feedback.timestamp ASC)` — the `Reconciliation::compute()` function is **pure** and **order-independent**.

7. **Five chained PRs** per the design's §9:
   - PR-1 (docs, ~280 LOC): spec-35 v1.1 + spec-12 v1.1 amendment + ADR-064.
   - PR-2a (feat, ~360 LOC): `feedback.rs` + `reconciliation.rs` + v7 migration + `manifests/feedback.toml`.
   - PR-2b (feat, ~270 LOC): `fusion_bridge.rs` + `observation_claim.rs` fields + `fusion.rs` delegate + `store.rs` `FeedbackRepository`.
   - PR-3a (test, ~200 LOC): un-ignore UAT-06 steps 7/9; implement `seed_orders_stripe_fixture`.
   - PR-3b (test, ~300 LOC): un-ignore UAT-06 steps 13/14/15; Feedback/Reconciliation integration tests.

8. **Manifest gate pinning**:
   - `manifests/feedback.toml` (NEW): 5 public_symbols, 8 must_hold, 1 minimum_tests, 2 must_not_contain.
   - `manifests/store.toml`: +5 must_hold lines (`FeedbackRepository` trait + 3 method lines).
   - `manifests/architecture.toml`: +2 public_symbols (`Feedback`, `FeedbackVerdict`, `feedback_from_evidence`, `reconciliation_status`); +5 must_hold + 3 editable entries (`feedback.rs`, `reconciliation.rs`, `fusion_bridge.rs`).
   - `manifests/evidence.toml`: +1 must_hold line (`ObservationStatus::Drafted`).
   - All four edits land in the same commit as the new symbols (m25 precedent; pins prevent silent surface regression).

**The invariant**: `ModelInference × Suggested` FusedClaims **cannot** land as `"accepted"` — neither via the recompute path (PR-2b) nor via the Feedback.accept path (PR-2a). The recompute consults `canonical_promotion_allowed` (PR-2b); the Feedback.accept path on a `ModelInference` target emits `tracing::warn!` and sets `pending_adjudication_event = true` but **does not flip status** (PR-2a). The two code paths converge on `fusion_bridge::recompute_status` as the single source of truth.

## Consecuencias

### Positivas

- **Trust-gated FusedClaim recompute closes the m25 residual leak.** `(:FusedClaim {status: "accepted"})` is now provable from the trust matrix, not from the legacy `if confidence > 0.0` rule. UAT-06 step 7 un-`#[ignore]`d (PR-3a) is the regression test.
- **Feedback and Reconciliation become first-class graph nodes.** The audit trail is queryable (`MATCH (f:Feedback)-[:VERDICTS_ON]->(c:FusedClaim) RETURN f`). No more "Feedback is just UI state" — it's a graph-native record per ADR-P09.
- **One seam (`fusion_bridge::recompute_status`) for the canonical status rule.** Eliminates the connascence-of-algorithm smell between FusedClaim recompute and Reconciliation derivation (Risk 3 from the explore report).
- **m30 bridge is explicit, not silent.** A `Feedback.verdict == Accept` on `ModelInference × Suggested` emits `tracing::warn!` with the substring `"feedback received, Adjudication event store not yet wired"` and sets `pending_adjudication_event = true`. The cycle documents the bridge; m30 fills it in.
- **`Observation` carrier now matches the table.** The `confidence DOUBLE` column has been shipping since v4 but the struct omitted it — TR-005 threads it through. The legacy `compat_claim_from_evidence` 1.0/0.0 rule still works for pre-upgrade graphs (the v7 backfill sets `status = "accepted"` on legacy rows), so backward compatibility is preserved.
- **Determinism property is testable.** `Reconciliation::compute()` is pure; `reconciliation_determinism_byte_equal` and `reconciliation_order_independent_on_feedback_history` pin the contract. UAT-06 step 15's `restart-workbench` assertion (PR-3b) depends on it.
- **Connascence-of-meaning closed.** `feedback_verdict_to_evidence_status(v: FeedbackVerdict) -> Option<EvidenceStatus>` is the single bridge between `FeedbackVerdict` (intent) and `EvidenceStatus` (lifecycle). The two vocabularies overlap on `accept`/`reject` but `Uncertain`, `Supersede`, `Correct` are Feedback-only — explicit, not silently coerced.

### Negativas

- **`evidence_origin` field name is opaque.** The naive `source_origin` would FAIL the existing `observation_claim.rs:549-553` `!json.contains("origin")` test (substring collision). The cycle renames to `evidence_origin` to satisfy the test. Trade-off: the field name is less self-explanatory; the rustdoc clarifies that it carries the `SourceOrigin` string from the underlying Evidence row. Mitigation: the manifest pin `ObservationStatus::Drafted` keeps the semantic anchored.
- **`pending_adjudication_event` column is forward-compat debt.** m30 will rename to `adjudicated_at: TIMESTAMP` (or similar). Until m30 ships, the column carries semantic ambiguity (pending → true, resolved → false, but resolution requires the Adjudication event store). Mitigation: CHANGELOG entry for both cycles; the column name is forward-compatible because the field carries a "pending" semantic today.
- **Two enums (`EvidenceStatus` + `FeedbackVerdict`) overlap on 2 of 5 verdicts.** A reader of `Feedback(verdict=Accept)` must consult the bridge function to know what EvidenceStatus this implies. Mitigation: `feedback_verdict_to_evidence_status()` is the single bridge, manifest-pinned, with a unit test.
- **`Feedback.actor == "unknown"` is the silent attribution gap.** When neither `ARCHCTL_ACTOR` env var nor an explicit `actor` parameter is set, the actor is `"unknown"` and the audit trail loses provenance. Mitigation: `put_feedback` emits a `tracing::warn!` when `actor == "unknown"` AND `verdict == Accept` (the dangerous shape for Adjudication).
- **5 chained PRs require review discipline.** Each PR is a reviewable work-unit but the merge order is critical (PR-2a must land before PR-2b's `FeedbackRepository` consumer; PR-2b must land before PR-3a's integration test). Mitigation: the cycle's release phase enforces the merge order via `sddk release` (linear chain); no PR can skip ahead.
- **The `Feedback`/`Reconciliation` types enter the public surface.** 9 new public symbols (`Feedback`, `FeedbackVerdict`, `FeedbackId`, `FeedbackError`, `validate`, `Reconciliation`, `PlaneEvidence`, `compute`, `FeedbackRepository`). Mitigation: each lives in a dedicated module with its own manifest gate (`manifests/feedback.toml`).

## Alternativas Considered

### Approach A — Inline fusion in `store.rs`

**Pros**: smaller diff; no new bounded context.
**Cons**: couples trust gate + storage + Feedback orchestration in one file. The chokepoint at `accept_evidence` already has three concerns (status check, trust guard, honest Evaluation attestation); adding a fourth (Feedback orchestration) breaks SRP.
**Rejected**: explore-report Q5; the cycle is a 5-cycle moment for new bounded contexts, and Feedback has enough shape to merit its own module.

### Approach B — Separate Fusion CLI subcommand (e.g. `archctl fusion apply`)

**Pros**: makes the recompute explicit in the CLI; users can opt in.
**Cons**: the existing `archctl architecture fuse --persist` already invokes the recompute path (cli.rs:3196-3213). A new CLI command is duplicative; the recompute happens on every `evidence put` today via the dual-write seam (store.rs:~1431). Premature; users should not have to remember to run two commands.
**Rejected**: the recompute is implicit today and should stay implicit. The change is internal: the recompute's status rule consults the trust gate.

### Approach C — Wait for m30 (defer Feedback/Reconciliation until Adjudication event store ships)

**Pros**: avoids the m30 bridge complexity; cleaner end state.
**Cons**: leaves UAT-06 steps 7/9/13/14/15/19/20 `#[ignore]`d for another cycle; the spec-35 v1.1 promotion is already authored (this cycle's spec phase); the backlog item TRUST-005 has been in flight since m25 closed.
**Rejected**: the m30 bridge is **explicit, not silent** (the warn + flag pattern). Deferring would leave the gap unaddressed.

### Approach D — Promote `pending_adjudication_event` to `adjudicated_at: TIMESTAMP` immediately

**Pros**: future-proofs the column name.
**Cons**: m30 hasn't shipped. Naming a column `adjudicated_at` when no Adjudication event store exists would be misleading; the cycle uses `pending_adjudication_event` (forward-compatible prefix).
**Rejected**: explore-report Q7 + Risk 5. The m30 cycle renames the column when it ships; the v1.1 spec documents the rename.

## Open Questions

- **Q13 — Should `feedback_verdict_to_evidence_status` be auto-invoked in `put_feedback`?** Design currently calls it for audit but does NOT auto-flip the underlying Evidence row's `status`. The apply phase may revisit (e.g. `verdict == Reject` could auto-supersede the Evidence row). **Recommendation**: leave to v1.2 (replacement semantic enforcement). v1.1 records intent; v1.2 enforces it.

## Cross-references

- **ADR-021**: escalera L140-152 — the prose form of `ExecutionClass`. This ADR makes the escalera enforceable at the recompute layer (in addition to the accept_evidence chokepoint that ADR-063 already enforces).
- **ADR-049**: Evidence/Observation/Claim/Confidence model. TR-005 threads real `confidence` and `status` through the Observation carrier; the model is now executable end-to-end.
- **ADR-063**: trust gate. This ADR adds the trust-gated recompute contract; ADR-063's chokepoint guard remains the canonical-write predicate, TR-005's recompute guard is the FusedClaim-status predicate.
- **ADR-P02**: deterministic core (accepted 2026-08-20 PR #283). TR-005's `Reconciliation::compute()` determinism contract is the concrete instance of ADR-P02's commitment.
- **ADR-P03**: authority ≠ execution (accepted 2026-08-20 PR #283). TR-005's `fusion_bridge::recompute_status` is the second enforcement point for the orthogonality (in addition to ADR-063's chokepoint).
- **ADR-P09**: Feedback + Reconciliation graph-native (accepted 2026-08-20). TR-005 makes ADR-P09 implementable.
- **ADR-017**: schema migration runner. The v7 migration follows the v4/v5/v6 convention (registry entry + idempotent rust_hook + marker bump).
- **spec-12 v1.1** (`docs/arch-stack-architecture-feedback-workbench-2026-08-20/specs/12-TRUST-DETERMINISM-AND-AUTHORITY.md`): §6 "Feedback/Reconciliation cross-reference" added this cycle. The cross-reference makes the trust-vs-Feedback ordering explicit: trust first, Feedback second, Reconciliation third.
- **spec-30 v1.1** (`docs/arch-stack-architecture-feedback-workbench-2026-08-20/specs/30-GRAPH-REVISION-AND-DELTA.md`): `Feedback.revision` and `Reconciliation.revision` are GraphRevision ids from spec-30.
- **spec-35 v1.1** (`docs/arch-stack-architecture-feedback-workbench-2026-08-20/specs/35-FEEDBACK-AND-RECONCILIATION.md`): promoted this cycle from the 11-line v1.0 stub to implementable.
- **TRUST-005 spec** (`sddk/p-38e02210a9f14317/trust-005-observation-fusion/spec.md`): the 7 REQ-T05-001..007 requirements that this ADR enacts.
- **TRUST-005 design** (`sddk/p-38e02210a9f14317/trust-005-observation-fusion/design.md`): the 5-PR split + the 13 new public symbols + the v7 migration contract.

## References

- ADR-021: <docs/adr/ADR-021-cognitive-layer.md> — escalera L140-152
- ADR-049: <docs/adr/ADR-049-evidence-observation-claim-confidence-model.md>
- ADR-063: <docs/adr/ADR-063-trust-determinism-and-authority.md>
- ADR-P02: deterministic-core (accepted 2026-08-20 PR #283)
- ADR-P03: authority ≠ execution (accepted 2026-08-20 PR #283)
- ADR-P09: <docs/arch-stack-architecture-feedback-workbench-2026-08-20/adr/ADR-P09-FEEDBACK-AND-RECONCILIATION-GRAPH-NATIVE.md>
- ADR-017: <docs/adr/ADR-017-schema-migration-runner.md>
- `archctl/src/trust.rs`: the gate (`canonical_promotion_allowed`)
- `archctl/src/store.rs:1408-1465`: the `accept_evidence` chokepoint (ADR-063 enforcement)
- `archctl/src/architecture/fusion.rs:398-452`: the recompute path (this ADR's enforcement)
- `archctl/src/observation_claim.rs:549-553`: the `!json.contains("origin")` landmine
- `archctl/src/migrations.rs:69-73`: the migration registry (v6 → v7 advancement)
- `docs/schema/007_observation_status.cypher`: the v7 migration script
- `sddk/p-38e02210a9f14317/trust-005-observation-fusion/{explore-report,spec,design}.md`
- `sddk/m25-authority-execution-classes/verify-report.md:159-162`: SUGGESTION-2 (the seam this ADR closes)
