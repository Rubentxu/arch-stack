# Spec: durable-workspace-state (H1)

> **Horizon:** H1 — Utilidad humana
> **Cycle:** `m69-arch-stack-product-roadmap-convergence`
> **Status:** stub — full text in spec.md §2

## Purpose

Workspace state (camera, zoom, filters, selection) persists to XDG
(`~/.local/share/archctl/projects/<hash>/workspace.json`), NOT to localStorage.
This enables state restoration across ephemeral-port restarts.

## Public surface

- `archctl view` workspace persistence
- XDG state directory structure
- Corrupt-file recovery behavior

## Capability contract

See `spec.md` §2 "Capability: durable-workspace-state" for the full
Given-When-Then scenarios.

## Cross-references

- [ADR-004](adr/ADR-004-persistencia-externa-xdg.md) — XDG persistence
- [ADR-038](adr/ADR-038-one-product-five-invariants.md) — invariant 3 (XDG-only)
