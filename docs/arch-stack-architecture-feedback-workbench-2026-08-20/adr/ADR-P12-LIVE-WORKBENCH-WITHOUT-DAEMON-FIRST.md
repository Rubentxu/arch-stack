# ADR-P12 — Live Workbench Without a Daemon First
**Status:** Accepted (2026-08-20)

## Acceptance
**Date:** 2026-08-20
**Decision-maker:** Maintainer (Rubentxu)
**Validation:** Maintainer personal validation of the 16-ADR-Pxx blueprint as a coherent roadmap.
**Trade-off:** Batch validation bypasses per-ADR triangulation against existing ADRs (021/022/023/038/039/040/054/056/062). Implementation cycles reconcile conflicts when discovered.
**Authority:** [ROADMAP integration table](../../ROADMAP.md#plan-vivo--architecture-feedback-workbench-paquete-2026-08-20)

## Decision
`archctl view --watch` + worker + GraphRevision/GraphDelta + polling HTTP.

## Reopen
Axum/Tokio/SSE/WebSocket o `archctld` sólo con necesidad medida.
