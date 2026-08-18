# ADRs — `archctl`

> Esta especificación **reemplaza** los 7 ADRs planos que vivían
> anteriormente en este fichero. La consolidación quedó documentada en
> [ADR-000](ADR-000-reinicio-de-alcance.md). Cada ADR vive ahora en su
> propio fichero bajo `docs/adr/` para mantener la trazabilidad de cada
> decisión por separado.

## Índice

| ID | Título | Estado |
|---|---|---|
| [ADR-000](ADR-000-reinicio-de-alcance.md) | Reinicio de alcance | Aceptado |
| [ADR-001](ADR-001-opencode-first-archctl-sidecar.md) | OpenCode primero; `archctl` como sidecar | Aceptado (reforzado por ADR-013) |
| [ADR-002](ADR-002-topologia-de-agentes.md) | Topología mínima de agentes | Aceptado |
| [ADR-003](ADR-003-reutilizacion-y-adaptacion-de-skills.md) | Reutilización y adaptación de skills | Aceptado |
| [ADR-004](ADR-004-persistencia-externa-xdg.md) | Persistencia externa XDG por proyecto y worktree | Aceptado |
| [ADR-005](ADR-005-ladybugdb-grafo-canonico-y-evidencias.md) | LadybugDB como grafo canónico y evidencias | Aceptado |
| [ADR-006](ADR-006-adaptadores-de-herramientas-cli.md) | ~~Adaptadores de herramientas CLI existentes~~ | **DEPRECADO** (sustituido por ADR-012 + ADR-013) |
| [ADR-007](ADR-007-modelos-y-renderizadores-de-diagramas.md) | Diagramas como proyecciones del grafo + split render estático/interactivo | Aceptado (sustituido en sección render por ADR-013; revisado 2026-07-31 con pivot a workbench) |
| [ADR-008](ADR-008-recuperacion-versionado-y-evolucion.md) | Recuperación, versionado y evolución | Aceptado |
| [ADR-009](ADR-009-relaciones-semanticas-reificadas.md) | Relaciones semánticas reificadas y aristas derivadas | Aceptado |
| [ADR-010](ADR-010-concurrencia-ladybugdb.md) | Concurrencia de LadybugDB y procesos `archctl` (DB lock via `fs2::try_lock_exclusive`) | Aceptado (reforzado por ADR-013) |
| [ADR-011](ADR-011-renderers-locales-y-bloqueo-de-publicos.md) | Renderers locales y bloqueo de servicios públicos (alcance = `archctl` solamente) | Aceptado (alcance reducido por ADR-013; revisado 2026-07-31 con nota de performance para `archview`) |
| [ADR-012](ADR-012-adopcion-incremental-crates-analisis.md) | Política "descartar CLIs" + ciclo M5–M8 + renderers como librerías | Aceptado (complementado por ADR-013) |
| [ADR-013](ADR-013-viewer-ortogonal.md) | Viewer ortogonal basado en DiagramProjection (Code Knowledge Graph Workbench; performance-first) | **SUPERSEDED por ADR-038** (sección "repositorio separado" contradicha por ADR-033 + código) |
| [ADR-014](ADR-014-puerto-persistencia-sparrowdb.md) | Puerto de persistencia hexagonal + SparrowDB como adapter alternativo (Ola 1 implementada, Ola 2 pendiente) | Aceptado |
| [ADR-017](ADR-017-schema-migration-runner.md) | Migration runner + SourceArtifact identity (B1: schema migration runner, hash scheme split, source_origin en props) | Aceptado |
| [ADR-019](ADR-019-performance-budget.md) | Performance budget (hard contract) — TTFP, FPS, latency, memory targets + anti-patterns explícitos | Aceptado (nuevo 2026-07-31) |
| [ADR-020](ADR-020-renderer-stack.md) | Renderer stack: G6 5.x WebGPU + cosmos.gl + SolidJS + Rust/WASM (sustituye Sprotty + Cytoscape.js) | Aceptado (nuevo 2026-07-31) |
| [ADR-021](ADR-021-cognitive-layer.md) | Cognitive Layer (Agentic Intelligence) — 7-layer architecture, contract uniforme, escalation ladder, MCP boundary | Aceptado (nuevo 2026-07-31) |
| [ADR-022](ADR-022-agent-catalog.md) | Agent catalog — 9 agentes especializados (Semantic Curator, Architecture, Projection, Investigation, Impact, Planning, Documentation, Presenter, Review/Critic) | Aceptado (nuevo 2026-07-31) |
| [ADR-023](ADR-023-action-proposal-and-policy.md) | Action Proposal & Policy Engine — ActionProposal estructurado, Policy Engine TOML, MCP gateway, audit log inmutable | Aceptado (nuevo 2026-07-31) |
| [ADR-024](ADR-024-element-category-semantics.md) | Element.category = diagram family (`c4`/`code`/`uml`); Element.kind_id = projection kind (`mt.container`, etc.) — fix export query pipeline (M26) | Aceptado (nuevo 2026-08-05) |
| [ADR-025](ADR-025-ci-postmerge-toolchain-fijada.md) | CI post-merge + toolchain fijada + verificación local — disparador `push: [main]`, 1.97.1 pinned, MSRV 1.91, `bench-compare` contra `github.event.before`, pre-push local, protección de `main` con cero checks remotos | Aceptado (nuevo 2026-08-03) |
| [ADR-030](ADR-030-workflowctl-local-multi-repo.md) | Ejecutor local manual de GitHub workflows (multi-repo) — `workflowctl` en topología local-first, runner remoto e híbrido quedan como mejoras futuras no implementadas | Aceptado (nuevo 2026-08-03) |
| [ADR-031](ADR-031-c4-vertical-validation.md) | C4 vertical end-to-end validation: 6 bugs discovered por smoke testing con proyectos reales (axum) — apply path, Cypher quoting, write_evidence silent fail, version_id collision, status casing, bundle schema mismatch | Aceptado (nuevo 2026-08-05) |
| [ADR-026](ADR-026-state-machine-metamodel.md) | State machine metamodelo + extracción AST-pura — metatypes (`uml.state_machine`, `uml.state`, `uml.transition`, `uml.guard`, `uml.event`), predicates (`behavior.source_state`, `behavior.target_state`, `behavior.has_transition`), patrón MERGE apply-time, confidence < 1.0 para heurística | Aceptado (propuesto 2026-08-04) |
| [ADR-027](ADR-027-evidence-put.md) | Evidence put: identity scheme para hechos sin archivo + separation of concerns — `evidence_id = ev:sem:blake3(kind+claim+source_origin+props)`, `source_origin: UserInput`, `status: drafted`, solo Evidence+SourceArtifact (no Elements) | Aceptado (propuesto 2026-08-04) |
| [ADR-028](ADR-028-diagram-project.md) | Diagram project: ProjectSelector vs C4Kind + multi-format DSL projection — `ViewKind` enum (C4+UML+behavior), emitters PlantUML/Mermaid/Structurizr, grammar `<kind>:<scope>`, fuente DSL editable, relación con export (viewer-bundle) y render (SVG) | Aceptado (propuesto 2026-08-04) |
| [ADR-029](ADR-029-c4-component-light.md) | C4 component light — estrategia `components` en `c4-discover`: módulos internos → candidatos `mt.component` con confidence < 1.0, revisión y promoción por el agente (misma filosofía que ADR-026) | Aceptado (propuesto 2026-08-04) |
| [ADR-016](ADR-016-activegraph-packs-investigacion.md) | Investigación de `activegraph-packs` + 3 bloques de mejoras para `archctl` (B1 evidence graph, B2 manifest+gates, B3 trust-by-origin) | Investigación cerrada — B1 Decidido via [ADR-017 §Schema migration runner](ADR-017-schema-migration-runner.md); B2/B3 Pendiente con Reopen triggers (2026-08-18) |
| [ADR-032](ADR-032-bench-methodology.md) | Bench methodology — métricas, thresholds del release gate v1.0, FP/FN manual rubric, conteo solo mt.container (M28) | Aceptado (nuevo 2026-08-05) |
| [ADR-033](ADR-033-archctl-view-embedded-workbench.md) | `archctl view`: workbench embebido como servicio local one-shot — rust-embed + tiny_http, 127.0.0.1, COOP/COEP, stack distribuido como UN producto | Aceptado (nuevo 2026-08-06) |
| [ADR-034](ADR-034-e2e-coverage-expansion.md) | E2E coverage expansion: install + deploy + render + multi-language — 4 suites versionadas (install_e2e, render_e2e, smoke ampliado, sandbox-e2e) | **Propuesto** (2026-08-06) |
| [ADR-035](ADR-035-go-call-graph-extraction.md) | Go call-graph extraction — tree-sitter-go para functions y methods | Aceptado (2026-08-06) |
| [ADR-036](ADR-036-apply-writer-performance.md) | Apply writer performance: transaction + bulk import — D1/D2/D3 | Aceptado (2026-08-06) |
| [ADR-037](ADR-037-call-graph-language-strategy-consolidation.md) | Call-graph language strategy consolidation — 8 extractores refactorizados | Aceptado (2026-08-07) |
| [ADR-038](ADR-038-one-product-five-invariants.md) | Un producto, cinco invariantes (arch-stack identity) — supersedes ADR-013 "repositorio separado" | Aceptado (2026-08-09) |
| [ADR-039](ADR-039-renderer-reality-anti-roadmap.md) | Renderer reality + anti-roadmap — G6 canvas, no WASM/WebGPU/Arrow; deferred decisions con reopen triggers | Aceptado (2026-08-09) |
| [ADR-040](ADR-040-cognitive-conditional-activation.md) | Cognitive layer conditional activation — ADR-021/022/023 marcados conditional/parcial/diferido | Aceptado (2026-08-09) |
| [ADR-041](ADR-041-workspace-state-persistence.md) | Workspace state persistence — durable workspace state for `archctl view` | Aceptado (2026-08-10) |
| [ADR-042](ADR-042-ide-adapter-abstraction.md) | IDE adapter abstraction — multi-IDE plugin target | **Aceptado — 2026-08-11** (H4 closed en v1.35.0, M75; PR #151) |
| [ADR-043](ADR-043-modular-hexagonal-boundaries.md) | Límites hexagonales modulares por capacidad — port seams en el grafo canónico | Aceptado — 2026-08-13 (embodied en CliContext v1.43.0 p1-01 + repositories v1.43.0 p1-03) |
| [ADR-044](ADR-044-persistence-ports-and-raw-query-boundary.md) | Repositorios semánticos y frontera de query raw — `GraphStore::query` reemplazado por puertos tipados | **Aceptado — 2026-08-13** (foundational; **Superseded by**: ADR-045 §Puertos; ADR-059 RawGraphQuery split) |
| [ADR-045](ADR-045-capability-registry-single-source-of-truth.md) | Capability Registry como fuente única de verdad (P1-08) | Aceptado (2026-08-16) |
| [ADR-046](ADR-046-plugin-supply-chain-and-capability-security.md) | Seguridad de plugins y supply chain | **Aceptado (parcial) — 2026-08-13** (shipped: plugin tap M76 PR #152; deferred: capability gating per origin + signed plugins) |
| [ADR-047](ADR-047-pre-merge-ci-and-post-merge-quality-gates.md) | CI preventiva en PR y evidencia post-merge | Aceptado — 2026-08-13 (embodied en pr.yml + release.yml + verify-local.sh) |
| [ADR-048](ADR-048-ladybugdb-native-compatibility-boundary.md) | LadybugDB como adapter nativo con matriz de compatibilidad (`archctl doctor --scope storage` con 5-axis envelope) | Aceptado — 2026-08-13 (shipped en v1.42.0, PR #174) |
| [ADR-049](ADR-049-evidence-observation-claim-confidence-model.md) | Separar Observation, Evidence y Claim — además del modelo Evidence existente | **Aceptado (parcial) — 2026-08-13** (shipped: P2-09a compat carriers v1.58.0; deferred: full schema migration + backfill + dual-write) |
| [ADR-050](ADR-050-architecture-snapshots-and-git-identity.md) | Snapshots arquitectónicos ligados a identidad Git (`RepositoryIdentity` + `extractor_set_digest`) | Aceptado — 2026-08-13 (shipped en v1.49.0 + v1.50.0, PRs #194 + #196) |
| [ADR-051](ADR-051-loopback-workbench-session-security.md) | Token de sesión efímero para acciones del workbench | **Deferido — 2026-08-18** (Reopen: disclosed loopback-session hijack vector OR per-session permission scoping requirement) |
| [ADR-052](ADR-052-architecture-context-compiler.md) | Task Context Compiler para agentes — `archctl architecture context --task <text>` | Aceptado — 2026-08-13 (shipped en v1.57.0, P2-08) |
| [ADR-053](ADR-053-architecture-diff-as-first-class-capability.md) | Architecture Diff como capability first-class — `archctl architecture diff <id_a> <id_b>` | Aceptado — 2026-08-13 (shipped en v1.51.0, P2-02) |
| [ADR-054](ADR-054-architecture-policy-and-fitness-functions.md) | Políticas y fitness functions sobre el grafo canónico — 6 reglas (ADR-054) + SARIF/JUnit projectors | Aceptado — 2026-08-13 (shipped en v1.54.0 + v1.55.0, P2-05 + P2-06) |
| [ADR-055](ADR-055-sanitized-architecture-bundle.md) | Sanitized Architecture Bundle compartible | **Deferido — 2026-08-18** (Reopen: ADR-019 perf budget breach AND ≥1 external-distribution consumer) |
| [ADR-056](ADR-056-moldable-architecture-workbench.md) | Moldable Architecture Workbench y navegación semántica (LensSpec) — canonical anchor de ROADMAP §H3 entry criteria | **Deferido — 2026-08-18** (Reopen: ≥2 LensSpec-translatable consumers OR measured need; entry criteria en ROADMAP §H3) |
| [ADR-057](ADR-057-archctl-versioned-distribution.md) | `archctl` como CLI versionado distribuible (asdf-inspired) | **Aceptado — 2026-08-11** (H4 closed en v1.36.0, M73+M76; PR #149 + PR #152) |
| [ADR-058](ADR-058-self-update-github-releases.md) | Self-update via GitHub Releases (binarios pre-compilados) | **Aceptado — 2026-08-11** (H4 closed en v1.36.0, M76; PR #152) |
| [ADR-059](ADR-059-rawgraphquery-trait-split.md) | RawGraphQuery trait split + SemanticEdgeRepository boundary (P1-04) | Aceptado (2026-08-15) |
| [ADR-060](ADR-060-architecture-cli-snapshot-surface-deviation.md) | `architecture` CLI surface: drop the `snapshot` intermediate (Path B deviation) | **Aceptado — 2026-08-17** (H4 closed en v1.50.0, p2-02-followup; PR #196, commit `8e6c434`) |

## Cómo se relacionan

```
ADR-001 (sidecar) ─┐
                    ├─► ADR-013 (viewer ortogonal / workbench)
ADR-010 (no daemon)─┘

ADR-006 (CLI adapters) ──► DEPRECADO ─► ADR-012 (librerías + renderers)

ADR-005 (LadybugDB) ──► ADR-007 (proyecciones) ──► ADR-013 (bundle contract)
                                                   │
                                                   ├─► ADR-019 (performance budget)
                                                   ├─► ADR-020 (renderer stack) ──► ADR-039 (renderer reality / anti-roadmap)
                                                   └─► ADR-021 (cognitive layer) ──► ADR-040 (conditional activation)
                                                         ├─► ADR-022 (agent catalog: 9 agentes) ──► ADR-040
                                                         └─► ADR-023 (action proposal + policy) ──► ADR-040

ADR-011 (renderers locales) ──► ADR-013 (mismo principio en archview por construcción)
                              └─► ADR-019/020 (nota de performance para archview)

ADR-013 (viewer) ──► ADR-038 (arch-stack identity: un producto, cinco invariantes)

ADR-010 (sin daemon) ──► ADR-030 (workflowctl: local-first, sin runner remoto en MVP)
ADR-011 (todo local) ┘
                    └──► ADR-025 (CI post-merge: detección remota + prevención local)
```

## Documentos relacionados

- [`docs/README.md`](../README.md) — resumen ejecutivo.
- [`docs/Skills-para-agentes-IA-v2.md`](../Skills-para-agentes-IA-v2.md) — propuesta base revisada.
- [`docs/DATA-MODEL-LADYBUGDB.md`](../DATA-MODEL-LADYBUGDB.md) — modelo de grafo.
- [`docs/ROADMAP.md`](../ROADMAP.md) — milestones M0–M16 + proyecto `archview`.
- [`docs/schema/`](../schema/) — `001_initial_schema.cypher`, `metamodel-core.json`.

## Cómo añadir un nuevo ADR

1. Crear `docs/adr/ADR-NNN-slug.md` siguiendo el formato de los existentes.
2. Añadir la fila correspondiente a este índice.
3. Si la decisión **sustituye** a una anterior:
   - Indicar `**Sustituye:** ADR-XXX anterior` en la cabecera.
   - Marcar el ADR anterior como **DEPRECADO** (no borrarlo; preserva historia).
   - Actualizar el README.md de ADRs y `docs/manifest.json`.
4. Si la decisión **complementa** una existente (no la sustituye):
   - Indicar `**Complementa:** ADR-XXX` en la cabecera.
   - Actualizar el README.md de ADRs y `docs/manifest.json`.
5. Commit dedicado (`docs(adr): ADR-NNN ...`).

## ADRs deprecados

`docs/adr/ADR-006-adaptadores-de-herramientas-cli.md` está **DEPRECADO**. La política vigente vive en ADR-012. El texto original se conserva como registro histórico; no debe usarse para tomar decisiones.
