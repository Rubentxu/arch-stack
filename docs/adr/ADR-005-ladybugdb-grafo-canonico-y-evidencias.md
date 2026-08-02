# ADR-005 — LadybugDB como grafo canónico

**Estado:** Aceptado  
**Fecha:** 29 de julio de 2026  
**Sustituye:** ADR-005 anterior basado en SQLite

## Contexto

C4, casos de uso, clases y secuencias comparten relaciones y necesitan recorridos, caminos, evidencias y evolución temporal.

SQLite requeriría tablas manuales, CTE recursivas y reconstrucción del grafo en memoria. LadybugDB ofrece un grafo de propiedades tipado y embebido, consultable mediante Cypher.

## Decisión

Cada proyecto utilizará:

```text
architecture.lbdb
```

El grafo tendrá:

- catálogo de metatipos;
- catálogo de predicados;
- elementos canónicos;
- versiones de elementos;
- relaciones semánticas;
- versiones de relaciones;
- aristas materializadas;
- evidencias;
- snapshots;
- artefactos;
- ejecuciones.

## Esquema estricto

Se utilizará un subgrafo tipado, no un grafo abierto `ANY`.

La extensibilidad se implementará mediante `MetaType` y `Predicate`.

## Artefactos

PlantUML, Structurizr, Mermaid, draw.io y renders se guardan en el sistema de ficheros. LadybugDB conserva rutas, hashes y metadatos.

## Abstracción

El dominio de `archctl` dependerá de un trait:

```rust
pub trait GraphStore { /* ... */ }
```

La implementación inicial será `LbugStore` (adaptador sobre LadybugDB).

> **Nota de implementación (revisión 2026-08-01)**: el trait se
> llama `GraphStore` en el código (no `ArchitectureGraph`), y el
> adaptador es `LbugStore` (no `LadybugArchitectureGraph`). El nombre
> original del ADR se preserva como referencia histórica; los
> refactors `refactor-1b-filesystem-port`, `refactor-m9-debt-cleanup`
> (v0.9.1) y posteriores adoptaron el naming más corto. El contrato
> (port hexagonal + adapter LadybugDB) es idéntico al declarado en
> este ADR.

## Consecuencias positivas

- Recorridos y caminos nativos.
- Persistencia embebida.
- Cypher.
- C4 y UML sobre el mismo grafo.
- Menos lógica de grafo propia.

## Consecuencias negativas

- Dependencia nativa y migraciones.
- Restricciones de concurrencia entre procesos.
- El metamodelo semántico se valida en `archctl`.
- Se necesita una estrategia de exportación/importación.
