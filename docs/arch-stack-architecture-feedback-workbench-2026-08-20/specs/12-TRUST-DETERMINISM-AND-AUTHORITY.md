# Spec — Trust, Determinism & Authority

## Version 1.1 (2026-08-21)

> **Cycle:** `p-38e02210a9f14317/trust-005-observation-fusion` · **Phase:** specify · **Date:** 2026-08-21
> **Companion spec:** `specs/35-FEEDBACK-AND-RECONCILIATION.md` v1.1 (promoted this cycle).
> **Amendment:** §6 "Feedback/Reconciliation cross-reference" added below (3 paragraphs). Glossary table extended with 4 entries. Cross-references list extended with spec-35 v1.1.

**Migration: v1.0 → v1.1.** v1.0 (2026-08-20) defined the trust matrix, the canonical-write gate, and the Adjudication term. v1.1 (this amendment) cross-references the Feedback and Reconciliation types introduced by spec-35 v1.1 and makes the trust-vs-Feedback ordering explicit. The matrix, gate, and Adjudication term are unchanged.

## Purpose

Codificar el contrato de promoción canónica del grafo de verdad:
qué productores pueden escribir `CanonicalObservedFact` y bajo
qué autoridad. ADR-063. ADR-P02. ADR-P03.

## Glossary

| Term | Definition |
|---|---|
| `CanonicalObservedFact` | Evidence row with `status == Accepted` reaching `archctl/src/diagram/export.rs:109` filter. NOT a new Rust type. Maps to ADR-021's "observed fact" in prose. |
| `ExecutionClass` | How a producer computed an answer (4 variants). |
| `AuthorityClass` | The epistemic weight an answer carries (5 variants). |
| `Adjudication` | An explicit human verdict that elevates a row from `Suggested` or `Normative` to `Adjudicated`. Distinct from ADR-023's `Approval` (different object — facts vs side effects). |
| `Canonical-write gate` | The function `trust::canonical_write_allowed(exec, authority) -> Result<(), TrustViolation>`. The single 2-input predicate every transition to `status == Accepted` must pass. |
| `Feedback` | A graph-native record of a human (or programmatic) verdict on a `FusedClaim` target. Persisted as `(:Feedback)` node with typed edge `(:Feedback)-[:VERDICTS_ON]->(:FusedClaim)`. See spec-35 v1.1 §2. |
| `Reconciliation` | A graph-native record deriving the `computed_status` of a target `FusedClaim` from the union of its `Evidence` set and the `Feedback` history targeting it. Persisted as `(:Reconciliation)` node with typed edge `(:Reconciliation)-[:RECONCILES]->(:FusedClaim)`. See spec-35 v1.1 §3. |
| `FeedbackVerdict` | The 5-entry intent enum on Feedback (`accept, reject, uncertain, supersede, correct`). See spec-35 v1.1 §5. |
| `pending_adjudication_event` | A boolean flag on `FusedClaim` set to `true` when `Feedback.verdict == accept` lands on a `ModelInference`-origin FusedClaim and the m30 Adjudication event store is not yet wired. The bridge from spec-35 v1.1 §5.2 to the future m30 cycle. |

## Producers (transcribed from `architecture/12-…:16-24`)

| Producer | SourceOrigin | tool_name | Execution | Authority |
|---|---|---|---|---|
| Tree-sitter | UserWorkspace | "tree-sitter" | PureDeterministic | Observed |
| SCIP | ToolOutput | "scip" | PureDeterministic | Observed |
| SCC | ToolOutput | "scc" | PureDeterministic | Derived |
| naming heuristic | ToolOutput | "naming_heuristic" | DeterministicHeuristic | Suggested |
| LLM analyst | ModelInference | "llm_analyst" | ModelInference | Suggested |
| ADR accepted | UserInput | "adr_accepted" | HumanDecision | Normative |
| reject/correction | UserInput | "human_adjudication" | HumanDecision | Adjudicated |

## Invariant

`ModelInference` no escribe directamente `CanonicalObservedFact`.
La matriz 4×5 codifica la excepción: `ModelInference × Suggested`
es la única celda verde para `ModelInference` (visibilidad de
candidatos, ADR-P02).

## Cross-references

- ADR-063: this spec's authority
- `archctl/src/trust.rs`: implementation
- `archctl/tests/uat_06_false_agent_claim.rs`: UAT-06 critical gate
- `sddk/m25-authority-execution-classes/spec.md` §3: Given-When-Then
- `sddk/m25-…/spec.md` §4: UAT-06 step-by-step
- `specs/35-FEEDBACK-AND-RECONCILIATION.md` v1.1: Feedback + Reconciliation types; see §6 below for the cross-reference invariant
- `specs/30-GRAPH-REVISION-AND-DELTA.md` v1.1: `revision` field on Feedback + Reconciliation

## See also

- `specs/30-GRAPH-REVISION-AND-DELTA.md` Version 1.1 — what changes in the graph revision/delta model when this spec lands.

---

## 6. Feedback/Reconciliation cross-reference

Trust enforcement runs **before** Feedback processing. The `accept_evidence` chokepoint in `archctl/src/store.rs` consults `canonical_write_allowed(ExecutionClass, AuthorityClass)` (and `canonical_promotion_allowed` for the stricter promotion predicate) before flipping any Evidence row's `status` to `Accepted`. The `put_feedback` chokepoint — added by TRUST-005 in `archctl/src/feedback.rs` — sits *downstream* of the trust gate: Feedback is the record of human intent, but the trust gate has already decided whether that intent can result in canonical elevation. The ordering is invariant: trust first, Feedback second, Reconciliation third. A `Feedback` row that arrives on a target whose underlying Evidence was already rejected by `canonical_write_allowed` is recorded for audit but cannot elevate the FusedClaim's `status` to `"accepted"`.

A `Feedback.verdict == accept` that lands on a row whose FusedClaim's `(ExecutionClass, AuthorityClass)` classification fails `canonical_write_allowed` is an **explicit error case, not a silent promotion**. Specifically: when the target FusedClaim is `ModelInference` × `Suggested`, the system MUST NOT silently flip `FusedClaim.status` to `"accepted"`. Instead, the system MUST (a) emit a `tracing::warn!` event whose message includes the substring `"feedback received, Adjudication event store not yet wired"` (the m30 bridge; see spec-35 v1.1 §5.2), and (b) set `FusedClaim.pending_adjudication_event = true` so that future cycles can detect the unresolved state. The Feedback row itself persists (human intent is the audit trail); the FusedClaim's `status` field remains at its trust-gated value until the m30 Adjudication event store wires an Adjudication event into the trust gate.

`Reconciliation.computed_status` is *derived from* the Feedback history but cannot *contradict* the trust gate. The `Reconciliation::compute(...)` function (added by TRUST-005 in `archctl/src/reconciliation.rs`) reads the target FusedClaim's `(ExecutionClass, AuthorityClass)` classification and the Feedback history (sorted by `(id, revision, timestamp)`), and applies a priority-ordered rule: trust gate first, Feedback history second, rationale string cites both. If the trust gate denies the classification, the resulting `computed_status` is derived from the trust verdict (`"drafted"` or `"pending_adjudication"`), NOT from the Feedback history. The most recent Feedback's intent is recorded in the `rationale` field for audit but does not elevate `computed_status`. This is the same invariant m25 enforced at `accept_evidence`: `canonical_write_allowed` is the gate; nothing downstream can override it. The m30 Adjudication event store is the only future path that can elevate a `ModelInference` FusedClaim to `Adjudicated`; until m30 ships, the trust gate holds and Feedback intent is recorded but not authoritative.
