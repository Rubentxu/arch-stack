# Auditoría archctl — ADR vs implementación vs ROADMAP

**Fecha**: 2026-08-01
**Alcance**: 23 ADRs (docs/adr/), ROADMAP v2.5, 19 tags semánticos (v0.1.0–v0.10.0), implementación actual en `archctl/src/`
**Método**: 2 sub-agents en paralelo (ADRs 000–013 y 014–023) + cross-check manual de ROADMAP y git log.

---

## Resumen ejecutivo

| Métrica | Valor |
|---|---:|
| ADRs revisados | 23 |
| Implementados como dice el ADR | 12 |
| Implementados con drift menor (cosmético, naming) | 4 |
| Implementados con drift semántico (reificación, renderers) | 2 |
| N/A por contrato (1.x / archview) | 6 |
| ADR-006 deprecated por autor | 1 |
| **Drift críticos** | **2** |
| **Drift significativos** | **3** |
| Drift menores / cosméticos | 6 |

**Veredicto global**: ✅ arquitectura fundamentalmente alineada. Hay **2 drift críticos** que requieren acción (ADR-011 redrendere POST a kroki.io sin opt-in; ADR-009 reificación rota por call-graph writer). El resto son naming/cosmética.

---

## Findings priorizados

### 🔴 CRÍTICO — acción inmediata

#### F1. ADR-011 (Renderers locales) — implementación hace lo opuesto del ADR

**Drift crítico**: `archctl render` hace `POST` HTTP a `kroki_url` (default `http://localhost:18000`), exactamente el patrón que ADR-011 prohíbe. La política dice "renderers locales only + bloqueo de servicios públicos por defecto + opt-in requiere `--allow-public-renderer` + audit en `AnalysisRun` + warning en consola". El código tiene ninguno de esos controles.

**Evidencia** (`archctl/src/render.rs:1-68`):
```rust
let client = reqwest::blocking::Client::builder().timeout(15s)?;
client.post(&url).body(body).send()?;  // POST a kroki sin opt-in
```

**Deps ausentes**: `plantuml-little 1.2026.2-4`, `merman 0.8.0-alpha.3`, `petgraph + dagre-rs + svg`. ADR-012 los declara explícitamente como "renderers adopted in M9". Cero en `Cargo.toml`/`Cargo.lock`.

**Manifest `manifests/render.toml` debe_hold gate**: enforce `reqwest` + `.archctl-rendered` literal. Enforce el comportamiento *actual* (kroki POST), no el ADR.

**Severidad**: 🔴 — potencial exfiltración silenciosa de bundle structures vía POST a `https://kroki.io` (un usuario podría pasar `--kroki-url https://kroki.io` sin warning). El `AGENTS.md:42` promete "0 secretos en el repo", pero el repo tiene un canal de egreso oculto.

**Fix propuesto**: ciclo dedicado "M9-renderers-as-libraries" — vendorizar `plantuml-little` + `merman` (o re-implementar subset), agregar `--allow-public-renderer` flag + audit log en `AnalysisRun` + console warning. Mantener `--kroki-url` solo si flag está presente.

---

#### F2. ADR-009 (Relaciones semánticas reificadas) — bypass por call-graph writer

**Drift crítico**: schema declara el modelo reificado (`SemanticRelation` + `REL_SOURCE` + `REL_TARGET` + `RELATION_TYPE` + `RelationVersion`), pero el único writer de relaciones en la app (`archctl/src/code/call_graph.rs:1012-1113`) escribe directamente al rel table `SEMANTIC_EDGE`, saltándose la reificación. El invariante "toda arista activa referencia una SemanticRelation" **no se enforce** porque las aristas activas están solo en `SEMANTIC_EDGE`.

**Evidencia**:
```rust
// archctl/src/code/call_graph.rs:1033-1035
// "Writing to SEMANTIC_EDGE … rather than the reified
//  REL_SOURCE→SemanticRelation→REL_TARGET pattern, because the sequence
//  projection reads from SEMANTIC_EDGE and needs r.props"
```

**Consecuencias**:
- `RelationVersion` table existe en schema (`docs/schema/001_initial_schema.cypher:76-87`) pero nunca se escribe ni se lee.
- `archctl graph repair-index` y `archctl graph verify-index` (ADR-009 §Operaciones) **no existen** en CLI.
- Si en el futuro `archview` necesita navegar "todas las versiones de una relación", no hay datos.

**Severidad**: 🔴 — la reificación es un pilar arquitectónico (ADR-005 + ADR-009). El schema la declara pero el call-graph writer rompe el invariante.

**Fix propuesto**: o bien (a) refactorizar `write_call_edge` para escribir el patrón reificado completo + actualizar `query_semantic_edges` para que lea de las tablas reificadas; o bien (b) marcar ADR-009 como "reificación deferred" y actualizar el schema para reflejar el modelo implementado. La opción (b) es más barata y honesta — la opción (a) requiere más trabajo y un refactor de `query_semantic_edges`.

---

### 🟡 SIGNIFICATIVO — merece ciclo dedicado

#### F3. ADR-008 (Recuperación, versionado, evolución) — schema de AnalysisRun sin writer

**Drift significativo**: schema tiene `AnalysisRun` + `Snapshot` + `AT_SNAPSHOT` + `RUN_INPUT_SNAPSHOT` + `RUN_OUTPUT_SNAPSHOT` + `RUN_USED_TOOL` + `RUN_PRODUCED_ARTIFACT`. Cero código de aplicación los escribe o lee. `archctl run resume` (top-level command del ADR) **no existe** en CLI.

**Evidencia**: `GraphStore` trait (post-sub-trait split) no expone `put_snapshot`, `put_analysis_run`, `link_run_*`. `archctl run` no es sub-command.

**Severidad**: 🟡 — el MVP no necesitaba `archctl run resume` (M0–M11 son todos one-shot commands). El ADR lo declara como feature de recovery que llega con M14+. Documentado como "decisión deliberada". Pero el schema ya está cargado — costo de mantener la tabla `AnalysisRun` sin uso.

**Fix propuesto**: o eliminar las tablas no usadas del schema (rollback suave) o implementar `archctl run` (rollback completo). Documentar la decisión.

#### F4. ADR-001 — custom tool names declarados no existen en CLI

**Drift significativo**: ADR-001 declara 8 custom tools (`arch_project`, `arch_run`, `arch_scan`, `arch_graph`, `arch_snapshot`, `arch_scenario`, `arch_diagram`, `arch_artifact`). El perfil OpenCode en `profile/plugins/archctl-env.ts:1-39` los reemplaza con env-var injection. **Cero** de esos nombres existen como subcomandos CLI en `archctl/src/cli.rs:303-352`.

**Consecuencias**: las 5 subagentes en `profile/agents/*.md` invocan comandos inexistentes (`archctl scenario …`, `archctl scan …`, `archctl graph evidence/path/repair-index`, `archctl diagram put/materialize`). Si se ejecutan las subagentes, fallarán en runtime.

**Severidad**: 🟡 — la divergencia entre docs (profile/) e implementación (cli.rs) bloquea el caso de uso real del agente. El plugin env-var injection es una mitigación parcial pero no resuelve los subcomandos faltantes.

**Fix propuesto**: o bien (a) actualizar `profile/agents/*.md` y `profile/skills/*` para usar la CLI actual (`archctl evidence extract/list/accept/...`, `archctl inventory tree/languages/depends`, `archctl diagram export/validate/apply`, `archctl graph init/stat/query/neighbours`); o bien (b) implementar los subcomandos faltantes. La opción (a) es más barata y honesta.

#### F5. ADR-007 — view.edge nunca implementado

**Drift significativo**: ADR-007 §"Persistencia de vistas" declara `view.diagram`, `view.member`, `view.edge`, `view.group`. Schema implementa `Diagram`, `ViewMember`, `ViewGroup`. **`ViewEdge` no existe** en schema ni en port. El enum `Command` solo tiene `move-member | collapse-group | set-label` — no `add-edge` / `remove-edge` / `edit-edge`.

**Severidad**: 🟡 — feature mínima viable sin edge-level view overrides. El bundle se genera correctamente. Pero si `archview` quiere overlay de aristas (decoradores, badges, highlighting), no hay forma de persistir esos overrides.

**Fix propuesto**: implementar `ViewEdge` table + `add-edge` / `edit-edge` / `remove-edge` commands. Pequeño ciclo de 3–4 commits.

#### F6. ADR-005 — `LadybugArchitectureGraph` renombrado a `LbugStore`

**Drift cosmético**: el ADR nombra el trait `ArchitectureGraph` y el adapter `LadybugArchitectureGraph`. El refactor `refactor-1b-filesystem-port` + posteriores renombraron a `GraphStore` + `LbugStore`. ADR no actualizado.

**Severidad**: 🟢 cosmético — el contrato estructural es correcto (port hexagonal con adapter único). Solo naming.

**Fix**: actualizar el ADR para reflejar el naming actual.

#### F7. ADR-004 — XDG path usa UUID, no `<host>/<owner>/<repo>--<id>`

**Drift cosmético**: ADR §"Persistencia" describe `$XDG_DATA_HOME/archctl/projects/<host>/<owner>/<repo>--<repository-id>/`. El código (`archctl/src/identity.rs:130-155`) usa `portable_project_id` que genera UUIDv4, no `<host>/<owner>/<repo>--<id>`. `AGENTS.md:42-44` y `manifests/project.toml:4` reflejan el código, no el ADR.

**Severidad**: 🟢 cosmético.

**Fix**: actualizar ADR §"Estructura" para reflejar UUID. O bien revertir a `<host>/<owner>/<repo>--<id>` (más legible para humanos).

---

### 🟢 MENOR — ya documentado o aceptado

#### M1. ADR-016 — ubicación incorrecta

`docs/ADR-016-activegraph-packs-investigacion.md` vive en `docs/` (raíz), no en `docs/adr/`. El `docs/adr/README.md` lo referencia como `docs/adr/ADR-016-...md` (link roto). Estado: `Investigación cerrada. Decisiones pendientes` — el contenido sigue siendo relevante (B1 source/eval types sí se implementó) pero la ubicación física es drift.

**Fix**: mover a `docs/adr/ADR-016-activegraph-packs-investigacion.md` o fusionar con ADR-017 (migration runner, que sí implementó B1).

**Resolution (2026-08-18):** ADR-016 ya fue relocalizado a `docs/adr/ADR-016-activegraph-packs-investigacion.md` por commit `fe66349 docs(roadmap): M3+M1+M2 audit fixes` (2026-08-02). Adicionalmente, en 2026-08-18 (cycle `adr-backlog-acceptance`) se reescribió el Status header per-bloque: B1 = Decidido via [ADR-017](ADR-017-schema-migration-runner.md); B2/B3 = Pendiente con reopen triggers. Esta finding queda cerrada.

#### M2. ADR-015 y ADR-018 — referencias huérfanas

- `ADR-015` (Ports faltantes Clock/Environment/Filesystem) referenciado en `docs/STATE.md` (snapshot histórico). Nunca se escribió como ADR separado — los ports Clock y Environment se implementaron en commits posteriores; Filesystem en `refactor-1b-filesystem-port`. La decisión está consolidada implícitamente.
- `ADR-018` referenciado en `docs/ROADMAP.md:454` ("ADR-018 eliminado"). Nunca se escribió — fue propuesto y descartado.

**Fix**: o escribir los ADRs retroactivamente como histórico, o actualizar los docs para reflejar que fueron rolled-up into otros ADRs.

**Resolution (2026-08-18):** decisión tomada y documentada en `docs/ROADMAP.md:1262` ("Decisión sobre ADR-015 / ADR-018"): **no escribir retroactivamente**. ADR-015 está consolidado en `archctl/src/clock.rs`, `archctl/src/environment.rs`, `archctl/src/filesystem.rs`. ADR-018 fue eliminado en el planning de `m9-archctl-export-apply` (ver `sddk/m9-archctl-export-apply/coherence-report.md:196`). Esta finding queda cerrada con la nota per-cycle-2026-08-18 en sddk/adr-backlog-acceptance.

#### M3. ROADMAP cycle table (línea 290) desactualizada

La tabla "Cambios SDD completados" lista hasta `m11-call-graph-sequence` (v0.9.0). Faltan las 3 cycles más recientes (todas ya cerradas con sus "Cycle cerrado" sections abajo de la tabla):

| Cycle | Tag | Status |
|---|---|---|
| `refactor-m9-debt-cleanup` | v0.9.1 | ✅ Cerrado (Cycle cerrado §) |
| `refactor/store-port-seams` | v0.9.2 | ✅ Cerrado (Cycle cerrado §) |
| `m20-benchmark-suite` | v0.10.0 | ✅ Cerrado (Cycle cerrado §) |

**Fix**: añadir las 3 rows a la tabla. 1-line edit.

#### M4. ADR-012 — `archctl skills sync` usa `Command::new("git")` en vez de `gix`

ADR-012 dice "descartar CLIs; usar librerías". `gix = "0.86"` ya es dep. Pero `archctl/src/skills.rs:47-65` aún usa `Command::new("git").arg("clone").arg("checkout")`. Lo mismo para `archctl doctor --version` checks via `Command::new("curl")` (`archctl/src/doctor.rs:79-92`).

**Severidad**: 🟢 — la ADR permite excepciones para build metadata e IaC; `git clone` para skill sync es razonable pero podría usar `gix`. No bloqueante.

**Fix**: ciclo de cleanup "prefer gix over git subprocess" — 2 commits.

#### M5. ADR-019 — `export p99 <2s for <10k` no se cumple en medium-1k

Baseline medido (v0.10.0): `export_query_semantic_edges_medium: 1k nodos / 2500 rels / ~2.8 s`. El budget ADR-019 es `<2s for <10k nodos`. A 1k ya estamos en 2.8s — **10× sobre budget** para un dataset 10× menor.

**Causa raíz**: la bulk Cypher insert del seed (`MATCH ... CREATE` en `SEMANTIC_EDGE` REL TABLE) domina el costo del iter del bench, no el query real. Documentado en `benchmarks/README.md` §"Follow-ups".

**Severidad**: 🟡 — el budget ADR-019 es aspiracional; el bench actual mide seed cost, no export cost. Requiere "seed-cost decomposition" antes de poder validar el budget.

**Fix**: ciclo "m20-seed-decomposition" — mover la bulk insert del seed a `criterion::BenchmarkGroup` setup phase (no en el loop de medición).

---

## ADRs N/A para `archctl` (verificados correctos)

| ADR | Estado | Por qué N/A |
|---|---|---|
| **ADR-006** | DEPRECATED por autor | "Sustituido por ADR-012 + ADR-013". El propio ADR se declara obsoleto. |
| **ADR-014 Ola 2 (SparrowDB)** | Deferido por contrato | El ADR §Próximos pasos etiqueta Ola 2 como "cuando se decida". Cero SparrowDB en `Cargo.toml`/`Cargo.lock` — correcto. |
| **ADR-020** | archview exclusivo | G6, cosmos.gl, ELK.js, SolidJS, Apache Arrow, wasm-bindgen, RoaringBitmap — todo TS/web. `archctl` no tiene nada. Correcto. |
| **ADR-021** | 1.x (M21) | Cognitive Layer — `AgentObserver`, `AgentContext`, `AgentOutput`. Cero código en `archctl/src/`. ROADMAP §M21 marca 1.x. |
| **ADR-022** | 1.x (M22) | 9 agentes — `ArchitectureAgent`, `ProjectionAgent`, etc. N/A. |
| **ADR-023** | 1.x (M23) | `ActionProposal`, `Policy`, MCP gateway. N/A. |

---

## Cross-check ROADMAP ↔ git log ↔ tags

| Source | Claimed | Actual | Match |
|---|---|---|---|
| ROADMAP cycles table | 11 cycles cerrados + 2 "En curso" | 14 cycles cerrados (v0.1.0–v0.10.0 = 11 minor/patch + 3 refactor/bench) | ⚠️ Tabla stale (M2 arriba) |
| `Cambios SDD completados` table | hasta v0.9.0 | v0.9.0 ✓, v0.9.1/v0.9.2/v0.10.0 faltan en tabla | ⚠️ M3 |
| Tags semánticos | v0.1.0–v0.9.0 (10 patches + minor) | v0.1.0–v0.10.0 (11 minor/patch) | ✅ 19 tags totales |
| `Cycle cerrado` sections | v0.9.1, v0.9.2, v0.10.0 | Todas presentes con summary + tag SHA | ✅ |
| `Próximo candidato` después de M11 | M12 o M17.0 | Sigue válido (no se ha abierto ninguno nuevo) | ✅ |
| `Próximo candidato` después de M8 | M11 (call graph + sequence) | ✅ shipped | ✅ |
| Tag v0.6.1 hygiene | "v0.6.1 patch: hygiene commit" | Presente en tags | ✅ |

**Veredicto ROADMAP**: alineado con implementación, excepto por la tabla de "Cambios SDD completados" que no se actualizó con los 3 cycles más recientes. El cuerpo del documento (sections individuales) sí está actualizado.

---

## Resumen de ciclos cerrados vs ROADMAP claimed

| Cycle | Tag | ROADMAP §"Cycle cerrado" | sddk/ artifacts | Match |
|---|---|---|---|---|
| refactor-1b-filesystem-port | v0.1.0 | ✅ | ✅ | ✅ |
| refactor-1c-scope-port | v0.1.1 | ✅ | ✅ | ✅ |
| b1-source-evaluation-types | v0.2.0 | ✅ | ✅ | ✅ |
| more-manifests-clock-env-identity | v0.2.1 | ✅ | ✅ | ✅ |
| fix-parallel-lbug-test-races | v0.2.2 | ✅ | ✅ | ✅ |
| b1-lifecycle-drafted-accepted | v0.3.0 | ✅ | ✅ | ✅ |
| refactor-extract-cell-to-json-map | v0.3.1 | ✅ | ✅ | ✅ |
| m9-archctl-export | v0.4.0 | ✅ | ✅ | ✅ |
| hygiene-local-only-policy | v0.4.1 | ✅ | ✅ | ✅ |
| more-manifests-2 | v0.5.0 | ✅ | ✅ | ✅ |
| m9-archctl-export-apply | v0.6.0 | ✅ | ✅ | ✅ |
| m8-c4-boundary-inference | v0.7.0 | ✅ | ✅ | ✅ |
| m11-call-graph-sequence | v0.8.0/v0.8.1/v0.9.0 | ✅ | ✅ | ✅ |
| roadmap-pivot-v2.4 | (diferido) | ✅ | ✅ (N/A no release) | ✅ |
| roadmap-pivot-v2.5 | (diferido) | ✅ | ✅ (N/A no release) | ✅ |
| **refactor-m9-debt-cleanup** | **v0.9.1** | ✅ | ✅ | ✅ pero **falta en tabla Cambios SDD** (M3) |
| **refactor/store-port-seams** | **v0.9.2** | ✅ | ✅ | ✅ pero **falta en tabla Cambios SDD** (M3) |
| **m20-benchmark-suite** | **v0.10.0** | ✅ | ✅ | ✅ pero **falta en tabla Cambios SDD** (M3) |

---

## Acciones recomendadas por prioridad

| # | Acción | Ciclo sugerido | Esfuerzo |
|---|---|---|---|
| 1 | F1 — implementar renderers-as-libraries + bloqueo público | `M9-renderers-local` (re-pivot) | 🟠 medium-large (vendorizar 2 libs, agregar flag, audit log) |
| 2 | F2 — decidir fate de reificación de relaciones | `M9-relations-decision` | 🟡 small (decisión + ADR update + 2 ciclos de cleanup) |
| 3 | M3 — actualizar tabla "Cambios SDD completados" | commit único en main | 🟢 trivial (3 rows) |
| 4 | F3 — decidir fate de `AnalysisRun` + `Snapshot` (eliminar o implementar) | ciclo de cleanup | 🟡 small (decisión + ADR update + schema change) |
| 5 | M1 — mover ADR-016 a `docs/adr/` | commit único | 🟢 trivial |
| 6 | M2 — escribir ADR-015/018 retroactivo o quitar referencias | commit único | 🟢 trivial |
| 7 | F4 — alinear `profile/agents/*.md` con CLI actual (o implementar subcomandos faltantes) | ciclo de docs o M | 🟡 medium |
| 8 | F5 — implementar `ViewEdge` (table + commands) | ciclo de feature | 🟡 small (3-4 commits) |
| 9 | M5 — `seed-cost decomposition` en benches | ciclo de bench refinement | 🟡 small (refactor del seed helper + bench loops) |
| 10 | F6, F7, M4 — cosmetic drift fixes (naming, gix over git) | commits individuales | 🟢 trivial cada uno |

---

## Conclusión

**Estado del proyecto**: sólido. 260 tests passing, 19 tags semánticos, 11+ ciclos cerrados, 23 ADRs escritos, manifests gates limpias, bench harness en sitio (M20). La arquitectura fundamental está alineada con los ADRs.

**Problemas críticos** (2): F1 (renderer network egression) y F2 (relación reificación rota). Ambos son corregibles con un ciclo de 4–8 commits cada uno.

**Documentación drift** (5): M1, M2, M3, F6, F7 — todos cosméticos, todos corregibles con edits de docs y/o rename.

**Próximo ciclo recomendado**: **F1 — `M9-renderers-local`** porque es el único drift con implicaciones de seguridad (egreso silencioso de bundles via POST a kroki público). El segundo en prioridad sería F2.

Veredicto final: **alineado con 2 issues críticos a resolver antes de abrir features grandes** (M12, M17.0).