# ADR-P11 — Causal Journal, Not Event Sourcing Yet
**Status:** Accepted (2026-08-20)

## Acceptance
**Date:** 2026-08-20
**Decision-maker:** Maintainer (Rubentxu)
**Validation:** Maintainer personal validation of the 16-ADR-Pxx blueprint as a coherent roadmap.
**Trade-off:** Batch validation bypasses per-ADR triangulation against existing ADRs (021/022/023/038/039/040/054/056/062). Implementation cycles reconcile conflicts when discovered.
**Authority:** [ROADMAP integration table](../../ROADMAP.md#plan-vivo--architecture-feedback-workbench-paquete-2026-08-20)
**Note:** Self-deferred by design ("Not Event Sourcing Yet"). Acceptance records the decision to start with journal; reopen trigger documented.

## Decision
Journal append-only para causalidad/audit/subscriptions. Ladybug sigue como estado semántico canónico.

## Reopen
Sólo si replay parity se demuestra continuamente.
