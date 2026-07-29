# ADR-008 — Recuperación, versionado y evolución

**Estado:** Aceptado  
**Fecha:** 29 de julio de 2026

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
