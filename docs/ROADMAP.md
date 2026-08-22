# Roadmap — OpenCode Architecture Diagrammer

**Estado:** v1.87.0 ALCANZADO (2026-08-22) — TRUST-008 m30 bridge hard fail + Adjudication bounded context shipped (closes REQ-M25-006); T0 Trust cerrado end-to-end (TRUST-001..008). Wave 3 parcial sigue con items 30 (session token, ADR-051 gated) + 34 (P3-05 lens recommendation, ADR-056/062 gated) pendientes. Pendiente menor: report de redacciones en strict bundles + persistir cutoff de staleness por proyecto en XDG.
**Versión:** 2.16
**Fecha:** 22 de agosto de 2026
**Cambios vs 2.15:** 8 cycle log rows (`t0-trust-001-eventlog-reopen` v1.81.0 / `t0-trust-002-event-ids-causation` v1.82.0 / `m25-authority-execution-classes` v1.83.0 / `trust-005-observation-fusion` v1.84.0 / `trust-006-context-bundle` v1.85.0 / `trust-007-feedback-port` v1.86.0 / `trust-008-m30-bridge-promotion` v1.87.0 / `no-stubs-mocks-placeholders-hardcoded` documentation rule) + bump de `docs/STATE.md` (Estado del trunk rows Tip/Versión/Tests/LOC/Tags) y añadidas las 7 filas v1.81.0–v1.87.0 en la tabla "Capacidades shipped" (que terminaba en v1.80.0).

> **Estado vigente del programa**: para Wave 0/1/2 cerrado y Wave 3
> parcial (items 19/22/27/28+29/31–33 cerrados; 30 y 34 pendientes con
> gates), ver [docs/STATE.md](STATE.md) §"Plan vigente" +
> "Anti-roadmap" + "Próxima acción del usuario". Este doc se
> mantiene como anchor histórico (M0–M34+, milestones 73–76 con H4
> cerrado).

---

## Principios

1. OpenCode, agentes y skills son el producto.
2. `archctl` es una CLI sidecar.
3. `archview` (embebido via rust-embed en `archctl view`) es el workbench interactivo que consume bundles de `archctl` — un solo producto (ver ADR-038).
4. LadybugDB entra pronto porque C4 y UML deben compartir identidades.
5. Se entregan verticales completas.
6. Cero escritura dentro del repositorio.
7. Se reutilizan herramientas existentes — preferentemente como librerías Rust, no como CLIs.
8. No se añade un daemon hasta que la concurrencia lo justifique (ADR-010).
9. Cada diagrama tiene propósito, alcance y evidencia.
10. Adoptamos crates de análisis como librerías, no como CLIs (ADR-012).

---

## Plan vivo — Architecture Feedback Workbench (paquete 2026-08-20)

> **Blueprint consolidado** que evoluciona `archctl` de *code knowledge
> graph workbench* a un **entorno de razonamiento visual sobre software**
> donde código, documentación, intención, runtime, tests, agentes y
> feedback humano comparten identidades y evidencia. Las fuentes, los
> ADRs propuestos, las specs, los schemas JSON, los ejemplos y los UAT
> journeys viven en
> [`docs/arch-stack-architecture-feedback-workbench-2026-08-20/`](arch-stack-architecture-feedback-workbench-2026-08-20/README.md)
> (80 ficheros: 68 markdown + 16 ADR-Pxx + 12 specs + 5 JSON Schemas 2020-12
> + 6 examples + UAT). Este ROADMAP queda como **índice** que los
> referencia desde aquí y los cruza con los ADRs ya aceptados del repo.

### North Star del paquete

> Reducir drásticamente el esfuerzo necesario para que un humano forme,
> verifique, mantenga y corrija su modelo mental de un sistema software
> complejo.

### Frontera invariante (P02 + P03) — corazón del diseño

`ExecutionClass` y `AuthorityClass` son ortogonales. **Una afirmación
deliberadamente falsa de un LLM nunca puede promocionarse a `Observed`**.
El escenario [`uat-06-false-agent-claim.yaml`](arch-stack-architecture-feedback-workbench-2026-08-20/examples/uat-06-false-agent-claim.yaml)
es el verification gate (`false_canonical_promotions: 0`).

```text
LLM / modelo     ──►  ModelInference        ──►  Suggested   (no canonical)
Algoritmo deter  ──►  PureDeterministic     ──►  Observed | Derived
Heurística       ──►  DeterministicHeuristic ─►  Suggested
Humano           ──►  HumanDecision         ──►  Normative | Adjudicated
```

Una heurística puede ser determinista y seguir siendo sólo *Suggested*;
una decisión humana puede ser *Normative* sin ser un cálculo
determinista. Esa ortogonalidad es lo que evita:

```text
LLM hallucination
       ↓
canonical architecture fact
```

### Horizontes T0–T11 — siguiente capa tras H0–H3

> **Convención**: T0–T11 **no sustituyen** a H0–H3 (ya shipped o
> parcial, ver §"Horizons H0–H3" abajo). Son la **siguiente capa**
> de evolución hacia *Architecture Feedback Workbench*. Cada T arranca
> sólo cuando su exit gate (UAT journey) se cumple.

| H | Título | Exit gate (UAT) | ADRs blueprint | ADRs repo ya aceptados | Estado real |
|---|---|---|---|---|---|
| **T0** | Epistemic Trust | UAT-06 + reopen safe | P02, P03 | [ADR-021](../adr/ADR-021-cognitive-layer.md) §Reglas, [ADR-022](../adr/ADR-022-agent-catalog.md), [ADR-040](../adr/ADR-040-cognitive-conditional-activation.md), [ADR-063](../adr/ADR-063-trust-determinism-and-authority.md), [ADR-064](../adr/ADR-064-fusion-bounded-context.md) | **TRUST-001..008 ALL SHIPPED** (v1.81.0 → v1.87.0) — T0 Trust cerrado end-to-end: 001 reopen safe ✅ · 002 causation/correlation ✅ · 003 + 004 typology+canonical-write gate (closed in m25 v1.83.0) ✅ · 005 epistemic plumbing closed ✅ · 006 AgentContext.feedback_history ✅ · 007 FeedbackRepository::summaries_for_claims (data-plane ADR-P02) ✅ · 008 m30 bridge hard fail + Adjudication BC ✅ |
| **T1** | Incremental Knowledge Engine | UAT-04 + equality gate | P05, P06 | — | backlog only |
| **T2** | Structured Docs + Tantivy | UAT-08 recall | P04 | — | backlog only |
| **T3** | Live Revision Loop | UAT-04/UAT-10 + budget | P11, P12 | [ADR-010](../adr/ADR-010-concurrencia-ladybugdb.md), [ADR-033](../adr/ADR-033-archctl-view-embedded-workbench.md) | backlog only |
| **T4** | Visual Reasoning Foundation | UAT-01/02/03/10 | P01, P07, P08 | [ADR-013](../adr/ADR-013-viewer-ortogonal.md), [ADR-020](../adr/ADR-020-renderer-stack.md), [ADR-038](../adr/ADR-038-one-product-five-invariants.md), [ADR-039](../adr/ADR-039-renderer-reality-anti-roadmap.md), [ADR-056](../adr/ADR-056-moldable-architecture-workbench.md) (parcial), [ADR-062](../adr/ADR-062-moldable-workbench-partial.md) | **foundation shipped** (M17 workbench + M18 semantic zoom + M19 ELK + M21 G6 culling/LOD + M22 sidebar tabs + M23 ADR-019 perf-ci-gate como verification gate) |
| **T5** | Intent & Reconciliation | UAT-05 | P09 | P2-10 (v1.59.0, intent vs reality MVP — 4-class delta + self-dogfood `archctl-intent.toml`) | **MVP shipped** — falta la matriz y el mapa |
| **T6** | Agent ↔ Visual ↔ Feedback | UAT-06/UAT-14 | P10, P11 | [ADR-021](../adr/ADR-021-cognitive-layer.md), [ADR-022](../adr/ADR-022-agent-catalog.md) (parcial 2/9) | backlog only |
| **T7** | Change Intelligence | UAT-09 | P07, P09 | [ADR-054](../adr/ADR-054-policy-rules.md) (6 closed rules) | backlog only |
| **T8** | Stories & Causality | UAT-07/UAT-13 | P11 | — | backlog only |
| **T9** | Runtime Reality | runtime drift explained | P15 | [ADR-015](../adr/ADR-015-activegraph-packs-investigacion.md) (deferred) | backlog only |
| **T10** | What-if | (post-T7) | P14 | [ADR-039](../adr/ADR-039-renderer-reality-anti-roadmap.md) §anti-roadmap | conditional |
| **T11** | Advanced Intelligence | (gated by T7) | P13 | [ADR-039](../adr/ADR-039-renderer-reality-anti-roadmap.md) §anti-roadmap | deferred |

### Backlog PR-sized (80 items, agrupados por horizonte)

Detalle completo en
[`roadmap/51-IMPLEMENTATION-BACKLOG.md`](arch-stack-architecture-feedback-workbench-2026-08-20/roadmap/51-IMPLEMENTATION-BACKLOG.md).
Tickets:

| Horizonte | Tickets | Resumen |
|---|---|---|
| **T0 Trust** | TRUST-001..006 | EventLog open + reopen regression; event IDs + correlación; AuthorityClass/ExecutionClass mapping; no canonical write from model-backed output; real confidence/status; FreshnessPolicy por fuente |
| **T1 Index** | IDX-001..009 | ArtifactLedger BLAKE3; notify watcher; debounce/coalesce; Rayon extraction; ObservationBatch canonicalization; changed-file apply; removed/renamed invalidation; differential harness; Criterion cold/warm benches |
| **T2 Docs/Search** | DOC-001..003 + SRCH-001..004 | Document/Section extraction; ADR recognizer; deterministic reference linker; Tantivy schema/index adapter; revision-aware commit/rebuild; hybrid seed resolver; ContextBundle `included_because` |
| **T3 Live** | LIVE-001..007 | GraphRevision; GraphDelta; revision/delta HTTP; index worker en `view --watch`; archview polling store; style vs topology update; selection/viewport preservation |
| **T4 Visual** | VIS-001..012 | SelectionBus; adjacency index; InspectorRegistry; Smart System Overview; internal LensDefinition; migrar C4 + Impact consumers; DSM sparse; Canvas2D matrix; Graph↔DSM↔Source linking; System Map d3-hierarchy; metric overlay contract |
| **T5 Intent** | INT-001..006 | IntentCandidate/AcceptedIntent; deterministic Reconciliation; projection/API; Reconciliation Matrix; Intent Map; Intent Coverage |
| **T6 Agent/Feedback** | AGV-001..007 | ProjectionSpec↔VisualRequest compatibility; Visual Compiler; VisualArtifact; selection→AgentContext; Feedback write/retrieval; proposed/ghost visual state |
| **T7 Change** | CHG-001..006 | Expected Change Surface; IntentDiff; SemanticReview; synchronized before/after; test impact; UAT impact |
| **T8+** | STORY-*, CAUSAL-*, OTEL-*, WHATIF-* | Diferidos — sólo cuando los gates previos se cierren |

### ADRs propuestos (P01–P16) — política de promoción

Los `ADR-Pxx` viven en
[`arch-stack-architecture-feedback-workbench-2026-08-20/adr/`](arch-stack-architecture-feedback-workbench-2026-08-20/adr/README.md)
con status **Proposed**. **No son ADRs aceptados del repo.** Antes de
promover cada uno a `ADR-032+`, el ciclo de aceptación debe:

1. Buscar si ya existe decisión equivalente (mapa en esta sección).
2. Si solapa con ADR existente, **amend/supersede** — no duplicar.
3. Asignar número real del repo sólo tras amend/supersede ratificado.
4. Preservar el ADR histórico (no reescribir historia).

| ADR-P | Tema | ADR repo relacionado | Acción recomendada |
|---|---|---|---|
| P01 | Visual workbench primary interface | [ADR-013](../adr/ADR-013-viewer-ortogonal.md), [ADR-038](../adr/ADR-038-one-product-five-invariants.md), [ADR-039](../adr/ADR-039-renderer-reality-anti-roadmap.md), [ADR-062](../adr/ADR-062-moldable-workbench-partial.md) | amend ADR-062 |
| P02 | Deterministic core / probabilistic edge | [ADR-021](../adr/ADR-021-cognitive-layer.md) §Reglas | amend ADR-021 §"Reglas" con ExecutionClass × AuthorityClass |
| P03 | Authority vs execution | [ADR-021](../adr/ADR-021-cognitive-layer.md) §Contrato + [ADR-022](../adr/ADR-022-agent-catalog.md) §Output schema | amend ADR-021/022 |
| P04 | Polyglot local projections | [ADR-007](../adr/ADR-007-modelos-y-renderizadores-de-diagramas.md), [ADR-012](../adr/ADR-012-adopcion-incremental-crates-analisis.md), [ADR-020](../adr/ADR-020-renderer-stack.md), [ADR-039](../adr/ADR-039-renderer-reality-anti-roadmap.md) | amend ADR-007/020 |
| P05 | Incremental index | (nuevo) | proponer ADR-032 |
| P06 | Tiered code intelligence | [ADR-006](../adr/ADR-006-adaptadores-de-herramientas-cli.md) superseded por [ADR-012](../adr/ADR-012-adopcion-incremental-crates-analisis.md) | heredado de ADR-012 |
| P07 | Coordinated task-fit lenses | [ADR-056](../adr/ADR-056-moldable-architecture-workbench.md) (parcial), [ADR-062](../adr/ADR-062-moldable-workbench-partial.md) | amend ADR-062 (P3-05 sigue deferida per ADR-056 entry criteria) |
| P08 | Visual technology partition | [ADR-020](../adr/ADR-020-renderer-stack.md), [ADR-039](../adr/ADR-039-renderer-reality-anti-roadmap.md) | amend ADR-020 |
| P09 | Feedback/Reconciliation graph-native | [ADR-023](../adr/ADR-023-action-proposal-and-policy.md), [ADR-054](../adr/ADR-054-policy-rules.md), P2-10 v1.59.0 | amend ADR-023 + ADR-054 |
| P10 | Agent↔Visual protocol | [ADR-021](../adr/ADR-021-cognitive-layer.md), [ADR-022](../adr/ADR-022-agent-catalog.md), [ADR-023](../adr/ADR-023-action-proposal-and-policy.md) | amend ADR-022 §Protocol |
| P11 | Causal journal (no event sourcing) | [ADR-040](../adr/ADR-040-cognitive-conditional-activation.md) (deferred) | proponer ADR-033 |
| P12 | Live workbench without daemon | [ADR-010](../adr/ADR-010-concurrencia-ladybugdb.md), [ADR-033](../adr/ADR-033-archctl-view-embedded-workbench.md) | heredado |
| P13 | Semantic retrieval deferred | [ADR-039](../adr/ADR-039-renderer-reality-anti-roadmap.md) §anti-roadmap | heredado |
| P14 | Thinking canvas proposal space | [ADR-039](../adr/ADR-039-renderer-reality-anti-roadmap.md) §anti-roadmap | nuevo spike (gated por T7) |
| P15 | Runtime evidence via OTel | [ADR-015](../adr/ADR-015-activegraph-packs-investigacion.md) (deferred) | reopen per ADR-015 §Trigger |
| P16 | Human comprehension release gate | [ADR-019](../adr/ADR-019-performance-budget.md) §enforcement (M23 ya shipped) | amend ADR-019 §enforcement con hierarchy "deterministic > human > LLM advisory" |

### Especificaciones, schemas y ejemplos ejecutables

12 specs, 5 JSON Schemas 2020-12 válidos, 6 examples parsean — viven en
[`arch-stack-architecture-feedback-workbench-2026-08-20/{specs,schemas,examples}/`](arch-stack-architecture-feedback-workbench-2026-08-20/specs/README.md).

Convención: cualquier cycle SDD que materialice un T0–T11 debe
**versionar primero la spec/schema** correspondiente antes de tocar
código. Detalle por spec:

| Spec | Tema | Schema JSON asociado |
|---|---|---|
| 30-GraphRevision-and-Delta | Revision/Delta API | [`graph-delta.schema.json`](arch-stack-architecture-feedback-workbench-2026-08-20/schemas/graph-delta.schema.json) |
| 31-Lens-Definition | Internal LensDefinition (gated por ADR-056/062) | — |
| 32-Selection-Bus | Coordinated selections | — |
| 33-Inspector-Registry | Moldable inspectors | — |
| 34-Visual-Request-and-Artifact | Agent→Visual protocol | [`visual-request.schema.json`](arch-stack-architecture-feedback-workbench-2026-08-20/schemas/visual-request.schema.json) |
| 35-Feedback-and-Reconciliation | Feedback/reconciliation graph-native | [`feedback.schema.json`](arch-stack-architecture-feedback-workbench-2026-08-20/schemas/feedback.schema.json) |
| 36-Architecture-Story | Causality + stories | — |
| 37-Semantic-Review | Change intelligence | — |
| 38-Incremental-Index | Notify + BLAKE3 + Rayon | — |
| 39-Search-Context-Bundle | Tantivy + ContextBundle | [`context-bundle.schema.json`](arch-stack-architecture-feedback-workbench-2026-08-20/schemas/context-bundle.schema.json) |
| 40-Agent-Event-Journal | Causal journal | [`event-envelope.schema.json`](arch-stack-architecture-feedback-workbench-2026-08-20/schemas/event-envelope.schema.json) |
| 41-UAT-Graph | UAT como subgrafo | — |

### UAT — 14 journeys, no "¿se renderizó?"

Detalle en
[`arch-stack-architecture-feedback-workbench-2026-08-20/uat/`](arch-stack-architecture-feedback-workbench-2026-08-20/uat/60-UAT-STRATEGY.md).
Authority order explícito:

```text
deterministic data/DOM assertions
        >
human task correctness
        >
human subjective measures
        >
multimodal LLM advisory
```

El **más crítico**: **UAT-06 (false-agent-claim)** — gate
`false_canonical_promotions: 0`. Es el test que demuestra que P02 está
cableado, no sólo documentado: una afirmación falsa pero plausible del
agente no puede promocionarse a hecho canónico, el humano puede
rechazarla, el rechazo persiste tras reinicio, futuras invocaciones
reciben la corrección. El escenario YAML ejecutable vive en
[`examples/uat-06-false-agent-claim.yaml`](arch-stack-architecture-feedback-workbench-2026-08-20/examples/uat-06-false-agent-claim.yaml).

Otros journeys notables: UAT-01 (first insight), UAT-02 (why),
UAT-03 (graph vs DSM dense coupling), UAT-04 (incremental edit),
UAT-05 (intent vs reality), UAT-07 (agent causality), UAT-08
(knowledge retrieval), UAT-09 (semantic review), UAT-10 (context
preservation), UAT-11 (scale), UAT-12 (accessibility), UAT-13
(architecture story), UAT-14 (feedback reuse).

### Riesgos vivos (resumen)

Detalle completo en
[`roadmap/53-RISKS-AND-OPEN-QUESTIONS.md`](arch-stack-architecture-feedback-workbench-2026-08-20/roadmap/53-RISKS-AND-OPEN-QUESTIONS.md).
Los tres con mayor leverage:

1. **LLM contamina truth** → authority gate (T0).
2. **layout instability** → stable positions + topology-aware update (T3).
3. **docs vuelven a quedar stale** → capability/traceability dogfooding per-cycle.

### Próxima acción derivada del paquete

El paquete es **propuesta, no work-in-progress**. Antes de iniciar el
primer cycle que ataque T0 (TRUST-001..006):

1. Abrir un ADR de amend que **endurezca [ADR-021](../adr/ADR-021-cognitive-layer.md)
   §Reglas** con la separación `ExecutionClass × AuthorityClass` y el
   principio "false canonical promotion impossible".
2. Versionar la spec [`30-GRAPH-REVISION-AND-DELTA`](arch-stack-architecture-feedback-workbench-2026-08-20/specs/30-GRAPH-REVISION-AND-DELTA.md)
   con semantic version propio antes de tocar código.
3. Cerrar el amend con **UAT-06** como verification gate.

- m25-authority-execution-classes (2026-08-20, v1.83.0): shipped via 3 chained PRs (#287 +159 docs, #288 +738 code, #289 +407 verify). Cycle satisfies the "amending ADR + spec-30 versioning" pre-condition via ADR-063 and spec-30 v1.1. Closes the first live breach of ADR-P02. Diff: `cbce2d3..d8c4a6a` (10 files, +1172/-27).

Cualquier cycle posterior sigue el mismo patrón: spec → schema → ADR
amend/propose → código → UAT.

---

## Horizons H0–H3 (outcome-driven)

> Los milestones M0–M32 son anclas históricas. El roadmap futuro se expresa en horizontes de resultado.

### H0 — Ejecutable / verdad verificable

Entregable: bundle ejecutable que cumple el contrato de schema.

- **Contrato ejecutable**: `schemas/diagram-projection.schema.json` es la única fuente de verdad para el `viewer-bundle`. Rust DTOs (`archctl/src/diagram/export_types.rs`) y TypeScript types (`archview/src/loader/types.ts`) alineados campo a campo.
- **Selector configurable**: `archctl view` acepta `GET /api/export?selector=c4-context:<id>` (no hardcoded `container:*`).
- **Verificación**: `archctl diagram validate` contra schema + bundle válido en CI.

### H1 — Utilidad humana

Entregable: estado de workspace durable que sobrevive a los reinicios del servidor efímero.

- **XDG persistence**: workspace state (camera, zoom, filters, selection) persiste en `~/.local/share/archctl/projects/<hash>/workspace.json`, no en `localStorage`.
- **Drawer de solo lectura**: source drawer muestra `file:line` de la evidencia como texto read-only; path traversal rechazado; handoff al IDE via `$EDITOR`.
- **Verificación**: workspace restaura correctamente tras `archctl view` en puerto diferente.

### H2 — Editor visual

Entregable: ChangeSet cosmético con integridad de `baseRevision`.

- **Round-trip**: cosmetic ChangeSet (move-member / collapse-group / set-label) aplica via `archctl diagram apply --changes` contra `baseRevision` stored.
- **Integridad**: apply rechaza stale revisions con mensaje claro.
- **Undo/redo**: inverse ChangeSets en secuencia.
- **`.arrows` adapter**: import/export Arrows.app sin mutar el grafo canónico.
- **Verificación**: apply succeeds on matching baseRevision; rejected on stale; undo restores position.

### H3 — Moldabilidad demostrada

Entregable: LensSpec introducido solo cuando hay evidencia real de necesidad.

- **Entry criteria**: LensSpec NO se añade a menos que (a) 2+ consumidores repitan la misma lógica de traducción, o (b) una necesidad medida (UAT evidence, perf budget breach) demande abstracción.
- **Reversibilidad**: cláusula de rollback documentada en el ADR si consumers < 2.
- **Verificación**: ADR gate bloquea PRs que añaden LensSpec sin evidencia.
- **Canonical anchor**: [ADR-056 — Moldable Architecture Workbench y navegación semántica](adr/ADR-056-moldable-architecture-workbench.md) (Deferido — 2026-08-18 con los entry criteria de §H3 documentados como Reopen trigger del ADR).

### H4 — Distribución & ciclo de vida del CLI (asdf-inspired)

**Estado: CERRADO en v1.36.0 (M73+M75+M76) — 2026-08-11**

Entregable: `archctl` se distribuye, actualiza, y desinstala como un CLI versionado moderno, con abstracción para múltiples IDEs agenticos.

Inspirado en asdf-vm (multi-version, tap model, install/update/uninstall,
per-project pin via `.tool-versions`). Adaptado a un binario pre-compilado
Rust (no desde source) + assets embebidos (skills/agents/plugin).

- **Multi-version**: N versiones de `archctl` instaladas simultáneamente en `~/.local/share/archctl/installs/v<version>/`. Symlink `current` cambia la versión activa. Shim binario en `/usr/local/bin/archctl` (8 líneas) delega al binario activo.
- **Self-update**: `archctl self update [--channel=stable|rc|nightly]` descarga desde GitHub Releases con SHA256SUMS verify + migration scripts (rollback automático si falla). Sin firma GPG en v1 (M76).
- **Per-project pin**: `.arch-version` (formato idéntico a `.tool-versions` de asdf) walking hasta `$HOME`. Override con `$ARCHCTL_VERSION` o `--archctl-version X.Y.Z`.
- **Uninstall**: `archctl self uninstall [--purge]` elimina el binario activo + opcionalmente `~/.local/share/archctl/` completo.
- **IDE adapter abstraction** (`archctl ide <subcommand>`): trait `IdeAdapter` con adapters built-in para OpenCode, ZCode, Claude Code, Codex. Cada adapter implementa `install_stack/remove_stack/diff_stack` para su discovery path nativo. Plugin tap para adapters externos en M76.
- **Plugin tap model** (M76): `archctl plugin install <author>/<plugin>@<version>` desde un tap JSON. Skills/agents de terceros sin recompilar `archctl`.
- **Backward compatibility**: `archctl stack install/update/status` queda como alias deprecated de `archctl ide install opencode` durante un ciclo. Removal en M77.
- **Verificación**: `e2e/install_e2e.sh` extendido con multi-version, self-update (mocked), uninstall, pin per-project.

Ver [ADR-057](../adr/ADR-057-archctl-versioned-distribution.md),
[ADR-058](../adr/ADR-058-self-update-github-releases.md),
[ADR-042](../adr/ADR-042-ide-adapter-abstraction.md),
[specs/stack-distribution.md](../specs/stack-distribution.md),
[specs/ide-adapters.md](../specs/ide-adapters.md).

Milestones roadmap H4: **M73** (multi-version + self-update + uninstall), **M75**
(IDE adapters OpenCode/ZCode/Claude Code/Codex), **M76** (plugin tap + firma GPG).
**H4 cerrado en v1.36.0** — M73+M75+M76 completados.

---

# `archctl` — milestones del sidecar

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

## M8 — `tree-sitter-graph` para extractores declarativos — **CLOSED (REMOVED)**

## M9 — Renderers como librerías (PlantUML, Mermaid, Structurizr propio)

## M9-archctl-export — `archctl diagram export` + `archctl diagram apply`

Antes de cerrar M9, `archctl` necesita emitir bundles que `archview`
pueda consumir.

**Pivot v2.4 (2026-07-31) + M69 convergence (2026-08-09):** M9 ya no es "renderers como librerías (PlantUML, Mermaid, Structurizr propio)". Es **Code Knowledge Graph Workbench** — un workbench con 5 vistas coordinadas (C4 contextual, call graph, sequence, class, package) renderizado con el stack shipped (ver [ADR-019](adr/ADR-019-performance-budget.md) para el budget contractual y [ADR-039](adr/ADR-039-renderer-reality-anti-roadmap.md) §Renderer realidad para el stack realmente implementado: **G6 5.x canvas** — sin WebGPU, sin WASM, sin cosmos.gl; ADR-020 superseded). El target es developers/arquitectos, no BI. M9 incluye también el setup inicial del workbench (M17.0–M17.1) y la primera validación con `archctl code c4 discover` + `archctl code call-graph`. El workbench se distribuye como **un solo producto** ([ADR-038](adr/ADR-038-one-product-five-invariants.md), [ADR-033](adr/ADR-033-archctl-view-embedded-workbench.md)) vía `archctl view` — `archview` embebido en el binario `archctl` vía `rust-embed`.

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

## M17 — Workbench entregado vía `archctl view` (sustituye a Av0–Av6) — **→ superseded by H0–H3**

> La sustancia de M17 (bundle contract, 5 vistas, G6 canvas) se reenmarca en H0–H3. El milestone anchor se preserva como ancla histórica.
> **Criterio rector:** [ADR-038](adr/ADR-038-one-product-five-invariants.md) (arch-stack = un producto, cinco invariantes) + [ADR-039](adr/ADR-039-renderer-reality-anti-roadmap.md) (renderer realidad + anti-roadmap con reopen triggers medibles). ADR-013 sección "repositorio separado" queda **superseded** por ADR-038.

**Pivot v2.4 + M69 convergence (2026-08-09):** arch-stack es **un producto** ([ADR-038](adr/ADR-038-one-product-five-invariants.md)) — `archctl` (CLI sidecar, Rust) + `archview` (workbench SolidJS) **embebido vía `rust-embed`** en el binario `archctl`. El comando de entrada es `archctl view`; **no hay repositorio separado**. El renderer shipped es **G6 5.x canvas** ([ADR-039](adr/ADR-039-renderer-reality-anti-roadmap.md) §Renderer realidad) — sin WebGPU, sin WASM, sin cosmos.gl. Las decisiones aspiracionales (WASM compute, Arrow, cosmos.gl, SceneGraph, WIT) viven en el **anti-roadmap** de ADR-039 con reopen triggers medibles.

> **Avance 2026-08-03 (explore + m17-contract-alignment, v0.14.3):** el explore `m17-workbench-state` reveló que M17.0 está hecho y M17.1–M17.7 tienen MVPs de lista-texto (7 vistas en `archview/src/views/`), pero el loader consumía un formato C4 custom incompatible con el `viewer-bundle` real de `archctl diagram export`. `m17-contract-alignment` (v0.14.3) alineó el loader con el schema canónico (`manifest`/`projection`/`evidence`/`styles`), cerró 2 deudas HIGH (time-mutation, boundary g6→types) y añadió el contrato compartido `types.ts` + tests E2E con fixture validado por `archctl diagram validate`. **`m17-routing-fix` cerrado ✅ (v0.14.4)** — CallGraphView/PackageView ahora alcanzables via `routing.ts` resolveView total discriminant. **`fix-m17-package-view-onselect` cerrado ✅ (v0.14.5)** — PackageView onSelect `pkg.name`→node agora povoa o sidebar via synthetic `GraphNode` (Option D, `buildPackageNode`). **`m26-c4-contract-integrity` cerrado ✅ (v0.14.9)** — fixture exporter-derived ARREGLADO: `export.rs` ahora usa `category='c4'` y `kind_id CONTAINS` para matchear `c4_discover` que escribe `category='c4', kind_id='mt.container'`. ADR-024 formaliza la semántica. **`m26-c4-vertical-validation` cerrado ✅ (v0.14.10)** — 6 bugs adicionales descubiertos al ejecutar la pipeline contra `tokio-rs/axum` (workspace real): (B1) `apply()` usaba `cwd` directo en lugar de `info.project_dir`; (B2) Cypher inválido por IDs sin comillas en `IN [...]`; (B3) `write_evidence` silenciaba errores con `.ok()`; (B4) `version_id` colisionaba porque el hash no incluía el `element_id`; (B5) inconsistencia `"Drafted"` vs `"drafted"` rompía `evidence accept`; (B6) bundle schema mismatch (`type="c4"`, `status="active"`). ADR-031 documenta cada bug + fix. Vertical C4 ahora produce bundles válidos contra `tokio-rs/axum` (4 containers detectados, 4 evidences aceptadas, `diagram validate` OK). **Pendiente (per ADR-039 anti-roadmap, no bloqueante)**: WGPU renderer — reopen trigger = benchmark p99 render >16ms AND JS Worker profiling muestra hot path no cabe en budget. Benchmarks M27 sobre 10+ proyectos reales multi-lenguaje antes de v1.0.

- **M17.0** ✅ **CERRADO v0.14.0**: SolidJS + G6 5.x **canvas** ([ADR-039](adr/ADR-039-renderer-reality-anti-roadmap.md) §Renderer realidad; ADR-020 superseded). Setup inicial del workbench, scaffold, build pipeline. Scope MVP: bundle loader + pan/zoom + sidebar de evidencias. **Embebido en el binario `archctl` vía `rust-embed`** ([ADR-038](adr/ADR-038-one-product-five-invariants.md), [ADR-033](adr/ADR-033-archctl-view-embedded-workbench.md)) — un solo producto, sin repo separado.
- **M17.1**: Semantic zoom para C4 (Context → Container → Component → Code). — **MVP lista-texto shipped** en `archview/src/views/`. El semantic zoom interactivo continuo **difiere per ADR-039 anti-roadmap**: reopen trigger = G6 canvas FPS <30 en drill-down de >500 nodos, medido en `bench/`.
- **M17.2**: Call graph view (1-N niveles, blast radius, async flow). — **MVP lista-texto shipped**. Generación de call graph via `archctl code call-graph` (v0.8.0). El blast radius computacional (subgrafo N-hop) **difiere per ADR-039**: reopen cuando ≥1 usuario real solicita la métrica con un dataset concreto.
- **M17.3**: Sequence diagram view (call chains, async flows). — **MVP lista-texto shipped**; generación via `archctl code sequence` (v0.9.0).
- **M17.4**: Class diagram view (UML). — **MVP lista-texto shipped**; extracción via `archctl code class-diagram` (v0.13.0).
- **M17.5**: Package diagram view (dependencias, ciclos, cohesión). — **MVP lista-texto shipped**.
- **M17.6**: Drift detection (C4 declarado vs actual; cross-validation). — **Won't Do v1.x** ([M13](#m13--workbench-actions--wont-do-en-v1x-decisión-2026-08-02)). [ADR-038](adr/ADR-038-one-product-five-invariants.md) Invariante 4 (apply cosmético) limita el scope de "drift" sin reintroducir el grafo canónico en el write path.
- **M17.7**: Impact analysis (blast radius de un cambio propuesto). — **Won't Do v1.x** ([M14](#m14--versionado-recuperación-y-rollback--wont-do-en-v1x)). Reactivación solo con un workflow HITL real ([ADR-040](adr/ADR-040-cognitive-conditional-activation.md)).

Performance budget ([ADR-019](adr/ADR-019-performance-budget.md)):
- **Productor** (`archctl diagram export` + `apply`): bench medible en `archctl/benches/` (M20, v0.10.0). Producer side budget cumplido para el rango objetivo.
- **Consumidor** (`archctl view` workbench): canvas cubre **5k–50k nodos** sin fricción ([ADR-039](adr/ADR-039-renderer-reality-anti-roadmap.md) §Renderer realidad). Los números nominales del budget original (TTFP <1s, pan/zoom 60 FPS, filter <50ms, memory <500MB) **siguen siendo el techo contractual**; el camino para alcanzarlos a 100k nodos es canvas + optimizaciones JS hasta que el benchmark (per ADR-039 trigger: node count >100k AND FPS <30 durante >500ms) demuestre insuficiencia y abra el reopen de cosmos.gl.

**Distribución**: `archctl view` (un solo comando, un solo binario, [ADR-038](adr/ADR-038-one-product-five-invariants.md)). El workbench se distribuye con el binario de `archctl` ([ADR-033](adr/ADR-033-archctl-view-embedded-workbench.md)) bajo `archctl v<semver>`. **No hay repositorio separado.**

## M18 — Reactive runtime (event log + behaviors + planners) — **→ superseded by anti-roadmap (ADR-039)**

> M18 reactive runtime deferred indefinitely. Ver [ADR-039](adr/ADR-039-renderer-reality-anti-roadmap.md) §anti-roadmap para reopen triggers (≥2 third-party consumers needing shared compute).

**Pivot v2.4 + v2.5:** Reactive runtime inspirado en ActiveGraph pero implementado en Rust puro. Defer a 1.x (después del workbench estable). Features: event log, subscriptions, behaviors como WASM plugins, planners, capabilities. Ver sección del doc sobre Reactive Runtime.

> **Pivot v2.5 (2026-07-31, post-capa-cognitiva) + M69 (2026-08-09):** M18 se reposiciona como el substrate sobre el cual corre la Cognitive Layer (ver M21-M23). El reactive runtime añade la capacidad de que comportamientos (algoritmos deterministas) Y agentes (LLM) reaccionen al estado del grafo. Ver [ADR-021](adr/ADR-021-cognitive-layer.md) — **estado actual per [ADR-040](adr/ADR-040-cognitive-conditional-activation.md): Aceptado (conditional)**, reactivación solo con workflow HITL real.

## M19 — Custom wgpu renderer (solo si cosmos.gl no alcanza) — **→ superseded by anti-roadmap (ADR-039)**

> M19 WGPU renderer deferred indefinitely. Ver [ADR-039](adr/ADR-039-renderer-reality-anti-roadmap.md) §anti-roadmap para reopen triggers (benchmark p99 fails ADR-019 budget AND JS/Worker insufficient).

**Pivot v2.4 + M69 convergence (2026-08-09):** Si el canvas shipped ([ADR-039](adr/ADR-039-renderer-reality-anti-roadmap.md) §Renderer realidad — G6 5.x canvas cubre 5k–50k nodos sin fricción) demuestra insuficiencia en el rango objetivo (per ADR-039 trigger: node count >100k AND FPS <30 durante >500ms consecutivos, con el bottleneck en el render y no en el layout), construir un renderer custom en Rust + wgpu + WGSL. 2.0. **Deferred per ADR-039 anti-roadmap** — el reopen trigger debe ser medido en `bench/` antes de iniciar M19.

## M20 — Performance validation cycle — **→ H0 (ejecutable)**

> M20 benchmark validation cycle shipped. El performance budget es parte de H0.

**Pivot v2.4:** Cycle dedicado a implementar el benchmark suite de ADR-019. Datasets canónicos (`benchmarks/datasets/{small,medium,large}.json`), CI gate, profiling setup. Sin esto, el performance budget es teoría.

**Hecho (v0.10.0 + v0.13.6 + v0.13.7):** harness criterion (export/apply/query/class-diagram pipelines), 3 datasets canónicos, doctor scope gate, **CI gate GitHub Actions** (build/test/clippy/fmt/doctor + bench smoke + bundle cap ≤2MB), **regresión >10% vs main** (`scripts/bench-compare.sh` + job CI en PRs, ADR-019 §1).

**Pendiente opcional (no bloqueante):** profiling-on-regression flamegraph, PR-comment bot.

## M21 — Cognitive Layer foundation — **→ superseded by H1 (conditional)**

> M21 shipped but marked conditional. Ver [ADR-040](adr/ADR-040-cognitive-conditional-activation.md) para reactivation trigger.

**Estado:** Implementado en v0.15.0 (PR #27 mergeado). Foundation sienta las bases para M22.

**Pivot v2.5 (2026-07-31, post-capa-cognitiva):** Substrate sobre el cual corren los agentes especializados. Outputs:
- Contrato `ReactiveObserver` + `AgentContext` + `AgentOutput` (ver [ADR-021](adr/ADR-021-cognitive-layer.md))
- ModelPolicy + AgentBudget + escalation ladder (heurística → local → potente → humana)
- MCP gateway mínimo (3 tools read-only: `graph_query`, `schema_validate`, `run_tests_local`)
- CLI: `agent list/dispatch` y `mcp list-tools/invoke` subcommands
- 9 E2E tests para agent/mcp commands

Output verificable: queries del workbench responden con output estructurado (no solo texto). Foundation sienta las bases para M22.

## M22 — Agent catalog v1 — **→ superseded by H1 (conditional)**

> M22 shipped (2/9 agents) but marked partial. Ver [ADR-040](adr/ADR-040-cognitive-conditional-activation.md).

**Estado:** Implementado en v0.15.0 (PR #30 mergeado). ArchitectureAgent + ProjectionAgent como ReactiveObserver heurísticos y deterministas.

**Pivot v2.5:** Catálogo inicial de los 9 agentes especializados (ver [ADR-022](adr/ADR-022-agent-catalog.md)):
- Semantic Curator · Architecture · Projection · Investigation · Impact · Planning · Documentation · Presenter · Review/Critic

Para v1.0 (M16) solo Architecture + Projection (heurística pura). Para 1.x, los otros 7 agentes con LLM local (Phi-3 / Llama-3-8B) + LLM potente (Claude/GPT) para los más sensibles (Investigation, Planning, Review).

## M23 — Action Proposal & Policy Engine — **→ superseded by H1 (conditional)**

> M23 deferred (phase 1 PR #32 closed stale). Ver [ADR-040](adr/ADR-040-cognitive-conditional-activation.md) para reactivation trigger.

**Pivot v2.5:** Implementación completa del ActionProposal + Policy Engine + MCP gateway (ver [ADR-023](adr/ADR-023-action-proposal-and-policy.md)):
- ActionProposal estructurado (goal + command + capabilities + approval + evidence esperada + rollback)
- Policy Engine con reglas declarativas (TOML) editables sin recompilar
- MCP gateway como frontera de capabilities (resources = read-only, tools = con efectos, prompts = procedimientos)
- Audit log append-only en el grafo (inmutable)
- HITL UI en `archview` (mostrar proposals pendientes al usuario)

Output: el sistema puede ejecutar acciones gobernadas (no solo leer). Por ejemplo: `archctl code c4 discover --auto-apply` (corre agentes, valida confidence > 0.9, ejecuta propuesta vía MCP).

> **Pipeline de v1.x (gated per [ADR-040](adr/ADR-040-cognitive-conditional-activation.md))**: M18 (reactive runtime) → M20 (benchmark) → M21 (cognitive foundation) → M22 (agent catalog) → M23 (action proposal + policy). Cada cycle valida el anterior. **Toda esta cadena está condicionada a un workflow HITL real** — la reactivación no es por fecha ni por versión, sino por la existencia de un usuario real que necesite agent-driven actions más allá de heurísticas ([ADR-040](adr/ADR-040-cognitive-conditional-activation.md) §Trigger de reactivación). Sin ese workflow, la cadena permanece en estado conditional/parcial/diferido (ADR-021/022/023 headers actualizados por ADR-040).

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

## M26.5 — C4 vertical end-to-end validation — **COMPLETO** ✅

**Estado:** Implementado en v0.14.10. ADR-031 documenta los 6 bugs.

**Objetivo:** Validar que el vertical C4 (`discover --apply → evidence accept → export → validate`) funciona con proyectos reales de GitHub, no solo con `TempDir` + `MockGraphStore`. El smoke test inicial contra `tokio-rs/axum` descubrió **6 bugs** que no se habían detectado en la suite existente.

**Bugs encontrados (todos arreglados):**

| # | Bug | Severidad | Fix |
|---|-----|-----------|-----|
| B1 | `apply()` usaba `cwd` directo, `graph_query` usaba `info.project_dir` (XDG) — DBs distintas | CRÍTICO | Pasar `info.project_dir` a todos los `apply()` |
| B2 | `query_evidence_for_versions` y `query_version_props`: `IN [id1, id2]` sin comillas → Cypher inválido | CRÍTICO | Envolver IDs en comillas simples |
| B3 | `write_evidence` silenciaba errores con `.ok()` → 0 evidences persistidas | CRÍTICO | Quitar columnas inválidas del SET, propagar errores |
| B4 | `version_id = blake3(version_props)` colisionaba — todos los containers al mismo ElementVersion | ALTO | Incluir `element_id` en el hash |
| B5 | `"status": "Drafted"` (mayúscula) vs `parse_label` solo acepta lowercase → `accept_evidence` no-op | ALTO | Cambiar a `"drafted"` (minúscula) |
| B6 | Bundle schema mismatch: `type="c4"`, `status="active"` vs schema `enum:["context",…]`, `enum:["accepted",…]` | ALTO | Funciones `kind_id_to_type()` y `schema_valid_status()` |

**Resultado:** Vertical C4 validado contra `tokio-rs/axum`:
```
discover --apply    → 4 elements + 4 evidences
evidence accept     → 4 evidences aceptadas
diagram export      → bundle con 4 elements + 4 evidence
diagram validate    → ✅ Bundle is valid
```

402 tests siguen pasando (no se rompió nada).

**Lecciones:** Los tests `TempDir` ocultan bugs de path. El patrón `.ok()` en queries Cypher primarias es peligroso. El casing de strings es un contrato frágil. **No se puede shippear v1.0 sin smoke tests con proyectos reales.**

**Referencias:** [ADR-031](adr/ADR-031-c4-vertical-validation.md), PR #46 (pendiente)

## M27 — Sandbox + Benchmarks (pre-v1.0) — **COMPLETO** ✅ (v0.22.0)

**Estado:** Implementado en v0.22.0 (PR #47, #48, #49, #50 mergeados a main). Desbloquea v1.0.

**FP/FN manual review (2026-08-06):** completada en
`bench/reports/fpfn-rubric-2026-08-06.md`. **7/7 datasets pasan** ambos
thresholds tras [M28](#m28--single-package-jsts--benchmark-gate-fixes--completo-2026-08-06)
(strategy npm-single + conteo corregido). Bug real fixed en el ciclo:
strategy dockerfile detectaba su propio source (`dockerfile.rs`) → FP
"docker" en dogfood. **Gate manual 100% verde → v1.0 desbloqueado.**

**Objetivo:** Demostrar empíricamente que `archctl` funciona en proyectos reales multi-lenguaje antes de declarar v1.0. Hoy, el vertical C4 está validado contra **un solo proyecto** (axum). Necesitamos datos sistemáticos.

**Scope:**

**Sandbox (Quadlet):**
- Imagen base: `ubuntu:24.04` (first-party LTS) + `rustup default 1.97.1` dentro del Containerfile (matches `rust-toolchain.toml`). **NOT** `catthehacker/ubuntu:rust-latest` (floating community tag, supply-chain risk; rejected per ADR-032 Q2).
- Quadlet `archctl-bench.container` con:
  - `--uidmap=1000:0:1` + `RemapUsers=true` (rootless sin daemon; uid mapping for XDG path translation)
  - Volúmenes: `/datasets` (proyectos GitHub), `/reports` (output), `~/.cargo` (cache deps)
  - `Type=oneshot` (no persistente)
  - Sin `--bind`, sin `--reuse`, sin `--container-daemon-socket`
- `bench/run-bench.sh` orquestador que:
  1. Verifica prerequisites (podman, ubuntu:24.04 image)
  2. Para cada candidato:
     - `git clone --depth 1` en `~/.cache/archctl-smoke/`
     - Ejecuta el vertical C4 completo con timeout
     - Captura métricas (exit code, wall time, RSS, JSON validity)
     - Compara con baseline (regression >10% bloquea)
  3. Genera `bench/reports/<date>.md` con tabla resumen

**Datasets candidatos (10+ multi-lenguaje):**

| Lenguaje | Repo | Tamaño | Estrategia |
|---|---|---|---|
| Rust | `tokio-rs/axum` | 6MB | cargo workspace |
| Rust | `BurntSushi/ripgrep` | 5.7MB | single crate |
| Rust | `clap-rs/clap` | 21MB | cargo workspace |
| TypeScript | `pmndrs/zustand` | 8MB | npm workspace |
| TypeScript | `vueuse/vueuse` | 17MB | npm workspace |
| JavaScript | `expressjs/express` | 10MB | npm workspace |
| Go | `labstack/echo` | 7MB | call-graph + state-machine |
| Python | `psf/requests` | 14MB | call-graph + class-diagram |
| Java | `square/javapoet` | 2MB | call-graph + class-diagram |
| Kotlin | `mockk/mockk` | 16MB | call-graph + state-machine |
| **Dogfood** | `archctl` mismo | varies | TODAS las estrategias |

**Métricas automáticas:**

1. **Exit code** — 0 = OK, non-zero = error (con timeout 60s por extractor)
2. **Wall time** — `time` en ms (3 corridas, mediana)
3. **Peak RSS** — `/usr/bin/time -v` o `ps -o rss` durante el run
4. **Output validity**:
   - `manifest.json` con `schemaVersion: "1.0.0"` y `format: "viewer-bundle"`
   - `projection.json` parseable, `nodes[]` no vacío si hay elements
   - `evidence.json` parseable, `evidence[]` no vacío si hay evidences aceptadas
   - `diagram validate <bundle>` exit 0
5. **Determinism** — ejecutar 2 veces, comparar `baseRevision` (debe ser idéntico)
6. **FP/FN ratio** (manual, no automático):
   - Comparar `nodes[]` con la realidad del repo (leer README/structure)
   - True positives = containers reales correctamente detectados
   - False positives = phantom containers reportados
   - False negatives = containers reales no detectados

**Criterios de éxito para v1.0:**

| Criterio | Threshold |
|---|---|
| Exit code 0 en todos los candidatos para al menos 1 extractor | ≥ 90% |
| Wall time mediano `c4-discover --apply` | < 30s para proyectos < 30MB |
| Wall time mediano `diagram export container:*` | < 5s para < 100 nodes |
| Peak RSS | < 500MB |
| Bundle validity (`diagram validate`) | 100% (prohibido que un bundle generado sea inválido) |
| Determinism (baseRevision) | 100% |
| FP ratio (containers) | < 20% |
| FN ratio (containers) | < 30% |

**Si algún threshold no se cumple:** NO se shippea v1.0. Se abre un M28 (hotfix específico) o se retrasa v1.0.

**Out of scope:**
- Benchmarks de stress (10k+ nodes) → M20 ya cubierto
- Performance budget ADR-019 (TTFP <1s, 60 FPS) → workbench consumer-side ([ADR-038](adr/ADR-038-one-product-five-invariants.md), [ADR-039](adr/ADR-039-renderer-reality-anti-roadmap.md) §Renderer realidad)
- Tests de renderer → workbench consumer-side; canvas cubre 5k–50k; reopen de WGPU/cosmos.gl gated por ADR-039 anti-roadmap triggers medibles
- CI integration → cuadrar con `bench-compare.sh` existente

**Entregables:**
1. `bench/run-bench.sh` (~200 LOC bash)
2. `bench/datasets.toml` (~20 líneas TOML)
3. `bench/quadlets/archctl-bench.container` (~30 líneas)
4. `bench/Containerfile` (FROM ubuntu:24.04 + rustup default 1.97.1 + archctl mount from host)
5. `bench/reports/<date>.md` (output del run)
6. `docs/adr/ADR-032-bench-methodology.md` (si surge un patrón nuevo)

**Referencias:** [ADR-031](adr/ADR-031-c4-vertical-validation.md) (predecesor)

## M28 — Single-package JS/TS + benchmark gate fixes — **COMPLETO** ✅ (2026-08-06)

**Estado:** Implementado. Cierra el gate manual FP/FN de M27 — **7/7
datasets pasan** (FP <20%, FN <30%). v1.0 desbloqueado.

**Cambios:**
1. **Strategy `npm-single`** (nuevo, `strategies/npm_single.rs`, 7 tests):
   detecta package.json raíz como container cuando NO hay workspaces
   npm/pnpm reales. Maneja el edge case de `pnpm-workspace.yaml` con solo
   `allowBuilds:` (config de build, no monorepo — zustand). Confianza 0.70.
2. **Conteo FP/FN corregido (ADR-032)**: solo metatype `mt.container`
   (cargo-workspace, npm-workspace, npm-single, dockerfile, helm).
   Candidates de `components` (mt.component, confidence <1.0) excluidos
   del ratio → clap pasa de FP 27.3% a 0%.
3. **Re-clasificación datasets**: express/zustand → "npm single-package"
   en `bench/datasets.toml` + ADR-032 documentado.

**Resultado:**
| Dataset | FP ratio | FN ratio | Gate |
|---|---|---|---|
| axum | 0% | 0% | ✅ |
| ripgrep | 0% | 0% | ✅ |
| clap | 0% | 0% | ✅ |
| zustand | 0% | 0% | ✅ |
| vueuse | 0% | 28.6% | ✅ |
| express | 0% | 0% | ✅ |
| dogfood | 0% | 0% | ✅ |

**Referencias:** `bench/reports/fpfn-rubric-2026-08-06.md` (v2),
ADR-032 §Counting scope

## M29 — E2E coverage expansion: install, deploy, render, multi-language — **COMPLETO** ✅ (2026-08-06)

**Estado:** Implementado (PR #68). ADR-034 aceptado. Las 4 suites corren y pasan.

**Resultados verificados:**

| Suite | Checks | Resultado |
|---|---|---|
| M29.1 `e2e/install_e2e.sh` | 29 | ✅ PASS (HOME aislado, drift, idempotencia, doctor, view) |
| M29.2 `e2e/render_e2e.py` | 20 | ✅ PASS (5 samples + 4 bundles reales multi-lenguaje + 0 JS errors) |
| M29.3 `smoke_real_projects` v2 | 6/6 | ✅ PASS (rust/go/js/python/typescript, vertical completo) |
| M29.4 `bench/sandbox-e2e.sh` | 6 | ✅ PASS (in-container vertical + JSON verdict) |
| M29.5 integración | — | ✅ verify-local --full + post-release check |

**Bugs descubiertos por las suites (corregidos en el mismo ciclo):**
- CORS entre origins localhost en el diseño del render E2E (resuelto navegando al origin del server por-repo)
- Smoke no determinista: XDG compartida hacía `--apply` reportar "Applied: 0" por grafos previos (aislado por repo)
- `bash -c` con argumento posicional: el dataset se convertía en `$0` (sandbox)

**Referencias:** [ADR-034](adr/ADR-034-e2e-coverage-expansion.md), specs
`e2e-installation.md`, `e2e-render.md`, `e2e-sandbox.md`

**Objetivo:** Cerrar los 4 gaps E2E verificados en la revisión de 2026-08-06:
instalación, despliegue, render y multi-lenguaje no están cubiertos por
suites versionadas. La lección de ADR-031 (unit tests pasan, bugs de
integración sobreviven) se aplica al producto completo.

**Alcance (4 suites, especificadas):**

### M29.1 — E2E de instalación — `e2e/install_e2e.sh`
Flujo de usuario final contra HOME limpio (temp dir):
1. `archctl ide install <ide>` (default `opencode`) → skills/agents/plugin en paths del IDE adapter. `archctl stack install` queda como alias deprecated de `archctl ide install opencode` ([ADR-042](adr/ADR-042-ide-adapter-abstraction.md)) hasta M77.
2. `archctl ide doctor <ide>` (o `archctl ide list --installed`) → drift none
3. Idempotencia (re-install = 0 cambios)
4. `doctor` OK
5. Frontmatter SKILL.md válido
6. `view` sirve `/api/health`
Spec: [`docs/specs/e2e-installation.md`](specs/e2e-installation.md)

### M29.2 — E2E de render — `e2e/render_e2e.py` (playwright, versionado)
Por cada tipo de bundle (C4 context/container, sequence, class-diagram,
call-graph) + bundles REALES multi-lenguaje:
1. `archctl view` → cargar bundle en workbench
2. **Assert de DOM**: nodes visibles, labels, relaciones, vista activa
3. Cero errores de consola JS
4. Screenshots como artifacts
Habría detectado el bug `detectKind` (PR #57) en el primer run.
Spec: [`docs/specs/e2e-render.md`](specs/e2e-render.md)

### M29.3 — Smoke multi-lenguaje ampliado — `smoke_real_projects.rs`
Vertical COMPLETO por lenguaje (hoy solo discover→export→validate en 4
repos): c4-discover + evidence accept + export + validate para rust/js/ts;
call-graph + apply para go; class-diagram + apply para python.

### M29.4 — Sandbox reproducible — `bench/sandbox-e2e.sh`
Reemplaza el one-off manual de 2026-08-06:
1. Build container → compilar archctl DENTRO (glibc ubuntu:24.04)
2. Vertical C4 completo contra axum con asserts
3. Veredicto JSON (`PASS`/`FAIL`) para tooling/CI
Spec: [`docs/specs/e2e-sandbox.md`](specs/e2e-sandbox.md)

### M29.5 — Integración
- `verify-local.sh --full` ejecuta las suites disponibles (condicional a
  podman/playwright)
- Post-release check: descargar binario → smoke mínimo (`--version`,
  `view`, `stack status`) → marcar release verificado
- Workflow CI opcional (no bloqueante de PR): gates manuales primero

**Criterios de éxito:**
- Las 4 suites corren de punta a punta y pasan en un entorno limpio.
- El render E2E detecta un bug de clasificación de bundle (regresión
  detectKind) si se reintroduce.
- La instalación E2E valida el flujo de producto sin tocar la config real.

**Referencias:** [ADR-034](adr/ADR-034-e2e-coverage-expansion.md), specs
`e2e-installation.md`, `e2e-render.md`, `e2e-sandbox.md`

## M30 — Call-graph: visibilidad de lenguajes no soportados — **COMPLETO ✅ (2026-08-06, release v1.1.0)**

**Estado:** COMPLETO ✅ — soporte Go real (tree-sitter-go) vía ast-grep-language builtin-parser; released as v1.1.0 (PR #72, commit f3a00a7).

**Objetivo:** `archctl code call-graph` (y por consistencia `class-diagram` /
`state-machine`) comunican claramente cuándo no escanean archivos por el
límite de lenguajes MVP, en vez de devolver silencio. Eliminar la cobertura
falsa del smoke. Añadir Go como lenguaje soportado.

**Decisión tomada (2026-08-06):** Soporte Go real vía tree-sitter-go 0.25.0 (bundled en ast-grep-language 0.45.0 builtin-parser). No se añade nuevo crate. Confidence 0.85 (mismo que TypeScript).

**Alcance implementado:**
1. Go function extraction: `function_declaration` → FunctionNode, `method_declaration` → MethodNode, `func_literal` → NO node (calls attributed to enclosing named function).
2. Go call-edge extraction: direct calls (`identifier`), method calls (`selector_expression` → `field_identifier`).
3. Smoke: `smoke_echo()` assert extracción Go real (filesScanned > 0, nodes > 0) + `smoke_go_apply_fixture` (apply-path sobre fixture pequeño `tests/fixtures/go_callgraph/`).
4. Human loop: Fase 6 actualizada (Go soportado), Fase 9.2 actualizada (extracción rápida; apply-path cubierto por fixture smoke).
5. Error message actualizado: MVP lista = rust, typescript, python, go.

**Amendment (2026-08-06, ver M32):** el writer `--apply` es lento a escala
(~0.43s/elemento; zustand 212 el → 92s, echo 1307 el → 483s) — problema
preexistente expuesto por Go. Por eso el smoke y el human loop usan
extracción rápida + fixture pequeño para el apply-path; el fix del writer
se trackea en M32.

**Criterios de éxito:**
- `archctl code call-graph --cwd <go-repo> --json` devuelve `nodes.len() > 0` + `edges.len() > 0`.
- Smoke `smoke_echo` pasa con extracción Go real (rápido) + `smoke_go_apply_fixture` con `elements_written > 0` y `relations_written > 0`.
- Human loop Fase 9.2 pasa con Go produciendo extracción > 0 (rápido).
- ADR-035 documenta las decisiones de extracción.

**Referencias:** `archctl/src/code/call_graph.rs`, `archctl/tests/smoke_real_projects.rs`,
`archctl/tests/fixtures/go_callgraph/`, `e2e/HUMAN_LOOP_TEST.md` (Fase 6, 9.2),
`docs/adr/ADR-035-go-call-graph-extraction.md`

## M32 — Apply writer performance: transaction + bulk import — **NUEVO (2026-08-06)**

**Estado:** NUEVO — detectado durante M30 (el soporte Go expone el writer).
Plan de ejecución aprobado y registrado en **ADR-036**.

**Objetivo:** `archctl code call-graph --apply` (y por consistencia los
writers de class-diagram/state-machine/sequence) guarden en segundos, no
minutos.

**Problema detectado (evidencia medida):**

| Dataset | Lenguaje | Elementos | Tiempo --apply |
|---|---|---|---|
| pmndrs/zustand | typescript | 212 | 92s |
| labstack/echo | go | 1307 | 483s |

- ~0.43s por elemento, escalado lineal: `apply()` (`call_graph.rs` L1298)
  abre el store UNA vez pero emite ~5-6 queries por nodo + ~2 por edge
  (~10.500 queries para echo), cada una con su commit/checkpoint.
- **lbug 0.18.3 es Kùzu** (embedded graph DB, FFI C++, threads internos —
  `user 37min` vs `real 8min`), NO SQLite (corregido: la entrada anterior
  decía "SQLite-backed").
- **Sí existe parameter binding** (`Connection::prepare` + `execute`, lbug
  connection.rs L318-354) — el comentario en `store.rs:420` es erróneo.
- Preexistente desde m11 (call-graph) — invisible hasta que M30 activó Go.

**Medidas (orden de ejecución, ver ADR-036):**
1. **D1 — Transacción única** `BEGIN TRANSACTION`…`COMMIT` alrededor del
   apply: 10.500 commits → 1 checkpoint. Impacto 10-100x (echo → ~5-15s).
2. **D4 — Gate de regresión**: bench criterion de call-graph apply (echo
   <10s, zustand <5s, fixture <5s) + corregir comentarios erróneos
   (`store.rs:420`, este mismo entry).
3. **D2 — Bulk import UNWIND** con parámetros, lotes de ~500 (patrón nativo
   Kùzu): 10.500 queries → ~6. Impacto adicional 2-10x (echo → <3s).
4. **D3 — Prepared statements**: `prepare()` una vez por forma, `execute`
   con params; extender `GraphStore` sin romper `query(&str)`.
5. **D5 — Writers hermanos** (class-diagram/state-machine/sequence) si
   comparten el patrón.

**Criterios de éxito:**
- `call-graph --apply` en labstack/echo < 10s (hoy 483s); con D2 < 3s.
- Sin cambio de comportamiento: mismos elementos/relaciones escritos
  (idempotencia y skip de existentes intactos) — tests del writer verdes.
- Bench de regresión añadido (hoy no existe ninguno del writer).
- Correcciones documentales aplicadas (Kùzu, parameter binding).

**Referencias:** [ADR-036](adr/ADR-036-apply-writer-performance.md),
`archctl/src/code/call_graph.rs` (L1050-1290),
`archctl/src/store.rs` (L384-391 `query`, L420 comentario erróneo),
lbug 0.18.3 `src/connection.rs`, M30 amendment (este documento)

## M33 — Pre-push hook: bootstrap assets-stack en worktree fresco — **NUEVO (2026-08-06)**

**Estado:** NUEVO — detectado durante M30 (primer push tras el repair).

**Objetivo:** el pre-push hook (ADR-025, `.githooks/pre-push`) pueda pasar
en worktrees frescos sin intervención manual.

**Problema detectado (evidencia):**
- `archctl/assets-stack/` es generado por `scripts/embed-stack.sh` desde
  `profile/` (ADR-033, rust-embed) y está gitignored.
- `verify-local.sh` (cheap tier) NO bootstrapa assets-stack; el hook hace
  checkout de cada commit en worktree temporal y corre `cargo test` →
  `#[derive(RustEmbed)] folder '<wt>/archctl/assets-stack/' does not exist`
  → 8+ errores E0599 → el push se bloquea para CUALQUIER commit.
- Workaround actual: `git push --no-verify` tras verificación manual local
  (documentado en el hook, pero frágil).

**Alcance:**
1. `scripts/verify-local.sh` (cheap): si `archctl/assets-stack/` no existe,
   ejecutar `scripts/embed-stack.sh` antes de las gates (idempotente).
2. Verificar que pre-push pasa en un clone fresco sin `--no-verify`.

**Criterios de éxito:**
- `git push` de una branch con gates verdes pasa el pre-push sin bypass.
- Sin cambio de comportamiento cuando assets-stack ya existe.

**Referencias:** `.githooks/pre-push`, `scripts/verify-local.sh`,
`scripts/embed-stack.sh`, ADR-025, ADR-033

## M34 — Call-graph strategy consolidation + test hygiene — **CERRADO ✅**

**Estado:** Cerrado ✅ — code landed **v1.6.0** (PR #90, commit tip
`027527b`, merge 2026-08-07). 5 de 6 items del debt-report M30 cerrados:
D2 (`928446c`, −244 LOC en `call_graph.rs`), D3 (`d1eaf20`, fixture Go
unificado), W3 (ADR-037 decidió mantener `InvalidLanguage` con
`#[allow(dead_code)]`), W4 (`d1eaf20`, test real sobre TempDir), D4
(`Language::confidence()` centralizado, call_graph.rs:99-111), D6
(comentario duplicado removido per ADR-037:76-79). D5 (3 help strings
en `cli.rs`) se cerró inicialmente en `702190f` pero fue revertido por
`050a9ae` (capability phase 3) — **residual menor**, las strings siguen
duplicadas en `cli.rs:530, 570, 589` y omiten Java+Kotlin. ADR-037
aceptado documenta el rechazo explícito del strategy-pattern refactor
propuesto en este cuerpo. Verificación independiente archivada en
`sddk/m34-call-graph-strategy-consolidation/explore-report.md` (2026-08-22)
y cycle-log row al final de este ROADMAP.

**Objetivo:** consolidar la deuda detectada en M30 en un ciclo de limpieza
coherente. Ningún item es bloqueante; todos son WARN/SUGG del debt-report.

**Alcance (items del debt-report M30):**
1. **D2 (WARN, preexistente amplificado)** — 8 cuerpos `extract_*_function`
   casi idénticos en `call_graph.rs:433-740` (M30 añadió 2 más:
   `extract_go_function`, `extract_go_method`). Refactor a tabla/estrategia
   común: ~240 LOC reducibles.
2. **D3 (WARN, introducido por M30)** — doble fuente de verdad del fixture
   Go: `const GO_SAMPLE` inline en `tests/code_call_graph.rs` vs
   `tests/fixtures/go_callgraph/main.go`. Unificar (el test debe leer el
   fixture o el fixture ser el único origen).
3. **W3** — `CallGraphError::InvalidLanguage` dead code (clap value_enum):
   `#[allow(dead_code)]` o eliminar + reword spec scenario 11.
4. **W4** — `test_confidence_per_language` vacuo (aserta const contra sí
   misma): reemplazar por `extract` real sobre TempDir y assert
   `node.confidence == 0.85` (~20 LOC, fix recomendado in-cycle en M34).
5. **D4/D5/D6 (SUGG)** — confidence magic numbers en 9 sitios; 3 help
   strings repetidos en `cli.rs:249,289,308`; comentario duplicado en
   `write_call_edge`. Centralizar Language metadata (confianza/label por
   variante).

**Criterios de éxito:**
- Duplicación de extractores reducida sin cambio de comportamiento (tests
  verdes: 525+).
- Un único origen de verdad para el fixture Go.
- Confidence test es un gate real de regresión.
- Sin items WARN pendientes del debt-report M30.

**Referencias:** `sddk/m30-call-graph-go-support/debt-report.md`,
`archctl/src/code/call_graph.rs`, `archctl/tests/code_call_graph.rs`,
`archctl/src/cli.rs`

## M31 — Semántica unificada de `diagram export` sin proyecto/grafo — **NUEVO (2026-08-06)**

**Estado:** NUEVO — detectado durante el human-loop sandbox (M29.4).

**Objetivo:** Definir una única semántica para "export sin proyecto/grafo"
y alinearla entre CLI, server del workbench, tests y documentación.

**Problema detectado (evidencia — incoherencia CLI vs server):**
- CLI: `archctl diagram export container:* --cwd /tmp` → `exit 0`, salida
  "Exported 0 elements, 0 edges, 0 evidence" (éxito vacío).
- Server: `GET /api/export` del workbench sin `project_dir` → **HTTP 500**
  con error JSON (`archctl/src/view.rs` L311-317
  `export_without_project_is_500_json`).
- El guion humano original asumía error (`exit != 0`) — la ambigüedad es
  del producto, no del test.

**Alcance:**
1. Decidir semántica única: (a) error claro `exit != 0` cuando no hay
   proyecto/grafo, o (b) éxito vacío + warning explícito en stderr
   ("no graph found — exported 0").
2. Alinear CLI + server (`/api/export`) + `HUMAN_LOOP_TEST.md` (Fase 9.1) +
   tests unitarios.
3. Actualizar el human-loop sandbox a la semántica decidida.

**Criterios de éxito:**
- Misma semántica en CLI y server (test que cubra ambos).
- Comportamiento documentado en `docs/` y reflejado en el guion humano.

**Referencias:** `archctl/src/diagram/export.rs` (`export_rejects_malformed_selector`),
`archctl/src/view.rs` (`export_without_project_is_500_json`),
`e2e/human_loop_sandbox.sh` (Fase 9)

## Mejoras futuras — `workflowctl`

> Documentadas en [ADR-030](adr/ADR-030-workflowctl-local-multi-repo.md). **No implementadas ahora**: se dejan como referencia para una eventual promoción a topología distribuida. Mantener este bloque sincronizado con ADR-030; cualquier cambio de estado requiere abrir un nuevo ADR.

- MVP local-first: `workflowctl` se ejecuta en el host del desarrollador con `gh act` + Podman rootless, encapsulado en `systemd-run --user`. Concurrencia 1 workflow / 2 jobs internos. Perfiles estándar (4 CPU / 8 GiB), pesado (8 / 16) y benchmark (12 / 16 exclusivo). Snapshot copiado, sin `--bind`, sin `--reuse`, `--container-daemon-socket -`, cache/artifacts solo en `127.0.0.1`.
- Compatibilidad GitHub documentada por workflow y por job (`rust`, `web`, `bench-smoke`, `bench-compare`, `test-unit`, `test-e2e/ui/all`). Jobs con `nested-container` (Podman dentro del workflow) **no se ejecutan** en MVP.
- Diferido: runner remoto efímero dedicado (host bare-metal, identidad de servicio, secret store, `systemd` slice, auth por runner group); coordinator híbrido local + remoto; imagen base propia `act-runner:arch`; `forgejo-runner` o equivalente; perfiles declarativos por repositorio.
- Regla de promoción: exige simultáneamente host dedicado, demanda medida (contención, varios hosts pidiendo cola o workstation apagado bloqueando runs), suite de compatibilidad común, pin SHA en todas las actions, política única de routing, subuid/subgid disjuntos y SELinux en enforcing. Si falla una sola, se mantiene el MVP local.

---

# `archview` — workbench embebido (componente de `archctl`)

> [ADR-038](adr/ADR-038-one-product-five-invariants.md): archview es el workbench interactivo de arch-stack, **embebido en el binario `archctl` vía `rust-embed`** ([ADR-033](adr/ADR-033-archctl-view-embedded-workbench.md)). El comando de entrada es `archctl view` — no hay repositorio separado, no hay proceso servidor de larga vida (cumple ADR-010, sin daemon). ADR-013 sección "repositorio separado" queda **superseded**.

## Stack del workbench (shipped)

| Pieza | Librería shipped (per [ADR-039](adr/ADR-039-renderer-reality-anti-roadmap.md) §Renderer realidad) |
|---|---|
| Framework UI | SolidJS |
| Renderizador de grafos | G6 5.x **canvas** (drag-canvas / zoom-canvas / drag-element) |
| Layout | G6 built-in (dagre, d3-force) |
| Lenguaje | TypeScript |
| Build | Vite |
| Estado de workspace | XDG-only ([ADR-041](adr/ADR-041-workspace-state-persistence.md)) — `~/.local/share/archctl/projects/<hash>/workspace.json`. **NO localStorage** (ADR-038 Invariante 3) |
| Transporte | HTTP one-shot: `archctl view` levanta un servidor local efímero y sirve el workbench desde el binario |

Las decisiones aspiracionales (WASM compute, Apache Arrow, cosmos.gl para >100k, WIT Plugin SDK, SceneGraph abstraction) viven en el **anti-roadmap** de [ADR-039](adr/ADR-039-renderer-reality-anti-roadmap.md) con reopen triggers medibles.

## Contrato bundle ↔ workbench

`archctl diagram export` produce un bundle `DiagramProjection` JSON. `archctl view` lo sirve al workbench embebido:

```bash
archctl diagram export \
  diagram:orders-container \
  --format viewer-bundle \
  --output ~/.local/share/archctl/exports/orders-container/
```

Estructura del bundle (canónica):

```text
diagram-bundle/
├── manifest.json
├── projection.json
├── evidence.json
├── styles.json
└── assets/
```

Schema versionado en `schemas/diagram-projection.schema.json` (única fuente de verdad). Rust DTOs (`archctl/src/diagram/export_types.rs`) y TypeScript types (`archview/src/loader/types.ts`) alineados campo a campo (M71 contract alignment, v0.14.3).

## ChangeSet cosmético (apply path)

Cambios visuales del workbench vuelven como ChangeSet ([ADR-038](adr/ADR-038-one-product-five-invariants.md) Invariante 4 — apply **cosmético**, nunca muta el grafo semántico):

```bash
archctl view → workbench exporta viewer-changes.json
    ↓
archctl diagram apply --changes viewer-changes.json
```

`baseRevision` (blake3) asegura integridad: apply rechaza revisiones stale con mensaje claro. Undo/redo vía inverse ChangeSets.

## Mini-roadmap del workbench (M17 + H0–H3)

El workbench shipped cubre el bundle loader, pan/zoom y el sidebar de evidencias (M17.0, v0.14.0). Las vistas siguientes son **MVPs de lista-texto shipped** en `archview/src/views/`:

| Vista | Comando generador | Estado shipped |
|---|---|---|
| C4 (Context / Container / Component) | `archctl code c4 discover` + `archctl diagram export` | Lista-texto shipped; semantic zoom interactivo continuo **difiere per ADR-039** (reopen = G6 canvas FPS <30 en drill-down de >500 nodos) |
| Call graph | `archctl code call-graph` (v0.8.0) | Lista-texto shipped; blast radius N-hop **difiere per ADR-039** (reopen = ≥1 usuario real con dataset concreto) |
| Sequence | `archctl code sequence` (v0.9.0) | Lista-texto shipped |
| Class (UML) | `archctl code class-diagram` (v0.13.0) | Lista-texto shipped |
| Package | `archctl inventory depends` + extractors | Lista-texto shipped |

Las features avanzadas (semantic zoom continuo, blast radius computacional, drift detection, impact analysis) requieren triggers medibles per ADR-039/ADR-040 antes de reactivar. **Won't Do v1.x** las que reintroducirían el grafo canónico en el write path (M13/M14, [ADR-038](adr/ADR-038-one-product-five-invariants.md) Invariante 4).

---

# `archctl` — comparación de entry points (CLI vs workbench)

| Aspecto | `archctl <subcommand>` (CLI) | `archctl view` (workbench) |
|---|---|---|
| Lenguaje | Rust | TypeScript (SolidJS + G6 5.x canvas) |
| Tipo | CLI sidecar one-shot | Aplicación web local servida por el mismo binario |
| Persistencia | Lee/escribe LadybugDB (XDG, ADR-004) | Solo lee bundles + estado cosmético (XDG, ADR-041) |
| Red | Bloqueada (ADR-011) | Bloqueada por construcción (sin CDN, CSP local) |
| Output | `.svg`, `.dsl`, `.puml`, bundle JSON | HTML+SVG interactivo servido desde el binario |
| Distribución | Binario único (rust-embed, ADR-033) | Mismo binario — no hay artefacto separado |
| Lifecycle | Cada comando es una transacción corta | Una sesión de revisión; servidor efímero por invocación |
| Concurrencia | Lock por proyecto (ADR-010, fs2 flock) | N/A (no accede al grafo directamente) |

> **Regla mental:** `archctl view` es el único comando que abre el workbench. No hay `archview` standalone, no hay `archview serve`, no hay segundo binario.

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
- `archview` (workbench embebido en `archctl` vía `rust-embed` per [ADR-038](adr/ADR-038-one-product-five-invariants.md) / [ADR-033](adr/ADR-033-archctl-view-embedded-workbench.md)) consume los bundles vía `archctl view` cuando se necesita interactividad.
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
| `diagram-authoring-toolchain` | `feat/diagram-authoring-toolchain` (merged to main via PR #23) | `e8c1146` (merge commit) | **Cerrado** ✅ · tag `v0.14.6` |
| `m18-reactive-runtime` (PR1+PR2+PR3) | `feat/m18-reactive-runtime` (3 stacked PRs merged to main) | `b50dbfa` | **Cerrado** ✅ · tag `v0.14.0` |
| `m21-cognitive-layer` | direct commits on `main` (no SDDK cycle — cognitive foundation) | `e0224b8` | **Cerrado** ✅ · tag `v0.15.0` · ADR-021 header actualizado por ADR-040 a **Aceptado (conditional)** |
| `m22-agent-catalog` | `feat/m22-agent-catalog` (merged to main via PR #30) | `8b76ef5` | **Cerrado** ✅ · tag `v0.15.0` · ADR-022 header actualizado por ADR-040 a **Aceptado (parcial)** (2/9 agentes shipped; 7 deferred) |
| `m23-action-proposal-policy` | direct commits on `main` (M23 phases 1–6) | `ae83e61` | **Cerrado** ✅ · tag `v0.18.0` · ADR-023 header actualizado por ADR-040 a **Aceptado (diferido)** (phase 1 PR #32 closed stale; reactivación solo con workflow HITL real per ADR-040) |
| `m27-sandbox-benchmarks` | `feat/m27-pr4-cleanup` (merged to main) | `b87a902` | **Cerrado** ✅ · tag `v0.22.0` |
| `m30-call-graph-go-support` | `feat/m30-call-graph-go-support` (merged to main via PR #72) | `f3a00a7` | **Cerrado** ✅ · tag `v1.1.0` |
| `m32-apply-writer-performance` (PR1) | `feat/m32-apply-writer-performance` (merged to main via PR #76) | `7bdcc5f` | **Cerrado** ✅ · tag `v1.2.0-m32` · D4+D1 shipped |
| `m32-apply-writer-performance` (PR2) | `feat/m32-apply-writer-performance-pr2` (merged to main via PR #78) | `7a20c55` | **Cerrado** ✅ · tag `v1.3.0-m32-pr2` · D2 shipped (D3+D5+BREAK-1 deferred) |
| `m33-pre-push-hook-assets-stack` | `fix/m33-pre-push-hook-assets-stack` (merged to main via PR #80) | `5fb2be1` | **Cerrado** ✅ · tag `v1.3.1` · pre-push hook bootstrap + clippy fix |
| `m32-d5-sibling-writers-apply` | `feat/m32-d5-sibling-writers-apply` (merged to main via PR #82) | `e309aec` | **Cerrado** ✅ · tag `v1.4.0-m32-d5` · class_diagram + state_machine transaction wrap |
| `m32-break-1-remove-seed-writes` | `fix/m32-break-1-remove-seed-writes` (merged to main via PR #84) | `864aab7` | **Cerrado** ✅ · tag `v1.4.1` · BREAKING JSON shape — removed seed_writes |
| `m26-c4-contract-integrity` | `fix/m26-c4-contract-integrity` (merged to main via PR #44) | `18cf12b` | **Cerrado** ✅ · tag `v0.14.9` |
| `m26-c4-vertical-validation` | `fix/m26-vertical-validation` (merged to main) | `4f22224` | **Cerrado** ✅ · tag `v0.14.10` |
| `m27-sandbox-benchmarks` | `feat/m27-pr4-cleanup` (merged to main) | `b87a902` | **Cerrado** ✅ · tag `v0.22.0` |
| `m31-unified-diagram-export-empty-semantics` | `feat/m31-unified-diagram-export-empty-semantics` (merged to main via PR #86) | `09e69dc` | **Cerrado** ✅ · tag `v1.5.0` · envelope {empty, warning, manifest} |
| `m31-fu1-tracing-stderr-redirect` | `fix/m31-fu1-tracing-stderr-redirect` (merged to main via PR #88) | `a134dcc` | **Cerrado** ✅ · tag `v1.5.1` · closes M31 pre-existing MEDIUM (tracing → stderr) |
| `m34-call-graph-strategy-consolidation` | `refactor/m34-call-graph-strategy-consolidation` (merged to main via PR #90) | `027527b` | **Cerrado** ✅ · tag `v1.6.0` · consolidates 5 M30 debt-report items (~240 LOC reduction) |
| `m35-java-call-graph` | `feat/m35-java-call-graph` (merged to main via PR #92) | `c120be0` | **Cerrado** ✅ · tag `v1.7.0` · adds Java as 5th language to archctl code call-graph |
| `m36-kotlin-call-graph` | `feat/m36-kotlin-call-graph` (merged to main via PR #94) | `849b9d6` | **Cerrado** ✅ · tag `v1.8.0` · adds Kotlin as 6th language to archctl code call-graph |
| `m38-plantuml-mermaid-render-local` | `feat/m38-plantuml-mermaid-render-local` (merged to main via PR #96) | `d43d24a` | **Cerrado** ✅ · tag `v1.9.0` · wires Mermaid → SVG via merman (pure Rust); PlantUML deferred to M40 |
| `m37-json-schema-public-and-pure-flag` | `feat/m37-json-schema-public-and-pure-flag` (merged to main via PR #98) | `e238b75` | **Cerrado** ✅ · tag `v1.10.0` · pure --json stdout mode (no 5-file write) + round-trip schema validation |
| `m39-use-case-diagrams-end-to-end` | `feat/m39-use-case-shapes-and-e2e-render` (merged to main via PR #101) | `7fc3e96` | **Cerrado** ✅ · tag `v1.11.0` · use case view end-to-end (mermaid: actor=rect, usecase=circle) + usecase_view_e2e regression test; pre-M39 bare (Label) bug discovered and fixed |
| `m40-plantuml-render-local` | `feat/m40-plantuml-render-local` (merged to main via PR #103) | `5137ce1` | **Cerrado** ✅ · tag `v1.12.0` · PlantUML render via user-installed backend (Java CLI / docker / custom); plantuml-little rejected (hard-links graphviz); 3 install options in error |
| `m41-state-and-c4-e2e` | `feat/m41-state-and-c4-e2e` (merged to main via PR #105) | `c93cb53` | **Cerrado** ✅ · tag `v1.13.0` · state + C4 Mermaid projector bug fixed (id([Name]):::state + id(name)/id([Name])); every Mermaid view now renders end-to-end |
| `m43-use-case-plantuml-e2e-verify` | `feat/m43-use-case-plantuml-e2e-verify` (merged to main via PR #107) | `e6a8f05` | **Cerrado** ✅ · tag `v1.14.0` · use case projector (M39) + PlantUML backend (M40) verified end-to-end; new regression test |
| `m45-sequence-edge-labels` | `feat/m45-sequence-edge-labels` (merged to main via PR #109) | `6ef763c` | **Cerrado** ✅ · tag `v1.15.0` · sequence edge labels from edge.props["label"] (Mermaid + PlantUML); backward-compatible (absent/empty/non-string = bare arrow) |
| `m46-stale-manifest-public-symbols` | `feat/m46-stale-manifest-public-symbols` (merged to main via PR #111) | `bc8cbbc` | **Cerrado** ✅ · tag `v1.16.0` · remove 26 stale public_symbols (enum variants + struct fields) from 8 manifests; all 26 scopes now pass doctor |
| `m47-changelog-and-session-summary` | `feat/m47-changelog-and-session-summary` (merged to main via PR #113) | `50807b6` | **Cerrado** ✅ · tag `v1.17.0` · CHANGELOG backfill (14 cycles v1.4.1 → v1.16.0) + docs/README view specs + schemas index |
| `m48-sequence-plantuml-e2e-verify` | `feat/m48-sequence-plantuml-e2e-verify` (merged to main via PR #115) | `33292ce` | **Cerrado** ✅ · tag `v1.18.0` · sequence PlantUML e2e verify (mirrors M43 for sequence; closes M45+M40 wiring loop) |
| `m49-state-plantuml-e2e-verify` | `feat/m49-state-plantuml-e2e-verify` (merged to main via PR #117) | `a9cfadb` | **Cerrado** ✅ · tag `v1.19.0` · state PlantUML e2e verify (mirrors M43/M48 for state; closes M41+M40 wiring loop) |
| `m50-c4-plantuml-e2e-verify` | `feat/m50-c4-plantuml-e2e-verify` (merged to main via PR #119) | `f03fa37` | **Cerrado** ✅ · tag `v1.20.0` · C4 PlantUML e2e verify + fix projector (emit vanilla PlantUML actor/rectangle instead of Structurizr-style); verification triangle CLOSED |
| `m51-prepared-statements-and-parameter-binding` | `feat/m51-prepared-statements-and-parameter-binding` (merged to main via PR #121) | `6c40283` | **Cerrado** ✅ · tag `v1.21.0` · prepared statements + parameter binding on GraphStore port (M32 D3); lbug impl works; call_graph migration deferred due to JSON-vs-typed-value quirk |
| `m52-m32-d4-doc-fixes-and-bench-criterion` | `feat/m52-m32-d4-doc-fixes-and-bench-criterion` (merged to main via PR #123) | `700d425` | **Cerrado** ✅ · tag `v1.22.0` · fix 3 stale "no parameter binding" claims post-M51 (queries.rs, graph.rs); ROADMAP + bench criterion already correct from prior cycles |
| `m53-m32-d5-sequence-writer-audit` | `feat/m53-m32-d5-sequence-writer-audit` (merged to main via PR #125) | `64a8be3` | **Cerrado** ✅ · tag `v1.23.0` · M32 D5 audit verdict: N/A (sequence.rs is read-only per SCN-217, no apply/writer to migrate); M32 D5 doesn't apply |
| `m54-session-close` | `feat/m54-session-close` (merged to main via PR #127) | `TBD` | **Cerrado** ✅ · tag `v1.24.0` · session close (M51-M53 backfilled in CHANGELOG; Engram session summary); 21 cycles total in this session |
| `m55-codebase-state-study-and-roadmap-proposals` | `feat/m55-codebase-state-study-and-roadmap-proposals` (merged to main via PR #128) | `57e8e50` | **Cerrado** ✅ · tag `v1.25.0` · post-session state study at v1.24.0 + 11 prioritized improvement proposals (M56–M68) in `docs/sessions/2026-08-07-codebase-state-study.md` |
| `m56-dry-skip-on-missing-backend` | `feat/m56-dry-skip-on-missing-backend` (merged to main via PR #130) | `e7e07e3` | **Cerrado** ✅ · tag `v1.26.0` · DRY skip-on-missing-backend helper extracted to `archctl::test_helpers::plantuml::backend_available`; -7 LOC net across 5 e2e files |
| `m59-close-stale-pr-32` | (no PR; closed via gh) | `n/a` | **Cerrado** ✅ · tag none · closed stale PR #32 (M23 Phase 1/6, 4+ days open, merge conflicts with main); rationale + re-open path documented in PR comment |
| `m62-state-md-refresh` | `feat/m62-state-md-refresh` (merged to main via PR #132) | `b354e06` | **Cerrado** ✅ · tag none · STATE.md refresh to v1.26.0 (was dated v1.1.0); 22 cycles of new content; pure docs cycle (no tag bump) |
| `m60-resolve-todos` | `feat/m60-resolve-todos` (squash-merged to main via PR #134) | `44943ea` | **Cerrado** ✅ · tag `v1.27.0` · resolves 2 TODO markers from M55 study (dockerfile.rs:139 OCI LABEL parser; class_diagram.rs:1067 Python class method extraction); 12 new unit tests; 1 golden fixture regenerated |
| `m25-authority-execution-classes` | `feat/m25-trust-enforcement` (merged via PR #287 docs + PR #288 code + PR #289 verify) | `d8c4a6a` | **Cerrado** ✅ · tag `v1.83.0` · ADR-063 + UAT-06 gate green (`false_canonical_promotions: 0`); trust.rs + chokepoint + 10 unit tests + 2 integration tests (9 `#[ignore]`d skeletons pending TRUST-005 + spec-35) |
| `m57-contributing-md` | `docs/m57-contributing-md` (merged to main via PR #136) | `cb0b83f` | **Cerrado** ✅ · tag `v1.28.0` · adds CONTRIBUTING.md (248 lines) with cycle workflow, manifest hygiene conventions, bounded contexts, testing rules, what-not-to-do list; cross-referenced from AGENTS.md |
| `m58-specs-index` | `docs/m58-specs-index` (merged to main via PR #138) | `e16e249` | **Cerrado** ✅ · tag none · adds docs/specs/index.md (85 lines) with 13 specs grouped by audience (diagram views, code extraction, rendering, benchmarks, E2E); each row carries audience + one-line summary; pure docs cycle (M62 precedent, no tag bump) |
| `m61-cognitive-policy-tests` | `test/m61-cognitive-policy-tests` (merged to main via PR #140) | `78c1e0d` | **Cerrado** ✅ · tag `v1.29.0` · adds 22 unit tests for cognitive/policy/{context,decision} (the 0-test gap in M55 study M61 audit); side-fix: `PolicyResult` derives PartialEq; cognitive test count 111 → 133 |
| `p0-ladybug-compatibility-doctor-v2` | `feat/p0-ladybug-doctor-v2` (merged to main via PR #174) | `31b17e1` | **Cerrado** ✅ · tag `v1.42.0` · `archctl doctor --scope storage [--json]`: LadybugDB availability + crate/native alignment + schema init + CRUD smoke probe (ADR-048 5-axis envelope); debt-verify PASS_WITH_WARNINGS |
| `p0-03-native-release-runners` | `fix/p0-03-native-release-runners` (merged to main via PR #177) | `6680401` | **Cerrado** ✅ · tag `v1.43.0` · `release.yml` native runners per target (darwin on `macos-13`/`macos-14`, linux aarch64 on `ubuntu-24.04-arm`) — Wave 0 item 7/7 |
| `p1-09-dep-fitness-baseline` | `feat/p1-09-dep-fitness-baseline` (merged to main via PR #178) | `687ce4e` | **Cerrado** ✅ · tag `v1.43.0` · `scripts/check-dep-fitness.sh` report-only + baseline ratchet (`dep-fitness-baseline.txt`); wired into `verify-local.sh` cheap tier + `test-ci-gates.sh` |
| `p1-01-composition-root` | `feat/p1-01-composition-root` (merged to main via PR #179) | `f046247` | **Cerrado** ✅ · tag `v1.43.0` · `GraphStoreFactory`/`LbugStoreFactory` composition root; `CliContext` gains `clock` + `store_factory`; 9 store call sites + 8 clock literals rewired |
| `p1-03-architecture-repositories` | `feat/p1-03-architecture-repositories` (merged to main via PR #180, merge `9a1fb17`) | `95a2e5c` | **Cerrado** ✅ · tag `v1.43.0` (peels `95a2e5c`, ancestor of main via PR #180 — push directo bloqueado por GH006, 4º precedente) · 5 repository traits (Element/Evidence/Source/Evaluation/Diagram) implemented by `LbugStore`; `graph.rs` no longer imports `lbug` (dep-fitness 4→3) |
| `p1-04-raw-graph-query-boundary` | `feat/p1-04-raw-graph-query-boundary` (merged to main via PR #181) + patch `fix/p1-04-admin-query-guard` (PR #182) | `b039dee` | **Cerrado** ✅ · tags `v1.44.0`+`v1.44.1` · RawGraphQuery admin-only boundary (tokenized `is_read_only_query` guard), SemanticEdgeRepository, `ensure_metatype`, `diagram::queries`→`DiagramRepository`, ~300 LOC dead code out; ADR-059 (+amendment); verify PW→remediated · debt-verify PW (0 criticals) · UAT READY 3/3 |
| `p1-05-unit-of-work` | `feat/p1-05-unit-of-work-pr1` (merged to main via PR #184, merge `cf8de64`) + `feat/p1-05-unit-of-work` (merged to main via PR #185, merge `189f029`) | `189f029` | **Cerrado** ✅ · tag `v1.45.0` (peels `189f029`, annotated, pushed + verified remote) · `UnitOfWork` port + `Transaction<'a>` session newtype; 5 apply pipelines wrapped (call_graph, state_machine, class_diagram, c4_discover, diagram::apply_to_store); A-W1 (`+ RawGraphQuery` supertrait dropped) + C-W1 (`session_mut`/`execute_raw_cypher_for_test` cfg-gated under `test-fixtures` feature, `nm` escape-hatch gate = 0) closure; ADR-059 amendment L46.5; verify PASS · debt-verify PASS (0/0/6, DQS 5.7→7.1) · 838/838 tests · chained `--no-ff` PRs (GH006 5º precedente) |
| `p-38e02210a9f14317/m32-apply-writer-performance` | `feat/m32-apply-writer-performance` (PR1 merged via PR #187; PR2 merged via PR #188) + remediation `fix/m32-debt-remediation-r1` | `235c885` | **Cerrado** ✅ · tags `v1.47.0` (PR2) + `v1.47.1` (remediation r1); PR1 quedó sin tag (gap `v1.46.0` documentado en STATE.md; CHANGELOG `[1.46.0]` describe su contenido) · M32 D2 re-ship: UNWIND bulk import was regressed by P1-04 T3 commit `599c863`; PR1 shipped call_graph UNWIND + class_diagram N+1 hoist + UNWIND; PR2 shipped state_machine UNWIND (3 nesting levels) + c4_discover UNWIND; ADR-036 amendment documents D2 re-ship + D3 deferral + class_diagram N+1 fix; bench regression gate via `cargo bench --bench call_graph_apply -- --ignored go_fixture` · debt-verify inicial FAIL (2 CRIT + 6 HIGH: class_diagram UUID mismatch + apply_common port bypass) → remediation fix-forward cierra ambos CRITICALs y los 6 HIGHs, agrega suite de regresión CURRENT_VERSION cross-writer, re-audit debt-verify PASS_WITH_WARNINGS (0/0/3) · 849/849 tests · see ADR-036 amendment and CHANGELOG v1.46.0 + v1.47.0 + v1.47.1 |
| `p-38e02210a9f14317/p1-08-capability-registry` | `feat/p1-08-capability-registry` (merged via PR #191) | `1cc03c1` | **Cerrado** ✅ · tag `v1.48.0` · Wave 1 items 15+16 (P1-08): typed capability registry (79 entries, 8 categories) en nuevo bounded context `archctl/src/capability/`; `archctl capabilities --format json\|markdown` + `--check`; alignment tests bidireccionales (languages, strategies, IDE adapters); `docs/CAPABILITIES.md` generado + staleness gates (verify-local.sh + test-ci-gates.sh); ADR-045 promovido accepted; fix schema call-graph enum 3→6 lenguas (Go/Java/Kotlin fallaban validación); matrices stale README/MANUAL reemplazadas por pointer · verify inicial FAIL (schemaVersion casing + clippy --all-targets + CHANGELOG) → corrección 77e7cd9+5d34aaf+ec03b4b → PASS · debt-verify PASS_WITH_WARNINGS (0/0/10W+5S; CP-5 drift IDE fixeado in-cycle con test bidireccional nuevo) · 872/872 tests · see ADR-045 and CHANGELOG v1.48.0 |
| `p-38e02210a9f14317/docs-state-refresh-v148` | `docs/state-refresh-v148` (merged via PR #192) | `66e9c1e` | **Cerrado** ✅ · tag `v1.48.1` · Ciclo de actualización documental + micro refactor + drift fixes descubiertos por debt-verify: STATE.md refresh (Wave 0 7/7 DONE — era "6/7"; Wave 1 items 8–16 ALL DONE; tabla de versiones corregida: v1.41.6 CI gate, v1.42.0 ladybug doctor, v1.43.0 batch p0-03+p1-09+p1-01+p1-03, v1.45.0 UnitOfWork, v1.47.0 M32 PR2, v1.47.1 remediation; 118 tags, ~41.1K src, ~11.6K tests, ~900 benches; path `/var/home/...` → `/var/mnt/DiscoChino2-fast/...`; Próxima acción → Wave 2); ROADMAP M32 row corregida (v1.46.0 tag nunca existió — gap documentado); spec W1 (13→8 categorías/79 entries); cli.rs OE-2 (drop dead `_ctx` param) + relocate orphan row_to_json doc comment; manifest drift fixes (scope.toml `archctl/src/doctor.rs` → `doctor/` directorio; distribution.toml + Formula cross-link a `docs/maintainers/HOMEBREW_FORMULA.md` restaurado). Doctor full suite 28/30 → 30/30 OK · verify-local PASS · debt-verify PASS_WITH_WARNINGS (0/0/2S, ambos pre-existentes en main y fixeados in-cycle en 66e9c1e) · 872/872 tests · see CHANGELOG v1.48.1 |
| `p-38e02210a9f14317/p2-09-observation-claim` | `feat/p2-09-observation-claim` (apply done, pending verify) | `199a54b` | **Build** · P2-09a observation/claim carriers: `archctl/src/observation_claim.rs` (Observation, Claim, observation_from_evidence, compat_claim_from_evidence, observations_and_claims_for_version); CLI `archctl architecture observe --version-id <VID> [--json]`; re-exports at architecture::; unit tests S1–S9; integration tests S8/S8b/empty version; manifests/architecture.toml + cli.toml updated; CHANGELOG.md entry added · 734 tests · doctor --scopes architecture,cli 0 findings |
| `p-38e02210a9f14317/p2-01-snapshot-mvp` | `feat/p2-01-snapshot-mvp` (merged via PR #194, squash → `23dc50fa`) | `23dc50fa` | **Cerrado** ✅ · tag `v1.49.0` (peels `23dc50fa`, annotated, pushed + verified remote; GH006 7º precedente — push directo bloqueado, PR-merge + tag) · P2-01 snapshot metadata MVP: nuevo bounded context `archctl/src/architecture/{mod,snapshot,errors,digest}.rs` + `pub mod architecture;` en lib.rs; `RepositoryIdentity` carrier + `resolve_repository_identity()` en `identity.rs`; `extractor_set_digest()` en `architecture/digest.rs` (renderer/IDE-stable por construcción sobre `source_code` ∪ `source_cargo`); `SnapshotRepository` trait en `store.rs:438` (`create/get/list/label/pin/update_props/gc`) + impl LbugStore `store.rs:1680+` + supertrait extension `store.rs:207`; CLI `archctl architecture {create,list,gc}` con flags `--json --label --keep-last --dry-run --yes`; `manifests/architecture.toml` NEW + `manifests/store.toml` MOD; GC retention `(pinned) ∪ (last N)` + `SnapshotError::NotGitRepository`; `props.schema_version` (full semver) + `props.schema_compatibility` ("1.0") + `INT64` column = major; 9 integration tests + 4 unit tests verdes · verify PASS WITH WARNINGS (12/17 spec scenarios COMPLIANT, 5 UNTESTED/WARNING — todas desviaciones documentadas del proposal; 7 WARNINGs: CLI surface sin `snapshot` intermedio, `manifests/cli.toml` no actualizado, `use LbugStore` en snapshot.rs, missing renderer-bump regression test, "concurrent" test secuencial, `find_deepest_commit` returns HEAD, `--ref`/--dry-run-default/--keep-last-clamp no implementados, corruption checksum out-of-MVP) · debt-verify PASS WITH WARNINGS (smoke depth; 0C+0H+12W+7S; todos trazables a desviaciones documentadas del proposal: `LbugStore` import leak, `chrono::Utc::now()` no-determinístico, `find_deepest_commit` HEAD fallback, `props.pinned` JSON stamp coupling, manifest `must_not_contain` ausente) · 666/666 tests (657 lib + 9 architecture_snapshot integration + doctest) · see CHANGELOG v1.49.0 |

| `p-38e02210a9f14317/p2-01-followup` | `feat/p2-02-followup` (merged via PR #196) | `8e6c434` | **Cerrado** ✅ · tag `v1.50.0` · closes 7 WARNINGs from p2-01 verify (cli.toml registration, CAPABILITIES.md regen, CLI surface extension, regression tests, `find_deepest_commit` semantics, GC flags) |
| `p-38e02210a9f14317/p2-02-architecture-diff` | `feat/p2-02-architecture-diff` (merged via PR #198) | `8358575` | **Cerrado** ✅ · tag `v1.51.0` · architecture diff MVP — 7-field-group diff projection + `archctl architecture diff`; pure read-side (no lbug write) |
| `p-38e02210a9f14317/p2-03-explain-provenance` | `feat/p2-03-explain-provenance` (merged via PR #199) | `6151bc3` | **Cerrado** ✅ · tag `v1.52.0` · explain/provenance MVP — element/relation paths + honesty principle (unsubstantiated warning) |
| `p-38e02210a9f14317/p2-04-confidence-coverage` | `feat/p2-04-confidence-coverage` (merged via PR #200) | `e37f9ca` | **Cerrado** ✅ · tag `v1.53.0` · coverage metrics MVP — 4 bucket axes over live graph |
| `p-38e02210a9f14317/p2-05-policy-metamodel` | `feat/p2-05-policy-metamodel` (merged via PR #201) | `a4e801a` | **Cerrado** ✅ · tag `v1.54.0` · policy metamodel MVP — 6 closed rules ADR-054 + waivers + glob selector |
| `p-38e02210a9f14317/p2-06-fitness-evaluator` | `feat/p2-06-fitness-evaluator` (merged via PR #202) | `2fdf7da` | **Cerrado** ✅ · tag `v1.55.0` · SARIF 2.1.0 + JUnit XML projectors + `--format {json,sarif,junit}` |
| `p-38e02210a9f14317/p2-07-context-relevance` | `feat/p2-07-context-relevance` (merged via PR #203) | `806d387` | **Cerrado** ✅ · tag `v1.56.0` · context relevance engine — deterministic scoring + BFS expansion + ASCII-fold |
| `p-38e02210a9f14317/p2-08-task-context` | `feat/p2-08-task-context` (merged via PR #204) | `d79ffd4` | **Cerrado** ✅ · tag `v1.57.0` · task context compiler — budget truncation + dangling-relation closure |
| `p-38e02210a9f14317/p2-09-observation-claim` | `feat/p2-09-observation-claim` (merged via PR #205) | `7e63f2c` | **Cerrado** ✅ · tag `v1.58.0` · observation/claim compat carriers + `architecture observe` |
| `p-38e02210a9f14317/p2-10-intent-vs-reality` | `feat/p2-10-intent-vs-reality` (merged via PR #206) + `chore/bump-1.59.0` (PR #207) | `313f18b` | **Cerrado** ✅ · tag `v1.59.0` · intent vs reality MVP — 4-class delta + self-dogfood `archctl-intent.toml` (17 bounded contexts) |
| `wave-3-p2-09-claim-dual-write` | `feat/wave-3-p2-09-dual-write` (merged via PR #211) | `667a706` | **Cerrado** ✅ · P2-09b persistent Observation/Claim tables + dual-write + backfill (migraciones v4+v5) — contenido en CHANGELOG `[1.60.0]` |
| `wave-3-item-22-ide-doctor` | `feat/wave-3-item-22-ide-doctor` (merged via PR #212) | `098a96d` | **Cerrado** ✅ · ide doctor consolidado (JSON, exit codes, stack drift check) — contenido en CHANGELOG `[1.60.0]` |
| `wave-3-item-27-fusion-engine` | `feat/wave-3-item-27-fusion-engine` (merged via PR #213) | `3365abd` | **Cerrado** ✅ · fusion engine — agrega observaciones en fused claims — contenido en CHANGELOG `[1.60.0]` |
| `fusion-engine-followups` | `feat/fusion-engine-followups` (merged via PR #214) | `6f289ae` | **Cerrado** ✅ · tag `v1.60.0` · persistencia FusedClaims (migración v6), `ClaimEvaluator` trait, surfacing explain/coverage · verify PASS (1074 tests) · debt PASS_WITH_WARNINGS |
| `item-28-strict-archbundle-impl` | `feat/item-28-strict-archbundle` + follow-ups (merged via PRs #215–#222) | `dd7a1f9` | **Cerrado** ✅ · tag `v1.61.0` · `--profile strict` (ADR-055 via ADR-061) + archview read-only (Item 29); schema v1.1 `strict`+`checksum` · verify PW (gap archview read-only cerrado en re-iterate) |
| `item-27-residual-fused-claims` | `feat/item-27-residual-fused-claims` (merged via PR #223) | `e47f03a` | **Cerrado** ✅ · tag `v1.62.0` · fuse persiste por defecto + `--expire-stale` + fix `parse_observed_at` |
| `adr055-phase2-secret-scanner` | `feat/adr055-phase2-secret-scanner` (merged via PR #224) | `eded1c9` | **Cerrado** ✅ · tag `v1.63.0` · redact.rs zero-dep deny-by-default (AWS/GitHub/Slack/JWT/private keys/URLs/generic) |
| `p2-09b-backfill-timestamp` | `fix/p2-09b-backfill-timestamp` (merged via PR #225) | `affb740` | **Cerrado** ✅ · tag `v1.64.0` · backfill v5 pre-upgrade rows (parse_observed_at + timestamp() wrap) |
| `changelog-formal-v160-v164` | `chore/changelog-formal-v160-v164` (merged via PR #226) | `1a74a40` | **Cerrado** ✅ · docs-only, sin tag · CHANGELOG secciones por release v1.60.0–v1.64.0 |
| `fuse-on-write` | `feat/fuse-on-write` (merged via PR #227) | `ce28185` | **Cerrado** ✅ · tag `v1.65.0` · recompute_fused_for_versions tras cada write de evidencia + limpieza superseded |
| `fusion-params` | `feat/fusion-params` (merged via PR #228) | `ab5e9ba` | **Cerrado** ✅ · tag `v1.66.0` · `--cutoff-days` + StalenessWeightedEvaluator::new + evaluador en seam |
| `adr055-phase3-entropy` | `feat/adr055-phase3-entropy` (merged via PR #229) | `2ee1107` | **Cerrado** ✅ · tag `v1.67.0` · detección por entropía Shannon ≥4.0 bits/char + allowlist — **ADR-055 CERRADO** |
| `wave-3-workbench-ux` | `feat/wave-3-workbench-ux` (merged via PR #231, squash `f9d76df`) | `f9d76df` | **Cerrado** ✅ · tag `v1.68.0` · ADR-062 (reconsideración ADR-056 alcance parcial, items 31–33): NavigationTarget + pila de navegación (breadcrumbs, back/forward), action palette (copy id, zoom C4, explain vía `GET /api/explain`, relations), semantic zoom Context↔Container↔Component por re-export — sin LensSpec (P3-05 sigue deferida; nivel "Code" con reopen trigger propio) · strict bundles degradan explain · fixes pre-existentes: flock flakiness diagram_export (serialización mutex) + version drift ADR-038 (1.68.0) · verify: archview 147 tests + archctl 1107 tests + clippy/fmt/doctor + verify-local PASS · debt-verify PASS_WITH_WARNINGS (0/0/0, 2 LOW) · A-lite, fallback-path (delegación sddk-* rota) |
| `d2-deprecated-sweep` | `feat/d2-deprecated-sweep` (PR #235) | `11628e1` | **Cerrado** ✅ · tag `v1.69.0` · barrido deprecated (deuda D2 auditoría): `diagram::queries` eliminado (13 call sites → `crate::graph`), `evidence::put` + `extract_with_system_clock` eliminados, manifests sync · release pipeline reparado (PRs #236–#244: archview embed, runners, tag handling, SHA256SUMS) + self-update D5 |
| `uat-smoke-fixes` | `feat/uat-smoke-fixes` (merged via PR #245, squash `8a150d6`) | `8a150d6` | **Cerrado** ✅ · tag `v1.70.0` · UAT multi-lenguaje (smoke axum-rust + echo-go en sandbox Podman): 5 bugs de producto — `sanitize_identifier` para canonical keys con `@` (13 sitios), `batch_link_of_type` propaga errores (OF_TYPE silenciosos), schema 1.1.1 (`EvidenceEntry.status`), prefixes go/java/kotlin/javascript en `parse_from_selector`, categoría `code` en relevance/coverage/explain · harness `bench/smoke-matrix.sh` + `bench/build-in-sandbox.sh` · validación: 54 binarios de test ok, clippy -D warnings, fmt, doctor 6 scopes 0 findings · nota: rel_id no-ASCII en class-diagram latente |
| `uat-vueuse-paths` | `fix/vueuse-at-paths` (merged via PR #247, squash `6737671`) | `6737671` | **Cerrado** ✅ · tag `v1.71.0` · paths de evidence/source como DATA (UAT vueuse): `@` en rutas (snapshots npm scoped, patches) rompía `call-graph --apply` con `write_source_artifact` error · 5 sitios en `store.rs` pasan de charset-validation a quote-escaping (put_evidence, list_evidence, list_evidence_by_status, put_source, put_structural_evidence) · ADR-005 preservado (path real) · vueuse: 1239 elementos / 13878 relaciones · 2 regresiones (round-trip `@` + inyección con comillas) |
| `uat-vueuse-pnpm` | `fix/vueuse-pnpm-workspace` (merged via PR #249, squash `f631025`) | `f631025` | **Cerrado** ✅ · tag `v1.72.0` · detección de workspaces pnpm (UAT vueuse): NpmWorkspace parsea pnpm-workspace.yaml + expande globs `/*` (scoped + exclusiones `!`) — antes pasaba los globs como paths literales al walker y nunca detectaba miembros (npm/yarn/pnpm) · components ignora dirs ocultos (`packages/.test`) · ids `c4:container:@vueuse/core` sanitizados (fallaban batch OF_TYPE → rollback) · vueuse: 12 containers, export container:* = 12 nodes · tests npm-workspace 5 + components + c4_discover sanitize · suite 54 binarios, clippy/fmt/doctor ok |
| `uat-consistency-sprint` | `fix/uat-sprint-debts` (merged via PR #252, squash `e52ea18`) | `e52ea18` | **Cerrado** ✅ · tag `v1.73.0` · sprint de consistencia post-UAT: verify-local.sh resuelve binario vía `CARGO_TARGET_DIR` env / `~/.cargo/config.toml` (antes apuntaba al stale `archctl/target/release/archctl` v1.45.0) · bench/datasets.sh --populate-self-dogfood rsyncea el checkout local al cache (smoke rust archctl) · smoke-matrix.sh accept_cell falla con 0 evidences (gate no-vacuo) · e2e/human_loop_sandbox.sh Fase 9.2 path check corregido · c4_discover batch_link_of_type error incluye sample_id · doc state-machine corregido (rust/ts/python, no kotlin) |
| `m21-g6-culling-lod` | `feat/m21-g6-culling-lod` (merged via PR #266, squash `93bae6b`) | `93bae6b` | **Cerrado** ✅ · tag `v1.78.0` · G6 viewport culling + zoom LOD (M21): reduce overdraw en bundles 1000+ nodos · `optimize-viewport-transform` behavior + `applyZoomLod` post-render hook · CullingService DI seam con `noopCullingService` stub · viewport detection en `wheel`/`drag-canvas:end` · opt-in per view con M18 orthogonality guard (C4View solo cuando `levelFilter === null`) · 225/225 tests pass · AC-1/AC-2 validados manualmente pre-PR · `perf-ci-gate` issue pendiente para automatización CI |
| `m22-sidebar-tabs` | `feat/m22-sidebar-tabs` (merged via PR #1, squash `7b8773e`) | `7b8773e` | **Cerrado** ✅ · tag `v1.79.0` · Sidebar tabs (evidence / relations) con ARIA tablist: `<TabBar>/<TabPanel>` primitive con keyboard nav APG · Sidebar.tsx integration con `activeTab` signal reset per node · SourceDrawer inside evidence panel · +14 tests (6 unit + 4 integration + 2 M20 compat + 2 sidebar-actions compat) · 239/239 archview tests pass · cierra sprint M17.1 |
| `m23-perf-ci-gate` | PRs #274–280 (squash merges) | `70c8fbf` | **Cerrado** ✅ · tag `v1.80.0` · ADR-019 enforcement para archview: post-merge CI job `perf-cull` en `ci.yml` · nuevo script `scripts/bench-compare-archview.sh` (mirrors `bench-compare.sh` precedent) · refactorizado `archview/bench/perf-cull.mjs` con JSON output + fixes de bugs latentes (L47 hardcoded path, L172 undefined timestamps) · +66 LOC contract tests en `test-ci-gates.sh` §11 · ADR-019 §enforcement actualizado con estado de implementación por repo · lighthouse y 10k+100k datasets out of scope (debt)

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
  - **ADR-013** revisado: stack de `archview` reemplazado completamente. Sprotty y Cytoscape.js descartados. G6 5.x WebGPU + cosmos.gl + SolidJS + Rust/WASM. 5 vistas coordinadas explícitas. *(Historia: ADR-013 sección "repositorio separado" superseded por [ADR-038](adr/ADR-038-one-product-five-invariants.md); stack WebGPU+WASM+cosmos.gl superseded por [ADR-039](adr/ADR-039-renderer-reality-anti-roadmap.md) — el shipped es G6 5.x canvas).*
  - **ADR-019** nuevo: Performance budget (hard contract). TTFP <1s, pan/zoom 60 FPS, filter <50ms, memory <500MB para 100k nodos. 14 anti-patterns explícitos. Benchmark suite canónico + CI gate. *(Historia: techo contractual preservado en [ADR-039](adr/ADR-039-renderer-reality-anti-roadmap.md); canvas cubre 5k–50k; reopen de cosmos.gl gated por ADR-039 anti-roadmap trigger medible).*
  - **ADR-020** nuevo: Renderer stack. G6 5.x WebGPU primary, cosmos.gl adapter para >100k, ELK.js fallback jerárquico. SolidJS UI (no React). Rust → WASM compute. Apache Arrow + TypedArrays. Web Workers + SharedArrayBuffer. RoaringBitmap selections. *(Historia: ADR-020 superseded por [ADR-039](adr/ADR-039-renderer-reality-anti-roadmap.md) §Renderer realidad + §Anti-roadmap; el shipped es G6 5.x canvas sin WASM/Arrow/cosmos.gl. Las 10 decisiones del anti-roadmap de ADR-039 tienen reopen triggers medibles).*
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
  - **ADR-021 (cognitive layer)**: posición en 7 planos (Developer Experience / Cognitive / Projection / Reactive Runtime / Graph / Deterministic / Sensors); contrato uniforme `ReactiveObserver + AgentContext + AgentOutput`; escalera de resolución (heurística → local → potente → humana); coordinación vía estado (eventos), no conversación; MCP como capability boundary; v1.0 ship 2 agentes (heurística pura). *(Historia: ADR-021 header actualizado por [ADR-040](adr/ADR-040-cognitive-conditional-activation.md) a **Aceptado (conditional)** — fundación shipped; full scope reactivación solo con workflow HITL real).*
  - **ADR-022 (agent catalog)**: 9 agentes especializados (Semantic Curator, Architecture, Projection, Investigation, Impact, Planning, Documentation, Presenter, Review/Critic) con suscripciones, view, output schema, budget, capability. v1.0 (M16) ship Architecture + Projection; 1.x (M22) ship los otros 7 con LLM local Phi-3 / potente Claude. *(Historia: ADR-022 header actualizado por [ADR-040](adr/ADR-040-cognitive-conditional-activation.md) a **Aceptado (parcial)** — 2/9 shipped, 7 deferred hasta trigger HITL real).*
  - **ADR-023 (action proposal + policy engine)**: ActionProposal estructurado (goal + command + capabilities + approval + evidence esperada + rollback); Policy Engine con reglas TOML editables; MCP gateway como única frontera de ejecución; audit log inmutable en el grafo; HITL UI en `archview`. *(Historia: ADR-023 header actualizado por [ADR-040](adr/ADR-040-cognitive-conditional-activation.md) a **Aceptado (diferido)** — phase 1 PR #32 closed stale; reactivación solo con workflow HITL real).*
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

> Update 2026-08-18 (cycle `adr-backlog-acceptance`): el audit doc
> §M2 ya documenta estas dos referencias (ver el texto enlazado de
> `docs/audits/2026-08-01-archctl-adr-vs-impl.md:130-135`). Las refs
> aquí son **deliberadamente preservadas** como artefacto histórico
> — no reconstruimos el ADR-015 ni el ADR-018. ADR-015 está
> consolidado en `archctl/src/clock.rs` + `archctl/src/environment.rs`
> + `archctl/src/filesystem.rs` (puertos Clock/Environment/Filesystem).
> ADR-018 eliminado por planning de `m9-archctl-export-apply`
> (coherence-report.md:196). Ver también el §M2 fix-up del audit doc.

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
- **Próximo candidato**: M12 (class-diagram UML, prioridad 2) o M17.0 (workbench embebido vía `rust-embed`, prioridad 1 — *pre-ADR-038: "repo separado" era la planificación original, hoy superseded; el shipped es un solo binario `archctl`*) o cleanup del bench seed-decomposition (true amortization via `BatchSize::PerBatch(N)`).

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

## Cycle cerrado — `m27-sandbox-benchmarks` (v0.22.0)

- **Fecha**: 2026-08-06
- **Branch**: `feat/m27-pr4-cleanup` (merged to main via merge commit `b87a902`)
- **Tag**: `v0.22.0`
- **Verdict**: verify PASS · debt-verify PASS · archive PASS
- **Commits**: 7 (Phase 1-4 PRs merged + archive + verify corrections + merge commit)
- **Tests**: 402 passing (no new tests added by M27 bench code; archctl unchanged)
- **Output**: `bench/` directory con Containerfile, quadlets, datasets.toml, run-bench.sh orchestrator, report generator, + ADR-032 bench methodology
- **Sandbox**: podman Quadlet `archctl-bench.container` con ubuntu:24.04 + rustup 1.97.1
- **Bench**: 10+ datasets (Rust/TS/JS/Go/Python/Java/Kotlin), regression gate >10%, report template
- **Desbloquea**: v1.0 — M27 empirical validation completada
- **Próximo candidato**: Ejecutar `bench/run-bench.sh` contra los 10+ datasets y publicar resultados. Si thresholds de v1.0 se cumplen → preparar v1.0 release. Si no → abrir M28 hotfix.

## Cycle cerrado — `p0-ladybug-compatibility-doctor-v2` (v1.42.0)

- **Fecha**: 2026-08-14
- **Branch**: `feat/p0-ladybug-doctor-v2` (merged a main via PR #174)
- **Tag**: `v1.42.0` (`31b17e1`)
- **Verdict**: verify PASS (2 rondas de corrección) · debt-verify PASS_WITH_WARNINGS (1 CRITICAL residual `target_triple` cerrado post-audit con `archctl/build.rs`)
- **Commits**: 6 (doctor/mod.rs, storage.rs, manifest.rs, runner.rs, cli.rs, build.rs) + chore PR #175 (gitignore debt-verify artifacts)
- **Tests**: 9 integration tests cubriendo los 7 escenarios del spec
- **Output**: `archctl doctor --scope storage [--json]` — probe de compatibilidad LadybugDB (lbug): disponibilidad de crate, alineación crate/native, inicialización de schema y smoke CRUD. Módulo `archctl/src/doctor/` nuevo con `DoctorScope`, `LbugStorageProbe`, `NativeProbe`, smoke gate runner. JSON envelope de 5 ejes per ADR-048 (`archctlVersion`, `lbugCrateVersion`, `native`, `targetCompilerStdlib`, `findings[]`). Tier-1 CI smoke gate en `pr.yml` + release gate en `release.yml`.
- **Bugfix crítico**: `target_triple()` devolvía `"unknown"` porque `option_env!("TARGET")` requiere propagación desde `build.rs` (Cargo no la expone sin build script). Fix: `archctl/build.rs` con `println!("cargo:rustc-env=TARGET={}", ...)` — documentado como jurisprudencia.
- **Notas de proceso**: main es rama protegida → feature branch + PR obligatorio. El ciclo v1 (`p0-ladybug-compatibility-doctor`) quedó bloqueado permanentemente por receipt collision (UNIQUE constraint en `gate_receipts`); v2 reutilizó spec/design verbatim tras validar que seguían vigentes.
- **Wave 0 status**: este ciclo cierra el item 5 del plan de remediation (docs/arch-stack-proposals-2026-08-13). Wave 0 COMPLETA 7/7 — item 7 cerrado por `p0-03` (PR #177, native release runners).

## Cycle cerrado — batch `v1.43.0`: `p0-03` + `p1-09` + `p1-01` + `p1-03`

- **Fecha**: 2026-08-15 (tag `v1.43.0`); PRs merged 2026-08-14/15
- **Branches**: `fix/p0-03-native-release-runners` (PR #177, `6680401`), `feat/p1-09-dep-fitness-baseline` (PR #178, `687ce4e`), `feat/p1-01-composition-root` (PR #179, `f046247`), `feat/p1-03-architecture-repositories` (PR #180, merge `9a1fb17`)
- **Tag**: `v1.43.0` (peels `95a2e5c`, ancestro de main vía PR #180 — release cerrado por ruta PR tras GH006 en push directo)
- **Output**:
  - `p0-03`: `release.yml` native runners per target — darwin en `macos-13`/`macos-14`, linux aarch64 en `ubuntu-24.04-arm`; assets-stack bootstrap pre-release build. Cierra Wave 0 7/7.
  - `p1-09`: `scripts/check-dep-fitness.sh` report-only con baseline ratchet (4 findings legados con paydown paths); `--strict` para el futuro gate CI-blocking; wired a `verify-local.sh` (cheap) y `test-ci-gates.sh` (6 aserciones).
  - `p1-01`: composition root — `GraphStoreFactory`/`LbugStoreFactory`; `CliContext` con `clock: Arc<dyn Clock>` + `store_factory: Arc<dyn GraphStoreFactory>`; 9 store call sites y 8 clock literals rewired.
  - `p1-03`: 5 repository traits (`ElementRepository`, `EvidenceRepository`, `SourceRepository`, `EvaluationRepository`, `DiagramRepository`) implementados por `LbugStore`; `graph.rs` ya no importa `lbug` (dep-fitness baseline 4→3); `graph::Session` público eliminado; `LbugStore::open`+`init` reemplaza `open_and_init` en los 4 apply paths.
- **Notas de proceso**: 4º precedente GH006 — push directo a main bloqueado; release cerrado vía PR #180 --no-ff con tag en el ancestro. Ledger P1-03 con drift git-after-archive documentado y reparado.

## Cycle cerrado — `p1-04-raw-graph-query-boundary` (v1.44.0 + patch v1.44.1)

- **Fecha**: 2026-08-15
- **Branch**: `feat/p1-04-raw-graph-query-boundary` (merged to main via PR #181, `eea645e`) + patch `fix/p1-04-admin-query-guard` (PR #182, `b039dee`)
- **Tag**: `v1.44.0` (`eea645e`) + `v1.44.1` (`b039dee`), ambos verificados en remoto
- **Verdict**: verify PASS_WITH_WARNINGS → remediado in-branch · debt-verify PASS_WITH_WARNINGS (0 criticals) · UAT READY 3/3 · archive DONE (ledger CLOSED, 135 eventos)
- **Commits**: 13 (T1.1+T1.2 trait split, T2.2–T2.6 wiring, T3.1 golden test, T4.1 bench retarget `44df981`, T5.1 manifests+docs)
- **Tests**: 841+ passing final (suite verde tras remediations); 1 regression fixed during apply (state_machine atomic-abort test rewired to repository writes); UAT cazó 2 regresiones invisibles a unit tests → patch v1.44.1 (`open_raw` sin init en reads raw; guard substring dejaba pasar `MERGE` al inicio de token — fix: tokenización)
- **Output**:
  - `RawGraphQuery` trait: admin-only raw Cypher entry point in `store.rs` with `is_read_only_query` guard (rejects MERGE/CREATE/DELETE/SET/REMOVE). `execute_raw_cypher_for_test` escape hatch for test utilities.
  - `SemanticEdgeRepository` trait: `link_semantic_edge`, `link_call_edge_with_resolution` — replaces raw Cypher in `call_graph`, `class_diagram`, `state_machine` apply pipelines.
  - `ElementRepository::ensure_metatype`: metatype pre-seeded existence guarantee for `c4_discover` and `call_graph`.
  - `diagram::queries` rewired to `DiagramRepository::list_elements` / `list_semantic_edges` — 4 free functions removed.
  - `call_graph` dead code removed (~140 LOC): old apply scaffolding replaced by `SemanticEdgeRepository::link_call_edge_with_resolution`.
  - Deprecation re-exports in `diagram::queries` (`ElementRow`, `SemanticEdgeRow`, `VersionPropsRow`) with `#[deprecated(since = "1.43.0")]`.
  - `manifests/store.toml`: `RawGraphQuery` + `execute_raw_cypher_for_test` + `is_read_only_query` added to gates.
  - `manifests/diagram.toml`: P1-04 invariant documented; `.list_elements` / `.list_semantic_edges` in `must_hold`.
  - Benches retargeted: `export_pipeline.rs` uses `DiagramRepository`; `query_pipeline.rs` reads via `RawGraphQuery::query` (MATCH-only, passes guard); `common/mod.rs` uses `execute_raw_cypher_for_test` for all writes (MERGE/CREATE seeds).
- **T4.1 focused fix** (`44df981`): `common/mod.rs` seed writes were using `store.query(MERGE...)` which the `is_read_only_query` guard now rejects at runtime. Fixed by replacing with `execute_raw_cypher_for_test` (test escape hatch). Also corrected relation seed CREATE syntax to match `link_semantic_edge` pattern (MERGE on relation_id keyed by `relation_id` property, then SET for additional props — avoids Kùzu REL TABLE inline-property restriction).
- **Apply deviations from prior tasks**: test fix for `state_machine_apply_atomic_abort_on_write_error` required same pattern as prior class_diagram fix (upsert_element + execute_raw_cypher_for_test); no other deviations.

## Cycle cerrado — `p1-05-unit-of-work` (v1.45.0)

- **Fecha**: 2026-08-15
- **Branch**: `feat/p1-05-unit-of-work-pr1` (PR #184 → `cf8de64` --no-ff) + `feat/p1-05-unit-of-work` (PR #185 → `189f029` --no-ff); branches retained per orchestrator directive
- **Tag**: `v1.45.0` (peels `189f029`, annotated tag `0caf38a0`, pushed + verified remote peeled `189f029` == HEAD)
- **Verdict**: verify PASS · debt-verify PASS (0/0/6, DQS 5.7→7.1) · UAT READY (minor policy) · apply DONE (21 work commits across PR1 + PR2 + W1 remediation + 2 fmt sweeps) · archive PENDING (next phase)
- **Output**:
  - PR1 (`feat(store): UnitOfWork + Transaction`):
    - `pub trait UnitOfWork` + `pub struct Transaction<'a>` (Option γ, primitive-borrower newtype) in `store.rs`
    - `impl UnitOfWork for LbugStore` wrapping `GraphStore::begin/commit/rollback_transaction`
    - 5 apply pipelines collapsed/wrapped on `Transaction`: call_graph, state_machine, class_diagram, c4_discover, diagram::apply_to_store
    - `impl Drop for Transaction`: best-effort rollback on drop without commit (`tracing::warn!`, never panics)
    - 4 new atomic-abort integration tests (store_transaction, call_graph, c4_discover, diagram_apply)
    - `manifests/store.toml`: UnitOfWork + Transaction gates added
    - Version bump 1.44.1 → 1.45.0
  - PR2 (`chore(store): close A-W1 + C-W1`):
    - A-W1: `+ RawGraphQuery` supertrait dropped from `GraphStore` (`store.rs:204`); sole impl `impl RawGraphQuery for LbugStore` on concrete `&self`
    - C-W1: `session_mut` + `execute_raw_cypher_for_test` cfg-gated under `#[cfg(any(test, feature = "test-fixtures"))]`
    - `test-fixtures = []` feature declared in `Cargo.toml`; `archctl/benches/common/mod.rs` gated with `#![cfg(feature = "test-fixtures")]`
    - MockGraphStore + TinyGraphStore supertrait impls removed; `impl UnitOfWork` (stub) added
    - ADR-059 amended: P1-05 closure documented (amendment block + §Implementación L46.5)
    - CHANGELOG v1.45.0 entry added
  - P1-05 2.5b (`chore(ci): test-fixtures surface propagation`):
    - `scripts/verify-local.sh`, `.github/workflows/ci.yml`, `.github/workflows/pr.yml`: `--features test-fixtures` added to test/clippy commands
    - `AGENTS.md` Test Commands section updated; `CONTRIBUTING.md` verify-local description updated
    - Pre-existing clippy dead-code warnings cleaned up (unused imports + dead helpers)
- **Kùzu jurisprudence documented**: Kùzu 0.18.3 auto-reverts entire transaction on any query error. `link_with_merge_fallback` fixed to use idempotent `OPTIONAL MATCH ... WHERE r IS NULL ... CREATE` pattern — single query, no conditional error-throwing, no spurious auto-reverts.
- **Notas**: ADR-059 documents the trait-split decision; the 22 application call sites that were using `GraphStore::query` for writes are now using typed repository methods.
- **Próximo candidato**: P1-02 (CLI commands → handlers) o P1-06/P1-07 (extractor suite sobre el `UnitOfWork` port ahora estabilizado, o depuración de la connascence-of-Implementation residual con `*mut LbugStore` cuando llegue SparrowStore). A-W1 + C-W1 ya cerrados en este ciclo, así que ya no son deuda viva.

## Cycle cerrado — `m21-g6-culling-lod` (v1.78.0)

- **Fecha**: 2026-08-20
- **Cycle id**: `p-38e02210a9f14317/m21-g6-culling-lod`
- **Branch**: `feat/m21-g6-culling-lod` (PR #266 squash → `93bae6b`) + `release/v1.78.0` (PR #267 squash → `e8a313d`) + `fix/v1.78.0-cargo-lock` (PR #268 squash → `fec8130`)
- **Tag**: `v1.78.0` (annotated, peels to `fec8130`)
- **Verdict**: verify PASS_WITH_WARNINGS (2 warnings no bloqueantes + 1 suggestion) · debt-verify SKIPPED (reversibility=HIGH per proposal, A-lite `conditional`) · release REPORTED (status `RELEASE_PENDING`, terminal)
- **Output**:
  - **M21 work** (PR #266): G6 culling + zoom LOD con `CullingService` DI seam. `optimize-viewport-transform` behavior + zoom LOD post-render hook (labels<0.5, edges<0.25). `culling-service.ts` (~150 LOC, pure interface + factory + stub). Viewport detection bbox-vs-viewport + 10% margin + debounce 100ms via `setElementVisibility` batch. Per-view opt-in via `RendererOptions.enableCulling` (C4View, CallGraphView, ImpactView). M18 orthogonality guard: C4View desactiva culling cuando `levelFilter !== null`. Sample `c4-stress-1k.json` commiteado (1221 nodos / 3920 edges, hub `system:core`). Perf gate `bench/perf-cull.mjs` Playwright (manual pre-PR, decision D).
  - **Release** (PR #267): bump `archctl/Cargo.toml` 1.77.0 → 1.78.0 + CHANGELOG entry + STATE.md update.
  - **Cargo.lock follow-up** (PR #268): regenerated lockfile via `cargo build`; squashed into `fec8130`. Same pattern as `f9ffc7f` (v1.68.0 follow-up).
- **Tests**: 225/225 archview (+29 nuevos: 26 CullingService unit + 3 C4View integration). `cargo test --features test-fixtures` PASS. `pnpm lint` 0 errors. `pnpm build` OK. `cargo clippy -- -D warnings` clean. `archctl doctor --scopes diagram,evidence,store` OK.
- **DQS**: ~88/100 (connascence Position↔Visibility bounded; Information Bottleneck holds: views no importan de `@antv/g6` para cull).
- **Decisiones locked** (de explore → proposal → spec): A (umbrales LOD 0.5/0.25), B (opt-in por view), C (M18 pill orthogonality), D (CI gate out of scope, issue `#perf-ci-gate`), E (regenerar 1k sample en T0).
- **Jurisprudence**: M19 `LayoutService` DI pattern reusado para `CullingService` (mismo molde `archview/src/renderer/layout-client.ts:54-66`). M20 `<VirtualList>` primitive pattern no conflict — ortogonal. M18 semantic-zoom pill preservado.
- **Gaps framework documentados** (kernel 1.28.0):
  - `phase.plan.complete.a-lite` no existe en catálogo → cycle saltó `plan` directamente (`design → build` vía `phase.design.complete.a-lite`). No bloquea el trabajo pero requiere parche de framework.
  - `phase.release.complete` no es transition válida (terminal `RELEASE_PENDING`). `release-passed` gate sin evaluador registrado. Bookkeeping, no bloquea el release real.
- **Próximo candidato**: M22 (Sidebar con tabs evidence vs relations) o iterar sobre UX del culling con métricas del perf gate en CI (issue `#perf-ci-gate`).

## Cycle cerrado — `trust-005-observation-fusion` (v1.84.0)

- **Fecha**: 2026-08-21
- **Cycle id**: `p-38e02210a9f14317/trust-005-observation-fusion`
- **Branch**: `feat/trust-005-pr1-docs` → `feat/trust-005-pr2a-types` → `feat/trust-005-pr2b-bridge` → `feat/trust-005-pr3a-uat-7-9` → `feat/trust-005-pr3b-uat-13-15` (5 chained PRs)
- **Tag**: `v1.84.0` (annotated, peels `9205ec7`, pushed + verified remote)
- **Output**:
  - **ADR-064** (PR1): Fusion Bounded Context. Promoted from Proposed → Accepted. Documents the trust-gated FusedClaim recompute + Feedback/Reconciliation as first-class types + m30 bridge contract.
  - **spec-35 v1.1 + spec-12 v1.1** (PR1): Full implementable specs with field shapes, validation rules, determinism contract, m30 bridge.
  - **feedback.rs + reconciliation.rs** (PR2a): New bounded context modules. `Feedback`, `FeedbackVerdict {Accept, Reject, Uncertain, Supersede, Correct}`, `FeedbackError`, `validate()`. `Reconciliation`, `PlaneEvidence`, `Reconciliation::compute()` pure function.
  - **fusion_bridge.rs** (PR2b): Trust-gated `recompute_status()` seam. Single source of truth for FusedClaim status derivation consumed by both `fuse_observations_with` and `FeedbackRepository::put_feedback`.
  - **Observation struct fields** (PR2b): `evidence_origin`, `confidence`, `status: ObservationStatus`, `written_via_backfill`. Reads persisted columns (was hardcoded 1.0).
  - **FeedbackRepository trait** (PR2b): `put_feedback`, `read_feedback_for_claim`, `list_reconciliations` in store.rs. m30 bridge: `pending_adjudication_event` flag + `tracing::warn!` on `ModelInference × Suggested × Feedback.accept`.
  - **v7-observation-status migration** (PR2a): `status STRING` on `(:Observation)`, `pending_adjudication_event BOOLEAN` on `(:FusedClaim)`, `(:Feedback)` / `(:Reconciliation)` tables + typed edges.
  - **UAT-06 steps 7/9 un-ignore** (PR3a): `seed_orders_stripe_fixture` impl + integration tests verifying `ModelInference` FusedClaim lands as `"drafted"`.
  - **UAT-06 steps 13/14/15 un-ignore** (PR3b): Feedback/Reconciliation integration tests + `feedback_repository_round_trip`, `v7_migration_forward_only`, `pending_adjudication_event_flag_transition`, `multi_plane_bias_ordering_determinism`.
- **Tests**: pending (5 PRs in chain)
- **Loc**: ~1410 total across 5 PRs (< 400 PR)
- **Decisiones locked**: D1 (schema canonical path: `archctl/migrations/`), D2 (FusedClaim.status rule: `trust::canonical_promotion_allowed`), D7 (`pending_adjudication_event: bool` naming), D12 (5 PR chain, < 400 LOC each).

## Cycle cerrado — `trust-006-context-bundle` (v1.85.0)

- **Fecha**: 2026-08-21
- **Cycle id**: `p-38e02210a9f14317/trust-006-context-bundle`
- **Branch**: `feat/trust-006-a-bundle-verify` → `feat/trust-006-b-agent-context` (2 chained PRs)
- **Tag**: `v1.85.0` (annotated, peels `757b946`, pushed + verified remote)
- **Path**: A-lite
- **Output**:
  - **`FeedbackSummary` struct** (PR #299, TRUST-006-a): Slim read-only view of `Feedback` for `AgentContext`. Excludes `evidence`/`correlation_id` pipeline-internal fields. `From<&Feedback> for FeedbackSummary` impl. Backed by serde round-trip tests.
  - **Bundle projection helpers** (PR #299): `seed_bundle_fixture`, `assert_no_canonical_fact_in_bundle`, `assert_has_canonical_fact_in_bundle`. Enables verifiable assertions that bundle export excludes rejected claims and includes accepted canonical facts.
  - **`AgentContext.feedback_history`** (PR #300, TRUST-006-b): Additive field `Vec<FeedbackSummary>` with `#[serde(default)]` for backward compat. Plumbs prior feedback verdicts through to re-invoked agents.
  - **`cognitive::test_support` module** (PR #300): Exposes `FeedbackAwareMockAgent` + `MockOutcome` for deterministic tests. Gated on `test-fixtures` feature.
  - **UAT-06 steps 16/17 un-ignore** (PR #300): Re-invoke agent after Reject feedback → emits `NoAction`; FeedbackSummary round-trip excludes pipeline-internal fields.
  - **UAT-06 steps 19/20 un-ignore** (PR #299): Bundle excludes rejected ModelInference claim; bundle includes accepted replacement canonical fact.
  - **Fixture fix** (PR #299): `seed_orders_stripe_fixture` was storing `source_origin` as PascalCase; `SourceOrigin::parse_label` expects snake_case. Fixed to use `SourceOrigin::ModelInference.as_str()`. Without this fix, `accept_evidence`'s trust guard silently fell back to `UserWorkspace`, which would have masked the trust-first invariant.
- **Tests**: 843/843 green. Clippy `-D warnings` 0. rustfmt 0. UAT-06: 11/11 active steps.
- **Loc**: ~+442/-26 across 14 files (< 400 PR).
- **REQ-T06-001..008**: 7/8 shipped; **REQ-T06-003 (FeedbackRepository::summaries_for_claims) deferred to TRUST-007**.
- **Decisiones locked**: snake_case `source_origin` contract (parse_label); `#[serde(default)]` on additive AgentContext field; `test_support` placement outside `agents/` module (which is `mod agents;` private).
- **Próximo candidato**: TRUST-007 — `feedback_repository_summary_port` (REQ-T06-003 closure) + UAT-06 step 18 (workbench crash recovery, currently ignored).

## Cycle cerrado — `trust-007-feedback-port` (v1.86.0)

- **Fecha**: 2026-08-21
- **Cycle id**: `p-38e02210a9f14317/trust-007-feedback-port`
- **Branch**: `feat/trust-007-wu-{1..5}` + `fix/trust-007-validate-claim-ids` + `docs/changelog-trust-007` (7 chained PRs)
- **Tag**: `v1.86.0` (annotated, peels `eded594`, pushed + verified remote)
- **Path**: A-lite
- **Output**:
  - **`FeedbackRepository::summaries_for_claims`** (PR #303): new port method on the trait. Signature `fn summaries_for_claims(&mut self, claim_ids: &[&str]) -> Result<Vec<FeedbackSummary>>`. No default impl (trait sealed in practice; only `LbugStore` implements it).
  - **`LbugStore` implementation** (PR #304): single Cypher `MATCH (f:Feedback)-[:VERDICTS_ON]->(c:FusedClaim) WHERE c.id IN $claim_ids RETURN … ORDER BY c.id ASC, f.revision ASC, f.timestamp ASC, f.id ASC`. Reuses `validate_identifier` per claim id; reuses `FeedbackVerdict::parse_label` per row. Empty-input short-circuit (`Ok(vec![])` without dispatching a query).
  - **`AgentContext::with_feedback_history`** (PR #305): ergonomic constructor in `cognitive/context.rs` that takes pre-fetched `Vec<FeedbackSummary>`. Struct-literal form `feedback_history: vec![]` remains valid (no `#[non_exhaustive]`).
  - **8-site documentation pass** (PR #306): 7 sites got `// REQ-T06-003: feedback_history plumbing — see AgentContext::with_feedback_history` comment block. The 8th site (round-trip serde test at context.rs:104) keeps `vec![]` (wire-format stability test).
  - **Regression tests** (PR #307 + #309): `archctl/tests/feedback_summaries_port.rs` with 4 tests — empty input short-circuit, deterministic ordering (out-of-order insertion → sorted output), non-requested claim exclusion, invalid-identifier surfaces Err (SCN-T07-002b).
  - **SCN-T07-002b fix** (PR #309): the `LbugStore::summaries_for_claims` validation loop used `let _ = …` which silently discarded the Err from `validate_identifier`. Replaced with `?` propagation. Mirrors `read_feedback_for_claim` (store.rs:2790-2791).
  - **`manifests/store.toml`** updated: `must_hold += ["fn summaries_for_claims"]`.
- **Tests**: 846/846 green. Clippy `-D warnings` 0. rustfmt 0. UAT-06: 11/11 active steps.
- **Loc**: ~+431/-9 across 12 files (< 400 PR per WU).
- **Decisiones locked**: snake_case `source_origin` contract preserved; `#[serde(default)]` on additive `AgentContext.feedback_history` field preserved; trait extension without default impl (sealed-in-practice); SCN-T07-002b Err propagation via `?`.
- **Próximo candidato**: TRUST-008 — `m30_bridge_promotion` (REQ-M25-006 closure; promote `pending_adjudication_event = true` + `tracing::warn!` to hard fail on `ModelInference × Feedback.reject`).

## Cycle cerrado — `trust-008-m30-bridge-promotion` (v1.87.0)

- **Fecha**: 2026-08-21
- **Cycle id**: `p-38e02210a9f14317/trust-008-m30-bridge-promotion`
- **Branch**: `feat/trust-008-wu{1..6}-*` (6 chained PRs #312-#317) + `feat/trust-008-verify-fixes` (verify findings fixes)
- **Tag**: `v1.87.0` (pending — see `release.complete` transition)
- **Path**: A-lite (skips debt-verify; goes verify → release → archive)
- **Closes**: REQ-M25-006 (deferred from TRUST-005; named in TRUST-007's `archive-manifest`).
- **Output**:
  - **`AdjudicationEvent` carrier + bounded context** (PR #312): `archctl/src/adjudication.rs` with `AdjudicationEvent` (8 fields: id, target_fused_claim_id, adjudicator, evidence_refs, decided_at, decision) + `AdjudicationDecision` enum (Promote | Reject | Defer) + `AdjudicationEventError` (now `PartialEq + Eq`).
  - **`AdjudicationRepository` port + `LbugStore` impl + v8 migration** (PR #313): trait with 3 methods (`put_adjudication`, `read_adjudications_for_claim`, `list_pending_adjudications`) + `archctl/migrations/v8_adjudication_event_store.cypher` (28 LOC; `evidence_refs STRING` JSON-encoded — deviation from spec §3.4.2 `STRING[]`, accepted by maintainer) + `backfill_adjudication_event_diagnostics` rust migration hook (HITL preserved; non-mutating; emits `tracing::warn!` for pre-v8 offenders) + 4 integration tests in `archctl/tests/adjudication_events_port.rs`.
  - **`AgentContext.pending_adjudications`** (PR #314): additive field with `#[serde(default)]` + `with_pending_adjudications` constructor + 8-site REQ-M25-006 doc pass + SCN-T08-005a serde round-trip test.
  - **m30 bridge hard fail** (PR #315): new `promotion_requires_adjudication_event(trust, verdict) -> Result<(), TrustViolation>` predicate at `archctl/src/architecture/fusion_bridge.rs:108` returns `Err(TrustViolation::ModelInferenceWithoutAdjudicationEvent)` for `ModelInference × Suggested + Accept`. `should_warn_pending_adjudication` marked `#[deprecated(since = "v1.87.0")]`. v9 migration adds `(:FusedClaim).evidence_origin STRING`. `FeedbackRepository::put_feedback` chokepoint consults the predicate + `AdjudicationRepository::read_adjudications_for_claim`. **`put_evidence` regression fix**: now reads `ev.source_origin.as_str()` instead of hardcoding `'evidence_entry_derivation'` (carried from TRUST-007 verification).
  - **CLI surface** (PR #316): `archctl adjudication { list [--pending] [--json] | decide --claim <id> --verdict promote|reject|defer --adjudicator <id> [--evidence-refs <a,b,c>] [--json] | show --claim <id> [--json] }`. 167 LOC. `--pending` restricts to events whose target FusedClaim still has `pending_adjudication_event = true` (operational view for triaging the m30 bridge backlog).
  - **Manifests + migration tests** (PR #317): `manifests/store.toml` (+7 LOC) + `manifests/trust.toml` (+5/-1 LOC, public_symbol addition + minimum_tests 10→12) + new `manifests/adjudication.toml` (44 LOC) + `archctl/tests/migrations_v8.rs` (3 tests covering SCN-T08-003a/b/c). 2 visibility bumps `pub(crate)`→`pub` on `session_for_migrations` and `apply_pending` for test access.
- **Tests**: 853+ lib + integration tests green (final tally at `phase.verify.complete.a-lite` gate). 12 `trust::tests` (added 2). 7 `adjudication_events_port.rs` tests (added 2 for SCN-T08-004a+b+d). 3 `migrations_v8.rs` tests (rewrote v7_to_v8 hook-direct form to avoid `apply_pending` + `store.init()` interaction).
- **Loc**: ~+1500/-40 across 38 files (< 400 PR per WU).
- **DQS**: passed (no critical/major debt findings at `sddk-verify`).
- **Decisiones locked**: `AdjudicationEvent.id` is content-addressable via `blake3(target + adjudicator + decided_at)`; HITL preserved (no auto-decide); v9 graph writes `evidence_origin` for every FusedClaim; pre-v9 graphs are permissive (empty `evidence_origin` skips bridge consult); `evidence_refs` column type is STRING (JSON-encoded) — deviation from spec §3.4.2 STRING[] accepted.
- **Próximo candidato**: M34 (cognitive context compression, ledger tail) or M35 (severity scoring pipeline).

## Cycle cerrado — `m25-authority-execution-classes`

- **Fecha**: 2026-08-20
- **Cycle id**: `p-38e02210a9f14317/m25-authority-execution-classes`
- **Branch**: `feat/m25-trust-enforcement` (merged via PR #287 docs + PR #288 code + PR #289 verify)
- **Verdict**: verify PASS WITH WARNINGS (final; 2/2 critical gates green, 1147 tests green, 0 clippy, 0 fmt diff, 0 doctor findings across trust/evidence/store/diagram; 0 CRITICAL + 2 WARNING + 3 SUGGESTION at debt audit; both WARNINGs justified/non-blocking)
- **Tag**: `v1.83.0` (annotated, peels `d8c4a6a`, pushed + verified remote)
- **Diff inspected**: `cbce2d3..d8c4a6a` (10 files, +1172/-27)
- **Output**:
  - **ADR-063** (PR #287): Trust, Determinism and Authority. New module `archctl/src/trust.rs` exposes `ExecutionClass × AuthorityClass` typology + `canonical_write_allowed` + `canonical_promotion_allowed`. The 4×5 matrix encodes which (producer, authority) pairs may exist as Drafted candidates; the stricter promotion gate denies all `ModelInference × _` combinations until REQ-M25-006.
  - **Option D fix** (embedded in apply): Resolved matrix-vs-chokepoint contradiction. `accept_evidence` now calls `canonical_promotion_allowed` (the promotion gate), not `canonical_write_allowed` (the existence matrix). ADR-063 invariant text updated to clarify two-stage semantics.
  - **SourceOrigin::ModelInference** (T4): New variant stamped by future model-backed producers; classified as `Suggested` by default; scoped fail-closed `from_props` per Q4 maintainer decision.
  - **Honest Evaluation attestation** (T5): `accept_evidence` records `criterion = caller=<ARCHCTL_ACTOR>` (or `cli:caller` anonymous) and `evaluator = archctl:lifecycle_v1:<invocation_path>`. Replaces hardcoded `"user_accepted"` / `"archctl:lifecycle_v1"`.
  - **UAT-06 integration test** (T6): `archctl/tests/uat_06_false_agent_claim.rs` — critical gate verifies `ModelInference` claim cannot be promoted to canonical; negative control verifies `UserWorkspace` claim can. 9 skeleton steps ignored pending TRUST-005 + spec-35.
  - **manifests/trust.toml** (T7): Scope gate for trust module. 7 public symbols, 19 textual invariants, 10 minimum tests, 4 prohibitions.
- **Tests**: 1147/1147 green (822 lib + 321 integration + 4 doctest). `cargo clippy -- -D warnings` 0. `archctl doctor --scopes trust,evidence,store,diagram` 0 findings. UAT-06: 2/2 critical active; 9 `#[ignore]`d skeletons pending TRUST-005.
- **DQS**: N/A (release gate scope; multi-lens audit owned by `sddk-debt-verify` → `debt-report.md`).
- **Decisiones locked** (de explore → proposal → spec): ADR-063 invariant ("ModelInference jamás puede escribir CanonicalObservedFact directamente"), two-stage gate semantics (matrix allows existence; promotion gate denies direct write), Q4 scoped fail-closed default.
- **Jurisprudence**: ADR-063 amends ADR-021 §Reglas (escaleza + invariant); ADR-063 clarifies ADR-023 naming collision (`Adjudicated` vs `Approval`); Option D resolves architectural contradiction between architecture/12-…:33-38 (matrix green cell) and spec.md REQ-M25-002 (promotion denied).
- **Pre-existing issue resolved in-session**: UAT-06 critical gate test had a sanity-check assertion (line 161-164) that expected `ev:ws:orders-stripe` to exist. The fix-forward (T6 amend commit `fd3b769`) replaced it with `accepted_ids.is_empty()` — verifying the chokepoint actually denied the LLM claim.
- **Known deferred items**: REQ-M25-005 #17 programmatic API caller_id (`accept_evidence` lacks `caller_id` parameter); Path B direct-Cypher bypass for `link_semantic_edge`; 9 `#[ignore]`d UAT-06 skeletons; `thread_local CURRENT_INVOCATION_PATH` implicit cross-module coupling (justified by ADR-063 §Decisión §4).
- **Próximo candidato**: M26 (FusedClaim persistence, TRUST-005) or M30 (Adjudication event store, REQ-M25-006).
- **Próxima fase**: `sddk-archive` (per orchestrator release-before-archive sequence, ADR-0011).

## Cycle cerrado — `no-stubs-mocks-placeholders-hardcoded`

- **Fecha**: 2026-08-22
- **Branch**: `main` (linear, 17 commits directos + 3 follow-ups)
- **Trigger**: regla explícita del usuario "no se permitir codigo stub, mock, y placeholder ni harcoded, todo el codigo realizado debe ser productivo 100%". Registrada en AGENTS.md.
- **Path**: meta-ciclo (no es domain work; es auditoría + remediación del code base existente).
- **Output**:
  - **AGENTS.md regla** (commit `907ccd8`, line 463): bloquea stubs/mocks/placeholders/hardcoded en verify. Incluye grep commands para auditar.
  - **15 ciclos P1 de remediación** (commits `c8c47cb`…`b5ec458`): cada `Mock*`/`Fake*`/`*Adapter` shadow en `archctl/src/{cognitive,ide,code,doctor,diagram,observation_claim,architecture/*}` reemplazado por fixture real (LbugStore-backed o TraitImpl legítimo).
  - **Audit closure** (commit `b6cbb62`): los 3 hits legítimos restantes (doctor defaults detrás de env vars, view.rs loopback per ADR-011, xdg/environment test fixtures) documentados.
  - **Test fixture repair** (commit `b6b78ce`): `archctl/tests/diagram_validate.rs:95` escribía `.png` mientras el validador esperaba `.svg` (regresión silenciosa desde cycle 2). 2 tests fixed; integration suite ahora 100% green.
  - **DoD + Validation Matrix** (commit `748850f`): exige `cargo test --features test-fixtures --tests` (antes solo lib suite). Suite completa = 1204 tests, 0 failed.
  - **CHANGELOG entry** (commit `676234e`): ciclo registrado en `[Unreleased]` para trazabilidad.
  - **Decisión lbug** (commit `676234e` + STATE.md): bump 0.18.3 → 0.19.1 diferido; workaround documentado es la opción elegida.
- **Tests**: 1204/1204 green (859 lib + 345 integration + doctest). `cargo clippy -- -D warnings` 0. `cargo fmt --check` 0. `archctl doctor --scopes architecture,diagram,code,cognitive,ide,store,evaluation,evidence,feedback` 0 findings.
- **Próximo candidato**: M34 (cognitive context compression, ledger tail) o M35 (severity scoring pipeline), los dos post-TRUST-008 según ROADMAP:1836.
