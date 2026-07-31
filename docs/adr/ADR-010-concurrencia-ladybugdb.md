# ADR-010 — Concurrencia de LadybugDB y procesos `archctl`

**Estado:** Aceptado  
**Fecha:** 29 de julio de 2026  
**Última revisión:** 31 de julio de 2026 (lock file → DB lock vía `fs2`)

## Contexto

LadybugDB es embebida. No deben existir simultáneamente objetos de base independientes sobre el mismo fichero si uno opera en escritura.

OpenCode puede ejecutar varios subagentes y varias invocaciones de `archctl`.

LadybugDB ya impone un lock ligero sobre `architecture.lbdb` cuando se abre en `READ_WRITE` ("setting some permission flags on the database file" — `docs.ladybugdb.com/concurrency`). Esta protección es por-proceso y no se solapa con la de `archctl`.

## Decisión del MVP

Cada comando:

1. resuelve el proyecto;
2. adquiere un bloqueo exclusivo sobre `architecture.lbdb` mediante `fs2::try_lock_exclusive` (POSIX `flock` en Unix, `LockFileEx` en Windows);
3. abre `architecture.lbdb` para escritura;
4. crea una conexión;
5. ejecuta una transacción corta;
6. cierra la base;
7. libera el bloqueo (kernel-managed: el `File` handle al soltarse cierra el lock automáticamente).

El bloqueo vive **en el archivo de la base, no en un archivo separado**. Esta es la convención de SQLite, DuckDB y la propia LadybugDB. No se necesita `architecture.lock` adicional.

> **Revisión del 31 de julio de 2026:** la versión original de este ADR proponía un archivo `architecture.lock` separado bajo `$XDG_STATE_HOME/archctl/projects/<id>/locks/`. La implementación adoptada durante el ciclo `m9-archctl-export-apply` (obs 5349) eliminó ese archivo y usa `fs2` directamente sobre el `.lbdb`. La intención (exclusión mútua por proyecto, manejo de procesos concurrentes) se preserva; la letra (un archivo de lock separado) se reemplaza por el patrón estándar de la industria.

Las extracciones costosas se ejecutan fuera del bloqueo.

Solo la importación y actualización final bloquean la base.

## Lecturas

El MVP serializa también lecturas para evitar combinaciones inseguras entre procesos independientes (mismo lock sobre `.lbdb` las cubre).

## Stale recovery

No requiere código. Cuando un proceso muere (SIGKILL, panic, OOM, exit normal), el kernel libera el `flock` automáticamente al cerrarse el descriptor de archivo. El siguiente `archctl` puede abrir la base inmediatamente, sin `--force` ni limpieza manual.

## Evolución

Si la serialización se convierte en cuello de botella se añadirá opcionalmente:

```text
archctld
```

Responsabilidades:

- mantener un único objeto `Database` abierto;
- crear múltiples conexiones;
- aceptar peticiones por Unix socket;
- conservar la misma API de dominio.

No será requisito para instalación ni uso inicial.

## Consecuencias

- Comportamiento seguro y simple.
- Lock sobre el mismo archivo de la DB — mismo ciclo de vida que la base.
- Eliminación del lock es automática (kernel-managed); no hay "stale lock" detectable.
- Cross-platform (Linux, macOS, Windows) sin código de plataforma.
- Una sola dep nueva (`fs2 = "0.4"`, sin transitivas).
- Menor paralelismo de consultas.
- No se necesita servidor en el MVP.
- La extracción puede continuar en paralelo antes de importar.
