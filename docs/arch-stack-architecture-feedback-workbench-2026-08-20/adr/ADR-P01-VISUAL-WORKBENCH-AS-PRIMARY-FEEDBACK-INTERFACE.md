# ADR-P01 — Visual Workbench as Primary Architecture Feedback Interface
**Status:** Proposed

## Context
Los informes textuales y las vistas independientes no cierran el ciclo de comprensión y feedback.

## Decision
Archview se convierte en la interfaz humana primaria de feedback arquitectónico. CLI/JSON continúan como superficies de automatización.

## Consequences
Positivo: contexto, evidencia y feedback comparten superficie. Negativo: UX visual pasa a ser product-critical y requiere UAT humano.

## Rejected
Markdown reports como UI primaria; pestañas de diagramas aisladas.
