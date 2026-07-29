# ADR-0002: Neutral Architecture IR as Truth; Structurizr as C4 Projection

- **Status**: Accepted
- **Date**: 2026-07-29
- **Decides**: Which artifact is the single source of truth and how diagrams relate to it.
- **Accepted by**: orchestrator, per user directive on 2026-07-29.

## Context

The source design document is internally inconsistent about the canonical store: in one
section the neutral Architecture IR is "the model"; in another the Structurizr DSL
workspace is treated as the authoritative C4 source. This is a flagged ambiguity
(explore-report §8): two truths create a reconciliation problem the moment either changes.

The structural insight the source doc gets *right* is that tools which "draw a Mermaid"
invent architecture. The defence against that failure is a separation between an evidenced
neutral model and the rendered views derived from it.

## Decision

**The Architecture IR is the single source of truth.** A Structurizr `workspace.dsl` is a
**projection** of the IR — a pure, deterministic function `IR → DSL` — and is the
*canonical C4 projection*, but it is not the source.

- Projections are one-directional and pure: same IR ⇒ identical DSL, always.
- No consumer ever writes back to the IR except the Synthesizer.
- Mermaid is explicitly **non-canonical** (C4 syntax is experimental) — preview only.

## Consequences

- **Positive**: One truth eliminates reconciliation bugs; adding a renderer never threatens
  the model; the IR carries epistemic metadata (`classification`, `confidence`, evidence
  refs) that no DSL natively expresses.
- **Negative**: Structurizr DSL round-trip editing is not supported (editing the DSL would
  diverge from the IR). Human edits happen on the IR or a dedicated editable projection
  (draw.io), then re-derive.
- **Neutral**: Structurizr remains the canonical *projection* choice because it is
  model-as-code, versionable, and C4-native.

## Alternatives considered

- **Structurizr DSL as the source** — rejected: DSLs cannot natively express the evidence
  ontology (`fact/inference/...`, confidence, evidence refs); the epistemic core would be
  lost.
- **Mermaid as canonical** — rejected: Mermaid C4 is experimental and incomplete; layout is
  non-deterministic; it is a preview, not a source.
- **Multiple co-equal sources** — rejected: the exact ambiguity being resolved.
