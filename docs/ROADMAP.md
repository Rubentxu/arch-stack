# Roadmap — OpenCode Architecture Diagrammer

**Estado:** propuesta revisada (pivot a Code Knowledge Graph Workbench performance-first)
**Versión:** 2.4
**Fecha:** 31 de julio de 2026
**Cambios vs 2.3:** Pivot a "Code Knowledge Graph Workbench" (no BI dashboard). M9 reescrito como workbench de 5 vistas coordinadas con performance-first stack. Nuevos ADRs: **ADR-019 (Performance budget — hard contract)** y **ADR-020 (Renderer stack: G6 5.x WebGPU + cosmos.gl + SolidJS + Rust/WASM)**. ADR-007/011/013 revisados con el nuevo stack. M17 (archview workbench) promovido a prioridad 1. M11/M12 promovidos. M10/M14 deferred a 1.x. Ver `docs/Librerías-visualización-grafos-BI.md` para la investigación que sustenta el pivot.

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

**Pivot v2.4 (2026-07-31):** M9 ya no es "renderers como librerías (PlantUML, Mermaid, Structurizr propio)". Es **Code Knowledge Graph Workbench** — un workbench con 5 vistas coordinadas (C4 contextual, call graph, sequence, class, package) renderizado con stack performance-first (ver [ADR-019](adr/ADR-019-performance-budget.md) y [ADR-020](adr/ADR-020-renderer-stack.md)). El target es developers/arquitectos, no BI. M9 incluye también el setup inicial del workbench (M17.0–M17.1) y la primera validación con `archctl code c4 discover` + `archctl code call-graph`.

## M10 — Casos de uso y escenarios (era M9)

**Pivot v2.4:** Defer a 1.x. Bajo valor vs costo en el target de developers/arquitectos. Los use cases no son el dolor primario del target.

## M11 — Call graph + Sequence diagrams + C4 Dynamic (era M10) — **PRIORIDAD 1**

**Pivot v2.4:** Promovido a prioridad 1. M11 ahora incluye:
- **Call graph extraction** (via tree-sitter / LSP) — `archctl code call-graph` ✅ shipped v0.8.0
- **Sequence diagram generation** (call chain extraction, async flow tracking) — `archctl code sequence` ✅ shipped v0.9.0
- **C4 Dynamic** (relationships at runtime, opcional via OpenTelemetry)

Output: tres comandos CLI que se renderizan en `archview` como proyecciones del workbench.

## M12 — Diagramas de clases UML (era M11) — **PRIORIDAD 2**

**Pivot v2.4:** Promovido. Output: `archctl code class-diagram` (UML via tree-sitter CST walk, intra-file scaffold; LSP deferido a fase 2). Renderizado en `archview` como vista "class".

## M13 — Workbench actions — **WON'T DO en v1.x** (decisión 2026-08-02)

**Pivot v2.5:** Reubicado fuera del target v0.x. Drift detection C4, impact analysis y test mapping son features orientadas a enterprise/CI workflows, no al target developer/architect de `archview`. El workbench de M17 cubre los flujos interactivos necesarios (browse + filter + select). Si surge demanda real desde un usuario, reevaluar como v1.x.

## M14 — Versionado, recuperación y rollback — **WON'T DO en v1.x**

**Pivot v2.5:** Snapshots y rollback son features enterprise (compliance, audit, recovery). El grafo canónico en lbug ya está versionado por `current_version_id` en cada Element/Relation (ADR-008) — el "writable snapshot" es la base de datos misma. Multi-versioning y rollback explícito solo justificables si un usuario enterprise los pide.

## M15 — Herramientas semánticas opcionales — **WON'T DO en v1.x**

**Pivot v2.5:** OpenTelemetry traces, ML-based similarity, semantic clustering — todo nice-to-have. Sin demanda concreta. Defer indefinidamente.

## M16 — Endurecimiento 1.0 (era M15) — **PRE-M17 BLOCKER**

**Pivot v2.5:** Endurecimiento antes de M17. Tareas concretas:
- lbug infra gap: restaurar `doctor --scopes` runtime (F3.3)
- fmt-staged script + AGENTS.md nota (F3.2)
- audit `manifests/code.toml` (F2.3)
- `refactor/extract-code-apply-helpers` (~150 LOC deuda) — **Cerrado v0.13.2 ✅**

## M17 — `archview` workbench (sustituye a Av0–Av6) — **PRIORIDAD 1 — EN CURSO**

**Pivot v2.4:** Reframe del plan original de `archview` (Av0–Av6) en milestones explícitos:

> **Avance 2026-08-03 (explore + m17-contract-alignment, v0.14.3):** el explore `m17-workbench-state` reveló que M17.0 está hecho y M17.1–M17.7 tienen MVPs de lista-texto (7 vistas en `archview/src/views/`), pero el loader consumía un formato C4 custom incompatible con el `viewer-bundle` real de `archctl diagram export`. `m17-contract-alignment` (v0.14.3) alineó el loader con el schema canónico (`manifest`/`projection`/`evidence`/`styles`), cerró 2 deudas HIGH (time-mutation, boundary g6→types) y añadió el contrato compartido `types.ts` + tests E2E con fixture validado por `archctl diagram validate`. **`m17-routing-fix` cerrado ✅ (v0.14.4)** — CallGraphView/PackageView ahora alcanzables via `routing.ts` resolveView total discriminant. **`fix-m17-package-view-onselect` cerrado ✅ (v0.14.5)** — PackageView onSelect `pkg.name`→node agora povoa o sidebar via synthetic `GraphNode` (Option D, `buildPackageNode`). **Pendiente**: WebGPU/ADR-019 (0% implementado), fixture exporter-derived cuando archctl arregle el mismatch `category c4 vs container` (`c4_discover.rs:231` vs `diagram/queries.rs:72`).

- **M17.0**: SolidJS + G6 5.x WebGPU (ver ADR-020). Setup inicial del workbench, scaffold, build pipeline. **Single PR → tag v0.14.0 en repo separado `archview`**. Scope MVP: bundle loader + pan/zoom + sidebar de evidencias. Mínimo para que los bundles de M11/M12 sean visualizables.
- **M17.1**: Semantic zoom para C4 (Context → Container → Component → Code).
- **M17.2**: Call graph view (1-N niveles, blast radius, async flow).
- **M17.3**: Sequence diagram view (call chains, async flows).
- **M17.4**: Class diagram view (UML).
- **M17.5**: Package diagram view (dependencias, ciclos, cohesión).
- **M17.6**: Drift detection (C4 declarado vs actual; cross-validation). Requiere M13 si se reactiva, sino implement in-situ.
- **M17.7**: Impact analysis (blast radius de un cambio propuesto). Requiere M14 si se reactiva, sino implement in-situ.

Performance budget (ver ADR-019): TTFP <1s, pan/zoom 60 FPS, filter <50ms, memory <500MB para 100k nodos.

**Repositorio**: `archview` (separado de `archctl`). Primer release tag `v0.14.0` cuando M17.0 cierre. Co-evoluciona con `archctl` v0.14.x (consume bundles vía CLI).

## M18 — Reactive runtime (event log + behaviors + planners) — **NUEVO, 1.x**

**Pivot v2.4 + v2.5:** Reactive runtime inspirado en ActiveGraph pero implementado en Rust puro. Defer a 1.x (después del workbench estable). Features: event log, subscriptions, behaviors como WASM plugins, planners, capabilities. Ver sección del doc sobre Reactive Runtime.

> **Pivot v2.5 (2026-07-31, post-capa-cognitiva):** M18 se reposiciona como el substrate sobre el cual corre la Cognitive Layer (ver M21-M23). El reactive runtime añade la capacidad de que comportamientos (algoritmos deterministas) Y agentes (LLM) reaccionen al estado del grafo. Ver [ADR-021](adr/ADR-021-cognitive-layer.md).

## M19 — Custom wgpu renderer (solo si cosmos.gl no alcanza) — **NUEVO, 2.0**

**Pivot v2.4:** Si cosmos.gl + G6 WebGPU no cubren el caso de grafos de millones de elementos con latencia sub-16ms, construir un renderer custom en Rust + wgpu + WGSL. 2.0. Defer a menos que el benchmark suite (M17) muestre insuficiencia.

## M20 — Performance validation cycle — **COMPLETO ✅ (2026-08-03)**

**Pivot v2.4:** Cycle dedicado a implementar el benchmark suite de ADR-019. Datasets canónicos (`benchmarks/datasets/{small,medium,large}.json`), CI gate, profiling setup. Sin esto, el performance budget es teoría.

**Hecho (v0.10.0 + v0.13.6 + v0.13.7):** harness criterion (export/apply/query/class-diagram pipelines), 3 datasets canónicos, doctor scope gate, **CI gate GitHub Actions** (build/test/clippy/fmt/doctor + bench smoke + bundle cap ≤2MB), **regresión >10% vs main** (`scripts/bench-compare.sh` + job CI en PRs, ADR-019 §1).

**Pendiente opcional (no bloqueante):** profiling-on-regression flamegraph, PR-comment bot.

## M21 — Cognitive Layer foundation — **NUEVO, PRIORIDAD 1 (1.x)**

**Pivot v2.5 (2026-07-31, post-capa-cognitiva):** Substrate sobre el cual corren los agentes especializados. Outputs:
- Contrato `ReactiveObserver` + `AgentContext` + `AgentOutput` (ver [ADR-021](adr/ADR-021-cognitive-layer.md))
- ModelPolicy + AgentBudget + escalation ladder (heurística → local → potente → humana)
- MCP gateway mínimo (3 tools read-only: `graph_query`, `schema_validate`, `run_tests_local`)
- 2 agentes proof-of-concept (heurística pura): **Architecture Agent** + **Projection Agent**

Output verificable: queries del workbench responden con output estructurado (no solo texto). Foundation sienta las bases para M22.

## M22 — Agent catalog v1 — **NUEVO, 1.x**

**Pivot v2.5:** Catálogo inicial de los 9 agentes especializados (ver [ADR-022](adr/ADR-022-agent-catalog.md)):
- Semantic Curator · Architecture · Projection · Investigation · Impact · Planning · Documentation · Presenter · Review/Critic

Para v1.0 (M16) solo Architecture + Projection (heurística pura). Para 1.x, los otros 7 agentes con LLM local (Phi-3 / Llama-3-8B) + LLM potente (Claude/GPT) para los más sensibles (Investigation, Planning, Review).

## M23 — Action Proposal & Policy Engine — **NUEVO, 1.x**

**Pivot v2.5:** Implementación completa del ActionProposal + Policy Engine + MCP gateway (ver [ADR-023](adr/ADR-023-action-proposal-and-policy.md)):
- ActionProposal estructurado (goal + command + capabilities + approval + evidence esperada + rollback)
- Policy Engine con reglas declarativas (TOML) editables sin recompilar
- MCP gateway como frontera de capabilities (resources = read-only, tools = con efectos, prompts = procedimientos)
- Audit log append-only en el grafo (inmutable)
- HITL UI en `archview` (mostrar proposals pendientes al usuario)

Output: el sistema puede ejecutar acciones gobernadas (no solo leer). Por ejemplo: `archctl code c4 discover --auto-apply` (corre agentes, valida confidence > 0.9, ejecuta propuesta vía MCP).

> **Pipeline de v1.x**: M18 (reactive runtime) → M20 (benchmark) → M21 (cognitive foundation) → M22 (agent catalog) → M23 (action proposal + policy). Cada cycle valida el anterior.

**Pivot v2.4:** Cycle dedicado a implementar el benchmark suite de ADR-019. Datasets canónicos (`benchmarks/datasets/{small,medium,large}.json`), CI gate, profiling setup. Sin esto, el performance budget es teoría.

## M24 — Diagram authoring toolchain — **COMPLETO** ✅

**Estado:** Implementado en v0.14.6. PR #23 mergeado. ADRs 026-029 respetados.

**Objetivo:** Cerrar el pipeline de creación de diagramas donde `archctl` = herramientas (extracción → grafo → proyección → DSL) y las skills de `profile/skills/` = inteligencia (qué diagrama, qué destacar, cómo). Los agentes crean y dan sentido a los diagramas usando las herramientas de archctl.

**Alcance (3 gaps + realineación):**
- **G3 — `evidence put`** ([ADR-027](adr/ADR-027-evidence-put.md)): ingestión de hechos semánticos (actores, use cases, reglas de negocio, semántica de estados) con procedencia `SourceArtifact`. Identity scheme `ev:sem:blake3(...)` para hechos sin archivo. Solo Evidence + SourceArtifact, NO crea Elements.
- **G4 — `diagram project`** ([ADR-028](adr/ADR-028-diagram-project.md)): grafo → fuente PlantUML/Mermaid/Structurizr editable. `ViewKind` (c4-container/context/component, class, sequence, state, usecase). `ProjectSelector` independiente del `C4Kind` de export (ADR-013).
- **G1 — `code state-machine`** ([ADR-026](adr/ADR-026-state-machine-metamodel.md)): extractor AST-puro (Rust enums+match, TS unions+switch, Python decoradores). Metamodelo extendido: `uml.state_machine`, `uml.state`, `uml.transition`, `uml.guard`, `uml.event` + predicates (`behavior.source_state`, `behavior.target_state`, `behavior.has_transition`, `behavior.trigger`, `behavior.has_guard`). MERGE apply-time (patrón M11/M12), confidence < 1.0. Guards/eventos complejos → agente vía `evidence put`.
- **G5 — C4 component light** ([ADR-029](adr/ADR-029-c4-component-light.md)): estrategia `components` en `c4-discover` — módulos internos → candidatos `mt.component` con `confidence < 1.0`; el agente revisa y promueve vía `evidence accept`. Reutiliza framework Strategy existente.
- **Skills realineadas**: las 6 skills de `profile/skills/` referenciarán SOLO comandos reales (existencia verificada contra el CLI).

**Fuera de alcance:** render SVG nativo de PlantUML/Mermaid (bloqueado por `libgraphviz-dev`, ciclo de vendor separado), `diagram materialize` (requiere diseño de lock/override — deuda ADR-013), `run start/close`, `review put` (cubierto por `evidence put` + `accept`).

**Outputs verificables:** `archctl evidence put` funcional; `archctl diagram project --view <kind> --format <dsl>` emite fuente editable determinista; `archctl code state-machine --apply` puebla grafo con estados/transiciones; `archctl code c4-discover --strategy components` emite candidatos; skills con comandos verificados.

**Referencias:** `docs/adr/ADR-026-state-machine-metamodel.md`, `docs/adr/ADR-027-evidence-put.md`, `docs/adr/ADR-028-diagram-project.md`, `docs/adr/ADR-029-c4-component-light.md`, `sddk/diagram-authoring-toolchain/{proposal,spec,design}.md`

## M25 — Strategy pattern refactor (follow-up) — **COMPLETO** ✅

**Estado:** Implementado en v0.14.7. PR #25 mergeado.

**Objetivo:** Eliminar duplicación identificada en debt-verify de M24. El patrón Strategy tenía dos ramas de código paralelas (`StrategyId::Components` vs `StrategyId::Containers`) que diferían solo en el metatype destino.

**Cambios:**
- `trait Strategy` gana `fn metatype(&self) -> &'static str` (ADR-026)
- Implementado en 5 estrategias concretas: cargo/npm/dockerfile/helm → `mt.container`, components → `mt.component`
- Eliminado `enum StrategyId` y 3 funciones duplicadas: `write_component_element`, `write_component_element_version`, `link_component_element_edges`
- `apply()` unificado en ruta única que deriva `(metatype, element_prefix)` del nombre del strategy

**Resultado:** -88 LOC net en archivos de producción. 290 lib + 90 integration tests passing.

**Referencias:** PR #25, commit 47ae361

## Mejoras futuras — `workflowctl`

> Documentadas en [ADR-024](adr/ADR-024-workflowctl-local-multi-repo.md). **No implementadas ahora**: se dejan como referencia para una eventual promoción a topología distribuida. Mantener este bloque sincronizado con ADR-024; cualquier cambio de estado requiere abrir un nuevo ADR.

- MVP local-first: `workflowctl` se ejecuta en el host del desarrollador con `gh act` + Podman rootless, encapsulado en `systemd-run --user`. Concurrencia 1 workflow / 2 jobs internos. Perfiles estándar (4 CPU / 8 GiB), pesado (8 / 16) y benchmark (12 / 16 exclusivo). Snapshot copiado, sin `--bind`, sin `--reuse`, `--container-daemon-socket -`, cache/artifacts solo en `127.0.0.1`.
- Compatibilidad GitHub documentada por workflow y por job (`rust`, `web`, `bench-smoke`, `bench-compare`, `test-unit`, `test-e2e/ui/all`). Jobs con `nested-container` (Podman dentro del workflow) **no se ejecutan** en MVP.
- Diferido: runner remoto efímero dedicado (host bare-metal, identidad de servicio, secret store, `systemd` slice, auth por runner group); coordinator híbrido local + remoto; imagen base propia `act-runner:arch`; `forgejo-runner` o equivalente; perfiles declarativos por repositorio.
- Regla de promoción: exige simultáneamente host dedicado, demanda medida (contención, varios hosts pidiendo cola o workstation apagado bloqueando runs), suite de compatibilidad común, pin SHA en todas las actions, política única de routing, subuid/subgid disjuntos y SELinux en enforcing. Si falla una sola, se mantiene el MVP local.

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
| `m9-archctl-export-apply` (PR1 + PR2) | `feat/m9-archctl-export-apply-foundation` → `feat/m9-archctl-export-apply` (merged to main via --no-ff) | `f8c4101` | **Cerrado** ✅ · tag `v0.6.0` |
| `more-manifests-2` | direct commit on `main` (no PR — bulk manifest cycle) | `d2c27fe` | **Cerrado** ✅ · tag `v0.5.0` |
| `m8-c4-boundary-inference` | `feat/m8-c4-boundary-inference` (merged to main via --no-ff) | `2c6e4e1` | **Cerrado** ✅ · tag `v0.7.0` |
| `m11-call-graph-sequence` (PR1 + PR2) | `feat/m11-call-graph` + `fix/m11-call-graph-tsg-rules` + `feat/m11-sequence` (merged to main via --no-ff) | `4dd7211` | **Cerrado** ✅ · tags `v0.8.0`, `v0.8.1`, `v0.9.0` |
| `refactor-m9-debt-cleanup` | `refactor/m9-debt-cleanup` (merged to main via --no-ff) | `f287594` | **Cerrado** ✅ · tag `v0.9.1` |
| `refactor/store-port-seams` | `refactor/store-port-seams` (merged to main via --no-ff) | `71ea783` | **Cerrado** ✅ · tag `v0.9.2` |
| `m20-benchmark-suite` | `m20-benchmark-suite` (merged to main via --no-ff) | `e64f7f9` | **Cerrado** ✅ · tag `v0.10.0` |
| `m9-renderers-local` | `m9-renderers-local` (merged to main via --no-ff) | merge commit | **Cerrado** ✅ · tag `v0.11.0` |
| `m9-relations-decision` | `m9-relations-decision` (merged to main via --no-ff) | merge commit | **Cerrado** ✅ · tag `v0.12.0` |
| `refactor/bench-seed-decomposition` | `refactor/bench-seed-decomposition` (merged to main via --no-ff) | merge commit | **Cerrado** ✅ · tag `v0.12.1` |
| `m12-class-diagram` | `feat/m12-class-diagram` (merged to main via --no-ff) | `9e665ee` | **Cerrado** ✅ · tag `v0.13.0` |
| `ci-main-gates` | `test/main-gates-05-contract` (merged to main via --no-ff) | `443b6fe` (merge commit) | **Cerrado** ✅ · tag `v0.13.8` |
| `release-pipeline` | `release/release-pipeline` (merged to main via --no-ff) | `43748b8` (merge commit) | **Cerrado** ✅ · tag `v0.14.1` · dup-002 fix: PR #12 ✅ |
| `archview-lint` | `chore/archview-lint` (merged to main via --no-ff) | `d6c89f2` (merge commit) | **Cerrado** ✅ · tag `v0.14.2` |
| `m17-contract-alignment` | `feat/m17-contract-alignment` (merged to main via PR #17) | `d98e1de` (merge commit) | **Cerrado** ✅ · tag `v0.14.3` |
| `m17-routing-fix` | `feat/m17-routing-fix` (merged to main via PR #19) | `2b08140` (merge commit) | **Cerrado** ✅ · tag `v0.14.4` |
| `fix-m17-package-view-onselect` | `fix/m17-package-view-onselect` (merged to main via PR #21) | `cd661e6` (merge commit) | **Cerrado** ✅ · tag `v0.14.5` |

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

## Cycle cerrado — `m9-archctl-export-apply` (PR1 — Foundation)

- **Fecha**: 2026-07-31
- **Branch**: `feat/m9-archctl-export-apply-foundation` (merged to main via --no-ff)
- **Tag**: none (deferido a PR2 → v0.6.0; PR1 es infraestructura foundation, sin superficie CLI)
- **Verdict**: verify PASS WITH WARNINGS (4W + 3S; W-2 resolved post-verify) · debt-verify PASS WITH WARNINGS (5W + 5S; W-DV-1 + W-DV-2 resolved post-verify)
- **Commits**: 12 (11 cycle + 1 merge --no-ff via ce25825)
- **Tests**: 198 passing (183 baseline + 15 new)
- **Output**: schema v3 migration (`003_view_nodes.cypher`: 4 NODE TABLEs + 3 REL TABLEs), DB lock via `fs2::try_lock_exclusive` on `.lbdb` (ADR-010 gap cerrada), 8 GraphStore port methods additivos (put_diagram, get_diagram, put_view_member, link_member_of, link_renders, put_view_group, link_group_contains, get_view_members), ViewMember x/y/collapsed columns, ViewGroup collapsed column. Trait grew 8→16 methods. Zero breaking changes.
- **PR1 specific output**: view_types.rs (+194 LOC), store.rs (+817 LOC), 003_view_nodes.cypher (+141 LOC), tests (+63 LOC). HTML closing report: `docs/reports/closing-pr1-m9-archctl-export-apply-foundation.html`.
- **Key decisions**: D-1: fs2::try_lock_exclusive on .lbdb (vs separate lockfile — elimina 150 LOC de lock.rs); D-2: ViewMember x/y/collapsed en DDL (W-2 fix); D-5/6: MERGE SET + RETURN con todas las columnas (W-DV-1/2 fix).
- **Debt carry to PR2**: W-DV-3 (open_lbug_session duplicado), W-DV-4 (trait bloat 16→21), W-DV-5 (link_* duplication ×3).
- **Próximo candidato**: PR2 (apply surface: `archctl diagram apply` CLI + override/lock model).

## Cycle cerrado — `roadmap-pivot-v2.4` (performance-first workbench)

- **Fecha**: 2026-07-31
- **Branch**: direct commits en `main` (no PR — doc-only + ADR-only changes)
- **Tag**: deferido a `v0.7.0` (cuando M9-v2.4 cierre con workbench funcional)
- **Verdict**: N/A (no código, solo docs)
- **Commits**: 1 chore(adr) + 1 chore(roadmap) = 2 commits planeados
- **Tests**: N/A (0 cambios de código)
- **Output**: Pivot del roadmap de BI dashboard a Code Knowledge Graph Workbench. Performance-first stack.
  - **ADR-007** revisado: reframe del viewer como "workbench de 5 vistas coordinadas" (C4 / call graph / sequence / class / package).
  - **ADR-011** revisado: nota de performance para `archview` (COOP/COEP, CSP, OffscreenCanvas).
  - **ADR-013** revisado: stack de `archview` reemplazado completamente. Sprotty y Cytoscape.js descartados. G6 5.x WebGPU + cosmos.gl + SolidJS + Rust/WASM. 5 vistas coordinadas explícitas.
  - **ADR-019** nuevo: Performance budget (hard contract). TTFP <1s, pan/zoom 60 FPS, filter <50ms, memory <500MB para 100k nodos. 14 anti-patterns explícitos. Benchmark suite canónico + CI gate.
  - **ADR-020** nuevo: Renderer stack. G6 5.x WebGPU primary, cosmos.gl adapter para >100k, ELK.js fallback jerárquico. SolidJS UI (no React). Rust → WASM compute. Apache Arrow + TypedArrays. Web Workers + SharedArrayBuffer. RoaringBitmap selections.
  - **ROADMAP v2.4**: M9 redefinido como workbench. M8 (C4 boundary inference) y M11 (call graph + sequence) promovidos a prioridad 1. M17 (archview) promovido a prioridad 1. M10 (use cases) y M14 (versioning) deferred a 1.x. M18 (reactive runtime) y M19 (custom wgpu) nuevos. M20 (performance validation) nuevo.
- **Próximo candidato**: M9-PR2 (apply surface) → v0.6.0, luego M8 (C4 boundary inference) y M11 (call graph + sequence) como foundation del workbench.

## Cycle cerrado — `m8-c4-boundary-inference`

- **Fecha**: 2026-08-01
- **Branch**: `feat/m8-c4-boundary-inference` (merged to main via --no-ff)
- **Tag**: v0.7.0 (minor — new CLI surface: `archctl code c4 discover`)
- **Verdict**: verify PASS WITH WARNINGS · debt-verify round 1 PASS WITH WARNINGS (0 CRIT, 0 HIGH)
- **Commits**: 9 (5 main + 3 fix + 1 merge)
- **Tests**: 235 passing (vs 222 baseline, +13)
- **Output**: `archctl code c4 discover [--apply] [--strategy <s>] [--json]` — deterministic C4 Container boundary inference. 4 MVP strategies: cargo-workspace (0.85), npm-workspace (0.80), dockerfile (0.60), helm (0.70). `--apply` persists Element + ElementVersion + Evidence + SourceArtifact + EXTRACTED_FROM edges via existing GraphStore port methods (zero new port surface). New bounded context `code::c4_discover` (6 files in `archctl/src/code/`). New manifest `manifests/code.toml`. New schema `schemas/discover-report.schema.json`. New dep: `serde_yaml = "0.9.34"`.
- **Deuda técnica cerrada**: CRIT-1 (EvidenceKind schema mismatch) closed via snake_case fix; HIGH-1 (apply() god method) closed via refactor; HIGH-2 (missing integration tests) closed via 4 new integration tests.
- **Scope deviation**: 5 spec scenarios DEFERRED (integration fixtures missing); EvidenceKind not consolidated with evidence::EvidenceKind (W-3); Element.props pre-existing gap surfaced.
- **Fix branch preserved**: `refactor/debt-m8-c4-boundary-inference-1` kept as historical record (git-contract rule: never delete branches).
- **Próximo candidato**: M11 (call graph + sequence — needs Container boundaries as scope anchor) o M17.7 (drift detection — needs inferred Containers).

## Cycle cerrado — `roadmap-pivot-v2.5` (cognitive layer)

- **Fecha**: 2026-07-31
- **Branch**: direct commits en `main` (no PR — ADR-only changes)
- **Tag**: deferido a M22 → 1.x
- **Verdict**: N/A (no código, solo docs)
- **Commits**: 1 chore(adr) = 3 ADRs nuevos
- **Tests**: N/A (0 cambios de código)
- **Output**: Adopción de la **capa cognitiva** sobre el grafo de conocimiento (ver `docs/Librerías-visualización-grafos-BI.md` sección "Code Knowledge Graph Workbench"). Tres ADRs nuevos formalizan el patrón:
  - **ADR-021 (cognitive layer)**: posición en 7 planos (Developer Experience / Cognitive / Projection / Reactive Runtime / Graph / Deterministic / Sensors); contrato uniforme `ReactiveObserver + AgentContext + AgentOutput`; escalera de resolución (heurística → local → potente → humana); coordinación vía estado (eventos), no conversación; MCP como capability boundary; v1.0 ship 2 agentes (heurística pura).
  - **ADR-022 (agent catalog)**: 9 agentes especializados (Semantic Curator, Architecture, Projection, Investigation, Impact, Planning, Documentation, Presenter, Review/Critic) con suscripciones, view, output schema, budget, capability. v1.0 (M16) ship Architecture + Projection; 1.x (M22) ship los otros 7 con LLM local Phi-3 / potente Claude.
  - **ADR-023 (action proposal + policy engine)**: ActionProposal estructurado (goal + command + capabilities + approval + evidence esperada + rollback); Policy Engine con reglas TOML editables; MCP gateway como única frontera de ejecución; audit log inmutable en el grafo; HITL UI en `archview`.
  - **ROADMAP v2.5**: M18 (reactive runtime) reposicionado como substrate de la cognitive layer. M21 (cognitive foundation) + M22 (agent catalog v1) + M23 (action proposal + policy) añadidos al roadmap 1.x.
- **Próximo candidato**: PR2 (m9-archctl-export-apply v0.6.0) → commit pendiente, luego M8 (C4 boundary inference) y M11 (call graph + sequence) como foundation del workbench → M17 (archview workbench scaffold) → M20 (performance validation) → M21-M23 (cognitive layer 1.x).

## Cycle cerrado — `m11-call-graph-sequence` (PR1 + PR2)

- **Fecha**: 2026-08-01
- **Branch**: `feat/m11-call-graph` + `fix/m11-call-graph-tsg-rules` + `feat/m11-sequence` (merged to main via --no-ff)
- **Tag**: `v0.8.0` (`2d7a9e9`), `v0.8.1` (`cd7ba27`), `v0.9.0` (`f2ca194`)
- **Verdict**: verify PASS → fix cycle PASS → release v0.8.0 → bug fix → release v0.8.1 → release v0.9.0
- **Commits**: 21 across 3 PRs (PR1: 12 commits → v0.8.0; fix: 6 commits → v0.8.1; PR2: 9 commits → v0.9.0)
- **Tests**: 235 passing (vs 222 baseline, +13 across all 3 releases)
- **Output**: `archctl code call-graph` (PR1 → v0.8.0) via tree-sitter-graph; bug-fixed via direct tree-sitter walk (v0.8.1) after `basemind-tree-sitter-graph` 0.12 rejected TSG rule patterns; `archctl code sequence` (PR2 → v0.9.0) via BFS over persisted call edges. Three bounded contexts: `code/call_graph.rs`, `code/sequence.rs`, `code/c4_discover.rs` (added in earlier cycle). New manifest `manifests/code.toml` extended for each. New schemas `call-graph-report.schema.json` and `sequence-report.schema.json`.
- **Carried debt** (deferred to refactor-m9-debt-cleanup): none — M11 cycle closed clean.

## Cycle cerrado — `refactor-m9-debt-cleanup`

- **Fecha**: 2026-08-01
- **Branch**: `refactor/m9-debt-cleanup` (merged to main via --no-ff)
- **Tag**: `v0.9.1` (`f287594`)
- **Verdict**: verify PASS · debt-verify PASS_WITH_WARNINGS (0 CRIT, 0 HIGH) · archive PASS
- **Commits**: 9 (6 refactor + 2 style + 1 docs)
- **Tests**: 260 passing (vs 259 baseline; +1 new atomic round-trip test)
- **Output**: 6 carryover debt items from `m9-archctl-export-apply` PR2 closed:
  - **W-DV-3** (open_lbug_session duplication) — extracted `graph::create_db_session`; `open_session` and `open_lbug_session` delegate.
  - **W-DV-4** (GraphStore trait bloat, 16 methods) — split into 3 sub-traits: `EvidenceOps` (5), `SourceOps` (4), `DiagramOps` (9 incl. new `update_view_member_label`). `GraphStore: EvidenceOps + SourceOps + DiagramOps` super-trait pattern; `GraphStore` keeps only `open/init/stat/query` directly.
  - **W-DV-5** (3× link_* MERGE+fallback) — extracted `link_with_merge_fallback` helper; applied to 5 sites (`link_extracted_from`, `link_evaluates`, `link_member_of`, `link_renders`, `link_group_contains`).
  - **W-DV2-A1** (DIP regression in `dispatch_command`) — `apply_to_store` and `dispatch_command` now take `&mut dyn GraphStore`.
  - **W-DV2-A3** (OCP shotgun for `Command` variants) — added `Command::apply` inherent method on the enum; `dispatch_command` reduced to `#[cfg(test)]` thin wrapper.
  - **W-DV2-C2** (RMW brittleness in `SetLabel`) — atomic `MATCH ... SET ... RETURN` Cypher via new `update_view_member_label` GraphStore method; lbug 0.18.3 silently returns 0 rows so an explicit `result.next().is_some()` check + `bail!("member not found: {id}")` preserves the old RMW error contract.
- **Files changed**: 8 (998 ins / 726 del). Net +272 LOC. Increase driven by T3 (4 doc comments on sub-traits) and T5 (Command::apply body inline).
- **Carried forward** (non-blocking): 4 WARN items in `sddk/refactor-m9-debt-cleanup/debt-report.md` (OE-W1 ISP not yet realized; CP-W1 Filesystem port bypassed in `create_db_session`; CP-W2 atomic label uses ambient `chrono::Utc::now()` instead of injected `Clock`; CP-W3 `Command::apply` still requires full `GraphStore` super-trait). All are maintainability/testability follow-ups; no functional blocker.
- **Próximo candidato**: M12 (class-diagram UML, prioridad 2) o M17.0 (archview workbench scaffold, prioridad 1) o cleanup de los 4 WARN carried (CP-W1 + CP-W2 son port-seam hygiene, fácil cerrar).

> `refactor-m9-debt-cleanup`: 6 carryover debt items from `m9-archctl-export-apply` PR2 closed via 6 refactor commits + 2 style + 1 docs. `GraphStore` restructured into 3 sub-traits (ISP benefit unlocked); `Command::apply` method on enum (OCP win); atomic `update_view_member_label` replaces RMW (W-DV2-C2). 260 tests passing without behavioural change. Patch tag `v0.9.1`. Archivado en `sddk/refactor-m9-debt-cleanup/archive-report.md`.

## Cycle cerrado — `refactor/store-port-seams` (v0.9.2)

- **Fecha**: 2026-08-01
- **Branch**: `refactor/store-port-seams` (merged to main via --no-ff)
- **Tag**: `v0.9.2` (`71ea783`)
- **Verdict**: verify PASS · archive PASS
- **Commits**: 3 refactor (CP-W1, CP-W2, CP-W3+OE-W1)
- **Tests**: 260 passing (baseline preserved; 0 regressions)
- **Output**: 4 WARN items carried from `refactor-m9-debt-cleanup` debt audit closed:
  - **CP-W1** (`graph::create_db_session` bypassed Filesystem port) — `create_db_session` drops redundant mkdir; signature `&Path` → `path: &Path`. Both callers already do their own mkdir.
  - **CP-W2** (`update_view_member_label` used ambient `chrono::Utc::now()`) — SET clause for `vm.updated_at` removed. Column was set-but-unread (only `m.label` is hashed for `base_revision`).
  - **CP-W3** + **OE-W1** (apply pipeline required full `GraphStore`) — `apply_to_store`, `reexport_view`, and `Command::apply` now take `&mut dyn DiagramOps` (narrowest sub-trait covering all calls). Realises the ISP benefit of the v0.9.1 trait split.
- **Files changed**: 4 (77 ins / 30 del). Net +47 LOC (mostly doc-comment updates).
- **Carried forward**: 0. All 4 carried WARN items closed; the m9-debt-cleanup carryover chain is fully drained.
- **Próximo candidato**: M12 (class-diagram UML, prioridad 2) o M17.0 (archview scaffold, prioridad 1).

> `refactor/store-port-seams`: 4 WARN items from prior cycle's debt audit closed via 3 refactor commits. `MemoryFilesystem` test isolation now works correctly. Apply pipeline narrowed to `DiagramOps` sub-trait. Ambient `chrono::Utc::now()` removed from update path. 260 tests passing without behavioural change. Patch tag `v0.9.2`. Archivado en `sddk/refactor-store-port-seams/archive-report.md`.

## Cycle cerrado — `m20-benchmark-suite` (v0.10.0)

- **Fecha**: 2026-08-01
- **Branch**: `m20-benchmark-suite` (merged to main via --no-ff)
- **Tag**: `v0.10.0` (`e64f7f9`)
- **Verdict**: verify PASS · doctor benchmark+diagram+store 0 findings
- **Commits**: 6 (5 cycle + 1 style)
- **Tests**: 260 passing (baseline preservado)
- **Output**: M20 first slice — archctl-side bench harness + 3 deterministic fixtures + doctor scope gate + docs.
  - **`archctl/benches/`**: 3 cargo bench binaries (export_pipeline, apply_pipeline, query_pipeline) with 7 active + 2 gated bench functions.
  - **`benchmarks/datasets/`**: `small-100.json` (65 KB), `medium-1k.json` (660 KB), `large-10k.json` (6.6 MB) — deterministic generation via Python script (`random.seed(0xC0DE0001)`).
  - **`manifests/benchmark.toml`**: doctor scope gate for the bench harness. Validates public symbols (`seed_small/_medium/_large`) and must_hold invariants (the dataset serde structs + criterion_group/criterion_main macros).
  - **`benchmarks/README.md`**: user-facing documentation (layout, how to run, baseline measurements, ADR-019 budget mapping, follow-ups).
- **Baseline measurements** (--quick on mid-range dev machine):
  - export_query_elements_small:  ~380 ms (100 nodes)
  - export_query_semantic_edges_medium:  ~2.8 s (1k nodes, 2.5k rels)
  - export_base_revision_hash:  ~570 µs (100-node blake3)
  - apply_set_label_small:  ~370 ms (100 nodes)
  - apply_move_member_medium:  ~2.9 s (1k nodes)
  - query_count_elements_small:  ~360 ms (100 nodes)
  - query_semantic_edges_medium:  ~2.8 s (1k nodes, 2.5k rels)
- **New dev-dep**: `criterion = "0.5"` (html_reports). ~20 transitive dev-deps (plotters, walkdir, etc.) — all dev-only, never in the release binary.
- **Follow-ups** (documented in `benchmarks/README.md`):
  - Seed-cost decomposition: split seed from measurement loop (medium benches are seed-cost dominated).
  - Full `run_export` bench: requires ElementVersion + SUPPORTED_BY + Evidence seed.
  - Cold-start bench: `cargo run archctl --version` to first output byte.
  - RSS measurement: peak memory during 10k-node query.
  - CI gate workflow: GH Actions (out of repo scope per AGENTS.md).
- **Próximo candidato**: M17.0 archview scaffold (PRIORIDAD 1) o M12 class-diagram UML (PRIORIDAD 2) o un ciclo de cleanup de los follow-ups (seed-cost decomposition).

> `m20-benchmark-suite`: M20 first slice — archctl-side bench harness + 3 deterministic datasets + doctor scope gate + docs. ADR-019 producer-side budget is now measurable; consumer-side (archview TTFP, pan/zoom) belongs in M17. 260 tests passing without behavioural change. Minor tag `v0.10.0` (new feature surface: bench harness). Archivado en `sddk/m20-benchmark-suite/verify-report.md`.

## Cycle cerrado — `m9-renderers-local` (v0.11.0)

- **Fecha**: 2026-08-01
- **Branch**: `m9-renderers-local` (merged to main via --no-ff)
- **Tag**: `v0.11.0` (merge commit)
- **Verdict**: verify PASS · doctor render 0 findings
- **Commits**: 5 (1 refactor security + 2 feat renderer + 1 manifest + 1 docs)
- **Tests**: 263 passing (260 baseline + 3 new structurizr tests)
- **Output**: closes audit finding F1 from `docs/audits/2026-08-01-archctl-adr-vs-impl.md`.
  - **`reqwest` dependency dropped.** archctl cannot reach the network at runtime. -19 transitive deps.
  - **`--kroki-url` CLI flag removed.** The escape hatch that could silently POST diagrams to a public service is gone.
  - **Custom Structurizr DSL → SVG renderer** (`archctl/src/render/structurizr.rs`). Pure-Rust via `petgraph 0.6 + svg 0.14`. Sugiyama-style layered layout. C4 subset only.
  - **Format detection** now recognises `.mmd` as Mermaid (previously fell into Structurizr branch — wrong).
- **Deferred (documented in render.rs + CHANGELOG)**:
  - **PlantUML rendering**: `plantuml-little 1.2026.2-4` requires `libgraphviz` at build time via the `graphviz-anywhere` transitive dep. Vendor strategy needs a separate cycle.
  - **Mermaid rendering**: `merman 0.8.0-alpha.3` has the same graphviz blocker.
  - Both paths yield a clear "not yet wired" error from `archctl render` today.
- **Files changed**: `archctl/Cargo.toml` (-2, +3), `archctl/Cargo.lock` (-267 lines net), `archctl/src/cli.rs`, `archctl/src/render.rs` (-19 +50 lines), `archctl/src/render/structurizr.rs` (+540 lines new), `manifests/render.toml` (-15, +31), `CHANGELOG.md`.
- **Smoke verified locally**: `archctl render /tmp/diagram.dsl` on a 4-node 3-relation DSL produces a 1.5 KB SVG with correct layered layout (Customer → Web App → API → Database, viewBox `0 0 220 520`).
- **Próximo candidato**: F2 (M9-relations-decision: resolver bypass de reificación en call-graph writer) o F3 (fate de AnalysisRun/Snapshot — eliminar schema no usado o implementar `archctl run resume`).

> `m9-renderers-local`: closes audit F1 (security risk of public-renderer POST). `archctl` no longer reaches the network at runtime; the local Structurizr renderer ships in pure Rust; PlantUML/Mermaid deferred to a follow-up that resolves the `libgraphviz` vendor strategy. 263 tests passing without behavioural regression on existing commands. Minor tag `v0.11.0` (public surface change: new `RenderKind` enum, removed `--kroki-url` flag).

## Decisión sobre ADR-015 / ADR-018 (audit M2)

`docs/audits/2026-08-01-archctl-adr-vs-impl.md` §M2 flaggeó que
`ADR-015` y `ADR-018` están referenciados en `docs/STATE.md` y
`docs/ROADMAP.md` §"Cambios SDD completados" pero nunca se escribieron
como ficheros en `docs/adr/`. Decisión: **no escribir retroactivamente**.

Razones:

- `ADR-015` (puertos faltantes Clock/Environment/Filesystem) se implementó
  implícitamente en los commits `refactor-1b-filesystem-port`,
  `refactor-1c-scope-port`, etc. Los puertos Clock, Environment y
  Filesystem son casos canónicos de "decisión tomada en commit
  individual, no consolidada en un ADR separado". El snapshot
  `docs/STATE.md` (tag `snapshot/pre-activegraph-investigation`) los
  documenta como "✅ ADR-015 parcial" — referencia histórica, no
  contradice el estado actual.
- `ADR-018` (lock path divergence) fue **explícitamente eliminado** en
  el planning de `m9-archctl-export-apply` (ver
  `sddk/m9-archctl-export-apply/coherence-report.md:196`). El
  enfoque de `fs2::try_lock_exclusive` sobre el `.lbdb` mismo es
  directamente la decisión final — no se necesita un ADR separado.
- `docs/STATE.md` es un snapshot histórico congelado
  (commit `aa171cd`, tag `snapshot/pre-activegraph-investigation`).
  Las referencias a `ADR-015`/`ADR-018` ahí son **deliberadamente
  preservadas** como artefacto histórico.


## Cycle cerrado — `m9-relations-decision` (v0.12.0)

- **Fecha**: 2026-08-01
- **Branch**: `m9-relations-decision` (merged to main via --no-ff)
- **Tag**: `v0.12.0` (merge commit)
- **Verdict**: verify PASS · 7 commits · 263 tests preserved
- **Output**: closes audit findings F2–F7 + M1–M3 + M5 from `docs/audits/2026-08-01-archctl-adr-vs-impl.md`.
  - **F2 (stat fix)**: `archctl/src/store.rs:353` and `archctl/src/graph.rs:112` now count `MATCH ()-[r:SEMANTIC_EDGE]->() RETURN count(r)` instead of `MATCH (:SemanticRelation)`. ADR-009 marked as DEFERRED for the reified model.
  - **F3 (ADR-008)**: `Snapshot` + `AnalysisRun` tables + `archctl run resume` deferred to 1.x. ADR-008 revised with rationale.
  - **F4 (profile)**: 18 references to non-existent subcommands across 8 `profile/agents/*.md` + `profile/skills/*/SKILL.md` files annotated with their current status.
  - **F5 (ADR-007)**: `ViewEdge` table + `add-edge`/`edit-edge`/`remove-edge` commands deferred to M17.x archview. ADR-007 revised.
  - **F6 (ADR-005)**: trait naming aligned — `GraphStore` (not `ArchitectureGraph`) + `LbugStore` (not `LadybugArchitectureGraph`).
  - **F7 (ADR-004)**: XDG path aligned — `<portable-project-id>/` UUIDv4 (not `<host>/<owner>/<repo>--<id>/`).
  - **M3 (ROADMAP table)**: added 4 missing rows (v0.9.1, v0.9.2, v0.10.0, v0.11.0).
  - **M1 (ADR-016 path)**: moved from `docs/` (orphaned) to `docs/adr/` (canonical). Cross-references updated.
  - **M2 (ADR-015/018)**: documented in `docs/ROADMAP.md` as historical artifacts.
  - **M5 (bench seed)**: `iter_with_setup` applied to all bench functions (semantically correct; full amortization requires `BatchSize::PerBatch(N)` follow-up).
- **Files changed**: 14 (8 in `profile/`, 5 in `docs/adr/`, 1 CHANGELOG, 1 ROADMAP, 1 `archctl/benches/`).
- **Próximo candidato**: M12 (class-diagram UML, prioridad 2) o M17.0 (archview scaffold, prioridad 1, repo separado) o cleanup del bench seed-decomposition (true amortization via `BatchSize::PerBatch(N)`).

> `m9-relations-decision`: closes 7 of 9 audit findings (F2, F3, F4, F5, F6, F7, M1, M2, M3, M5). F1 (security, kroki POST) was closed in v0.11.0. Combined with v0.11.0, all 9 audit findings + 5 doc drifts from the 2026-08-01 audit are resolved. Patch tag `v0.12.0` (no new feature surface; docs + 1-line stat fix + bench harness).

## Cycle cerrado — `refactor/bench-seed-decomposition` (v0.12.1)

- **Fecha**: 2026-08-02
- **Branch**: `refactor/bench-seed-decomposition` (merged to main via --no-ff)
- **Tag**: `v0.12.1` (merge commit)
- **Verdict**: verify PASS · 3 commits · 263 tests preserved
- **Output**: closes audit finding M5 follow-up (true amortization via `BatchSize::NumIterations(N)`).
  - **T1 (`export_pipeline.rs`)**: `bench_query_elements_small` + `bench_query_semantic_edges_medium` converted from `iter(|| { seed... })` to `iter_batched(seed, routine, NumIterations(10))`. `export_query_semantic_edges_medium` now measures ~16ms (was ~2.8s with seed dominating).
  - **T2 (`query_pipeline.rs`)**: same pattern applied to `query_count_elements_small` + `query_semantic_edges_medium`. `query_semantic_edges_medium` now measures ~18ms (was ~2.8s).
  - **T3 (`apply_pipeline.rs`)**: `bench_apply_chained_commands_large` (dead-code) converted to `iter_batched(NumIterations(5))`. Same-store re-apply is safe because SetLabel is idempotent on `label` and `updated_at` writes were dropped in v0.9.2 (CP-W2).
- **Files changed**: 3 (all in `archctl/benches/`) + CHANGELOG + ROADMAP.
- **Audit status**: 2026-08-01 audit is now **100% closed** (F1 closed in v0.11.0; F2–F7 + M1–M3 + M5 closed in v0.12.0; M5 follow-up closed in v0.12.1).

> `refactor/bench-seed-decomposition`: closes M5 seed-cost decomposition follow-up. Patch tag `v0.12.1` (no behavior change in library; bench harness only). True amortization: seed runs once per batch of N measured iters instead of once per iter.

## Cycle cerrado — `m12-class-diagram` (v0.13.0)

- **Fecha**: 2026-08-02
- **Branch**: `feat/m12-class-diagram` (merged to main via --no-ff)
- **Tag**: `v0.13.0` (`9e665ee`)
- **Verdict**: verify PASS_WITH_WARNINGS · debt-verify PASS_WITH_WARNINGS (C1/C2/C3 all closed)
- **Commits**: 20 (15 original + 3 C1/C2/C3 correction + 2 T7.3 C3)
- **Tests**: 292 passing (vs 263 baseline, +29); 5 ignored (4 pre-existing + 1 M12 composes gap)
- **Output**: `archctl code class-diagram` — tree-sitter CST walk for Rust/TypeScript/Python class extraction. `manifests/code.toml` extended with class_diagram public_symbols + must_hold gates. Schema `schemas/class-diagram-report.schema.json`. Criterion bench harness `benches/class_diagram_pipeline.rs`. 22/24 spec scenarios compliant (2 gaps: same-file composes edges deferred to `feat/code-class-diagram-composes`; `doctor --scopes code` blocked by pre-existing lbug infra gap).
- **Key corrections**:
  - **C1**: `must_hold` literal strings anchored as `//!` doc comments in `class_diagram.rs:6-7`
  - **C2**: 14 new integration tests added (20/20 pass + 1 ignored)
  - **C3**: `project.durationMs` removed from `ClassDiagramReport` — determinism test now 3/3 cold-cache PASS
- **Deuda técnica diferida**: `refactor/extract-code-apply-helpers` (~150 LOC helper duplication); `refactor/clippy-fmt-cleanup` (57 clippy + 137 rustfmt pre-existing); `feat/code-class-diagram-composes` (composes edge emission); `fix/lbug-doctor-infra` (lbug service for doctor).
- **Próximo candidato**: `sddk-release` (MANDATORY per AGENTS.md — no opt-in)



## Cycle cerrado — `refactor/clippy-fmt-cleanup` (v0.13.1)

- **Fecha**: 2026-08-02
- **Branch**: `refactor/clippy-fmt-cleanup` (merged to main via --no-ff)
- **Tag**: `v0.13.1` (`7738b2d`)
- **Commits**: 4 (F1.3 STATE.md + F1.1 clippy + F1.1 rustfmt + F1.2 composes edges)
- **Tests**: 254 passing (baseline preserved); 4 ignored (was 5; 1 composes test now passes)
- **Validation gates (post-merge)**: cargo build exit 0, cargo test exit 0, `cargo clippy -- -D warnings` exit 0, `cargo fmt --check` exit 0
- **Closes M12 W4**: composes edges emitted for same-file typed fields (e.g. `pub config: Config` inside `struct App`)
- **Closes F1 of post-v0.13.0 stabilization plan** (obs-5524)
- **Próximo candidato**: F2.1 (roadmap M13-M15 trim decision) → arrancar M17.0 archview scaffold (separate repo)

## Cycle cerrado — `refactor/extract-code-apply-helpers` (v0.13.2)

- **Fecha**: 2026-08-03
- **Branch**: `refactor/extract-code-apply-helpers` (merged to main via --no-ff)
- **Tag**: `v0.13.2` (`7a1205a`)
- **Verdict**: verify PASS_WITH_WARNINGS · debt-verify PASS_WITH_WARNINGS · archive PASS
- **Commits**: 1 (refactor only — A-min path)
- **Tests**: 293 passing (vs 292 baseline, +1); 4 ignored (pre-existing)
- **Output**: shared `code::apply_common` module extracting `escape_cypher_string`, `open_and_init`, `existing_canonical_keys`, `write_source_artifact`, and a local `Pipe` trait from four caller pipelines (`call_graph`, `c4_discover`, `class_diagram`, `sequence`). `scripts/fmt-staged.sh --edition` now derives Rust edition from `archctl/Cargo.toml` instead of hardcoding 2021. Net: ~103 insertions, ~150 deletions.
- **Deuda técnica diferida**: `SourceArtifact` id formula divergence (pre-existing on main, tracked as `refactor/debt-source-artifact-id-1`); `Pipe` trait single-use (drive-by `refactor/extract-code-apply-helpers-pipe-1`).
- **M16 status**: ENDURECIMIENTO 1.0 — `refactor/extract-code-apply-helpers` cerrado. F3.3 (lbug infra gap), F3.2 (fmt-staged script) y F2.3 (audit code.toml) pendientes.
- **Próximo candidato**: M17.0 archview scaffold (separate repo `archview`) o F3.3 (lbug doctor infra gap restore).

## Cycle cerrado — `ci-main-gates` (v0.13.8)

- **Fecha**: 2026-08-03
- **Branch**: `test/main-gates-05-contract` (merged to main via --no-ff)
- **Tag**: `v0.13.8` (`35bf6a25bf23`)
- **Verdict**: verify PASS_WITH_WARNINGS · debt-verify PASS_WITH_WARNINGS · archive PASS
- **Commits**: Feature Branch Chain (children 1-9), 24 commits total
- **Tests**: 241 passing · 81/81 contract tests
- **Output**: ADR-025 post-merge CI gates, pre-push hook with local verification, bench-compare with github.event.before baseline, MSRV 1.91, 5/5 original HIGH debt items cleared
- **DQS**: improved 0.62 → 0.83
- **CI Run**: https://github.com/Rubentxu/arch-stack/actions/runs/30821895028 (4/4 jobs green)
- **PR Chain**: PRs #1-#4 (nested debt fixes) → PR #9 (final tracker to main)
- **Próximo candidato**: M17.0 archview scaffold (separate repo) o next SDD cycle

## Cycle cerrado — `fix-m17-package-view-onselect` (v0.14.5)

- **Fecha**: 2026-08-03
- **Branch**: `fix/m17-package-view-onselect` (merged to main via PR #21)
- **Tag**: `v0.14.5` (`cd661e6`)
- **Verdict**: verify PASS · debt-verify PASS_WITH_WARNINGS (0 CRIT, 0 HIGH, 12 LOW) · archive PASS
- **Commits**: 1 (atomic: TDD RED→GREEN + regression suite)
- **Tests**: 101/101 passing (+4 new: primary click, triangulation, idempotent re-click, view-switch persistence)
- **Output**: click en package card agora correctly povoa o sidebar usando synthetic `GraphNode` via `buildPackageNode(pkgName, nodes)`. Constructs `{ id, label, kind: "package", meta: { evidence_refs } }` from package name + file list. Reuses existing `packageForFile` derivation. Isolated in `PackageGraph.ts`; forwarded by App's Package `Match` handler.
- **Design decision**: Option D (synthetic node) — correct pattern when a domain entity (package) has no physical node in the bundle but must participate in UI selection.
- **Deuda técnica**: C-001 (6 copy-paste `onSelect` handlers in `App.tsx:223-292`, pre-existing main debt) — follow-up: `refactor/extract-select-handler`.
- **Próximo candidato**: `refactor/extract-select-handler` (C-001) o M17 archview scaffold.
