# ADR-044 — Repositorios semánticos y frontera de query raw

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

`GraphStore` abstrae persistencia, pero ejecutar strings Cypher deja que el detalle
del motor cruce hacia application y complique sustituibilidad, contratos y
transacciones.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Separar `ArchitectureRepository`, `EvidenceRepository`, `ObservationRepository`,
`ProjectionRepository`, `SnapshotRepository`, `GraphReadModel` y `UnitOfWork`.
Conservar `RawGraphQuery` únicamente para administración/diagnóstico.

## Superficie propuesta

```rust
trait ArchitectureRepository { /* semantic operations */ }
trait GraphReadModel { /* traversal/impact/explain */ }
trait UnitOfWork { /* atomic mutation boundary */ }
trait RawGraphQuery { /* admin/debug only */ }
```

## Rationale y beneficios

DIP real, APIs orientadas a intención, tests por contrato y optimización interna de
Ladybug sin contaminar usecases.

## Costes y consecuencias negativas

Más interfaces; algunas consultas exploratorias necesitan read model flexible.





## Estrategia de migración

Adapters iniciales delegan al store actual. Migrar queries de usecases por slices.
Después estrechar visibilidad de raw query.

## Verificación y criterios de aceptación

- 0 Cypher en application;
- graph query CLI sigue disponible;
- contract tests fake/Ladybug;
- multi-write usa UnitOfWork.

## Alternativas consideradas

A) único GraphStore: fuga tecnología.
B) repository CRUD por entidad: mal fit para grafo.
C) CQRS/event sourcing: exceso.

## Referencias internas

`archctl/src/store.rs`, store transaction tests, ADR-005, ADR-017.

## Changelog

- 2026-08-13 | proposed | ADR-044 creado a partir de la auditoría de consolidación.
