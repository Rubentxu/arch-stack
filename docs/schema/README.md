# Esquema LadybugDB

Este directorio contiene el punto de partida del almacenamiento de `archctl`.

## Archivos

- `001_initial_schema.cypher`: tablas de nodos y relaciones.
- `metamodel-core.json`: tipos y predicados iniciales.

## Aplicación

```bash
lbug architecture.lbdb < 001_initial_schema.cypher
archctl schema seed metamodel-core.json
```

La ejecución real debe pasar por:

```bash
archctl db init
archctl db migrate
```

para registrar versión, backup y validación.

## Notas

- El esquema utiliza un grafo tipado.
- `Element` y `SemanticRelation` conservan identidades.
- Sus estados se almacenan en versiones.
- `SEMANTIC_EDGE` es un índice derivado.
- Los campos `JSON` requieren una versión de LadybugDB que soporte el tipo nativo.
- Antes de fijar una release de `archctl`, el DDL debe validarse contra la versión exacta de LadybugDB incluida.
