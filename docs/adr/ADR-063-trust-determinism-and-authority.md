# ADR-063 — Trust, Determinism, and Authority

**Status:** Accepted (2026-08-20)
**Date:** 2026-08-20
**Cycle:** m25-authority-execution-classes
**Aplica a:** `archctl` (Rust) — capa transversal sobre el grafo de verdad
**Refuerza:** ADR-021 (escalera), ADR-022 (per-agent max), ADR-016 (D2), ADR-027 (evidence put)
**Relacionado:** ADR-023 (naming-collision note on `Adjudication` vs `Approval`), ADR-040 (cognitive deferred), ADR-054 (post-hoc reuse candidate)
**Supersedes / Amends:** Amends ADR-021 §Reglas by adding the typology + invariant below.

## Contexto

ADR-021 `### Escalera de resolución` L140-152 already specifies the epistemic ladder that distinguishes deterministic extraction, heuristic, and model inference. ADR-P02 (accepted 2026-08-20 PR #283) commits to "una afirmación deliberadamente falsa de un LLM nunca puede convertirse en observed fact." Today the live breach is in `archctl/src/store.rs:1408-1465` (the `accept_evidence` chokepoint): the function reads `props["status"]` but never `props["source_origin"]`, and the resulting `Evaluation` hardcodes `"user_accepted"` and `"archctl:lifecycle_v1"` regardless of whether a human was ever involved. Two CLI commands — `evidence put` followed by `evidence accept` — mint a canonical fact from an LLM claim and forge the audit trail in one step.

The orthogonal axis (what epistemic weight the fact carries) is specified by ADR-P03 but not yet enacted. The term `CanonicalObservedFact` exists in `architecture/12-…:27` but does not appear in code (the equivalent is an Evidence row with `status == Accepted` reaching `archctl/src/diagram/export.rs:109`). The producer → class mapping is at `architecture/12-…:16-24` and must be made executable (today it is prose).

## Decisión

1. **Two new enums** in a new module `archctl/src/trust.rs`:
   - `ExecutionClass { PureDeterministic, DeterministicHeuristic, ModelInference, HumanDecision }` (4 variants, ADR-021 escalera + the rungs it omits, ADR-P03).
   - `AuthorityClass { Observed, Derived, Suggested, Normative, Adjudicated }` (5 variants, ADR-P03).
2. **One single-purpose predicate** `canonical_write_allowed(exec, authority) -> Result<(), TrustViolation>` encodes the 4×5 matrix (see `archctl/src/trust.rs` rustdoc). Rows not on a green cell return a typed `TrustViolation` enum. This is the SINGLE source of truth for the question "may this evidence be promoted to canonical?".
3. **One chokepoint guard** at `LbugStore::accept_evidence` between the status check (L1429) and the flip (L1439). The guard re-derives `(Execution, Authority)` from `props["source_origin"]` + `props["tool_name"]` — do not trust a stamp — and calls `canonical_write_allowed`. Failure is `anyhow::bail!` with the typed `TrustViolation` in the error chain.
4. **Honest `Evaluation` attestation** at `store.rs:1457`: replace the hardcoded `"user_accepted"` / `"archctl:lifecycle_v1"` pair with `caller=<ARCHCTL_ACTOR>` (or `cli=caller` anonymous) and `archctl:lifecycle_v1:<invocation_path>`. The invocation path is set by `cli.rs` dispatch via a thread-local setter.
5. **Scoped fail-closed `from_props`** at `evidence.rs:171-179`: absent `status` returns `Drafted` ONLY when `source_origin` is absent or `ModelInference`; legacy rows with `UserWorkspace`/`UserInput`/`ToolOutput` still read `Accepted`. See **Open Question Q4** below.
6. **One new `SourceOrigin` variant** `ModelInference` at `evidence.rs:97-124`. `default_for_origin(ModelInference) = Drafted`. Stamped by future model-backed producers (none exist today; ADR-040).
7. **One manifest gate** `manifests/trust.toml` registers the 6 public symbols + 8 unit tests + 19 textual invariants + 6 prohibitions. `manifests/evidence.toml` and `manifests/store.toml` are unchanged.
8. **One behaviour spec** `specs/12-TRUST-DETERMINISM-AND-AUTHORITY.md` formalises the 7 REQ-M25-001..007 requirements.

**The invariant**: `ModelInference` jamás puede escribir `CanonicalObservedFact` (an Evidence row with `status == Accepted`) directamente. The matrix permits `ModelInference × Suggested` only — exactly what ADR-P02 wanted for *candidate visibility* — and every promotion to `Accepted` is gated by the matrix.

## Consecuencias

### Positivas

- **One predicate to audit.** `canonical_write_allowed` is the 2-input gate every canonical promotion must pass. UAT-06 `false_canonical_promotions: 0` is now provable rather than merely tested.
- **One chokepoint.** Path A's two-command breach is closed at `store.rs:~1437`. Path B (`link_semantic_edge`) is intentionally left untouched (see below).
- **Auditable actor.** No more forged `"user_accepted"` evaluations. The `Evaluation.evaluator` field records the actual CLI command path.
- **Executable doc.** `producer_mapping_matches_arch_doc` pins the 7-row table at `architecture/12-…:16-24` verbatim — a doc drift becomes a test failure.
- **Connascence of meaning closed.** `classify()` is the single derivation `SourceOrigin → (Execution, Authority)`. The stale rustdoc at `evidence.rs:185-187` (which promised a fourth `observed/derived/inferred/confirmed` vocabulary) is corrected.

### Negativas

- **Path B remains structurally open.** `link_semantic_edge` is not gated this cycle. 57 integration tests (`code_class_diagram.rs` 24 + `code_c4_discover.rs` 16 + `code_call_graph.rs` 12 + `code_sequence.rs` 3 + `code_state_machine.rs` 2) are downstream. The threat is not live (no model-backed caller exists; ADR-040). The deferral is documented in this ADR and revisited when the first model-backed writer is proposed.
- **6 new types enter the public surface.** `ExecutionClass`, `AuthorityClass`, `TrustClassification`, `TrustViolation`, `classify`, `canonical_write_allowed`. Mitigation: they live in a single dedicated module with its own manifest.
- **Q4 residual blast radius.** Scoped fail-closed `from_props` may alter bundle projection for any deployed graph whose Evidence rows have absent `status` AND non-workspace origin. No in-repo fixture exercises this case. **Maintainer must confirm before apply.**

## Alternativas Considered

### Approach A — Guard both Path A and Path B (full invariant)

Pros: closes the structural hole in `link_semantic_edge`; invariant holds by construction.
Cons: touches the `SemanticEdgeRepository` trait signature consumed by 4 extractors and 57 downstream integration tests; the threat it defends against does not yet exist (ADR-040); large hard-to-review diff.
**Rejected**: see explore §7.

### Approach C — Documentation-only (ADR amend + spec, no code)

Pros: satisfies the ROADMAP pre-condition cheaply.
Cons: leaves the live two-command breach open; UAT-06 stays red; ADR-P02's acceptance unmet.
**Rejected**: the ADR's job is to make the invariant *enforceable*, not to describe it.

## Open Question Q4

Scoped fail-closed `from_props` returns `Drafted` for absent `status` when `source_origin` is absent or `ModelInference`. Legacy rows with `UserWorkspace`/`UserInput`/`ToolOutput` still read `Accepted` (back-compat). **If any deployed graph has Evidence rows lacking `status` AND a non-workspace origin, this alters their bundle projection. No repo fixture exercises this case; the manifest gate does not cover it.** Maintainer confirmation required before apply (T4 acceptance depends on this confirmation; the apply phase will check for an `approval` record before T4 lands).

## Supersedes / Amends

- **Amends ADR-021** §Reglas by adding the typology + invariant above. The escalera (L140-152) is the prose form of `ExecutionClass`; this ADR makes it enforceable.
- **Amends ADR-022** "`Determinism level`" field by clarifying that it is per-agent *max*, not per-fact. `ExecutionClass` is per-fact. An LLM-class agent can still emit a fact whose ExecutionClass is `PureDeterministic` (it ran a Cypher query).
- **Naming collision note for ADR-023**: `AuthorityClass::Adjudicated` is intentionally distinct from ADR-023's `Approval`. The former elevates a fact to canonical weight; the latter permits a side effect on the world. They overlap in vocabulary but operate on different objects. ADR-023 is not modified; the disambiguation lives here.

## References

- ADR-021: `docs/adr/ADR-021-cognitive-layer.md` — escalera L140-152
- ADR-022: `docs/adr/ADR-022-agent-catalog.md` — per-agent determinism max
- ADR-023: `docs/adr/ADR-023-action-proposal-and-policy.md` — Approval (different object)
- ADR-040: `docs/adr/ADR-040-cognitive-conditional-activation.md` — model write path deferred
- ADR-054: `docs/adr/ADR-054-policy-and-fitness-functions.md` — post-hoc reuse candidate
- ADR-P02: deterministic-core (accepted 2026-08-20 PR #283)
- ADR-P03: authority ≠ execution (accepted 2026-08-20 PR #283)
- `architecture/12-TRUST-DETERMINISM-AND-AUTHORITY.md`: typology + producer table
- `sddk/m25-authority-execution-classes/{explore-report,proposal,spec}.md`
- `docs/ROADMAP.md:214-222` — Plan vivo pre-conditions
- `archctl/src/store.rs:1408-1465` — the chokepoint
- `archctl/src/evidence.rs:97-179` — `SourceOrigin` and `from_props`
- `archctl/src/trust.rs` — the gate
- `archctl/tests/uat_06_false_agent_claim.rs` — the verification
