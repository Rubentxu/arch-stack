# ADR-017 — Schema migration runner + SourceArtifact identity

> **Ciclo:** `b1-source-evaluation-types`
> **Estado:** Aceptado
> **Fecha:** 2026-07-30
> **Complementa:** [ADR-016 §Bloque B1](../ADR-016-activegraph-packs-investigacion.md) + [ADR-005](ADR-005-ladybugdb-grafo-canonico-y-evidencias.md)

## Problema

`archctl` tenía el schema de LadybugDB codificado en dos sitios (`graph.rs` + `store.rs`) como un `BOOTSTRAP_VERSION` constante + lógica de script inline. Añadir nuevas tablas (`Evaluation`) requería modificar ambos sitios de forma coordinada y manual. No había forma de aplicar migraciones incrementales ni de hacer rollback.

Además, `Evidence.props` no persistía `source_origin` (D4), y el `SourceArtifact` node type existía en el schema pero no se creaba desde Rust — dejando `EXTRACTED_FROM` huérfano.

## Decisiones

### D1 — Migration runner en la capa Session, no en GraphStore

**Elección:** Se añade `archctl/src/migrations.rs` con `apply_pending(session, fs, marker) -> Vec<String>` que:
1. Lee el marker `<project>/.archctl-schema`
2. Aplica todo `.cypher` cuyo `version > marker`
3. Escribe el nuevo marker solo si todos los statements aplican

Ambos `graph::init` y `LbugStore::init` llaman a `apply_pending`. El `BOOTSTRAP_VERSION` se elimina de `graph.rs` y `store.rs`.

**Alternativas rechazadas:**
- Añadir `GraphStore::execute_raw(cypher)` + runner sobre el port → rejected: el port es para dominio, no para bootstrap; migrations runs antes de que el port sea usable.
- Runner en el CLI → rejected: el runner debe ser transparente a cualquier caller.

**Rationale:** migrations son infraestructura de bootstrap, no capacidad de dominio. Mantenerlas en la capa `Session` (donde ya corre `001`) preserva D6 ("extender GraphStore para writes de dominio, no para plumbing").

### D2 — SourceArtifact identity = relative_path + content_hash

**Elección:** `SourceArtifact.id = "src:" + blake3(relative_path + content_hash)`. El `content_hash` es el SHA-256 ya calculado por `evidence::content_hash_of`. `blake3` se usa solo para el `id`.

**Alternativas rechazadas:**
- blake3 del contenido → rejected: `Filesystem` solo tiene `read_to_string` (UTF-8), no `read_bytes`. Re-hashing re-lee el archivo y duplica trabajo ya hecho por `content_hash_of`.
- Nuevo hash para todo → rejected: rompería la invariant de que `Evidence.content_hash == SourceArtifact.content_hash`.

**Rationale:** un hash por archivo, calculado una vez, compartido por ambos nodes. La rama ya hace el split SHA-256 (contenido) / blake3 (identidad) — B1 sigue ese patrón.

### D3 — Evaluation es opcional en B1

**Elección:** `put_evidence` NO requiere `Evaluation`. El wrapper `put_with_source(evidence, sources, evaluation)` acepta `Option<&Evaluation>`. Si `evaluation.is_some()`: paso 4 aplica `put_evaluation` + `link_evaluates`. Si falla el paso 4, NO hace rollback de pasos 1-3.

**Rationale:** D3 del proposal; el ciclo B1 entrega la infraestructura, no el lifecycle `drafted → accepted`. El paso 4 es decorativo — si falla, la evidence sigue persistida sin evaluación.

### D4 — source_origin en Evidence.props (no columna)

**Elección:** `source_origin` se inyecta en `Evidence.props` como JSON key (`"source_origin": "user_workspace"`) en `evidence_from_match` y `from_tsg_node`. La columna no existe en el schema.

**Alternativas rechazadas:**
- Añadir columna `source_origin STRING` → rejected: requiere `ALTER TABLE`, que lbug 0.18.3 puede no honrar correctamente.
- Dejarlo fuera → rejected: toda Evidence row necesita provenance tag (invariant de ADR-016-B3).

**Rationale:** sigue el patrón existente de `language`, `start_byte`, `end_byte`, `text_preview` que ya viven en `props`. Cero cambios de schema.

### D5 — Sin backfill de Evidence rows pre-B1

**Elección:** Las Evidence rows existentes (creadas antes de B1) siguen sin `EXTRACTED_FROM` edge. No se genera `SourceArtifact` sintético para ellas.

**Rationale:** D5 del proposal; el read path nunca hace JOIN en `EXTRACTED_FROM`; "source unknown" es la forma legacy explícitamente documentada.

### D6 — GraphStore port naming: put_source / put_evaluation / link_extracted_from

**Elección:** Los nuevos métodos siguen el patrón `put_*` del port existente (`put_evidence`). No se usa `create_*` ni se expone `session`.

**Rationale:** consistencia con el port. La orchestration vive en `evidence::put_with_source` (caso de uso), no en el port. El port se mantiene granular.

## Nota técnica: lbug 0.18.3 MERGE on REL TABLE

lbug 0.18.3 rechaza `MERGE` en REL TABLE (relaciones sin propiedades):

```
MERGE (e:Evidence {id:'...'})-[:EXTRACTED_FROM]->(s:SourceArtifact {id:'...'})
→ BinderException: Invalid input: expecting node
```

**Mitigación:** `link_extracted_from` y `link_evaluates` usan un fallback:

```cypher
-- Fallback (idempotent: si el edge ya existe, CREATE es no-op)
MATCH (e:Evidence {id: '$eid'}), (s:SourceArtifact {id: '$sid'})
CREATE (e)-[:EXTRACTED_FROM]->(s);
```

El fallback es idempotente porque:
1. Si el edge ya existe → segundo `CREATE` es no-op en lbug single-graph mode
2. Si los nodos no existen → el MATCH no encuentra nada y no se crea nada (orden de pasos en `put_with_source` garantiza que los nodos ya existen)

## Archivos afectados

| Archivo | Cambio |
|---------|--------|
| `archctl/src/migrations.rs` | Nuevo — runner + `MIGRATIONS` const |
| `archctl/src/source.rs` | Nuevo — `SourceArtifact` struct |
| `archctl/src/evaluation.rs` | Nuevo — `Evaluation` struct |
| `archctl/src/evidence.rs` | Editado — `source_origin` en props, `put_with_source` |
| `archctl/src/store.rs` | Editado — `put_source`, `put_evaluation`, `link_extracted_from`, `link_evaluates` |
| `archctl/src/graph.rs` | Editado — usa `migrations::apply_pending` |
| `archctl/src/lib.rs` | Editado — exports nuevos módulos |
| `docs/schema/002_source_evaluation.cypher` | Nuevo — `Evaluation` + `EVALUATES` |
| `manifests/source.toml` | Nuevo — scope gate para `source.rs` |
| `manifests/evaluation.toml` | Nuevo — scope gate para `evaluation.rs` |
| `docs/DATA-MODEL-LADYBUGDB.md` | Editado — documenta `Evaluation` + `source_origin` en props |
| `docs/adr/README.md` | Editado — ADR-017 indexado |

## Contexto de implementación

- **Idempotencia:** todas las operaciones usan `MERGE` en `id` (SourceArtifact, Evaluation) o en el par de ids (edges).
- **Marker format:** archivo `<project>/.archctl-schema` con contenido `v2-source-evaluation` (untracked, en XDG_DATA_HOME del proyecto).
- **Recovery de partial-apply:** si B1 aplica parcialmente `002` y crashea antes de escribir el marker, el siguiente run intenta aplicar `002` de nuevo → lbug error "table already exists" en el `CREATE NODE TABLE`. El runner para en el primer error. Recovery manual: `rm <project>/.archctl-schema && archctl init`.
- **No transacciones multi-statement:** B1 usa single-writer CLI (ADR-010). Orphan `SourceArtifact` en crash mid-sequence son deduplicados en el siguiente run por el `MERGE` en `id`.

## Consequences

### Positivos
- Schema extensible sin tocar código existente
- `SourceArtifact` y `Evaluation` como tipos first-class
- `source_origin` persiste para auditabilidad
- Runner idempotente — re-runs seguros

### Negativos
- Nueva surface API (`put_source`, `put_evaluation`, `link_extracted_from`, `link_evaluates`)
- Dependencia de `blake3` + `sha2` (ya presente)
- Error de lbug MERGE-on-REL TABLE documentado pero no resuelto en el motor

### Riesgos residuales
- Partial-apply de `002` requiere recovery manual (bajo riesgo: single-writer, `CREATE NODE TABLE` es rápido)
- `source_origin` en JSON no es queryable como columna — limitamos queries a solo el grafo, no a `props` como tabla
