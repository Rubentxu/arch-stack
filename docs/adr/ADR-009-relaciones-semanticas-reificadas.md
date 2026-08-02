# ADR-009 — Relaciones semánticas reificadas y aristas derivadas

**Estado:** Aceptado (con **reificación diferida**, revisado 2026-08-01 — ver `docs/audits/2026-08-01-archctl-adr-vs-impl.md` §F2 + `sddk/m9-relations-decision/`)  
**Fecha original:** 29 de julio de 2026  
**Fecha de revisión:** 1 de agosto de 2026

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

## Decisión original (29 de julio de 2026)

La autoridad sería un nodo:

```text
SemanticRelation
```

Con enlaces:

```text
Element -[:REL_SOURCE]-> SemanticRelation
SemanticRelation -[:REL_TARGET]-> Element
SemanticRelation -[:RELATION_TYPE]-> Predicate
```

El estado temporal residiría en:

```text
RelationVersion
```

Además se mantendría:

```text
Element -[:SEMANTIC_EDGE]-> Element
```

como índice derivado para recorridos.

## Revisión (1 de agosto de 2026) — **reificación diferida**

El modelo reificado (`SemanticRelation` + `REL_SOURCE` + `REL_TARGET` +
`RELATION_TYPE` + `RelationVersion`) se mantiene **declarado en el
schema** (`docs/schema/001_initial_schema.cypher`) pero **no se escribe
desde el código de aplicación**. La implementación de `archctl` usa el
**modelo directo de aristas** (`Element -[:SEMANTIC_EDGE]-> Element` con
propiedades en el rel) por las siguientes razones:

1. **Rendimiento del escritor de call-graph.** El call-graph writer
   (`archctl/src/code/call_graph.rs::write_call_edge`) hace una sola
   operación `MATCH … MERGE … CREATE` por arista. El modelo reificado
   requeriría 3 round-trips por arista (insert `SemanticRelation`,
   insert `REL_SOURCE`, insert `REL_TARGET` + `RELATION_TYPE`) más un
   `RelationVersion` opcional.
2. **Proyección de sequence diagram.** La vista de sequence
   (`archctl/src/diagram/sequence.rs`) necesita leer `r.props`
   (call_kind, line, async) directamente en la arista. El modelo
   reificado requiere un hop extra a `RelationVersion` para acceder a
   esas propiedades.
3. **Schema sigue siendo válido.** Las tablas reificadas existen en
   `001_initial_schema.cypher` y están disponibles para una
   migración futura si la proyección de sequence cambia de
   requisitos, o si `archview` necesita navegar versiones de
   relaciones para diffs evolutivos.

### Invariantes relajados en el modelo implementado

- ~~Toda arista activa referencia una `SemanticRelation`.~~ El modelo
  actual usa `SEMANTIC_EDGE` directo. La identidad de la arista es
  `relation_id` (columna del rel) en lugar del nodo
  `SemanticRelation.id`.
- `SEMANTIC_EDGE` es reconstruible. ✅ (es la fuente de verdad actual).
- Las evidencias se enlazan a `Evidence`, no a `RelationVersion`
  (simplificación del modelo; ver ADR-005).
- Los diagramas referencian `Element.id` directamente, no
  `SemanticRelation.id`.

### Operaciones diferidas

```bash
archctl graph repair-index   # diferido a 1.x
archctl graph verify-index   # diferido a 1.x
```

### Cuándo revocar esta deferral

El modelo reificado se vuelve necesario si:
- `archview` necesita mostrar **diffs evolutivos de relaciones**
  (RelationVersion con `valid_from`/`valid_until`).
- `archview` necesita **filtrar por predicado** sin colapsar la
  arista a un solo row.
- El equipo necesita **multi-tenancy o RBAC por arista**
  (atributos de `RelationVersion` permitirían scoping sin tocar
  `SEMANTIC_EDGE`).

## Decisión actual (post-revisión)

**`archctl` v0.10+ implementa el modelo directo de aristas.**
El modelo reificado queda **reservado en el schema** para uso futuro.
La decisión de implementar la reificación es una **re-revisión de este
ADR** condicionada a los criterios de arriba.

## Consecuencias positivas

- Relaciones direccionables.
- Historia y evidencia completas.
- Recorridos eficientes.
- Agrupación de varias relaciones en una arista visual.

## Consecuencias negativas

- Duplicación controlada.
- Necesidad de validación y reparación.
- Más escrituras por actualización.
