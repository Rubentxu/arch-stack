# ADR-P03 — Separate Epistemic Authority from Execution Mechanism
**Status:** Accepted (2026-08-20)

## Acceptance
**Date:** 2026-08-20
**Decision-maker:** Maintainer (Rubentxu)
**Validation:** Maintainer personal validation of the 16-ADR-Pxx blueprint as a coherent roadmap.
**Trade-off:** Batch validation bypasses per-ADR triangulation against existing ADRs (021/022/023/038/039/040/054/056/062). Implementation cycles reconcile conflicts when discovered.
**Authority:** [ROADMAP integration table](../../ROADMAP.md#plan-vivo--architecture-feedback-workbench-paquete-2026-08-20)
**First cycle:** `authority-execution-classes` (TRUST-003 + TRUST-004, gated by UAT-06).

## Decision
Modelar `ExecutionClass` y `AuthorityClass` por separado.

## Rationale
Una heurística puede ser determinista y seguir siendo sólo Suggested; una decisión humana puede ser Normative sin ser un cálculo determinista.
