# Arch Stack — Architecture Intelligence Consolidation Pack

Este paquete convierte la auditoría técnica realizada sobre `main@518bb79d4c87a491fc901d54441de15e72c40bc2` en un
conjunto de decisiones, especificaciones y tareas implementables.

No es un reemplazo automático de `docs/ROADMAP.md`, `docs/adr/` ni `docs/specs/`.
Está diseñado para estudiarse, debatirse y después integrarse selectivamente mediante
PRs pequeños y reversibles.

## Tesis

Arch Stack tiene potencial para evolucionar de «herramienta que extrae y dibuja
arquitectura» a un **motor local-first de Architecture Intelligence**:

```text
source code
   │
   ▼
deterministic extractors
   │
   ▼
observations / evidence
   │
   ▼
canonical Architecture Evidence Graph
   │
   ├── C4 / UML / sequence / state / package projections
   ├── explain / provenance / confidence
   ├── architecture diff / history
   ├── fitness policies
   ├── task context for AI agents
   └── moldable Architecture Workbench
```

El valor diferencial no debe ser acumular formatos de diagrama. Debe ser que cada
afirmación arquitectónica pueda **rastrearse, explicarse, compararse y reutilizarse**
por humanos y agentes.

## Orden recomendado de lectura

1. [`00-EXECUTIVE-SUMMARY.md`](00-EXECUTIVE-SUMMARY.md)
2. [`01-ROADMAP-CONSOLIDATION.md`](01-ROADMAP-CONSOLIDATION.md)
3. [`02-TARGET-ARCHITECTURE.md`](02-TARGET-ARCHITECTURE.md)
4. [`04-MIGRATION-STRATEGY.md`](04-MIGRATION-STRATEGY.md)
5. ADRs 043–056 en [`adr/`](adr/)
6. Specs en [`specs/`](specs/)
7. [`03-IMPLEMENTATION-BACKLOG.md`](03-IMPLEMENTATION-BACKLOG.md)
8. Checklists en [`checklists/`](checklists/)

También se genera [`ALL-IN-ONE.md`](ALL-IN-ONE.md) como versión de lectura continua.

## Numeración ADR

El repositorio auditado contiene dos `ADR-040` y dos `ADR-041`. Este paquete **no
reescribe historia automáticamente**. Las nuevas propuestas empiezan en `ADR-043`.
La resolución de las colisiones existentes forma parte de P0 y está especificada
en `specs/adr-integrity-gate.md`.

## Principios preservados

- local-first;
- cero escritura en el código fuente;
- grafo canónico único;
- evidence/provenance como requisito de confianza;
- extractores deterministas antes que inferencia LLM;
- IA como consumidor/orquestador, no fuente de verdad opaca;
- workbench embebido, sin daemon obligatorio;
- adopción incremental y reversible;
- rendimiento medible.

## Fuera de alcance deliberadamente

No se propone ahora: SaaS central, colaboración multiusuario propia, RBAC de servidor,
vector DB como fuente de verdad, event sourcing completo, plataforma LLM genérica,
RAG genérico, microservicios ni una carrera por soportar decenas de renderers.
