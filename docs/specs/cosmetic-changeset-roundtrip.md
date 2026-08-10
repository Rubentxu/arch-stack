# Spec: cosmetic-changeset-roundtrip (H2)

> **Horizon:** H2 — Editor visual
> **Cycle:** `m69-arch-stack-product-roadmap-convergence`
> **Status:** stub — full text in spec.md §2

## Purpose

A cosmetic ChangeSet (move-member / collapse-group / set-label) round-trips
through `archctl diagram apply` with `baseRevision` integrity validation.
Undo/redo works via inverse ChangeSets applied in sequence.

## Public surface

- `archctl diagram apply --changes <file>`
- `baseRevision` field in ChangeSet JSON
- Inverse ChangeSet application for undo

## Capability contract

See `spec.md` §2 "Capability: cosmetic-changeset-roundtrip" for the full
Given-When-Then scenarios.

## Cross-references

- [ADR-013](adr/ADR-013-viewer-ortogonal.md) — ChangeSet contract
- [ADR-038](adr/ADR-038-one-product-five-invariants.md) — invariant 4 (cosmetic-only apply)
- [ADR-019](adr/ADR-019-performance-budget.md) — performance budget
