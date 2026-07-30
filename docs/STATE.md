# Estado de `archctl` — Snapshot pre-investigación

> Snapshot persistente del estado del proyecto antes de la
> investigación de `activegraph-packs`. Para volver a este estado
> desde cualquier sesión futura:

```bash
cd /var/home/rubentxu/Proyectos/agentesIA/archctl
git checkout snapshot/pre-activegraph-investigation
```

> **Tag git**: `snapshot/pre-activegraph-investigation` apunta a
> `aa171cd` (HEAD al cierre).
>
> **Memoria engram**: topic_key `archctl/snapshot-pre-activegraph-investigation`
>
> **Fecha del snapshot**: 30 de julio de 2026.

---

## Resumen ejecutivo

| Métrica | Valor |
|---|---|
| Tests | **69 verde** (54 unit + 15 nuevos en refactor hexagonal) |
| Warnings | 0 |
| Branch | `feat/m5-gix-identity` |
| Commit HEAD | `aa171cd` |
| Último release conceptual | M8 (tree-sitter-graph TSG) + Ola 1 ports hexagonales |
| Siguiente paso planeado | Refactor 1b — Filesystem port |

## Stack hexagonal en construcción

El proyecto transita de monolito con leaks de infra a hexagonal estricto.
Los ports implementados hasta el cierre:

| Port | Trait | Adapter prod | Adapter test | Estado |
|---|---|---|---|---|
| Persistencia | `GraphStore` (`store.rs`) | `LbugStore` (LadybugDB 0.18.3) | — | ✅ ADR-014 |
| Clock | `Clock` (`clock.rs`) | `SystemClock` (chrono::Utc) | `FixedClock` | ✅ |
| Environment | `Environment` (`environment.rs`) | `SystemEnvironment` | `FixedEnvironment` | ✅ ADR-015 parcial |
| Filesystem | (no existe) | — | — | ⏳ Refactor 1b |
| HTTP | (no existe) | `reqwest::blocking` directo | — | ⏳ futuro |

### Capas

```
domain (evidence, identity, project, tsg, clock)
  │
  ├─ port (store::GraphStore, clock::Clock, environment::Environment)
  │     │
  │     └─ adapter (LbugStore, SystemClock, SystemEnvironment)
  │
  └─ use cases (mezclados en cli.rs hoy) — ⏳ Refactor 3 los extrae
```

## Refactors recientes (en orden cronológico)

| Refactor | Commit | Detalle |
|---|---|---|
| M5 — gix port | `adc6c18` | Reemplaza `Command::new("git")` por API in-process. |
| M6 — cargo_metadata | `16865db` | `archctl inventory depends` usa `cargo_metadata` 0.23. |
| M7 — ast-grep-language | `320f277` | Tree-sitter languages via `ast-grep-language 0.45` + builtin-parser. **Kotlin soportado.** |
| M8 — tree-sitter-graph | `08ce173` | DSL declarativo (.tsg) via fork `basemind-tree-sitter-graph`. |
| ADR-014 — persistence port | `cb6a796` | `GraphStore` trait. SparrowDB descartado. |
| Clock port | `352e345` | `Clock::now_rfc3339()`. Tests golden-file posibles. |
| Row tipado | `6f6c90c` | `Row` + `Cell` reemplazan `Vec<serde_json::Value>`. |
| Environment port | `aa171cd` | `Environment` trait + `CliContext { env: Arc<dyn Environment> }`. `run` (producción) + `run_inner` (tests). |

## Decisiones arquitectónicas (ADRs)

| ADR | Título | Estado |
|---|---|---|
| ADR-000 | Reinicio de alcance | Aceptado |
| ADR-001 | OpenCode primero, archctl sidecar | Aceptado |
| ADR-002 | Topología mínima de agentes | Aceptado |
| ADR-003 | Reutilización y adaptación de skills | Aceptado |
| ADR-004 | Persistencia externa XDG | Aceptado |
| ADR-005 | LadybugDB grafo canónico (sigue válido, no contradice ADR-014) | Aceptado |
| ADR-006 | ~~Adaptadores CLI~~ DEPRECADO | Sustituido por ADR-012 + ADR-013 |
| ADR-007 | Diagramas como proyecciones | Aceptado |
| ADR-008 | Recuperación, versionado, evolución | Aceptado |
| ADR-009 | Relaciones semánticas reificadas | Aceptado |
| ADR-010 | Concurrencia LadybugDB | Aceptado |
| ADR-011 | Renderers locales | Aceptado (alcance = archctl solamente) |
| ADR-012 | Política "descartar CLIs" + M5–M8 + renderers | Aceptado |
| ADR-013 | Viewer ortogonal (proyecto archview separado) | Aceptado |
| ADR-014 | Persistence port + SparrowDB deferred | Aceptado (SparrowDB descartado en sesión posterior) |
| ADR-015 | Ports faltantes (Clock, Environment, Filesystem) | **Parcial** — Clock y Environment hechos; Filesystem pendiente. |

## Tests clave (smoking guns)

Cada refactor hexagonal viene con al menos un test que verifica el
contrato a través de la interfaz pública, no del detalle de
implementación. Principio operativo explícito del usuario:
*"No leo el código que escriben mis agentes. En su lugar, los rodea
de controles como tests, análisis de mutaciones y métricas de
calidad."*

| Test | Qué prueba | Si el port se bypassa |
|---|---|---|
| `extract_stamps_observed_at_from_clock` | `FixedClock("2030-01-01")` → todos los `observed_at` son `"2030-01-01T00:00:00Z"` | Test falla (timestamp no determinístico) |
| `run_inner_uses_injected_cwd_not_process_cwd` | Dos cwds inyectados distintos → dos project_ids distintos | Si el port se bypassa, ambos colapsan al cwd real → ids idénticos → test falla |
| `fixed_environment_errors_when_cwd_unset` | Sin `with_cwd`, `current_dir()` retorna `Err`, no panic ni fallback silencioso | Si alguien hace lazy default, el test falla |
| `row_object_cell_preserves_order_internally` | `Cell::Object` mantiene orden internamente vía `Vec<(String, Cell)>` | El test distingue orden interno vs JSON serializado |

## Lo que NO está hecho (siguiente sesión)

### Refactor 1b — Filesystem port

`Filesystem` trait propuesto:

```rust
pub trait Filesystem: Send + Sync {
    fn read_to_string(&self, path: &Path) -> Result<String>;
    fn write(&self, path: &Path, content: &[u8]) -> Result<()>;
    fn create_dir_all(&self, path: &Path) -> Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn remove_file(&self, path: &Path) -> Result<()>;
    fn canonicalize(&self, path: &Path) -> Result<PathBuf>;
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
    fn metadata_is_file(&self, path: &Path) -> bool;
    fn metadata_is_dir(&self, path: &Path) -> bool;
}
```

**Complejidad**: `inventory::tree` usa `ignore::WalkBuilder` que internamente llama `std::fs::*`. El port no puede inyectarse ahí sin reimplementar el walker. Scope: ports para los sitios donde se puede inyectar limpiamente (`store`, `evidence`, `render`, `skills`, `project`, `identity`). `inventory::tree` queda fuera del port por ahora.

### Refactor 3 — Use cases

Extraer lógica de `cli.rs` (466 LOC) a `archctl/src/usecase/`. Cada use case recibe `&dyn GraphStore`, `&dyn Environment`, `&dyn Clock`, `&dyn Filesystem`. El CLI queda con: parseo + formateo.

### ADR-015 finalizar

Documentar los tres ports (Clock, Environment, Filesystem) como un único ADR en lugar de tres commits separados.

### Métricas de calidad

Clippy como `cargo clippy -- -D warnings` en CI. Hoje compila sin warnings, pero no hay enforcement automático.

## Decisiones pendientes

- ¿Cómo migrar `inventory::tree` a Filesystem port sin reimplementar `ignore::Walk`? Opciones: (a) reescribir walker, (b) escribir wrapper `IgnoreFilesystem` que envuelva `ignore::WalkBuilder` y lo exponga vía trait, (c) dejar fuera del port. Decisión pendiente.
- ¿Vale la pena eliminar `graph.rs` (legacy compat layer) ahora? Beneficio: ~100 LOC menos, una sola fuente de verdad. Costo: rompe `pub use graph::*` en `lib.rs`. Decisión pendiente hasta confirmar que ningún usuario externo depende.

## Archivos clave para retomar la sesión

- `archctl/src/lib.rs` — declaración de módulos y `pub use`.
- `archctl/src/store.rs` — port + adapter. Patrón a replicar en Filesystem.
- `archctl/src/environment.rs` — `CliContext` pattern que Filesystem debe extender.
- `archctl/src/clock.rs` — ejemplo del port más simple (un solo método).
- `archctl/src/row.rs` — port con tipo de dominio (no infra).

## Comandos de recuperación

```bash
# Volver al estado del snapshot
cd /var/home/rubentxu/Proyectos/agentesIA/archctl
git checkout snapshot/pre-activegraph-investigation

# Verificar que tests siguen verde
cd archctl
cargo test

# Continuar refactor 1b (Filesystem)
# Diseñar el trait siguiendo el patrón de environment.rs
```

## Próxima acción del usuario

Investigar `https://github.com/yoheinakajima/activegraph-packs` y
producir un reporte en `docs/` con ideas y propuestas de mejora
para `archctl` basadas en ese proyecto. El reporte se publicará
junto con este snapshot.
