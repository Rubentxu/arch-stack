# ADR-053 — Architecture Diff como capability first-class

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

Ya existen `cognitive/delta.rs`, DriftView e ImpactView. Falta un contrato público
unificado para convertirlos en change intelligence reusable.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Usecase independiente de UI: baseline vs target, cambios de elements, relations,
confidence, policies, cycles, boundaries y evidence. CLI/MCP/DriftView consumen el
mismo DTO.

## Superficie propuesta

```bash
archctl architecture diff main..HEAD --format json
```

## Rationale y beneficios

Alto valor en PRs y reutiliza piezas existentes en vez de duplicarlas.

## Costes y consecuencias negativas

Extractor/schema changes pueden crear ruido; report debe marcar comparabilidad.





## Estrategia de migración

Refactor delta a output versionado; snapshot provider; CLI; DriftView; luego
annotations.

## Verificación y criterios de aceptación

- stable JSON;
- cosmetic changes fuera;
- evidence refs;
- policy regressions separadas;
- output ordenado determinista.

## Alternativas consideradas

A) diff projections: pierde semántica.
B) source diff: no arquitectura.
C) DB temporal por branch: innecesario.

## Referencias internas

cognitive/delta.rs, DriftView, ImpactView, ADR-050.

## Changelog

- 2026-08-13 | proposed | ADR-053 creado a partir de la auditoría de consolidación.
