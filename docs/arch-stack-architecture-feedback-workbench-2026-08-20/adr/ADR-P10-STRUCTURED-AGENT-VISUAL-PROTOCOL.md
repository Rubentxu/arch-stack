# ADR-P10 — Structured Agent ↔ Visual Protocol
**Status:** Accepted (2026-08-20)

## Acceptance
**Date:** 2026-08-20
**Decision-maker:** Maintainer (Rubentxu)
**Validation:** Maintainer personal validation of the 16-ADR-Pxx blueprint as a coherent roadmap.
**Trade-off:** Batch validation bypasses per-ADR triangulation against existing ADRs (021/022/023/038/039/040/054/056/062). Implementation cycles reconcile conflicts when discovered.
**Authority:** [ROADMAP integration table](../../ROADMAP.md#plan-vivo--architecture-feedback-workbench-paquete-2026-08-20)

## Decision
Agentes emiten `VisualRequest`/structured outputs. Un Visual Compiler determinista decide query/projection/layout/renderer.

## Rejected
Agentes emitiendo pixels, coordenadas o HTML arbitrario como contrato primario.
