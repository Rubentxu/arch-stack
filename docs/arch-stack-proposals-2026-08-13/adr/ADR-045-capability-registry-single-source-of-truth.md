# ADR-045 — Capability Registry como fuente única de verdad

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

Feature/language support evoluciona a ritmos distintos. Call Graph, Class Diagram y
State Machine no tienen la misma matriz; comentarios/errores históricos pueden quedar
desincronizados de la implementación.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Crear `CapabilityRegistry` tipado/serializable con capability, language, maturity,
requirements, determinism, schema y availability. CLI, doctor, MCP, docs y archview
consumen el mismo contrato.

## Superficie propuesta

```json
{"capability":"code.call_graph","language":"kotlin","maturity":"beta",
 "deterministic":true,"schema":"call-graph-report/1"}
```

## Rationale y beneficios

Elimina matrices duplicadas, habilita feature negotiation, maturity visible y
extensión por plugins.

## Costes y consecuencias negativas

Riesgo de mega-config; debe describir capacidades, no wiring.





## Estrategia de migración

Registrar estado actual sin cambiar comportamiento; alignment tests; exponer
`archctl capabilities --json`; generar docs/tablas.

## Verificación y criterios de aceptación

- provider sin registry o viceversa falla;
- docs generadas limpias;
- archview habilita acciones desde registry.

## Alternativas consideradas

A) enums locales: drift.
B) reflexión automática: no expresa maturity.
C) YAML-only: posible, pero el core debe conservar typing.

## Referencias internas

`archctl/src/code/*`, cognitive MCP, manifests, docs/specs/index.md.

## Changelog

- 2026-08-13 | proposed | ADR-045 creado a partir de la auditoría de consolidación.
