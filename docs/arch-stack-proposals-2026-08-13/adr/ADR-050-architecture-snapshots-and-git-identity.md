# ADR-050 — Snapshots arquitectónicos ligados a identidad Git

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

Diff e historia necesitan estados coherentes. El estado actual por sí solo no
responde reproduciblemente qué cambió entre dos refs.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Snapshot lógico identificado por repo identity + commit SHA + schema version +
extractor-set digest. No event sourcing completo.



## Rationale y beneficios

Diff reproducible, forensics, contexto por commit y base de PR analysis.

## Costes y consecuencias negativas

Disco y retención; extractor/schema changes requieren compatibility metadata.





## Estrategia de migración

MVP create explícito; luego on-demand desde diff; retention configurable en XDG.

## Verificación y criterios de aceptación

- misma tuple → misma identity;
- schemas incompatibles requieren rebuild/migration;
- snapshots fuera del repo;
- GC conserva pins/recent.

## Alternativas consideradas

A) event sourcing: demasiado.
B) recalcular siempre worktrees: fallback lento.
C) JSON manual: sin lifecycle.

## Referencias internas

identity.rs, xdg.rs, cognitive/delta.rs, ADR-004/008.

## Changelog

- 2026-08-13 | proposed | ADR-050 creado a partir de la auditoría de consolidación.
