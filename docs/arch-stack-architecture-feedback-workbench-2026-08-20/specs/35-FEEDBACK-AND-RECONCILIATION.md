# Spec — Feedback & Reconciliation

## Version 1.1 (2026-08-21)

> **Cycle:** `p-38e02210a9f14317/trust-005-observation-fusion` · **Phase:** specify · **Date:** 2026-08-21
> **Authority:** ADR-P09 (Feedback + Reconciliation are graph-native, accepted 2026-08-20) → ADR-063 (Trust gate, m25) → ADR-049 (Evidence/Observation/Claim/Confidence model) → ADR-P02 (deterministic core) → ADR-P03 (authority ≠ execution).
> **Companion specs:** spec-12 v1.1 (TRUST-DETERMINISM-AND-AUTHORITY, amended this cycle), spec-30 v1.1 (GRAPH-REVISION-AND-DELTA, unchanged from m25).
> **Migration path:** v1.0 (2026-08-20, 11-line stub) → v1.1 (this file) — see [§10 Migration: v1.0 → v1.1](#10-migration-v10--v11).

This is the **v1.1 implementable** version of the Feedback & Reconciliation spec. The v1.0 stub (2026-08-20) defined the *intent* in 11 lines; v1.1 fills in the field shapes, validation rules, determinism contract, and the m30 Adjudication event store bridge.

---

## 1. Glossary

| Term | Definition | Source |
|---|---|---|
| **`Feedback`** | A graph-native record of a human (or programmatic) verdict on a `FusedClaim` target. Persisted as `(:Feedback)` node with typed edge `(:Feedback)-[:VERDICTS_ON]->(:FusedClaim)`. Carries intent, not state. | this spec |
| **`FeedbackVerdict`** | The 5-entry intent enum: `accept, reject, uncertain, supersede, correct`. Maps to `EvidenceStatus` only where semantics align. | this spec |
| **`Reconciliation`** | A graph-native record deriving the `computed_status` of a target `FusedClaim` from the union of (a) its underlying `Evidence` set and (b) the `Feedback` history targeting it. Persisted as `(:Reconciliation)` node with typed edge `(:Reconciliation)-[:RECONCILES]->(:FusedClaim)`. | this spec |
| **`PlaneEvidence`** | The set of `Evidence` rows for a single evidence plane (e.g. static-analysis plane, runtime-trace plane, intent plane). v1.1 ships **one plane**; `planes: Vec<PlaneEvidence>` is reserved for v1.2. | this spec |
| **`pending_adjudication_event`** | A boolean flag on `FusedClaim` set to `true` when `Feedback.verdict == accept` lands on a `ModelInference`-origin FusedClaim and the m30 Adjudication event store is not yet wired. m30 renames to `adjudicated_at: TIMESTAMP`. | spec-12 v1.1 cross-ref |
| **`computed_status`** | The status string written by `Reconciliation::compute(...)`. Derived from the trust classification + Feedback history. Cannot contradict `trust::canonical_promotion_allowed`. | this spec |

---

## 2. Feedback record schema

The `Feedback` record carries human (or programmatic) intent on a `FusedClaim`. It is persisted as a `(:Feedback)` node in the semantic graph.

### 2.1 Field shape (v1.1)

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | `STRING` | yes | Namespaced id: `fdbk:<blake3(target + verdict + revision)>` |
| `target` | `STRING` | yes | The id of the target FusedClaim (`clm:fused:...`) |
| `verdict` | `STRING` (one of `accept`, `reject`, `uncertain`, `supersede`, `correct`) | yes | The intent verdict |
| `replacement` | `STRING?` | no | Optional replacement statement text. Valid only with `verdict ∈ {reject, supersede, correct}`. Contradicts `verdict == accept`. |
| `actor` | `STRING` | yes | The actor id (e.g. `"caller=alice"`, `"cli=caller"`, `"api:code-review-bot"`). Defaults to `"unknown"` when absent at the API layer; the persisted value is always present. |
| `revision` | `STRING` | yes | The graph revision id at the time of the verdict. Used for determinism ordering. |
| `timestamp` | `TIMESTAMP` (RFC 3339 string) | yes | The wall-clock time of the verdict. |
| `evidence` | `LIST<STRING>?` (carried in `props`) | no | Optional list of Evidence ids backing the verdict. v1.1 records only; v1.2 may validate. |
| `correlation_id` | `STRING?` (carried in `props`) | no | Forward-compat for spec-35 v1.2: thread all Feedback for one user session. v1.1 records; v1.2 queries. |

### 2.2 Graph persistence

```
(:Feedback {
    id: STRING,
    target: STRING,
    verdict: STRING,
    replacement: STRING?,
    actor: STRING,
    revision: STRING,
    timestamp: TIMESTAMP
})
    -[:VERDICTS_ON]-> (:FusedClaim {id: target})
```

The `evidence` and `correlation_id` fields are stored in `(:Feedback).props` (JSON carry, Evidence-style precedent; ADR-016-B3).

---

## 3. Reconciliation record schema

The `Reconciliation` record derives `computed_status` for a `FusedClaim` from the union of (a) the Evidence rows backing it and (b) the Feedback history targeting it.

### 3.1 Field shape (v1.1)

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | `STRING` | yes | Namespaced id: `recon:<blake3(assertion_id + revision)>` |
| `assertion_id` | `STRING` | yes | The id of the target FusedClaim |
| `subject` | `STRING` | yes | The subject of the assertion (e.g. `"Orders"`) |
| `predicate` | `STRING` | yes | The predicate of the assertion (e.g. `"calls"`) |
| `object` | `STRING` | yes | The object of the assertion (e.g. `"Stripe"` or `"PaymentProvider"`) |
| `evidence_set` | `LIST<STRING>` | yes | Evidence ids backing the target FusedClaim. v1.1: a flat list of evidence ids from the single static-analysis plane. |
| `computed_status` | `STRING` | yes | Derived status (see §5 below). One of `drafted, accepted, rejected, superseded, pending_adjudication`. |
| `rationale` | `STRING` | yes | Human-readable explanation of how `computed_status` was derived. Cites the trust classification + the most recent Feedback row (by `(id, revision, timestamp)` order). |
| `revision` | `STRING` | yes | The graph revision id at compute time. |

### 3.2 Graph persistence

```
(:Reconciliation {
    id, assertion_id, subject, predicate, object,
    evidence_set, computed_status, rationale, revision
})
    -[:RECONCILES]-> (:FusedClaim {id: assertion_id})
```

### 3.3 Reserved for v1.2 (NOT used in v1.1)

- `planes: Vec<PlaneEvidence>` — multi-plane reconciliation (static-analysis + runtime-trace + intent planes). v1.1 reserves the field in the Rust struct but `planes.len() == 1` always.

---

## 4. Determinism property

> **Given identical inputs (assertion, evidence_set, feedback_history), `Reconciliation::compute(...)` MUST produce identical output (computed_status, rationale, revision).**

The determinism contract is the same shape as ADR-049 D4 (best-effort + tracing::warn on failure). Specifically:

- **Sort order**: by `feedback.id` ascending, then `feedback.revision` ascending, then `feedback.timestamp` ascending.
- **Empty history**: produce a derived-only result based on `trust::canonical_promotion_allowed(exec, authority)`.
- **Byte-equal outputs**: two invocations with byte-identical inputs MUST produce byte-equal `(computed_status, rationale)` strings. Pinned by `reconciliation_determinism_byte_equal` unit test.
- **Order-independence**: Feedback history can be replayed in any order; the sort produces the same input sequence to the reconciliation logic.

This property is what makes the graph replayable across sessions (UAT-06 step 15's `restart-workbench` assertion depends on it).

---

## 5. Verdict semantics

The 5-entry `FeedbackVerdict` enum has the following intent semantics:

| Verdict | Maps to EvidenceStatus | Mutation? | Rationale |
|---|---|---|---|
| `accept` | `Accepted` (only when trust gate allows) | yes (when allowed) | "The target FusedClaim IS correct as stated." Triggers a Reconciliation row write. If the target FusedClaim is `ModelInference` × `Suggested`, emits `tracing::warn!` and sets `pending_adjudication_event = true` (m30 bridge; see spec-12 v1.1 §X). |
| `reject` | `Superseded` (only when trust gate allows) | yes (when allowed) | "The target FusedClaim is wrong." Triggers a Reconciliation row with `computed_status = "rejected"`. May carry a `replacement` text. |
| `uncertain` | none | no | "I cannot tell whether the target FusedClaim is right or wrong." Records the human's epistemic state; does not mutate `EvidenceStatus`. |
| `supersede` | `Superseded` | yes | "Replace the target FusedClaim with a new one." Carries a `replacement` text. Triggers a separate `supersede` path on the Evidence rows backing the target (v1.2; v1.1 records intent only). |
| `correct` | none | no | "The target FusedClaim is partially right; here's the corrected version." Records the `replacement` text as a new claim candidate; does NOT auto-supersede (v1.1 records; v1.2 enforces). |

### 5.1 Validation rules

- `verdict == accept` + `replacement.is_some()` → **CONTRADICTORY**, reject with `FeedbackError::ContradictoryFields`.
- `verdict == reject` + `replacement.is_some()` → valid (the canonical "false claim + corrected replacement" shape).
- `verdict ∈ {uncertain, supersede, correct}` + `replacement.is_none()` → valid.
- `target` must reference an existing FusedClaim (validated by `FeedbackRepository::put_feedback`).

### 5.2 Trust-aware accept (the m30 bridge)

`Feedback.verdict == accept` on a target FusedClaim that fails `trust::canonical_promotion_allowed(ExecutionClass, AuthorityClass)` MUST NOT silently promote the FusedClaim to `status = "accepted"`. Specifically:

- The system emits `tracing::warn!` with the substring `"feedback received, Adjudication event store not yet wired"` when the FusedClaim's classification is `(ModelInference, Suggested)`.
- The system sets `FusedClaim.pending_adjudication_event = true`.
- The Feedback row persists regardless (human intent is the audit trail).
- `FusedClaim.status` remains `"drafted"` until the m30 Adjudication event store wires the Adjudication event into the trust gate.
- See spec-12 v1.1 §X "Feedback/Reconciliation cross-reference" for the full invariant.

---

## 6. computed_status derivation rule

The `Reconciliation::compute(...)` function applies the following rules, in priority order:

1. **Trust gate first**: read the target FusedClaim's `(ExecutionClass, AuthorityClass)` via `trust::classify(Observation.source_origin, Observation.tool_name)`.
2. **`canonical_promotion_allowed` verdict**:
   - If denied for any reason (including `ModelInference × Suggested`) → `computed_status` is derived from the trust classification, NOT from Feedback. The Feedback history is recorded but does not elevate the status.
3. **Apply Feedback history** (sorted by `(id, revision, timestamp)`):
   - If the most recent Feedback is `verdict == accept` AND trust gate allows → `computed_status = "accepted"`. For `ModelInference` × `Suggested`, this is `pending_adjudication` (m30 bridge; see §5.2).
   - If the most recent Feedback is `verdict == reject` AND trust gate allows → `computed_status = "rejected"`.
   - If the most recent Feedback is `verdict ∈ {supersede, correct}` AND trust gate allows → `computed_status = "superseded"` (with `replacement` text in rationale).
   - If the most recent Feedback is `verdict == uncertain` OR no Feedback exists → `computed_status` is the trust-gate verdict (`"accepted"` or `"drafted"`).
4. **Rationale string** cites the trust classification + the most recent Feedback row by `(id, revision, timestamp)` ordering.

The function is **pure** (no I/O). It is invoked by `FeedbackRepository::put_feedback` after the Feedback row is persisted; the resulting `Reconciliation` row is also persisted.

---

## 7. Cross-references

- **ADR-P09**: this spec's authority ("Feedback + Reconciliation are graph-native, not just UI state").
- **ADR-063**: trust gate that runs *before* Feedback processing (m25 cycle).
- **ADR-049**: Evidence/Observation/Claim/Confidence model that provides the source data.
- **spec-12 v1.1** (`12-TRUST-DETERMINISM-AND-AUTHORITY.md`): the trust enforcement spec, amended this cycle with §X "Feedback/Reconciliation cross-reference". The cross-reference makes explicit: trust enforcement runs BEFORE Feedback processing; `Feedback.verdict=accept` on a row that failed `canonical_write_allowed` is an explicit error case (not silent); `Reconciliation.computed_status` is derived from Feedback history but cannot contradict the trust gate.
- **spec-30 v1.1** (`30-GRAPH-REVISION-AND-DELTA.md`): `Feedback.revision` and `Reconciliation.revision` are GraphRevision ids from spec-30.
- **m25 spec.md §3** (`m25-authority-execution-classes/spec.md`): the trust matrix that gates Feedback processing.
- **TRUST-005 spec** (`sddk/p-38e02210a9f14317/trust-005-observation-fusion/spec.md`): this spec's companion cycle.

---

## 8. Implementation seams (informational)

> **Note:** the implementation is out of scope for this spec (spec phase writes contracts only). The seams below are named so design and apply phases have anchors.

- **New Rust module**: `archctl/src/feedback.rs` (bounded context "feedback"). Carries `Feedback`, `Reconciliation`, `FeedbackVerdict`, `PlaneEvidence`, `FeedbackError`, validation helpers.
- **New Rust module**: `archctl/src/reconciliation.rs` (split for single-responsibility). Carries `Reconciliation::compute(...)`.
- **New trait**: `FeedbackRepository` in `archctl/src/store.rs` (next to `EvidenceOps`, `DiagramRepository`, `RawGraphQuery`). Methods: `put_feedback`, `read_feedback_for_claim`, `list_reconciliations`.
- **New manifest**: `manifests/feedback.toml` (5 public_symbols, 8 must_hold, 1 minimum_tests, 2 must_not_contain).
- **Schema migration**: v7 (if TRUST-005 design Q1 = A) — `docs/schema/007_observation_status.cypher` adds `status STRING` to Observation. Independent of spec-35 v1.1 (which ships without v7 if Q1 = B/C).
- **Manifest pin updates**: `manifests/store.toml` (+3 must_hold lines for new trait methods); `manifests/architecture.toml` (+2 public_symbols lines for `feedback_from_evidence`, `reconciliation_status`).

---

## 9. See also

- v1.0 (2026-08-20, 11-line stub) — see [§10 Migration: v1.0 → v1.1](#10-migration-v10--v11) for the diff.
- ADR-P09 (Feedback + Reconciliation are graph-native).
- spec-12 v1.1 (TRUST-DETERMINISM-AND-AUTHORITY, amended this cycle).
- spec-30 v1.1 (GRAPH-REVISION-AND-DELTA).
- TRUST-005 spec (`sddk/p-38e02210a9f14317/trust-005-observation-fusion/spec.md`).

---

## 10. Migration: v1.0 → v1.1

### 10.1 v1.0 (2026-08-20, 11-line stub)

The original v1.0 content (preserved verbatim for provenance per the workbench folder's append-only rule):

```markdown
# Spec — Feedback & Reconciliation

## Feedback
`id, target, verdict, replacement?, actor, revision, timestamp, evidence?, correlation_id?`.
Verdicts: accept, reject, uncertain, supersede, correct.

## Reconciliation
Assertion id o subject/predicate/object, evidence set por plano, computed status, rationale, revision.

## Determinism
Mismas assertions/evidence/feedback => mismo resultado.
```

### 10.2 v1.1 delta (this cycle)

The v1.1 promotion adds:

| Section | What changed | Why |
|---|---|---|
| Header | Added cycle, phase, date, authority chain, companion specs. | Traceability. |
| §1 Glossary | Added `Feedback`, `FeedbackVerdict`, `Reconciliation`, `PlaneEvidence`, `pending_adjudication_event`, `computed_status`. | New terms introduced by v1.1. |
| §2 Feedback record schema | Expanded from a comma-separated list to a full field table (id, target, verdict, replacement?, actor, revision, timestamp, evidence?, correlation_id?) with types, required-ness, and graph persistence shape. | v1.0's list was not implementable. |
| §3 Reconciliation record schema | Added the full field table (id, assertion_id, subject, predicate, object, evidence_set, computed_status, rationale, revision) with types and graph persistence shape. | v1.0's single-line description was not implementable. |
| §4 Determinism property | Expanded from one sentence to a full contract (sort order, empty history, byte-equal outputs, order-independence). | v1.0's "Mismas assertions/evidence/feedback => mismo resultado" was not pinned to a sort strategy or a unit test. |
| §5 Verdict semantics | Added full verdict table (5 entries × Maps/Does-not-Map columns) + validation rules + the m30 Adjudication event store bridge (`§5.2 Trust-aware accept`). | v1.0 listed the 5 verdicts without semantics. |
| §6 computed_status derivation rule | Added the priority-ordered rule (trust gate first → Feedback history application → rationale string). | v1.0 had no derivation rule. |
| §7 Cross-references | Added ADR-P09, ADR-063, ADR-049, spec-12 v1.1, spec-30 v1.1, m25 spec.md, TRUST-005 spec. | v1.0 had no cross-references. |
| §8 Implementation seams | Added informational anchors for design/apply (new modules, new trait, new manifest, schema migration v7). | v1.0 had no implementation anchors. |
| §9 See also | Added. | Discoverability. |
| §10 Migration | Added this section. | Provenance of the v1.0 stub. |

### 10.3 What v1.1 does NOT change

- The 5-entry verdict enum is preserved (`accept, reject, uncertain, supersede, correct`).
- The field names on Feedback are preserved verbatim (`id, target, verdict, replacement?, actor, revision, timestamp, evidence?, correlation_id?`).
- The determinism property direction is preserved (same inputs → same output); only the contract shape is filled in.

### 10.4 Deferred to v1.2

- Multi-plane reconciliation (`planes: Vec<PlaneEvidence>` populated, not reserved).
- `correlation_id` threading (the field exists in v1.1 but unused; v1.2 adds query support).
- `replacement` semantic enforcement (v1.1 records; v1.2 triggers an auto-supersede via a separate path).
- Cross-feedback consensus (3 accepts of the same FusedClaim = Adjudication event — this is the bridge to m30's Adjudication event store).
