# ADR-047 — CI preventiva en PR y evidencia post-merge

> **Estado:** Aceptado — 2026-08-13 (embodied in `.github/workflows/pr.yml` pre-merge fast gate + `.github/workflows/release.yml` post-merge quality gate + `scripts/verify-local.sh` local-first cheap mode)
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack

## Contexto

CI auditado prioriza push a main y hooks locales para prevención. Dependencias
nativas y múltiples contratos hacen posible que fallos específicos de runner entren
en main antes de detectarse.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Dos niveles: PR fast gate determinista; post-merge para E2E/benchmarks/corpus.
Release solo consume commit verde y compila Tier-1 en runner apropiado.



## Rationale y beneficios

Reduce main rojo sin convertir cada PR en benchmark largo. Branch protection
reproducible.

## Costes y consecuencias negativas

Más minutos CI y riesgo de flakiness; caching y separación mitigarán.





## Estrategia de migración

Añadir pull_request manteniendo post-merge. Observar duración un ciclo. Convertir
fast checks en required.

## Verificación y criterios de aceptación

- compile error bloquea PR;
- ADR duplicate falla rápido;
- benchmark largo fuera de PR normal;
- release no parte de commit rojo.

## Alternativas consideradas

A) hooks-only: no reproducible.
B) todo benchmark en PR: lento.
C) merge queue desde día 1: opcional posterior.

## Referencias internas

`.github/workflows/ci.yml`, `release.yml`, verify-local, ADR-025.

## Changelog

- 2026-08-13 | proposed | ADR-047 creado a partir de la auditoría de consolidación.
