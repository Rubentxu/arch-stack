# Spec: lens-spec-entry-criteria (H3)

> **Horizon:** H3 — Moldabilidad demostrada
> **Cycle:** `m69-arch-stack-product-roadmap-convergence`
> **Status:** stub — full text in spec.md §2

## Purpose

A LensSpec abstraction is NOT added unless either (a) two concrete consumers
repeat the same lens translation logic, or (b) a measured user need (UAT
evidence, perf budget breach) demands abstraction.

## Public surface

- LensSpec ADR gate (2 consumers OR measured need required)
- Reversibility clause for abstraction rollback

## Capability contract

See `spec.md` §2 "Capability: lens-spec-entry-criteria" for the full
Given-When-Then scenarios.

## Cross-references

- [ADR-021](adr/ADR-021-cognitive-layer.md) — cognitive layer (conditional)
- [ADR-039](adr/ADR-039-renderer-reality-anti-roadmap.md) — anti-roadmap
- ADR-040 — reactivation trigger
