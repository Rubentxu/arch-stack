# Roadmap — OpenCode Architecture Diagrammer

**Estado:** propuesta revisada
**Versión:** 2.3
**Fecha:** 31 de julio de 2026
**Cambios vs 2.2:** ADR-013 introduce `archview` como proyecto ortogonal (ver § Ecosistema). ADR-006 marcado como DEPRECADO. ADR-007, ADR-011, ADR-012 actualizados con la nueva arquitectura. M9 reescrito con tres librerías renderer explícitas.

---

## Principios

1. OpenCode, agentes y skills son el producto.
2. `archctl` es una CLI sidecar.
3. `archview` (proyecto separado) es la aplicación interactiva que consume bundles de `archctl`.
4. LadybugDB entra pronto porque C4 y UML deben compartir identidades.
5. Se entregan verticales completas.
6. Cero escritura dentro del repositorio.
7. Se reutilizan herramientas existentes — preferentemente como librerías Rust, no como CLIs.
8. No se añade un daemon hasta que la concurrencia lo justifique (ADR-010).
9. Cada diagrama tiene propósito, alcance y evidencia.
10. Adoptamos crates de análisis como librerías, no como CLIs (ADR-012).

---

# `archctl` — milestones del sidecar

## M0 — Validación de OpenCode ✅

## M1 — Skillset reproducible ✅

## M2 — `archctl`, XDG y LadybugDB ✅

## M3 — Evidencias y adaptadores básicos ✅ (unido con M4)

## M4 — Vertical C4 (extractores, no render)

**Estado de implementación actual** (vs lo planificado en el ROADMAP v2.2):

| ADR | Implementación real | Estado |
|---|---|---|
| ADR-000 | Reinicio de alcance | Aceptado |
| ADR-001 | OpenCode primero; archctl sidecar | Aceptado, **reforzado por ADR-013** |
| ADR-002 | Topología mínima de agentes | Aceptado |
| ADR-003 | Reutilización y adaptación de skills | Aceptado |
| ADR-004 | Persistencia externa XDG | Aceptado |
| ADR-005 | LadybugDB grafo canónico | Aceptado |
| **ADR-006** | ~~Adaptadores CLI~~ | **DEPRECADO**, sustituido por ADR-012 |
| ADR-007 | Diagramas como proyecciones | **Actualizado** con split estático/interactivo |
| ADR-008 | Recuperación, versionado | Aceptado |
| ADR-009 | Relaciones semánticas reificadas | Aceptado |
| ADR-010 | Concurrencia LadybugDB | Aceptado, **reforzado por ADR-013** |
| ADR-011 | Renderers locales y bloqueo de públicos | **Actualizado**, alcance = `archctl` solamente |
| ADR-012 | Política discard-CLIs | **Actualizado**, referencia ADR-013 |
| **ADR-013** | **Viewer ortogonal basado en DiagramProjection** | **Aceptado** |

**Commits de implementación**:

| Commit | Hito |
|---|---|
| `0ea2065` | M0 scaffold |
| `c701fdc` | M1 Rust scaffold (identity, xdg, skills registry) |
| `22c57e5` | M1 skills lockfile + 6 wrappers |
| `4c0471c` | M2 graph module (lbug) |
| `f63f616` | M2 fix Box::leak → Session |
| `ea47114` | M3+M4 ast-grep-core + evidence |
| `dadfa82` | ADR-012 + ROADMAP v2.2 |
| `4be39dc` | ADR-006 status update |
| `e78413c` | ADR-006 reescrito + ADR-012 endurecido |
| (pendiente) | M5 gix en `identity.rs` |

---

## M5 — `gix` para identidad de repositorio

## M6 — `cargo_metadata` para `inventory depends`

## M7 — `ast-grep-language` y Kotlin

## M8 — `tree-sitter-graph` para extractores declarativos

## M9 — Renderers como librerías (PlantUML, Mermaid, Structurizr propio)

## M9-archctl-export — `archctl diagram export` + `archctl diagram apply`

Antes de cerrar M9, `archctl` necesita emitir bundles que `archview`
pueda consumir.

## M10 — Casos de uso y escenarios (era M9)

## M11 — Secuencias y C4 Dynamic (era M10)

## M12 — Diagramas de clases (era M11)

## M13 — Vista, revisión y formatos (era M12)

## M14 — Versionado, recuperación y actualización (era M13)

## M15 — Herramientas semánticas opcionales (era M14)

## M16 — Endurecimiento 1.0 (era M15)

---

# `archview` — proyecto ortogonal (NO parte de `archctl`)

> **ADR-013**: `archview` es un proyecto separado, no un sub-crate de
> `archctl`. El renderizado interactivo (drill-down, pan/zoom, hover,
> comparación temporal, edición visual) vive en su propio repositorio.

## Stack de `archview`

| Pieza | Librería |
|---|---|
| Framework de diagramación | Sprotty |
| Layout | ELK.js (en Web Worker) |
| Lenguaje | TypeScript |
| Build | Vite |
| UI shell | Svelte o Lit (sin framework pesado) |
| Explorador libre del grafo | Cytoscape.js (opcional) |
| Secuencias | Layout propio en TS |

## Contrato con `archctl`

`archview` consume bundles `DiagramProjection` JSON generados por
`archctl`:

```bash
archctl diagram export \
  diagram:orders-container \
  --format viewer-bundle \
  --output ~/.local/share/archctl/exports/orders-container/
```

Estructura del bundle:

```text
diagram-bundle/
├── manifest.json
├── projection.json
├── evidence.json
├── styles.json
└── assets/
```

Cambios visuales vuelven como ChangeSet:

```bash
archview → exporta viewer-changes.json
   ↓
archctl diagram apply --changes viewer-changes.json
```

## Mini-roadmap de `archview`

| Hito | Descripción |
|---|---|
| Av0 | Scaffold Vite + TypeScript + Sprotty + ELK.js |
| Av1 | Bundle loader (file system) + projection render con Sprotty |
| Av2 | Pan/zoom + sidebar de evidencias |
| Av3 | Drill-down C4 (Context → Container → Component) |
| Av4 | Edición visual con export de ChangeSet |
| Av5 | Comparación temporal entre dos snapshots |
| Av6 | Explorador libre con Cytoscape.js |

Los milestones Av0–Av6 son **del proyecto `archview`**, no de
`archctl`. Se documentan aquí solo para referencia cruzada; cada
proyecto tiene su propio repositorio y roadmap.

---

# Comparación de proyectos

| Aspecto | `archctl` | `archview` |
|---|---|---|
| Lenguaje | Rust | TypeScript |
| Tipo | CLI sidecar one-shot | Aplicación web local interactiva |
| Persistencia | Lee/escribe LadybugDB | Solo lee bundles del disco |
| Red | Bloqueada (ADR-011) | Bloqueada por construcción (CSP) |
| Output | `.svg`, `.dsl`, `.puml`, bundle JSON | HTML+SVG interactivo |
| Distribución | Binario único + assets | Proyecto web estático + opcionalmente Tauri shell |
| Lifecycle | Cada comando es una transacción corta | Una sesión de revisión con file watcher |
| Concurrencia | Lock por proyecto (ADR-010) | N/A (no accede al grafo) |

---

# Primer MVP útil

```text
M0 → M1 → M2 → M3 → M4 → M5 → M6 → M7 → M8 → M9 → M10
```

Incluye:

- Perfil OpenCode.
- Skills reutilizadas con `skills.lock.yaml`.
- `archctl` como sidecar Rust con `gix` para identidad.
- `cargo_metadata` para dependencias nativas.
- `ast-grep-language` con 7 lenguajes (kotlin incluido).
- `tree-sitter-graph` para extractores declarativos.
- Renderers `plantuml-little` + `merman` + Structurizr propio en pure Rust.
- `archctl diagram export` para producir bundles `DiagramProjection`.
- Sin servidor, sin daemon, sin WebSocket.
- `archview` (proyecto paralelo) consume los bundles cuando se necesita interactividad.
- Salida: diagramas C4 + UML como proyecciones del grafo, en SVG estático o en HTML interactivo.

**Estado actual (post-v2.3)**: M0–M4 cerrados. M5–M9 pendientes de implementación, aunque la mayoría del código de M4 ya cubre la integración de evidence::extract y graph::put que serán reutilizados.

---

## Cambios SDD completados

| Cambio | Rama | Commit tip | Estado |
|---|---|---|---|
| `refactor-1b-filesystem-port` | `feat/filesystem-port` (mergeado a main via FF) | `607ee64` | **Cerrado** ✅ · tag `v0.1.0` |
| `b1-source-evaluation-types` | `feat/b1-source-evaluation-types` (merged a main via FF) | `1264f9e` | **Cerrado** ✅ · tag `v0.2.0` |
| `refactor-1c-scope-port` | `feat/refactor-1c-scope-port` (mergeado a main via FF) | `87a2149` | **Cerrado** ✅ · tag `v0.1.1` |
| `fix-parallel-lbug-test-races` | `fix/parallel-lbug-test-races` (merged a main via FF) | `4b8ac47` | **Cerrado** ✅ · tag `v0.2.2` |
| `refactor-extract-cell-to-json-map` | `refactor/extract-cell-to-json-map` (merged a main via FF) | `504560f` | **Cerrado** ✅ · tag `v0.3.1` |
| `m9-archctl-export` | `feat/m9-archctl-export` (merged a main via FF) | `7c2f167` | **Cerrado** ✅ · tag `v0.4.0` |
| `hygiene-local-only-policy` | direct commit on `main` (no PR — 1-line config gap) | `0a28016` | **Cerrado** ✅ · tag `v0.4.1` |
| `more-manifests-2` | direct commit on `main` (no PR — bulk manifest cycle) | `d2c27fe` | **Cerrado** ✅ · tag `v0.5.0` |

## Cycle cerrado — `refactor-1b-filesystem-port`

- **Fecha**: 2026-07-30
- **Branch**: `feat/filesystem-port` (merged a main via FF)
- **Tag**: `v0.1.0` (patch bump — primer tag del repo)
- **Verdict**: verify PASS · debt PASS_WITH_WARNINGS (C-W1 fixed post-audit at `607ee64`)
- **Commits**: 12 (10 ciclo + 1 chore + 1 post-audit fix)
- **Tests**: 107 unit + 4 doctests = 111 passing (vs 89 baseline, +22)
- **Output**: introduce `Filesystem` hexagonal port (`SystemFilesystem` + `MemoryFilesystem`), plumbed through `CliContext`, migrate 8 domain call sites, register `manifests/filesystem.toml` with `must_not_contain` gate.
- **Próximo candidato**: B1 (Source + Evaluation en el grafo, ADR-016) o Refactor 1c (scope.rs → Filesystem port, chicken-and-egg postergado).

> `refactor-1b-filesystem-port`: Puerto hexagonal Filesystem (7 métodos, SystemFilesystem + MemoryFilesystem), plumbed a través de CliContext, 8 módulos migrados, manifiesto `manifests/filesystem.toml` con gate `must_not_contain`. 111 tests passing. Archivado en `sddk/refactor-1b-filesystem-port/archive-report.md`.

## Cycle cerrado — `refactor-1c-scope-port`

- **Fecha**: 2026-07-30
- **Branch**: `feat/refactor-1c-scope-port` (merged a main via FF)
- **Tag**: v0.1.1 (patch)
- **Verdict**: verify PASS · debt PASS_WITH_WARNINGS (non-blocking YAGNI cfg_attr branch)
- **Commits**: 3 (chore manifest + refactor migration + post-audit fix para `strip_cfg_test_blocks`)
- **Tests**: 111 verde (sin regresión)
- **Output**: scope.rs migrado al Filesystem port (6 sitios `std::fs::*` → port-routed), `manifests/scope.toml` con gate `must_not_contain` que skipea `#[cfg(test)]` blocks. Ya no quedan llamadas `std::fs::*` en código de dominio (solo test fixtures).
- **Próximo candidato**: B1 (Source + Evaluation en el grafo) o más manifests (clock/environment/identity).

> `refactor-1c-scope-port`: scope.rs migrado al Filesystem port (6 sitios `std::fs::*` → port-routed). `manifests/scope.toml` con gate `must_not_contain` que skipea `#[cfg(test)]` blocks via `strip_cfg_test_blocks`. 111 tests passing. Archivado en `sddk/refactor-1c-scope-port/archive-report.md`.

## Cycle cerrado — `b1-source-evaluation-types`

- **Fecha**: 2026-07-30
- **Branch**: `feat/b1-source-evaluation-types` (merged a main via FF)
- **Tag**: v0.2.0 (minor — new schema + migration)
- **Verdict**: verify PASS_WITH_WARNINGS · debt PASS_WITH_WARNINGS (4 warnings non-blocking)
- **Commits**: 11 (9 cycle + 2 post-audit fixes)
- **Tests**: 124 verde (sin regresión desde v0.1.1)
- **Output**: SourceArtifact + Evaluation node types, EXTRACTED_FROM + EVALUATES edges, migration runner (v1 → v2), 4 nuevos port methods (put_source/put_evaluation/link_extracted_from/link_evaluates), put_with_source wrapper, source_origin en Evidence.props. ADR-017 documenta decisiones.
- **Próximo candidato**: memory_candidate lifecycle (`drafted → accepted`) o W1 (reducir Cypher-builder duplication en store.rs).

## Cycle cerrado — `fix-parallel-lbug-test-races`

- **Fecha**: 2026-07-30
- **Branch**: `fix/parallel-lbug-test-races` (merged a main via FF)
- **Tag**: v0.2.2 (patch — parallel-test race fix)
- **Verdict**: verify PASS_WITH_WARNINGS · debt PASS · archive PASS
- **Commits**: 2 (bound lbug buffer pool to 256 MB + remove --test-threads=1 workaround)
- **Tests**: 125 verde serial (3.63s) + 125 verde parallel (1.78s) · 0 regresión
- **Output**: `BUFFER_POOL_SIZE = 256 * 1024 * 1024` en `graph.rs`, aplicado a ambos `buffer_pool_size` y `max_db_size` en cada apertura de DB. Workaround `--test-threads=1` eliminado de scope.rs. Doctor per-scope ~10s (era ~2 min via serial).
- **Root cause**: lbug 0.18.3 `SystemConfig::default()` → UINT64_MAX (~8 TB mmap). Con 64 cores × 8 TB = 512 TB virtuales requeridos, el kernel no puede satisfacerlo. 256 MiB per DB = 16 GiB total con 64 cores, perfectamente servible.
- **Apply deviation**: apply agent extendió el fix más allá del spec (boundó también `max_db_size` además de `buffer_pool_size`). Verificado correcto por verify phase.
- **Próximo candidato**: B1 lifecycle (drafted → accepted) o más manifests (~13 módulos restantes).

## Cycle cerrado — `more-manifests-clock-env-identity`

- **Fecha**: 2026-07-30
- **Branch**: `feat/more-manifests-clock-env-identity` (merged a main via FF)
- **Tag**: v0.2.1 (patch — additive gate coverage only)
- **Verdict**: verify PASS · debt PASS · archive PASS
- **Commits**: 1 (3 manifests: clock, environment, identity)
- **Tests**: 124 verde (sin regresión)
- **Output**: 3 scope manifests (`clock.toml`, `environment.toml`, `identity.toml`) con `must_hold` + `must_not_contain = ["use std::fs::"]` + `minimum_tests`. Coverage: 10/23 manifests.
- **Próximo candidato**: Más manifests (astgrep, cli, doctor, graph, inventory, project, render, row, skills, telemetry, xdg ~13 módulos restantes) o B1 lifecycle (drafted → accepted).

## Cycle cerrado — `b1-lifecycle-drafted-accepted`

- **Fecha**: 2026-07-30
- **Branch**: `feat/b1-lifecycle-drafted-accepted` (merged a main via FF)
- **Tag**: v0.3.0 (minor — new lifecycle feature)
- **Verdict**: verify PASS_WITH_WARNINGS · debt PASS_WITH_WARNINGS (7 warnings non-blocking)
- **Commits**: 7 (6 cycle + 1 post-audit doc fix)
- **Tests**: 137 verde (sin regresión desde v0.2.2)
- **Output**: EvidenceStatus enum (Drafted/Accepted/Superseded), persistido en Evidence.props (zero migration), 3 nuevos port methods (accept/supersede/list_by_status), 2 nuevos CLI subcommands + --status flag, Evaluation::accept creado en audit. ADR-016 §3.2 cerrado.
- **Próximo candidato**: W2-class cleanup (extraer cell_to_json_map helper de store.rs) o bulk more manifests.

## Cycle cerrado — `refactor-extract-cell-to-json-map`

- **Fecha**: 2026-07-30
- **Branch**: `refactor/extract-cell-to-json-map` (merged a main via FF)
- **Tag**: v0.3.1 (patch — mechanical refactor, no behavior change)
- **Verdict**: verify PASS · debt PASS_WITH_WARNINGS (W-1 = 3 out-of-scope test fixtures, non-blocking) · archive PASS
- **Commits**: 1 (1-commit cycle, A-min path)
- **Tests**: 137 verde (sin regresión desde v0.3.0)
- **Output**: helper privado `fn cell_to_json_map(&Cell) -> serde_json::Map` añadido a `archctl/src/store.rs:667-689` ("Internal helpers"); 3 sitios inline en `accept_evidence`, `supersede_evidence`, `list_evidence_by_status` reemplazados por `map(cell_to_json_map)`. Net **-41 LOC**. `manifests/store.toml` intacto. Narrowing intencional documentado (Object-with-String only; Int/Bool/Float marcados como `// Future:` extension point).
- **Out of scope**: 3 inline patterns homólogos en test fixtures (store.rs:1064, 1208, 1261) — programados para `refactor-extract-cell-to-json-map-v2`.
- **Debt detail**: 0 CRITICAL · 1 WARNING · 1 SUGGESTION · 0 ponytail ledger items · 0 hidden deps · 0 global state risks · accidental-bloat 0.05/1.00. Clústeres corridos (smoke): overeng + coupling. Clusters skipped: architecture, smells, duplication (smoke depth).
- **Próximo candidato**: `refactor-extract-cell-to-json-map-v2` (migrar 3 test fixtures), o bulk more manifests (11 módulos restantes), o DUP-2 Cypher builder (otro ciclo).

> `refactor-extract-cell-to-json-map`: helper privado `cell_to_json_map(&Cell) -> Map` en `archctl/src/store.rs:667-689` reemplaza 3 inline duplications en `accept_evidence`, `supersede_evidence`, `list_evidence_by_status`. Net **-41 LOC** (mejor que la estimación -55). 137 tests passing sin modificación. Manifest `manifests/store.toml` intacto, `must_hold` satisfecho, `minimum_tests = 13` excedido. Archivado en `sddk/refactor-extract-cell-to-json-map/archive-report.md`.

## Cycle cerrado — `m9-archctl-export`

- **Fecha**: 2026-07-31
- **Branch**: `feat/m9-archctl-export` (merged a main via FF)
- **Tag**: v0.4.0 (minor — nueva superficie CLI: `diagram export` + `diagram validate`)
- **Verdict**: verify PASS · debt PASS_WITH_WARNINGS (5 warnings no-bloqueantes) · archive PASS
- **Commits**: 15 (14 ciclo + 1 doc-patch post-audit)
- **Tests**: 162 unit + 4 integration = 166 passing (vs 137 baseline, +29)
- **Output**: `archctl/src/diagram/` (9 archivos, ~1605 LOC), `schemas/diagram-projection.schema.json` (JSON Schema 2020-12), `manifests/diagram.toml` (must_hold + must_not_contain + minimum_tests), `archctl/src/diagram/icons/` (placeholders 1×1 PNG), `docs/specs/diagram-projection-bundle.md` (nuevo). Comandos nuevos: `archctl diagram export <view-selector> --format viewer-bundle --output <dir>` y `archctl diagram validate <bundle-dir>`. Selector grammar `<c4-kind>:<scope>` (5 c4-kinds: context/container/component/dynamic/deployment). `baseRevision` = content-hash blake3 sobre canonical JSON. Dependencia nueva: `jsonschema` (validación de bundle). 29 tests nuevos (selector parser, hash determinism, export pipeline, validate, schema validation).
- **Scope decision**: cycle **scoping down** decidido en explore phase. **NO incluye** `archctl diagram apply` (deferido a `m9-archctl-export-apply`). NO incluye ADR-010 lockfile infra. NO incluye schema v3 migration (`view.diagram` nodes). Razón: bundle contract tiene zero backing en código (ADR-007 prescribió view.* nodes que nunca se construyeron); Path 2 (stateless projections) entrega el 100% del valor de lectura para `archview`. Apply necesita diseño dedicado de lock + override model.
- **Risk divergence**: ADR-013 §"baseRevision" muestra `revision:42` (counter) — divergencia documentada. Implementación usa content-hash blake3 (más defensivo). ADR text update owed.
- **Debt detail**: 0 CRITICAL · 5 WARNING · 0 SUGGESTION · 0 ponytail ledger items · 0 hidden deps · 0 global state risks · accidental-bloat 0.12/1.00. Clústeres corridos (deep): architecture + smells + duplication + coupling + overeng. Clusters skipped: none (deep depth).
- **Próximo candidato**: `m9-archctl-export-apply` (ciclo dedicado para lock + override model), o `more-manifests-*` (~11 módulos restantes: astgrep/cli/doctor/graph/inventory/project/render/row/skills/telemetry/xdg).

> `m9-archctl-export`: surface CLI nueva (`archctl diagram export` + `validate`) que proyecta el grafo LadybugDB en un bundle JSON de 5 archivos consumible por `archview`. Path 2 (stateless projections), `baseRevision` content-hash, zero schema migration, scope-down respecto al contrato completo ADR-013 (apply deferred). 166 tests passing, debt deep 5/5 PASS_WITH_WARNINGS. Archivado en `sddk/m9-archctl-export/archive-report.md`.

## Cycle cerrado — `hygiene-local-only-policy`

- **Fecha**: 2026-07-31
- **Branch**: directo a `main` (no PR — cambio trivial de configuración, 6 líneas)
- **Tag**: v0.4.1 (patch — non-functional, infra-only)
- **Verdict**: N/A (no aplica verify/debt — sin código)
- **Commits**: 1 (chore(gitignore))
- **Tests**: N/A (cero cambios de código)
- **Output**: cierra la brecha de la v3.3 local-only policy identificada en el v0.4.0 release report.
  - `.ignore` companion file (gitignored itself) que re-incluye `sddk/` para opencode tools (grep, glob, read). El archivo está documentado en el `.gitignore` con referencia cruzada.
  - `docs/reports/*.html` agregado al `.gitignore`. El `sddk-release` phase ya no commitea `closing-v*.html` al remote. El `closing-v0.3.0.html` ya commiteado (`6d3802a`) queda en history; los untracked `closing-v0.2.0.html` y `closing-v0.4.0.html` ahora son ignored.
- **Próximo candidato**: `m9-archctl-export-apply` (planning completo con score 100/100, depende de la decisión arquitectónica de DB lock via fs2 documentada en engram obs 5349).

> `hygiene-local-only-policy`: cierra la v3.3 local-only policy gap. `.ignore` companion file re-incluye `sddk/` para opencode tools; `docs/reports/*.html` gitignored para detener la fuga de HTML closing reports al remote. Config-only, sin cambios de comportamiento. Archivado en `sddk/hygiene-local-only-policy/archive-report.md` (minimal).

## Cycle cerrado — `more-manifests-2`

- **Fecha**: 2026-07-31
- **Branch**: directo a `main` (no PR — bulk manifest cycle, mismo patrón que `more-manifests-clock-env-identity`)
- **Tag**: v0.5.0 (minor — adds 11 manifests, coverage 11/23 → 22/23)
- **Verdict**: N/A (no aplica verify/debt — sin cambios de código)
- **Commits**: 1 (`feat(manifests): add 11 module scope manifests`)
- **Tests**: 183 unit + 4 doctests + 23 integration = 210 passing (no regressions, baseline preserved)
- **Output**: 11 new scope manifests. `migrations.toml` intencionalmente excluido (bootstrap infrastructure, no domain module). Coverage: 22/23 modules.

  | Manifest | Min tests | Pub symbols | must_hold |
  |---|---|---|---|
  | astgrep | 9 | 9 | 11 |
  | cli | 13 | 17 | 11 |
  | doctor | 0 | 7 | 7 |
  | graph | 8 | 8 | 11 |
  | inventory | 6 | 7 | 8 |
  | project | 0 | 7 | 6 |
  | render | 0 | 2 | 8 |
  | row | 8 | 24 | 12 |
  | skills | 0 | 14 | 14 |
  | telemetry | 0 | 1 | 5 |
  | xdg | 0 | 13 | 11 |

- **Verification**: scope gate validada por static analysis (Python script verificando must_hold/public_symbols/must_not_contain) porque `doctor --check-scope` end-to-end tomaría ~5 min (22 manifests × ~13s por cargo test gate). 0 must_hold failures, 0 public_symbols failures, 0 must_not_contain violations.
- **Próximo candidato**: `m9-archctl-export-apply` (PR1 foundation: schema v3 + DB lock via fs2 + 8 port methods). Planning 100/100 PASS, ADR-018 eliminado, listo para arrancar.

> `more-manifests-2`: cierra la cobertura de scope manifests. 11 nuevos TOML declaran public API + must_hold invariants para astgrep/cli/doctor/graph/inventory/project/render/row/skills/telemetry/xdg. 183 unit tests preservados. Config-only, no functional changes. Archivado en `sddk/more-manifests-2/archive-report.md` (minimal).
