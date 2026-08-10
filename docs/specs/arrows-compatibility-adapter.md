# Spec: arrows-compatibility-adapter (H2)

> **Horizon:** H2 — Editor visual
> **Cycle:** `m69-arch-stack-product-roadmap-convergence`
> **Status:** stub — full text in spec.md §2

## Purpose

An `.arrows` import/export adapter for Arrows.app interchange. Imports produce
a Projection bundle without mutating the canonical graph; exports produce a
`.arrows` file readable by Arrows.app.

## Public surface

- `archctl import arrows <file>`
- `archctl export arrows <bundle>`

## Capability contract

See `spec.md` §2 "Capability: arrows-compatibility-adapter" for the full
Given-When-Then scenarios.

## Cross-references

- [ADR-007](adr/ADR-007-modelos-y-renderizadores-de-diagramas.md) — projections
- [ADR-038](adr/ADR-038-one-product-five-invariants.md) — invariant 1 (canonical graph)
