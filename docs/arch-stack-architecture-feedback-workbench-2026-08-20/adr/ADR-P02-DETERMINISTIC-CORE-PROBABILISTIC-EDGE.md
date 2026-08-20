# ADR-P02 — Deterministic Core, Probabilistic Edge
**Status:** Accepted (2026-08-20)

## Acceptance
**Date:** 2026-08-20
**Decision-maker:** Maintainer (Rubentxu)
**Validation:** Maintainer personal validation of the 16-ADR-Pxx blueprint as a coherent roadmap.
**Trade-off:** Batch validation bypasses per-ADR triangulation against existing ADRs (021/022/023/038/039/040/054/056/062). Implementation cycles reconcile conflicts when discovered.
**Authority:** [ROADMAP integration table](../../ROADMAP.md#plan-vivo--architecture-feedback-workbench-paquete-2026-08-20)
**First cycle:** `authority-execution-classes` (TRUST-003 + TRUST-004, gated by UAT-06).

## Decision
Agentes con modelo emiten candidates/proposals. Canonical facts sólo provienen de observations, derivaciones deterministas o adjudicación explícita.

## Acceptance
Una afirmación deliberadamente falsa de un LLM nunca puede convertirse en observed fact.
