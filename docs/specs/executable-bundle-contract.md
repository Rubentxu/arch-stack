# Spec: executable-bundle-contract (H0)

> **Horizon:** H0 — Ejecutable / verdad verificable
> **Cycle:** `m69-arch-stack-product-roadmap-convergence`
> **Status:** stub — full text in spec.md §2

## Purpose

The `viewer-bundle` JSON is the cross-language executable contract between
`archctl` and `archview`. `schemas/diagram-projection.schema.json` is the
single source of truth; Rust DTOs and TypeScript types must be field-aligned.

## Public surface

- `archctl diagram export --format viewer-bundle`
- `GET /api/export?selector=c4-context:<id>` (configurable selector)
- `schemas/diagram-projection.schema.json`

## Capability contract

See `spec.md` §2 "Capability: executable-bundle-contract" for the full
Given-When-Then scenarios.

## Cross-references

- [ADR-038](adr/ADR-038-one-product-five-invariants.md) — one product identity
- [ADR-019](adr/ADR-019-performance-budget.md) — TTFP budget
