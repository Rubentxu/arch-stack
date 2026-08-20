# ADR-P04 — Polyglot Local Persistence by Responsibility
**Status:** Accepted (2026-08-20)

## Acceptance
**Date:** 2026-08-20
**Decision-maker:** Maintainer (Rubentxu)
**Validation:** Maintainer personal validation of the 16-ADR-Pxx blueprint as a coherent roadmap.
**Trade-off:** Batch validation bypasses per-ADR triangulation against existing ADRs (021/022/023/038/039/040/054/056/062). Implementation cycles reconcile conflicts when discovered.
**Authority:** [ROADMAP integration table](../../ROADMAP.md#plan-vivo--architecture-feedback-workbench-paquete-2026-08-20)

## Decision
Ladybug canonical graph; JSONL causal journal; Tantivy lexical search; in-memory TreeCache; workspace JSON; optional rebuildable vector index.

## Rule
No segunda base canónica.
