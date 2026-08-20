# ADR-P01 — Visual Workbench as Primary Architecture Feedback Interface
**Status:** Accepted (2026-08-20)

## Acceptance
**Date:** 2026-08-20
**Decision-maker:** Maintainer (Rubentxu)
**Validation:** Maintainer personal validation of the 16-ADR-Pxx blueprint as a coherent roadmap.
**Trade-off:** Batch validation bypasses per-ADR triangulation against existing ADRs (021/022/023/038/039/040/054/056/062). Implementation cycles reconcile conflicts when discovered.
**Authority:** [ROADMAP integration table](../../ROADMAP.md#plan-vivo--architecture-feedback-workbench-paquete-2026-08-20)

## Context
Los informes textuales y las vistas independientes no cierran el ciclo de comprensión y feedback.

## Decision
Archview se convierte en la interfaz humana primaria de feedback arquitectónico. CLI/JSON continúan como superficies de automatización.

## Consequences
Positivo: contexto, evidencia y feedback comparten superficie. Negativo: UX visual pasa a ser product-critical y requiere UAT humano.

## Rejected
Markdown reports como UI primaria; pestañas de diagramas aisladas.
