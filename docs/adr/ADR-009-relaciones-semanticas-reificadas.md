# ADR-009 — Relaciones semánticas reificadas y aristas derivadas

**Estado:** Aceptado  
**Fecha:** 29 de julio de 2026

## Contexto

Una arista property-graph puede tener propiedades, pero el sistema necesita que una relación arquitectónica posea:

- ID estable;
- versiones;
- varias evidencias;
- estado y confianza;
- referencias desde diagramas;
- agrupación;
- historial.

Las relationship tables de LadybugDB identifican implícitamente una relación por sus extremos, lo que no cubre por sí solo toda la identidad semántica necesaria.

## Decisión

La autoridad será un nodo:

```text
SemanticRelation
```

Con enlaces:

```text
Element -[:REL_SOURCE]-> SemanticRelation
SemanticRelation -[:REL_TARGET]-> Element
SemanticRelation -[:RELATION_TYPE]-> Predicate
```

El estado temporal reside en:

```text
RelationVersion
```

Además se mantiene:

```text
Element -[:SEMANTIC_EDGE]-> Element
```

como índice derivado para recorridos.

## Invariantes

- Toda arista activa referencia una `SemanticRelation`.
- La fuente, destino y predicado deben coincidir.
- `SEMANTIC_EDGE` es reconstruible.
- Las evidencias se enlazan a `RelationVersion`.
- Los diagramas referencian `SemanticRelation`.

## Operaciones

```bash
archctl graph repair-index
archctl graph verify-index
```

## Consecuencias positivas

- Relaciones direccionables.
- Historia y evidencia completas.
- Recorridos eficientes.
- Agrupación de varias relaciones en una arista visual.

## Consecuencias negativas

- Duplicación controlada.
- Necesidad de validación y reparación.
- Más escrituras por actualización.
