# ADR-043 — Límites hexagonales modulares por capacidad

> **Estado:** Aceptado — 2026-08-13 (embodied in current port seams: `CliContext` composition root v1.43.0 p1-01, `repository` ports v1.43.0 p1-03, `store.rs` GraphStore/Cypher-boundary discipline)
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack

## Contexto

Arch Stack aplica inversión de dependencias en filesystem, store capabilities e IDE
adapters, pero el crecimiento ha concentrado orchestration, infraestructura y
presentación en módulos grandes. La arquitectura es hexagonal por intención, no aún
por restricción compilable.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Adoptar **Modular Hexagonal Architecture** por bounded capability. Primera etapa:
módulos + reglas de dependencia; extracción a Cargo crates solo tras estabilidad.

Boundaries objetivo: `arch-model`, `arch-analysis`, `arch-knowledge`,
`arch-projection`, `arch-workbench`, `arch-distribution`, `archctl` composition root
y `archview-contract`.



## Rationale y beneficios

Evita que `cli.rs` y `store.rs` sean hubs, permite tests aislados, reduce coupling
con librerías nativas/web y permite reutilización desde CLI, MCP y workbench.

## Costes y consecuencias negativas

Más DTO mappings y estructura. Si se extraen crates prematuramente aparece
fragmentación y cycles artificiales.



## No objetivos

No microservicios. No ocho crates de golpe. No cambiar DB. No rediseñar CLI
por estética.

## Estrategia de migración

1. dependency map/gates;
2. composition root;
3. usecases por familia;
4. ports semánticos;
5. reorganización modular;
6. crates solo con boundary estable.

## Verificación y criterios de aceptación

- CI detecta imports prohibidos;
- usecases testeables con fakes;
- CLI no crea stores/filesystems;
- lbug/tiny_http/reqwest no aparecen en domain/application;
- outputs públicos equivalentes.

## Alternativas consideradas

A) feature-only indefinido: erosiona límites.
B) carpetas globales domain/application: demasiado horizontales.
C) reescritura: riesgo inaceptable.

## Referencias internas

`archctl/src/cli.rs`, `store.rs`, `code/`, ADR-038, ADR-042.

## Changelog

- 2026-08-13 | proposed | ADR-043 creado a partir de la auditoría de consolidación.
