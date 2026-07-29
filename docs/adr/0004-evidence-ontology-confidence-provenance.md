# ADR-0004: Evidence Ontology and Confidence Provenance

- **Status**: Proposed
- **Date**: 2026-07-29
- **Decides**: How observations are classified and how certainty is represented and gated.

## Context

The documented failure mode of every prior architecture-drawing tool is that it *invents*
elements and presents them with false confidence. The source document's strongest idea is an
evidence-first ontology that separates what is grounded from what is inferred. Without an
explicit epistemic classification and a gating rule, the system would reproduce exactly that
failure.

However, the *calibration* of numeric confidence — how a 0.85 differs from a 0.7 in a
validated, reproducible way — has no established method. The source doc assigns numbers
without justification.

## Decision

**Every element and relationship carries:**

- `classification ∈ { fact, inference, hypothesis, unknown, conflict }` — the epistemic class.
- `confidence ∈ [0,1]` — a self-reported certainty, **always with provenance** (the `extractor`
  that produced it and the evidence it cites).
- `evidence: [evidenceId…]` — references into the append-only ledger.

**Hard invariant (non-negotiable quality gate):**

> An element or relationship with `confidence ≥ 0.9` (or classified `fact`) **and zero
> evidence references** is an **unsupported claim**. `unsupported_claims_high_confidence > 0`
> is a **HARD FAIL** — the pipeline aborts and ships no model.

**Severity policy by confidence × evidence:**

| confidence | has evidence? | class | action |
|---|---|---|---|
| High (≥ 0.9 / fact) | No | — | 🔴 HARD FAIL |
| Medium (0.6–0.89) | No | `unknown` | recorded, auditable, does not block |
| Low (< 0.6) | No | `hypothesis` | recorded, re-auditable, does not block |

**Epistemic fields are system-derived, never repo-supplied.** Repository strings (source,
docs, comments, README prose) are **untrusted DATA**. They cannot directly assign
`classification` or `confidence`, and they are never executed as instructions (no evaluation
of markdown/source content as prompts — only structural fields are consumed). The pipeline's
system rules plus corroborating evidence determine every epistemic field; prompt-injection-
shaped text in the repo is treated as evidence *about* the repo, not as a command to the
agents. This neutralizes the "malicious README sets `confidence=1.0`" class of attack.

**Confidence calibration is explicitly unresolved.** v1 uses heuristic assignment with full
provenance; the calibration *method* is an open Phase-1 experiment, not a solved design.

## Consequences

- **Positive**: A fact without evidence is an ontological contradiction the pipeline rejects
  — this is the project's reason to exist. The auditor role is the enforcement point.
- **Negative**: The numeric confidence is not yet calibrated; comparisons across extractors
  may be inconsistent until calibration is researched.
- **Neutral**: `classification` is the more trustworthy signal in v1; `confidence` is a
  secondary, provenance-bearing hint until calibration lands.

## Alternatives considered

- **No classification, raw confidence only** — rejected: a number without an epistemic class
  is exactly the invent-with-confidence failure mode.
- **Externally validated calibration from day one** — rejected as infeasible: no method
  exists yet; pretending one does would be dishonest. It is declared an open experiment.
- **Soft warning instead of hard fail** — rejected for high-confidence claims: the gate is
  what makes the output trustworthy; medium/low claims are already soft.
