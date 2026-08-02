# ADR-008 — Recuperación, versionado y evolución

**Estado:** Aceptado (con **recuperación parcial**, revisado 2026-08-01 — ver `docs/audits/2026-08-01-archctl-adr-vs-impl.md` §F3)
**Fecha original:** 29 de julio de 2026
**Fecha de revisión:** 1 de agosto de 2026

## Contexto

Las sesiones pueden compactarse o interrumpirse. El repositorio cambia y LadybugDB también evolucionará.

## Decisión

### Identidad y versiones

- `Element` y `SemanticRelation` tienen identidad estable.
- `ElementVersion` y `RelationVersion` son inmutables.
- `Snapshot` agrupa versiones.
- El estado actual se materializa mediante relaciones `CURRENT_*`.

### Ejecuciones

Cada `AnalysisRun` registra:

- petición;
- snapshot de entrada;
- snapshot de salida;
- agentes y skills;
- herramientas;
- artefactos;
- errores;
- checkpoint.

### Recuperación

`archctl run resume` devuelve:

- etapas válidas;
- artefactos;
- evidencias obsoletas;
- snapshot activo;
- siguiente acción.

### Actualización incremental

1. Git detecta ficheros modificados.
2. Se invalidan evidencias.
3. Se recalculan solo extractores afectados.
4. Se crean versiones nuevas.
5. Se actualizan aristas materializadas.
6. Se marcan diagramas `stale`.

### Migración

- versión de esquema;
- migraciones idempotentes;
- backup antes de migrar;
- exportación LadybugDB;
- importación en base vacía;
- validación posterior;
- rollback conservando la base anterior.

## Consecuencias

- Recuperación independiente de la conversación.
- Historia por commit y worktree.
- Reproducción de resultados.
- Necesidad de fixtures de migración.

## Revisión (1 de agosto de 2026) — **recuperación parcial**

El modelo conceptual completo (Snapshot, AnalysisRun, archiva de
ejecuciones, `archctl run resume`) está **declarado en el schema** pero
**no implementado en `archctl`**. La aplicación actual implementa solo
los pasos 1–3 de la actualización incremental:

- ✅ `ElementVersion` inmutable (deterministic blake3 de `version_props`)
- ✅ `CURRENT_VERSION` + `VERSION_OF` edges (escritas por call-graph y c4-discover)
- ✅ Inserción de Evidence con lifecycle (`Drafted|Accepted|Superseded`)

**Deferred a 1.x** (alineado con ADR-021 Cognitive Layer):

- ❌ `Snapshot` y `AnalysisRun` tables (declaradas en el schema, no escritas
  por código de aplicación). Los rel tables asociados
  (`AT_SNAPSHOT`, `PARENT_SNAPSHOT`, `RUN_*`) tampoco se usan.
- ❌ `archctl run resume` — el subcomando `archctl run` no existe.
- ❌ Steps 4–6 de actualización incremental (mark stale diagrams,
  refresh materialized edges from new versions).
- ❌ Backup/rollback/export/import de la DB LadybugDB (solo el migration
  runner con v1-initial → v3-view-nodes funciona; rollback
  documentado como `rm .archctl-schema && archctl init`).
- ❌ Herramientas de validación de índice (`archctl graph repair-index`,
  `archctl graph verify-index`).

### Por qué diferir

- ADR-021 (Cognitive Layer) marca el `archctl run` como 1.x porque
  requiere el runtime cognitivo (modelos, agentes, replay determinista).
- Mover el modelo a 1.x evita un refactor doble cuando se implemente
  el runtime cognitivo.
- El schema se mantiene en su forma declarada para que un futuro
  cycle de M15+ (M19+ Performance validation, M20+ cognitive layer)
  pueda activar las tablas sin migración adicional.

### Cuándo revocar esta deferral

- Si `archview` necesita navegar versiones de elementos/relaciones
  para diffs visuales (entonces `Snapshot` + `AT_SNAPSHOT` se vuelven
  críticos).
- Si el equipo decide priorizar el runtime cognitivo (ADR-021)
  antes de archview.

## Decisión actual (post-revisión)

**`archctl` v0.10+ implementa los pasos 1–3 de actualización
incremental y la migración de schema (v1-initial → v3-view-nodes).**
Las tablas `Snapshot`/`AnalysisRun` y el comando `archctl run` quedan
**declarados en schema, diferidos a 1.x**.
