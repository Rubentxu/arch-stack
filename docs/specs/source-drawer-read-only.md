# Spec: source-drawer-read-only (H1)

> **Horizon:** H1 — Utilidad humana
> **Cycle:** `m69-arch-stack-product-roadmap-convergence`
> **Status:** stub — full text in spec.md §2

## Purpose

A source drawer renders the source file referenced by a node's evidence
as read-only text. Resolves `file:line` securely (no path-traversal outside
project root) and offers "open in IDE" handoff.

## Public surface

- Source drawer component in archview
- `evidence_refs` → file path resolution
- IDE handoff via `$EDITOR` / configured command

## Capability contract

See `spec.md` §2 "Capability: source-drawer-read-only" for the full
Given-When-Then scenarios.

## Cross-references

- [ADR-005](adr/ADR-005-ladybugdb-grafo-canonico-y-evidencias.md) — evidence model
- [ADR-038](adr/ADR-038-one-product-five-invariants.md) — invariant 4 (cosmetic-only)
