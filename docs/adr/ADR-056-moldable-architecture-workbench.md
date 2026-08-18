# ADR-056 — Moldable Architecture Workbench y navegación semántica

> **Estado:** Deferido — 2026-08-18
> **Reopen trigger:** ≥2 consumers with LensSpec-translatable duplication (entry criterion per [ROADMAP §H3](../ROADMAP.md#h3--moldabilidad-demostrada)) OR a measured need (UAT evidence: ≥3 users reporting the same lens problem, OR perf p99 breach traceable to view-strategy variance).
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack

## Contexto

Archview ya tiene múltiples vistas. El paso útil no es añadir pestañas infinitas,
sino cambiar representación según entidad, escala y pregunta preservando identidad.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Semantic zoom bidireccional, action palette, capability-driven lens selection,
composición C4/sequence/class/source/evidence/impact/confidence y rationale de
auto-lens. Workspace persiste navegación, no semántica.



## Rationale y beneficios

Reduce necesidad de conocer notaciones y convierte el producto en entorno de
comprensión.

## Costes y consecuencias negativas

UX compleja; automatismo debe ser explicable/reversible.





## Estrategia de migración

Cross-view identity → action palette → semantic zoom → lens recommendation.
Mantener vistas actuales hasta paridad.

## Verificación y criterios de aceptación

- entidad cruza vistas;
- breadcrumbs;
- rationale auto-lens;
- back/forward estable;
- budget 10k nodes.

## Alternativas consideradas

A) tabs independientes: menor utilidad.
B) canvas libre: pierde semántica.
C) UI generada LLM: no determinista.

## Referencias internas

archview views, ADR-013/033/045.

## Changelog

- 2026-08-13 | proposed | ADR-056 creado a partir de la auditoría de consolidación.
