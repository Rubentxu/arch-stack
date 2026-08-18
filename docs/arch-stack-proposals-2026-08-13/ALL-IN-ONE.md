# Arch Stack Consolidation Pack — ALL IN ONE

Baseline auditado: `main@518bb79d4c87a491fc901d54441de15e72c40bc2`  
Generado: 2026-08-13

> Concatenado para estudio. Los ficheros individuales son la unidad recomendada para Git.


---

<!-- SOURCE: 00-EXECUTIVE-SUMMARY.md -->

# Executive Summary — de Diagrammer a Architecture Intelligence

**Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`  
**Fecha:** 2026-08-13

## Diagnóstico

La base del producto es sólida: persistencia local, grafo canónico, evidencia,
extractores deterministas, proyecciones, `archview`, adaptadores de IDE y capa
cognitiva. El principal riesgo ya no es la falta de features, sino la **erosión de
límites** provocada por el rápido crecimiento.

Indicadores observados:

- `archctl/src/cli.rs` ≈ 99 KB;
- `archctl/src/store.rs` ≈ 97 KB;
- `archctl/src/code/call_graph.rs` ≈ 85 KB;
- `archctl/src/code/class_diagram.rs` ≈ 60 KB;
- `archctl/src/code/state_machine.rs` ≈ 47 KB;
- existe `cognitive/` con context, delta, MCP, policy y agents;
- `archview` ya dispone de C4, Call Graph, Class, Sequence, Drift, Impact y Package;
- existen manifests y quality gates propios;
- los ADR 040 y 041 están duplicados;
- CI/release estaban bloqueados en la auditoría por el acoplamiento nativo de LadybugDB.

## Goal propuesto

> Arch Stack es un motor local-first de Architecture Intelligence que transforma
> evidencia verificable del software en un grafo arquitectónico canónico, y lo
> proyecta en representaciones adecuadas para comprender, validar, comparar y
> modificar software con ayuda de humanos y agentes.

## Cuatro outcomes

### Trust
Toda afirmación arquitectónica importante puede responder: qué la originó, qué
extractor/agente la produjo, qué fichero/línea/commit la soporta, qué confianza
tiene y si existen observaciones contradictorias.

### Change intelligence

```bash
archctl architecture diff main..HEAD
```

explica **qué arquitectura cambia**, no únicamente qué archivos cambian.

### Agent context

```bash
archctl context compile   --task "añadir cache distribuida a checkout"   --budget-tokens 12000
```

construye contexto arquitectónico relevante, trazable y acotado.

### Moldable exploration

`archview` evoluciona de viewer a **Architecture Workbench**: una pregunta o selección
determina el lens/proyección adecuada y permite navegar System → Container →
Component → Module/Class → Function → Source y volver.

## Prioridad

```text
P0 — Stabilize truth
  build/release, Ladybug boundary, plugins, ADR integrity, licenses, PR CI

P1 — Enforce architecture
  modular hexagonal boundaries, repositories/ports, capability registry,
  contract tests, fitness gates

P2 — Deliver intelligence
  diff, explain, confidence/coverage, policies, context compiler, evidence fusion

P3 — Compound utility
  snapshots, sanitized bundles, moldable workbench, plugin trust/capabilities
```

## Regla estratégica

No ampliar horizontalmente el roadmap hasta completar P0 y la mayor parte de P1.
Una nueva notación de diagrama solo entra si demuestra un resultado que no pueda
resolverse como proyección/adaptador de capacidades actuales.

## Definition of Done del programa

1. `main` y release matrix verdes en targets soportados.
2. Límites de módulos/crates verificados automáticamente.
3. Application/domain no dependen de LadybugDB, HTTP, filesystem real ni GitHub.
4. ADR con identidad única exigida por CI.
5. Capacidades/lenguajes en un único registro.
6. `architecture diff`, `explain` y `context compile` con salidas estables.
7. Políticas arquitectónicas bloquean regresiones mediante formatos estándar.
8. `archview` consume esos resultados sin duplicar semántica.


---

<!-- SOURCE: 01-ROADMAP-CONSOLIDATION.md -->

# Roadmap de consolidación — H5 a H8

Este roadmap es un **delta propuesto** sobre el roadmap existente. Mantiene los
horizontes históricos y añade outcomes centrados en fiabilidad, arquitectura y utilidad.

## H5 — Stabilization & Trust Boundary

**Objetivo:** producto compilable, distribuible y seguro antes de crecer.

### P0.1 LadybugDB compatibility boundary
- encapsular crate/native/ABI/toolchain;
- `doctor --scope storage`;
- pin de artefacto nativo coherente;
- build de cada plataforma en runner compatible;
- matriz de compatibilidad explícita.

### P0.2 Plugin hardening
- corregir namespace XDG;
- crear install root antes de staging;
- value objects `PluginAuthor/PluginName/PluginVersion`;
- rechazo de path traversal;
- checksum obligatorio para remoto;
- unpack seguro;
- trust metadata.

### P0.3 Documentation integrity
- resolver IDs ADR duplicados sin romper enlaces;
- `check-adr-integrity`;
- licencia raíz coherente con Cargo/README;
- referencias ADR verificables.

### P0.4 Pre-merge CI
PR gate rápido y determinista; post-merge conserva benchmarks costosos.

**Exit H5:** release reproducible + security/integrity gates verdes.

---

## H6 — Enforced Hexagonal Architecture

**Objetivo:** transformar la hexagonal conceptual en propiedad del build.

- composition root explícito;
- módulos por bounded capability;
- store → repositories/ports;
- extractors por lenguaje;
- Capability Registry;
- contract tests;
- Architecture Fitness Gate.

Boundaries objetivo:

```text
arch-model
arch-analysis
arch-knowledge
arch-projection
arch-workbench
arch-distribution
archctl
archview-contract
```

**Exit H6:** imports prohibidos fallan antes de merge y handlers CLI no construyen
infraestructura fuera del composition root.

---

## H7 — Explainable Architecture Intelligence

**Objetivo:** que el grafo sea herramienta de razonamiento verificable.

- Architecture Diff reutilizando `cognitive/delta.rs` y Drift/Impact;
- Explain/provenance;
- Confidence & Coverage;
- Intent vs Reality / Fitness;
- Task Context Compiler extendiendo `cognitive/context.rs`;
- Evidence Fusion.

**Exit H7:** un PR obtiene diff + policy report + context package con provenance sin
servicio externo.

---

## H8 — Moldable Architecture Workbench

**Objetivo:** representación adaptable a pregunta, selección, escala y tarea.

- Git-linked snapshots;
- sanitized `.archbundle`;
- semantic zoom;
- action palette;
- moldable lenses;
- plugin trust/capabilities.

**Exit H8:** desde un nodo o pregunta se navega y explica arquitectura sin saber de
antemano qué tipo de diagrama hace falta.

## Regla de priorización

Una feature futura puntúa por:
1. trust,
2. reducción de coste cognitivo,
3. change safety,
4. agent context,
5. reutilización del grafo/evidence.

Si solo añade una nueva forma de dibujar lo ya conocido, queda detrás de H5–H8.


---

<!-- SOURCE: 02-TARGET-ARCHITECTURE.md -->

# Arquitectura objetivo — Modular Hexagonal Architecture Intelligence

## Vista conceptual

```text
                       HUMAN / AGENT
                            │
                            ▼
                 ┌─────────────────────┐
                 │ EXPERIENCE ADAPTERS │
                 │ CLI / MCP / archview│
                 └──────────┬──────────┘
                            │
                            ▼
                 ┌─────────────────────┐
                 │ APPLICATION USECASES│
                 └──────────┬──────────┘
                            │
       ┌────────────────────┼────────────────────┐
       ▼                    ▼                    ▼
   ANALYSIS             KNOWLEDGE           PROJECTION
       │                    │                    │
       └────────────────────┼────────────────────┘
                            ▼
                 ┌─────────────────────┐
                 │ ARCHITECTURE MODEL  │
                 │ Element / Relation  │
                 │ Observation / Claim │
                 │ Evidence / Confidence
                 └──────────┬──────────┘
                            │ PORTS
          ┌─────────┬───────┼────────┬─────────┐
          ▼         ▼       ▼        ▼         ▼
       Ladybug   Filesys    Git    Renderer   IDE/HTTP
       adapter   adapter  adapter   adapters   adapters
```

## Regla de dependencia

> Nada en `domain` o `application` puede importar tipos propios de LadybugDB,
> `tiny_http`, `reqwest`, G6, GitHub Releases o filesystem real.

Los adapters dependen hacia dentro. El composition root conecta implementaciones.

## Bounded capabilities

### `arch-model`
Tipos semánticos, value objects, IDs, relaciones, evidence/observation/claim,
confidence y errores de dominio. Sin I/O.

### `arch-analysis`
Orquesta extracción. Define interfaces de analyzer/extractor. Tree-sitter/ast-grep
son adapters de análisis, no el modelo.

### `arch-knowledge`
Casos de uso de ingestión/consulta y ports de repositorio. No contiene Cypher.

### `arch-projection`
Construye projections/bundles. Mermaid/PlantUML/Structurizr/Arrows son outputs,
no semántica central.

### `arch-workbench`
API local, session security, source preview, editor handoff. HTTP concreto es adapter.

### `arch-distribution`
Lifecycle, releases, plugins, IDE installation. Red y filesystem son adapters.

### `archctl`
CLI + composition root + formateo humano/JSON.

### `archview-contract`
Schemas/DTO compartidos/generados para evitar drift Rust/TypeScript.

## Migración sin big bang

No crear ocho crates de golpe. Regla **boundary-before-extraction**:

1. declarar ports y packages internos;
2. mover lógica sin cambiar comportamiento;
3. añadir dependency tests;
4. solo extraer crate cuando haya estabilidad o beneficio medido.

## Composition root

Debe ser el lugar donde se construyan implementaciones concretas:

```text
SystemFilesystem
LadybugRepositories
SystemGit
SystemClock
HttpClient
ReleaseProvider
EditorLauncher
```

CLI parsing y `println!` permanecen en adapter CLI.

## Query boundary

Cypher libre sigue siendo útil para:

```text
archctl graph query "<cypher>"
```

pero queda en `RawGraphQuery`, no como dependencia de los casos de uso.

Ports sugeridos:
- `ArchitectureRepository`
- `EvidenceRepository`
- `ObservationRepository`
- `ProjectionRepository`
- `SnapshotRepository`
- `GraphReadModel`
- `UnitOfWork`

## Read/write separation ligera

No CQRS completo. Sí separar comandos de ingest/apply de read models de traversal,
impact, explain, diff y projection.

## Self-dogfood

Arch Stack debe verificar su arquitectura con sus propias fitness functions:

```text
domain !-> reqwest
domain !-> lbug
application !-> tiny_http
application !-> std::process
projection !-> cli
analysis !-> view
archview !-> canonical semantic mutation
```


---

<!-- SOURCE: 03-IMPLEMENTATION-BACKLOG.md -->

# Implementation Backlog

Escala: **P0** bloqueo/reliability/security; **P1** deuda arquitectónica;
**P2** intelligence; **P3** experiencia/plataforma.  
Estimación XS/S/M/L/XL = tamaño relativo, no tiempo calendario.

## P0

| ID | Trabajo | Tamaño | DoD |
|---|---|---:|---|
| P0-01 | Resolver build Ladybug/native C++ | M | build/test Tier 1 verde |
| P0-02 | Matriz ABI/native explícita | M | doctor detecta mismatch |
| P0-03 | Runners release por OS | S | macOS construido en macOS |
| P0-04 | Corregir plugin XDG root | XS | test exact path |
| P0-05 | Crear plugin root antes de staging | XS | first-install E2E |
| P0-06 | Sanitizar plugin identifiers | S | traversal/property tests |
| P0-07 | Checksum remoto obligatorio | S | remote sin hash rechazado |
| P0-08 | Unpack seguro tar | S | tar traversal test |
| P0-09 | Resolver ADR-040/041 duplicados | S | IDs únicos |
| P0-10 | Gate de integridad ADR | S | PR falla por duplicate/broken link |
| P0-11 | Coherencia de licencia | XS | metadata y files coherentes |
| P0-12 | PR CI | S | fast gate en pull_request |
| P0-13 | Actualizar capability docs obsoletos | S | docs exactos |
| P0-14 | Filesystem contract tests | S | adapters pasan misma suite |

## P1

| ID | Trabajo | Tamaño | DoD |
|---|---|---:|---|
| P1-01 | Composition root | M | handlers no crean infra |
| P1-02 | CLI commands → handlers/usecases | M | cli parsing-only |
| P1-03 | Architecture repositories | L | usecases sin Cypher |
| P1-04 | RawGraphQuery boundary | S | raw solo admin |
| P1-05 | UnitOfWork | M | mutations atómicas |
| P1-06 | Extractor strategy por lenguaje | L | carriers comunes |
| P1-07 | Extractor contract suite | M | deterministic/idempotent |
| P1-08 | Capability Registry | M | CLI/docs/MCP derivan del registry |
| P1-09 | Dependency fitness rules | M | CI bloquea imports |
| P1-10 | `arch-model` boundary | M | pure semantic model |
| P1-11 | `archview-contract` alignment | M | Rust/schema/TS gate |

## P2

| ID | Trabajo | Tamaño | DoD |
|---|---|---:|---|
| P2-01 | Snapshot read model MVP | M | estado ligado a SHA |
| P2-02 | Architecture Diff | L | stable JSON + CLI |
| P2-03 | Explain/Provenance | M | relation→evidence chain |
| P2-04 | Confidence/Coverage | L | unknown/weak/conflict visible |
| P2-05 | Policy metamodel | M | declarative rules |
| P2-06 | Fitness evaluator | L | JSON/SARIF/JUnit |
| P2-07 | Context relevance engine | L | deterministic shortlist |
| P2-08 | Task Context Compiler | L | budgeted bundle |
| P2-09 | Observation/Claim migration | XL | evidence fusion |
| P2-10 | Intent vs Reality | M | desired vs observed |

## P3

| ID | Trabajo | Tamaño | DoD |
|---|---|---:|---|
| P3-01 | Snapshot history UX | M | temporal navigation |
| P3-02 | Sanitized `.archbundle` | L | no source/secrets default |
| P3-03 | Workbench session token | S | side effects protected |
| P3-04 | Semantic zoom model | L | cross-view stable IDs |
| P3-05 | Moldable lens selection | XL | query→projection composition |
| P3-06 | Node action palette | M | explain/evidence/impact/etc. |
| P3-07 | Plugin capability manifest | M | declared permissions |
| P3-08 | Plugin trust UX | M | trust states visible |

## Slicing

Nunca ejecutar P1-02 como “reescribir CLI”. Extraer familia por familia:
`doctor` → `diagram` → `code` → `view` → `plugin` → `self` → `ide`.
Cada slice compila, testea y se revierte aisladamente.


---

<!-- SOURCE: 04-MIGRATION-STRATEGY.md -->

# Estrategia de migración

## Principio

**Strangler refactor interno**, no reescritura. Cada paso conserva CLI/schema salvo
que una spec/ADR declare un cambio.

## A — Freeze de fronteras
- inventariar dependencias;
- golden outputs de CLI;
- baseline de imports y tamaños;
- no mover código aún.

## B — Composition root

```rust
struct Runtime {
    fs: Arc<dyn Filesystem>,
    architecture: Arc<dyn ArchitectureRepository>,
    evidence: Arc<dyn EvidenceRepository>,
    graph_query: Arc<dyn RawGraphQuery>,
    git: Arc<dyn GitRepository>,
    clock: Arc<dyn Clock>,
}
```

El nombre no es contrato; la propiedad importante es un único borde de construcción.

## C — Use cases

```text
clap DTO
  ↓
CLI adapter mapping
  ↓
UseCase::execute(Input)
  ↓
Output DTO
  ↓
human/json formatter
```

## D — Repositories
Introducir ports semánticos delante del store actual. Inicialmente el adapter puede
delegar en `GraphStore`; después se eliminan queries Cypher de usecases.

## E — Module boundaries
Crear `model`, `analysis`, `knowledge`, `projection`, `workbench`, `distribution`.
Añadir dependency tests.

## F — Optional crate extraction
Extraer crate solo cuando:
- boundary estable al menos un ciclo;
- sin ciclos;
- ownership/compile isolation aporta valor;
- o existen ≥2 consumidores.

## Compatibilidad
- mantener comandos;
- mantener JSON schemas;
- versionar contratos cuando realmente cambien;
- aliases deprecated solo un ciclo si son imprescindibles;
- no duplicar almacenamiento.

## Rollback
Cada PR estructural:
1. no mezcla feature nueva;
2. tiene equivalence/golden tests;
3. revierte sin migración de datos salvo schema PR;
4. no borra raw query mientras exista consumidor no migrado.

## Métricas
- imports prohibidos;
- Cypher fuera del adapter;
- `std::fs` fuera de adapters;
- `Command::new` fuera de adapters;
- handlers CLI con negocio;
- archivos >30 KB;
- usecases testeados sin I/O real.


---

<!-- SOURCE: 05-QUALITY-GATES.md -->

# Quality Gates

## Pull Request — prevention
1. fmt;
2. clippy `-D warnings`;
3. cargo check/test;
4. contract tests;
5. archview test/build;
6. JSON schema validation;
7. ADR integrity;
8. specs/index integrity;
9. architecture dependency rules;
10. license coherence;
11. plugin security tests;
12. bundle size cap.

Benchmarks largos solo con etiqueta/perf-sensitive o post-merge.

## Post-merge — evidence
- full integration/E2E;
- benchmark smoke;
- regression compare;
- real-project corpus;
- vulnerability/license scan;
- nightly matrix opcional.

## Release
- build nativo por Tier-1;
- Ladybug compatibility smoke;
- SHA256 manifest;
- artifact provenance;
- self-update/install/uninstall E2E;
- migration dry run;
- no publicar si falta Tier-1.

## Architectural fitness

```text
application -> domain/ports
domain -> std + pure approved crates
adapters -> application/domain/ports
cli -> application + formatting
archview -> projection contracts
```

## Debt ratchet
- nuevo archivo >20 KB exige justificación;
- archivo legacy grande no crece >5% sin excepción;
- `cli.rs`/`store.rs` target decreciente;
- excepción incluye issue de extracción.


---

<!-- SOURCE: 06-RISK-REGISTER.md -->

# Risk Register

| Riesgo | Prob. | Impacto | Mitigación |
|---|---|---|---|
| Ladybug crate/native ABI drift | Alta | Crítico | ADR-048 + doctor |
| Plugin path traversal/supply chain | Media | Crítico | ADR-046 |
| Refactor big-bang | Media | Alto | strangler + golden |
| Cypher leak perpetuo | Alta | Alto | ADR-044 + gate |
| Capability docs drift | Alta | Medio | ADR-045 |
| Evidence ambiguo | Media | Alto | ADR-049 |
| Snapshot storage sin límite | Media | Medio | retention/GC |
| Context ranking incompleto | Media | Alto | trace + unknowns |
| Policy DSL gigante | Media | Medio | rule set cerrado |
| Workbench acoplado a UI semantics | Media | Alto | contract IDs |
| ADR renumber rompe enlaces | Alta | Medio | mapping/tombstones |
| ArchBundle filtra secretos | Baja/Media | Crítico | allowlist + scanner |
| Demasiados crates | Media | Medio | boundary-before-extraction |
| Más diagramas diluye roadmap | Alta | Medio | outcome gate |


---

<!-- SOURCE: 07-ADR-INTEGRATION-MAP.md -->

# ADR Integration Map

## Problema existente
El árbol auditado contiene IDs duplicados:

| ID | Documentos |
|---|---|
| ADR-040 | versioned distribution / cognitive conditional activation |
| ADR-041 | self-update / workspace state persistence |

No hacer mass-renumber de todos los ADR posteriores. Preservar historial y enlaces.

## Nuevas decisiones propuestas

| ADR | Decisión | Prioridad |
|---|---|---|
| 043 | Modular hexagonal boundaries | P1 |
| 044 | Persistence ports/raw query | P1 |
| 045 | Capability Registry | P1 |
| 046 | Plugin supply-chain security | P0/P3 |
| 047 | Pre-merge CI | P0 |
| 048 | Ladybug native compatibility | P0 |
| 049 | Observation/Evidence/Claim | P2 |
| 050 | Git-linked snapshots | P2/P3 |
| 051 | Workbench session security | P3 |
| 052 | Task Context Compiler | P2 |
| 053 | Architecture Diff | P2 |
| 054 | Architecture Policy | P2 |
| 055 | Sanitized ArchBundle | P3 |
| 056 | Moldable Architecture Workbench | P3 |

## Workflow
`Proposed → Accepted → Implementing → Accepted/Implemented`.

Una decisión invalidada pasa a `Rejected` o `Superseded by ADR-NNN`. No se reescribe
un ADR aceptado para fingir que la decisión histórica fue otra.


---

<!-- SOURCE: 08-DECISION-MATRIX.md -->

# Decision Matrix

| Propuesta | Valor | Riesgo reducido | Coste | Orden |
|---|---:|---:|---:|---|
| Ladybug compatibility | 5 | 5 | 3 | ahora |
| Plugin hardening | 4 | 5 | 2 | ahora |
| ADR/license/PR CI | 4 | 4 | 2 | ahora |
| Hex boundaries | 5 | 5 | 4 | tras P0 |
| Capability Registry | 4 | 3 | 2 | P1 |
| Architecture Diff | 5 | 3 | 4 | P2 temprano |
| Explain provenance | 5 | 4 | 3 | P2 temprano |
| Fitness policy | 5 | 4 | 4 | P2 |
| Context Compiler | 5 | 3 | 4 | P2 |
| Evidence Fusion | 5 | 4 | 5 | P2 tardío |
| Snapshots | 4 | 2 | 3 | al servicio de Diff |
| ArchBundle | 4 | 4 | 3 | P3 |
| Moldable workbench | 5 | 1 | 5 | P3 |
| Nuevos renderer formats | 1–2 | 1 | 2–4 | defer |

Una propuesta de renderer solo adelanta si desbloquea un caso medido que no puede
resolverse mediante proyección/adaptador existente.


---

<!-- SOURCE: 09-IMPLEMENTATION-PR-PLAN.md -->

# Plan de PRs pequeños

## Wave 0 — remediation
1. plugin root + first-install tests;
2. plugin identifier validation + malicious tar;
3. ADR duplicate resolution + integrity gate;
4. license coherence;
5. Ladybug toolchain/native pin;
6. PR CI fast gates;
7. native release runners.

## Wave 1 — architecture scaffolding
8. dependency fitness baseline report-only;
9. composition root skeleton;
10. migrate `doctor`;
11. repository ports delegating to current store;
12. migrate `diagram` reads;
13. RawGraphQuery boundary;
14. filesystem contracts;
15. CapabilityRegistry current-state;
16. generated capability docs.

## Wave 2 — intelligence
17. Snapshot metadata MVP;
18. Architecture Diff schema/pure diff;
19. CLI diff + cognitive/delta adapter;
20. DriftView contract;
21. Explain v1;
22. coverage;
23. policy evaluator;
24. SARIF;
25. Task Context deterministic core;
26. MCP context tool;
27. Observation/Claim dual-write experiment.

## Wave 3 — platform
28. strict ArchBundle;
29. archview read-only bundle;
30. session token;
31. cross-view NavigationTarget;
32. action palette;
33. semantic zoom;
34. lens recommendation experiment.

## PR policy
- PR estructural ≠ feature no relacionada;
- actualizar ADR/spec/manifest si cambia contrato;
- golden antes/después;
- rollback explícito;
- benchmark si toca hot path.


---

<!-- SOURCE: 10-TRACEABILITY.md -->

# Traceability — hallazgo → decisión → spec → backlog

| Hallazgo/propuesta | ADR | Spec | Backlog |
|---|---|---|---|
| módulos grandes / SRP | 043 | architecture-consolidation | P1-01/02/10 |
| Cypher leak | 044 | architecture-consolidation | P1-03/04/05 |
| capability drift | 045 | capability-registry | P1-08/11 |
| plugin XDG/hash/traversal | 046 | plugin-security-hardening | P0-04..08 |
| CI post-merge only | 047 | pre-merge-ci | P0-12 |
| Ladybug native breakage | 048 | ladybug-compatibility-doctor | P0-01..03 |
| provenance/fusion | 049 | evidence-fusion / explain | P2-03/04/09 |
| temporal comparison | 050 | architecture-snapshots | P2-01 |
| localhost side effects | 051 | workbench-session-security | P3-03 |
| compact agent context | 052 | task-context-compiler | P2-07/08 |
| change intelligence | 053 | architecture-diff | P2-02 |
| intent vs reality | 054 | architecture-policy | P2-05/06/10 |
| portable sanitized knowledge | 055 | sanitized-archbundle | P3-02 |
| moldable development | 056 | workbench-moldable-navigation | P3-04..06 |


---

<!-- SOURCE: INDEX.md -->

# Package Index

Baseline: `main@518bb79d4c87a491fc901d54441de15e72c40bc2`

## Core

- [`00-EXECUTIVE-SUMMARY.md`](00-EXECUTIVE-SUMMARY.md)
- [`01-ROADMAP-CONSOLIDATION.md`](01-ROADMAP-CONSOLIDATION.md)
- [`02-TARGET-ARCHITECTURE.md`](02-TARGET-ARCHITECTURE.md)
- [`03-IMPLEMENTATION-BACKLOG.md`](03-IMPLEMENTATION-BACKLOG.md)
- [`04-MIGRATION-STRATEGY.md`](04-MIGRATION-STRATEGY.md)
- [`05-QUALITY-GATES.md`](05-QUALITY-GATES.md)
- [`06-RISK-REGISTER.md`](06-RISK-REGISTER.md)
- [`07-ADR-INTEGRATION-MAP.md`](07-ADR-INTEGRATION-MAP.md)
- [`08-DECISION-MATRIX.md`](08-DECISION-MATRIX.md)
- [`09-IMPLEMENTATION-PR-PLAN.md`](09-IMPLEMENTATION-PR-PLAN.md)
- [`10-TRACEABILITY.md`](10-TRACEABILITY.md)
- [`README.md`](README.md)

## ADRs

- [`../adr/ADR-043-modular-hexagonal-boundaries.md`](../adr/ADR-043-modular-hexagonal-boundaries.md)
- [`../adr/ADR-044-persistence-ports-and-raw-query-boundary.md`](../adr/ADR-044-persistence-ports-and-raw-query-boundary.md)
- [`../adr/ADR-045-capability-registry-single-source-of-truth.md`](../adr/ADR-045-capability-registry-single-source-of-truth.md)
- [`../adr/ADR-046-plugin-supply-chain-and-capability-security.md`](../adr/ADR-046-plugin-supply-chain-and-capability-security.md)
- [`../adr/ADR-047-pre-merge-ci-and-post-merge-quality-gates.md`](../adr/ADR-047-pre-merge-ci-and-post-merge-quality-gates.md)
- [`../adr/ADR-048-ladybugdb-native-compatibility-boundary.md`](../adr/ADR-048-ladybugdb-native-compatibility-boundary.md)
- [`../adr/ADR-049-evidence-observation-claim-confidence-model.md`](../adr/ADR-049-evidence-observation-claim-confidence-model.md)
- [`../adr/ADR-050-architecture-snapshots-and-git-identity.md`](../adr/ADR-050-architecture-snapshots-and-git-identity.md)
- [`../adr/ADR-051-loopback-workbench-session-security.md`](../adr/ADR-051-loopback-workbench-session-security.md)
- [`../adr/ADR-052-architecture-context-compiler.md`](../adr/ADR-052-architecture-context-compiler.md)
- [`../adr/ADR-053-architecture-diff-as-first-class-capability.md`](../adr/ADR-053-architecture-diff-as-first-class-capability.md)
- [`../adr/ADR-054-architecture-policy-and-fitness-functions.md`](../adr/ADR-054-architecture-policy-and-fitness-functions.md)
- [`../adr/ADR-055-sanitized-architecture-bundle.md`](../adr/ADR-055-sanitized-architecture-bundle.md)
- [`../adr/ADR-056-moldable-architecture-workbench.md`](../adr/ADR-056-moldable-architecture-workbench.md)

## Specs

- [`specs/adr-integrity-gate.md`](specs/adr-integrity-gate.md)
- [`specs/architecture-consolidation.md`](specs/architecture-consolidation.md)
- [`specs/architecture-diff.md`](specs/architecture-diff.md)
- [`specs/architecture-policy.md`](specs/architecture-policy.md)
- [`specs/architecture-snapshots.md`](specs/architecture-snapshots.md)
- [`specs/capability-registry.md`](specs/capability-registry.md)
- [`specs/confidence-coverage.md`](specs/confidence-coverage.md)
- [`specs/evidence-fusion.md`](specs/evidence-fusion.md)
- [`specs/explain-provenance.md`](specs/explain-provenance.md)
- [`specs/filesystem-contract-tests.md`](specs/filesystem-contract-tests.md)
- [`specs/ladybug-compatibility-doctor.md`](specs/ladybug-compatibility-doctor.md)
- [`specs/license-coherence.md`](specs/license-coherence.md)
- [`specs/plugin-security-hardening.md`](specs/plugin-security-hardening.md)
- [`specs/pre-merge-ci.md`](specs/pre-merge-ci.md)
- [`specs/sanitized-archbundle.md`](specs/sanitized-archbundle.md)
- [`specs/task-context-compiler.md`](specs/task-context-compiler.md)
- [`specs/workbench-moldable-navigation.md`](specs/workbench-moldable-navigation.md)
- [`specs/workbench-session-security.md`](specs/workbench-session-security.md)

## Examples

- [`examples/adr-check-output.json`](examples/adr-check-output.json)
- [`examples/architecture-diff-output.json`](examples/architecture-diff-output.json)
- [`examples/architecture-policy.toml`](examples/architecture-policy.toml)
- [`examples/capability-registry.yaml`](examples/capability-registry.yaml)
- [`examples/plugin-manifest.yaml`](examples/plugin-manifest.yaml)
- [`examples/task-context-output.json`](examples/task-context-output.json)

## Checklists

- [`checklists/P0-stabilization.md`](checklists/P0-stabilization.md)
- [`checklists/P1-hexagonal-consolidation.md`](checklists/P1-hexagonal-consolidation.md)
- [`checklists/P2-intelligence-features.md`](checklists/P2-intelligence-features.md)
- [`checklists/P3-workbench-platform.md`](checklists/P3-workbench-platform.md)


---

<!-- SOURCE: README.md -->

# Arch Stack — Architecture Intelligence Consolidation Pack

Este paquete convierte la auditoría técnica realizada sobre `main@518bb79d4c87a491fc901d54441de15e72c40bc2` en un
conjunto de decisiones, especificaciones y tareas implementables.

No es un reemplazo automático de `docs/ROADMAP.md`, `docs/adr/` ni `docs/specs/`.
Está diseñado para estudiarse, debatirse y después integrarse selectivamente mediante
PRs pequeños y reversibles.

## Tesis

Arch Stack tiene potencial para evolucionar de «herramienta que extrae y dibuja
arquitectura» a un **motor local-first de Architecture Intelligence**:

```text
source code
   │
   ▼
deterministic extractors
   │
   ▼
observations / evidence
   │
   ▼
canonical Architecture Evidence Graph
   │
   ├── C4 / UML / sequence / state / package projections
   ├── explain / provenance / confidence
   ├── architecture diff / history
   ├── fitness policies
   ├── task context for AI agents
   └── moldable Architecture Workbench
```

El valor diferencial no debe ser acumular formatos de diagrama. Debe ser que cada
afirmación arquitectónica pueda **rastrearse, explicarse, compararse y reutilizarse**
por humanos y agentes.

## Orden recomendado de lectura

1. [`00-EXECUTIVE-SUMMARY.md`](00-EXECUTIVE-SUMMARY.md)
2. [`01-ROADMAP-CONSOLIDATION.md`](01-ROADMAP-CONSOLIDATION.md)
3. [`02-TARGET-ARCHITECTURE.md`](02-TARGET-ARCHITECTURE.md)
4. [`04-MIGRATION-STRATEGY.md`](04-MIGRATION-STRATEGY.md)
5. ADRs 043–056 en [`adr/`](adr/)
6. Specs en [`specs/`](specs/)
7. [`03-IMPLEMENTATION-BACKLOG.md`](03-IMPLEMENTATION-BACKLOG.md)
8. Checklists en [`checklists/`](checklists/)

También se genera [`ALL-IN-ONE.md`](ALL-IN-ONE.md) como versión de lectura continua.

## Numeración ADR

El repositorio auditado contiene dos `ADR-040` y dos `ADR-041`. Este paquete **no
reescribe historia automáticamente**. Las nuevas propuestas empiezan en `ADR-043`.
La resolución de las colisiones existentes forma parte de P0 y está especificada
en `specs/adr-integrity-gate.md`.

## Principios preservados

- local-first;
- cero escritura en el código fuente;
- grafo canónico único;
- evidence/provenance como requisito de confianza;
- extractores deterministas antes que inferencia LLM;
- IA como consumidor/orquestador, no fuente de verdad opaca;
- workbench embebido, sin daemon obligatorio;
- adopción incremental y reversible;
- rendimiento medible.

## Fuera de alcance deliberadamente

No se propone ahora: SaaS central, colaboración multiusuario propia, RBAC de servidor,
vector DB como fuente de verdad, event sourcing completo, plataforma LLM genérica,
RAG genérico, microservicios ni una carrera por soportar decenas de renderers.


---

<!-- SOURCE: ../adr/ADR-043-modular-hexagonal-boundaries.md -->

# ADR-043 — Límites hexagonales modulares por capacidad

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

Arch Stack aplica inversión de dependencias en filesystem, store capabilities e IDE
adapters, pero el crecimiento ha concentrado orchestration, infraestructura y
presentación en módulos grandes. La arquitectura es hexagonal por intención, no aún
por restricción compilable.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Adoptar **Modular Hexagonal Architecture** por bounded capability. Primera etapa:
módulos + reglas de dependencia; extracción a Cargo crates solo tras estabilidad.

Boundaries objetivo: `arch-model`, `arch-analysis`, `arch-knowledge`,
`arch-projection`, `arch-workbench`, `arch-distribution`, `archctl` composition root
y `archview-contract`.



## Rationale y beneficios

Evita que `cli.rs` y `store.rs` sean hubs, permite tests aislados, reduce coupling
con librerías nativas/web y permite reutilización desde CLI, MCP y workbench.

## Costes y consecuencias negativas

Más DTO mappings y estructura. Si se extraen crates prematuramente aparece
fragmentación y cycles artificiales.



## No objetivos

No microservicios. No ocho crates de golpe. No cambiar DB. No rediseñar CLI
por estética.

## Estrategia de migración

1. dependency map/gates;
2. composition root;
3. usecases por familia;
4. ports semánticos;
5. reorganización modular;
6. crates solo con boundary estable.

## Verificación y criterios de aceptación

- CI detecta imports prohibidos;
- usecases testeables con fakes;
- CLI no crea stores/filesystems;
- lbug/tiny_http/reqwest no aparecen en domain/application;
- outputs públicos equivalentes.

## Alternativas consideradas

A) feature-only indefinido: erosiona límites.
B) carpetas globales domain/application: demasiado horizontales.
C) reescritura: riesgo inaceptable.

## Referencias internas

`archctl/src/cli.rs`, `store.rs`, `code/`, ADR-038, ADR-042.

## Changelog

- 2026-08-13 | proposed | ADR-043 creado a partir de la auditoría de consolidación.


---

<!-- SOURCE: ../adr/ADR-044-persistence-ports-and-raw-query-boundary.md -->

# ADR-044 — Repositorios semánticos y frontera de query raw

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

`GraphStore` abstrae persistencia, pero ejecutar strings Cypher deja que el detalle
del motor cruce hacia application y complique sustituibilidad, contratos y
transacciones.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Separar `ArchitectureRepository`, `EvidenceRepository`, `ObservationRepository`,
`ProjectionRepository`, `SnapshotRepository`, `GraphReadModel` y `UnitOfWork`.
Conservar `RawGraphQuery` únicamente para administración/diagnóstico.

## Superficie propuesta

```rust
trait ArchitectureRepository { /* semantic operations */ }
trait GraphReadModel { /* traversal/impact/explain */ }
trait UnitOfWork { /* atomic mutation boundary */ }
trait RawGraphQuery { /* admin/debug only */ }
```

## Rationale y beneficios

DIP real, APIs orientadas a intención, tests por contrato y optimización interna de
Ladybug sin contaminar usecases.

## Costes y consecuencias negativas

Más interfaces; algunas consultas exploratorias necesitan read model flexible.





## Estrategia de migración

Adapters iniciales delegan al store actual. Migrar queries de usecases por slices.
Después estrechar visibilidad de raw query.

## Verificación y criterios de aceptación

- 0 Cypher en application;
- graph query CLI sigue disponible;
- contract tests fake/Ladybug;
- multi-write usa UnitOfWork.

## Alternativas consideradas

A) único GraphStore: fuga tecnología.
B) repository CRUD por entidad: mal fit para grafo.
C) CQRS/event sourcing: exceso.

## Referencias internas

`archctl/src/store.rs`, store transaction tests, ADR-005, ADR-017.

## Changelog

- 2026-08-13 | proposed | ADR-044 creado a partir de la auditoría de consolidación.


---

<!-- SOURCE: ../adr/ADR-045-capability-registry-single-source-of-truth.md -->

# ADR-045 — Capability Registry como fuente única de verdad

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

Feature/language support evoluciona a ritmos distintos. Call Graph, Class Diagram y
State Machine no tienen la misma matriz; comentarios/errores históricos pueden quedar
desincronizados de la implementación.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Crear `CapabilityRegistry` tipado/serializable con capability, language, maturity,
requirements, determinism, schema y availability. CLI, doctor, MCP, docs y archview
consumen el mismo contrato.

## Superficie propuesta

```json
{"capability":"code.call_graph","language":"kotlin","maturity":"beta",
 "deterministic":true,"schema":"call-graph-report/1"}
```

## Rationale y beneficios

Elimina matrices duplicadas, habilita feature negotiation, maturity visible y
extensión por plugins.

## Costes y consecuencias negativas

Riesgo de mega-config; debe describir capacidades, no wiring.





## Estrategia de migración

Registrar estado actual sin cambiar comportamiento; alignment tests; exponer
`archctl capabilities --json`; generar docs/tablas.

## Verificación y criterios de aceptación

- provider sin registry o viceversa falla;
- docs generadas limpias;
- archview habilita acciones desde registry.

## Alternativas consideradas

A) enums locales: drift.
B) reflexión automática: no expresa maturity.
C) YAML-only: posible, pero el core debe conservar typing.

## Referencias internas

`archctl/src/code/*`, cognitive MCP, manifests, docs/specs/index.md.

## Changelog

- 2026-08-13 | proposed | ADR-045 creado a partir de la auditoría de consolidación.


---

<!-- SOURCE: ../adr/ADR-046-plugin-supply-chain-and-capability-security.md -->

# ADR-046 — Seguridad de plugins y supply chain

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

Plugin tap instala material de red localmente. La auditoría detectó namespace XDG
inconsistente, staging antes de asegurar root, checksum remoto opcional e identidad
author/name/version usada como path sin value objects estrictos.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Frontera no confiable: value objects; root bajo Arch Stack; checksum obligatorio
para remoto; extracción segura; manifest de capabilities; trust state
`local|verified|trusted|untrusted`; staging→verify→activate atómico.



## Rationale y beneficios

Reduce traversal/tampering, habilita least privilege y hace auditable el origen.

## Costes y consecuencias negativas

Más fricción de publicación. Hash aporta integridad/reproducibilidad, no autenticidad
plena si tap y hash se comprometen juntos.

## Riesgos y mitigaciones

Limitar además expanded size/file count y rechazar device/FIFO/symlink escapes.



## Estrategia de migración

P0 path/staging/hash/unpack; P1 manifest; P3 firma/trust enforcement. Legacy queda
`legacy-unverified` hasta reinstalar.

## Verificación y criterios de aceptación

- malicious names rechazados;
- tar no escapa staging;
- remote sin hash falla;
- first install funciona;
- current cambia atómicamente;
- inspect muestra source/hash/capabilities.

## Alternativas consideradas

A) HTTPS basta: no.
B) WASM sandbox ya: demasiado cambio.
C) firma GPG obligatoria en P0: puede bloquear adopción.

## Referencias internas

`archctl/src/plugin/mod.rs`, `plugin/install.rs`, ADR-004 y ADR-040 distribution.

## Changelog

- 2026-08-13 | proposed | ADR-046 creado a partir de la auditoría de consolidación.


---

<!-- SOURCE: ../adr/ADR-047-pre-merge-ci-and-post-merge-quality-gates.md -->

# ADR-047 — CI preventiva en PR y evidencia post-merge

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

CI auditado prioriza push a main y hooks locales para prevención. Dependencias
nativas y múltiples contratos hacen posible que fallos específicos de runner entren
en main antes de detectarse.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Dos niveles: PR fast gate determinista; post-merge para E2E/benchmarks/corpus.
Release solo consume commit verde y compila Tier-1 en runner apropiado.



## Rationale y beneficios

Reduce main rojo sin convertir cada PR en benchmark largo. Branch protection
reproducible.

## Costes y consecuencias negativas

Más minutos CI y riesgo de flakiness; caching y separación mitigarán.





## Estrategia de migración

Añadir pull_request manteniendo post-merge. Observar duración un ciclo. Convertir
fast checks en required.

## Verificación y criterios de aceptación

- compile error bloquea PR;
- ADR duplicate falla rápido;
- benchmark largo fuera de PR normal;
- release no parte de commit rojo.

## Alternativas consideradas

A) hooks-only: no reproducible.
B) todo benchmark en PR: lento.
C) merge queue desde día 1: opcional posterior.

## Referencias internas

`.github/workflows/ci.yml`, `release.yml`, verify-local, ADR-025.

## Changelog

- 2026-08-13 | proposed | ADR-047 creado a partir de la auditoría de consolidación.


---

<!-- SOURCE: ../adr/ADR-048-ladybugdb-native-compatibility-boundary.md -->

# ADR-048 — LadybugDB como adapter nativo con matriz de compatibilidad

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

El build de release auditado falló en bindings C++ de `lbug` por `<format>` ausente;
además una dependencia nativa mutable/versionada aparte puede desacoplar crate,
headers, compiler y ABI. El store es infraestructura crítica.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Encapsular LadybugDB detrás de adapter nativo con versions pinned, source digest,
compatibility probe, `doctor --scope storage`, builds nativos por OS y smoke
CRUD/migrations por target.

## Superficie propuesta

```text
archctl doctor --scope storage
  archctl, lbug crate, native library, c++ stdlib, db schema, status
```

## Rationale y beneficios

Aísla el riesgo, da errores accionables y hace release reproducible.

## Costes y consecuencias negativas

Mantener la matriz cuesta y puede exigir toolchains nuevos.





## Estrategia de migración

Recuperar build → registrar tuple exacta → probe → mover imports detrás del adapter
→ release gate.

## Verificación y criterios de aceptación

- doctor muestra crate/native/schema/toolchain;
- mismatch falla antes de DB;
- Tier-1 smoke;
- domain/application sin lbug;
- no artifact `latest` mutable.

## Alternativas consideradas

A) build from source siempre: caro.
B) cambiar DB ahora: no justificado.
C) latest mutable: no reproducible.

## Referencias internas

Cargo.toml, store.rs, DATA-MODEL-LADYBUGDB, release, ADR-005/010.

## Changelog

- 2026-08-13 | proposed | ADR-048 creado a partir de la auditoría de consolidación.


---

<!-- SOURCE: ../adr/ADR-049-evidence-observation-claim-confidence-model.md -->

# ADR-049 — Separar Observation, Evidence y Claim

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

Si varias estrategias detectan la misma relación, un único confidence puede ocultar
qué fue observado, inferido o contradicho. Explain y fusion necesitan cada señal.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Metamodelo: `SourceArtifact`, `Evidence`, `Observation`, `Claim`, arista
`supports|contradicts` y `Confidence` recalculable. Observation registra
producer/version/method.

## Superficie propuesta

```text
SourceArtifact <- Evidence <- Observation -(supports/contradicts)-> Claim
```

## Rationale y beneficios

Permite fusion, contradicciones, explain, confidence/coverage y evolución de
extractores sin perder procedencia.

## Costes y consecuencias negativas

Migración de schema y más nodos/aristas. Evitar falsa precisión en scores.





## Estrategia de migración

Añadir tipos sin borrar legacy; dual-write; backfill; cambiar readers; retirar
legacy tras paridad.

## Verificación y criterios de aceptación

- 2 extractores apoyan sin sobrescribir;
- contradicción visible;
- explain lineage completo;
- confidence recalculable;
- borrar observation no elimina otras.

## Alternativas consideradas

A) confidence por edge: insuficiente.
B) provenance en props: difícil de consultar.
C) probabilistic KB completa: exceso.

## Referencias internas

evidence.rs, metamodel-core.json, ADR-005/009/027.

## Changelog

- 2026-08-13 | proposed | ADR-049 creado a partir de la auditoría de consolidación.


---

<!-- SOURCE: ../adr/ADR-050-architecture-snapshots-and-git-identity.md -->

# ADR-050 — Snapshots arquitectónicos ligados a identidad Git

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

Diff e historia necesitan estados coherentes. El estado actual por sí solo no
responde reproduciblemente qué cambió entre dos refs.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Snapshot lógico identificado por repo identity + commit SHA + schema version +
extractor-set digest. No event sourcing completo.



## Rationale y beneficios

Diff reproducible, forensics, contexto por commit y base de PR analysis.

## Costes y consecuencias negativas

Disco y retención; extractor/schema changes requieren compatibility metadata.





## Estrategia de migración

MVP create explícito; luego on-demand desde diff; retention configurable en XDG.

## Verificación y criterios de aceptación

- misma tuple → misma identity;
- schemas incompatibles requieren rebuild/migration;
- snapshots fuera del repo;
- GC conserva pins/recent.

## Alternativas consideradas

A) event sourcing: demasiado.
B) recalcular siempre worktrees: fallback lento.
C) JSON manual: sin lifecycle.

## Referencias internas

identity.rs, xdg.rs, cognitive/delta.rs, ADR-004/008.

## Changelog

- 2026-08-13 | proposed | ADR-050 creado a partir de la auditoría de consolidación.


---

<!-- SOURCE: ../adr/ADR-051-loopback-workbench-session-security.md -->

# ADR-051 — Token de sesión efímero para acciones del workbench

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

El workbench ya escucha loopback y valida paths, pero endpoints con side effects
pueden ser invocados por otros orígenes/procesos locales si conocen el puerto.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Token aleatorio por proceso `archctl view`; exigir en side effects y aplicar
Origin/Host checks. Bootstrap sin persistir secret.



## Rationale y beneficios

Hardening contra cross-origin localhost sin cuentas/RBAC.

## Costes y consecuencias negativas

Más bootstrap y tests.





## Estrategia de migración

Introducir guard; health/static pueden seguir públicos; side effects pasan a auth.

## Verificación y criterios de aceptación

- ≥128 bits;
- no logs persistentes;
- POST/PUT sin token 403;
- path checks siguen;
- bind loopback.

## Alternativas consideradas

A) confiar loopback: insuficiente.
B) OAuth: exceso.
C) Unix socket: navegador/cross-platform.

## Referencias internas

view.rs, view/source.rs, view/editor.rs, ADR-033.

## Changelog

- 2026-08-13 | proposed | ADR-051 creado a partir de la auditoría de consolidación.


---

<!-- SOURCE: ../adr/ADR-052-architecture-context-compiler.md -->

# ADR-052 — Task Context Compiler para agentes

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

La capa cognitive ya contiene context/MCP/agentes. El siguiente salto debe compilar
contexto arquitectónico pequeño y verificable, no crear RAG genérico ni releer el
repo entero por tarea.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Evolucionar `cognitive/context.rs`: normalize task → deterministic seeds → graph
expansion → impact/policy/evidence enrichment → ranking → budget packing → trace.
LLM query expansion opcional nunca inventa entidades.

## Superficie propuesta

```bash
archctl context compile --task "..." --budget-tokens 12000 --json
```

## Rationale y beneficios

Reduce tokens/latencia, aumenta grounding y convierte el grafo en memoria
arquitectónica reusable.

## Costes y consecuencias negativas

Ranking puede omitir contexto; debe reportar truncation/unknowns y explicar selección.





## Estrategia de migración

CLI JSON deterministic primero; MCP después; preview en ImpactView; LLM expansion
solo posterior.

## Verificación y criterios de aceptación

- misma entrada y budget → mismo output en deterministic;
- IDs/provenance;
- excluded/truncated;
- no source completo default;
- golden budget tests.

## Alternativas consideradas

A) vector RAG: nueva index/source.
B) repomix completo: caro.
C) LLM elige ficheros: opaco.

## Referencias internas

cognitive/context.rs, cognitive/mcp, ImpactView, ADR-021.

## Changelog

- 2026-08-13 | proposed | ADR-052 creado a partir de la auditoría de consolidación.


---

<!-- SOURCE: ../adr/ADR-053-architecture-diff-as-first-class-capability.md -->

# ADR-053 — Architecture Diff como capability first-class

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

Ya existen `cognitive/delta.rs`, DriftView e ImpactView. Falta un contrato público
unificado para convertirlos en change intelligence reusable.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Usecase independiente de UI: baseline vs target, cambios de elements, relations,
confidence, policies, cycles, boundaries y evidence. CLI/MCP/DriftView consumen el
mismo DTO.

## Superficie propuesta

```bash
archctl architecture diff main..HEAD --format json
```

## Rationale y beneficios

Alto valor en PRs y reutiliza piezas existentes en vez de duplicarlas.

## Costes y consecuencias negativas

Extractor/schema changes pueden crear ruido; report debe marcar comparabilidad.





## Estrategia de migración

Refactor delta a output versionado; snapshot provider; CLI; DriftView; luego
annotations.

## Verificación y criterios de aceptación

- stable JSON;
- cosmetic changes fuera;
- evidence refs;
- policy regressions separadas;
- output ordenado determinista.

## Alternativas consideradas

A) diff projections: pierde semántica.
B) source diff: no arquitectura.
C) DB temporal por branch: innecesario.

## Referencias internas

cognitive/delta.rs, DriftView, ImpactView, ADR-050.

## Changelog

- 2026-08-13 | proposed | ADR-053 creado a partir de la auditoría de consolidación.


---

<!-- SOURCE: ../adr/ADR-054-architecture-policy-and-fitness-functions.md -->

# ADR-054 — Políticas y fitness functions sobre el grafo canónico

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

El grafo informa dependencias reales, pero sin políticas no previene regresiones.
Existe policy engine cognitivo que puede reaprovecharse si el contrato permanece
determinista.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Modelo mínimo de reglas: `forbid_dependency`, `require_dependency`, `forbid_cycle`,
`max_fanout`, `evidence_required`, `confidence_min`, selectors, severity y waivers.
Outputs JSON/SARIF/JUnit.

## Superficie propuesta

```toml
[[rules]]
id="HEX-001"
type="forbid_dependency"
from="module:domain/**"
to="crate:reqwest"
severity="error"
```

## Rationale y beneficios

Architecture-as-code, CI e Intent vs Reality. Arch Stack puede dogfood su propia
hexagonal.

## Costes y consecuencias negativas

Falsos positivos y riesgo de DSL gigante.





## Estrategia de migración

TOML/YAML con rule set cerrado; evaluator puro; warn primero, enforce después.

## Verificación y criterios de aceptación

- determinista;
- violation con rule+IDs+evidence;
- SARIF a source;
- waiver expira;
- self-policy.

## Alternativas consideradas

A) OPA/Rego: runtime/DSL extra.
B) Cedar: otro dominio.
C) hardcoded Rust tests: no consumible.

## Referencias internas

cognitive/policy, selectors, ADR-038/047.

## Changelog

- 2026-08-13 | proposed | ADR-054 creado a partir de la auditoría de consolidación.


---

<!-- SOURCE: ../adr/ADR-055-sanitized-architecture-bundle.md -->

# ADR-055 — Sanitized Architecture Bundle compartible

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

Para onboarding/consultoría/agentes interesa compartir arquitectura sin entregar
source, paths sensibles, secretos ni DB completa.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

`.archbundle` versionado deny-by-default: manifest, graph slice, claims/evidence
metadata sanitizada, policies, capabilities, optional snapshots/diffs, checksums.
Source bytes excluidos por defecto.



## Rationale y beneficios

Portable, offline, reproducible y local-first.

## Costes y consecuencias negativas

Sanitización puede filtrar nombres sensibles; requiere allowlist/scanner.





## Estrategia de migración

MVP export strict manual; archview read-only; profiles custom/firma después.

## Verificación y criterios de aceptación

- scanner malicioso;
- no source;
- no absolute paths;
- redaction manifest;
- hash;
- import read-only.

## Alternativas consideradas

A) zip viewer bundle: sin privacy contract.
B) DB export: filtra.
C) cloud share: contradice local-first.

## Referencias internas

projection bundle, evidence model, ADR-004/049.

## Changelog

- 2026-08-13 | proposed | ADR-055 creado a partir de la auditoría de consolidación.


---

<!-- SOURCE: ../adr/ADR-056-moldable-architecture-workbench.md -->

# ADR-056 — Moldable Architecture Workbench y navegación semántica

> **Estado:** Propuesto — 2026-08-13
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack
> **Naturaleza:** propuesta; este documento no modifica por sí mismo el repositorio

## Contexto

Archview ya tiene múltiples vistas. El paso útil no es añadir pestañas infinitas,
sino cambiar representación según entidad, escala y pregunta preservando identidad.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Semantic zoom bidireccional, action palette, capability-driven lens selection,
composición C4/sequence/class/source/evidence/impact/confidence y rationale de
auto-lens. Workspace persiste navegación, no semántica.



## Rationale y beneficios

Reduce necesidad de conocer notaciones y convierte el producto en entorno de
comprensión.

## Costes y consecuencias negativas

UX compleja; automatismo debe ser explicable/reversible.





## Estrategia de migración

Cross-view identity → action palette → semantic zoom → lens recommendation.
Mantener vistas actuales hasta paridad.

## Verificación y criterios de aceptación

- entidad cruza vistas;
- breadcrumbs;
- rationale auto-lens;
- back/forward estable;
- budget 10k nodes.

## Alternativas consideradas

A) tabs independientes: menor utilidad.
B) canvas libre: pierde semántica.
C) UI generada LLM: no determinista.

## Referencias internas

archview views, ADR-013/033/045.

## Changelog

- 2026-08-13 | proposed | ADR-056 creado a partir de la auditoría de consolidación.


---

<!-- SOURCE: checklists/P0-stabilization.md -->

# Checklist P0 — Stabilization

## Build / storage
- [ ] Reproducir fallo Ladybug en clean runner.
- [ ] Determinar versión exacta crate/native.
- [ ] Eliminar `latest` mutable.
- [ ] Definir compiler/C++ stdlib mínimos.
- [ ] `doctor --scope storage`.
- [ ] Linux x86_64 verde.
- [ ] Linux aarch64 verde.
- [ ] macOS x86_64 verde en runner macOS.
- [ ] macOS arm64 verde en runner macOS.

## Plugins
- [ ] `~/.local/share/archctl/plugins`.
- [ ] `create_dir_all` antes de staging.
- [ ] identity value objects.
- [ ] checksum remote obligatorio.
- [ ] safe tar extraction.
- [ ] malicious fixtures.
- [ ] first-install E2E.

## Governance
- [ ] Resolver duplicate ADR-040.
- [ ] Resolver duplicate ADR-041.
- [ ] ADR integrity gate.
- [ ] License decision + files.
- [ ] License coherence gate.
- [ ] PR CI fast gate.
- [ ] branch protection.

## Contracts
- [ ] Filesystem contract documented.
- [ ] SystemFS contract suite.
- [ ] MemoryFS contract suite.
- [ ] Stale capability comments corrected.


---

<!-- SOURCE: checklists/P1-hexagonal-consolidation.md -->

# Checklist P1 — Hexagonal Consolidation

- [ ] Dependency baseline.
- [ ] Composition root.
- [ ] Runtime/AppServices injectable.
- [ ] CLI golden tests.
- [ ] Migrar doctor.
- [ ] Migrar diagram.
- [ ] Migrar code.
- [ ] Migrar view.
- [ ] Migrar plugin.
- [ ] Migrar lifecycle/IDE.
- [ ] ArchitectureRepository.
- [ ] EvidenceRepository.
- [ ] GraphReadModel.
- [ ] UnitOfWork.
- [ ] RawGraphQuery aislado.
- [ ] Extractor language strategies.
- [ ] Extractor contracts.
- [ ] CapabilityRegistry.
- [ ] Generated capability docs.
- [ ] Dependency fitness gate.
- [ ] Size ratchet.
- [ ] Evaluar extracción a crates.


---

<!-- SOURCE: checklists/P2-intelligence-features.md -->

# Checklist P2 — Architecture Intelligence

## Diff
- [ ] Snapshot identity.
- [ ] Diff schema.
- [ ] cognitive/delta reuse.
- [ ] semantic vs cosmetic.
- [ ] policy regression.
- [ ] DriftView integration.

## Explain / confidence
- [ ] Explain relation.
- [ ] Explain element.
- [ ] Evidence lineage.
- [ ] contradiction model.
- [ ] coverage report.
- [ ] confidence overlay.

## Policies
- [ ] policy schema.
- [ ] core evaluators.
- [ ] SARIF.
- [ ] JUnit.
- [ ] waivers + expiry.
- [ ] self-dogfood.

## Context compiler
- [ ] deterministic seeds.
- [ ] graph expansion.
- [ ] impact enrichment.
- [ ] budget packer.
- [ ] selection trace.
- [ ] MCP contract.
- [ ] relevance corpus.


---

<!-- SOURCE: checklists/P3-workbench-platform.md -->

# Checklist P3 — Workbench & Portable Knowledge

- [ ] immutable snapshots + retention.
- [ ] `.archbundle` strict.
- [ ] sanitizer allowlist.
- [ ] secret/path tests.
- [ ] archview read-only bundle.
- [ ] loopback session token.
- [ ] cross-view canonical IDs.
- [ ] semantic zoom.
- [ ] breadcrumbs.
- [ ] action palette.
- [ ] Explain action.
- [ ] Evidence action.
- [ ] Impact action.
- [ ] History action.
- [ ] Violations action.
- [ ] lens rationale.
- [ ] plugin capabilities.
- [ ] plugin trust UX.


---

<!-- SOURCE: specs/adr-integrity-gate.md -->

# Spec — ADR Integrity Gate

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Garantizar identidad única y referencias consistentes en docs/adr.

## Scope

Filename/H1 ID, duplicates, status, links, index, supersedes/complements.



## Public surface

`scripts/check-adr-integrity [--json]`; exit 0 valid, 2 invalid.

## Modelo y semántica

ID = clave única. Colisión histórica se resuelve con mapping/tombstone, nunca silenciosamente.





## Escenarios Given / When / Then

Given duplicate 040, reporta ambos.
Given link missing, identifica source.
Given ADR nuevo no indexado, falla/warn según policy.

## Plan de implementación

Parser Markdown mínimo; fixtures; resolver 040/041; activar PR CI.

## Estrategia de pruebas

Fixtures valid/duplicate/broken/status; snapshot output.

## Métricas y SLOs de producto/ingeniería

<2s; 0 falsos positivos tras remediation.



## Dependencias y cross-references

ADR-047; docs/adr/README.md.

## Ejemplos

Mantener los IDs más arraigados en releases y reasignar los menos referenciados mediante PR explícito.


---

<!-- SOURCE: specs/architecture-consolidation.md -->

# Spec — Architecture Consolidation Program

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Reducir acoplamiento estructural sin reescribir ni romper superficies públicas.

## Scope

Composition root, usecases, ports, module boundaries, dependency rules y criterio de crates.

## Non-goals

No reescribir archctl; no DI framework; no microservices.

## Public surface

`Runtime/AppServices`, usecase inputs/outputs y ports semánticos; CLI externa estable.

## Modelo y semántica

Bounded capability como unidad. Movimiento estructural debe demostrar equivalencia.





## Escenarios Given / When / Then

### SCN-AC-01
Given golden output existente, when comando se migra a usecase, then exit/JSON igual.

### SCN-AC-02
Given application module, when importa lbug/reqwest/tiny_http, then gate falla.

## Plan de implementación

Baseline → composition root → migrate command → repositories → gates → repeat.

## Estrategia de pruebas

Unit con fakes; golden CLI; dependency tests; compile tests.

## Métricas y SLOs de producto/ingeniería

0 infra imports en domain/application; 0 Cypher en usecases; >80% core usecases sin I/O.



## Dependencias y cross-references

ADR-043, ADR-044, ADR-047.


---

<!-- SOURCE: specs/architecture-diff.md -->

# Spec — Architecture Diff

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Comparar arquitectura semántica entre dos Git refs/snapshots con provenance.

## Scope

Elements, relations, semantic props, confidence, policy, cycles, boundaries, unresolved.



## Public surface

`archctl architecture diff A..B`; schema `architecture-diff-report/1`.

## Modelo y semántica

Tipos: added/removed/changed/confidence_changed/policy_regression/improvement/cycle changes.





## Escenarios Given / When / Then

Given relation nueva, report + evidence.
Given visual move, semantic diff empty.
Given extractor version differs, compatibility metadata lo marca.

## Plan de implementación

Refactor cognitive/delta → snapshot provider → schema → CLI → DriftView.

## Estrategia de pruebas

Golden graph; deterministic ordering; schema compatibility; benchmark.

## Métricas y SLOs de producto/ingeniería

p95 <2s cached <10k nodes; hash estable.



## Dependencias y cross-references

ADR-050/053; cognitive/delta; DriftView.

## Ejemplos

Ver `../examples/architecture-diff-output.json`.


---

<!-- SOURCE: specs/architecture-policy.md -->

# Spec — Architecture Policy & Fitness Functions

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Expresar restricciones mínimas y evaluarlas sobre el grafo.

## Scope

Selectors, dependency/cycle/evidence/confidence, severity, waivers, JSON/SARIF/JUnit.



## Public surface

`archctl architecture check --policy <file> --format sarif`.

## Modelo y semántica

Rule IDs estables; violation referencia graph IDs/evidence/source; waiver con reason/expiry.





## Escenarios Given / When / Then

Given forbidden edge, error con path.
Given waiver expired, violation vuelve.
Given selector no match, warning.

## Plan de implementación

Pure evaluator → 6 rule types → outputs → self-dogfood → CI.

## Estrategia de pruebas

Rule fixtures; selector properties; SARIF schema; self-policy.

## Métricas y SLOs de producto/ingeniería

<1s 10k nodes base rules; deterministic.



## Dependencias y cross-references

ADR-054; cognitive/policy; ADR-043.

## Ejemplos

Ver `../examples/architecture-policy.toml`.


---

<!-- SOURCE: specs/architecture-snapshots.md -->

# Spec — Architecture Snapshots

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Persistir estados comparables ligados a Git sin event sourcing.

## Scope

Identity, creation, ref resolution, retention, schema/extractor digest.



## Public surface

`archctl architecture snapshot create/list/gc`; SnapshotRepository.

## Modelo y semántica

Key = repo identity + SHA + schema + extractor digest; label mutable puede apuntar a immutable snapshot.





## Escenarios Given / When / Then

Given same tuple, idempotent.
Given incompatible schema, diff rebuild/migration.
Given GC, pins remain.

## Plan de implementación

MVP metadata + graph materialization/delta; on-demand from diff; retention.

## Estrategia de pruebas

Idempotency; GC; corruption checksum; large snapshot.

## Métricas y SLOs de producto/ingeniería

Medir incremental size antes de compression complexity.



## Dependencias y cross-references

ADR-050; identity; XDG.


---

<!-- SOURCE: specs/capability-registry.md -->

# Spec — Capability Registry

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Unificar feature/language/maturity/requisitos y eliminar drift.

## Scope

Extractors, projections, render outputs, views, MCP y plugin extensions.



## Public surface

`archctl capabilities --json`; registry tipado; schema v1.

## Modelo y semántica

Key estable + providers + maturity + deterministic + requirements + output schema.





## Escenarios Given / When / Then

Given Kotlin provider, registry lo lista.
Given provider sin entry, alignment test falla.
Given dependency ausente, status unavailable con reason.

## Plan de implementación

Inventory → registry → alignment → CLI/MCP → generated docs.

## Estrategia de pruebas

Golden JSON; provider alignment; generated docs diff.

## Métricas y SLOs de producto/ingeniería

0 matrices manuales duplicadas tras rollout.



## Dependencias y cross-references

ADR-045; manifests; specs index.

## Ejemplos

Ver `../examples/capability-registry.yaml`.


---

<!-- SOURCE: specs/confidence-coverage.md -->

# Spec — Confidence & Coverage

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Representar incertidumbre, cobertura y contradicción como parte del producto.

## Scope

Claim confidence, subsystem coverage, unresolved calls, unsupported language, stale evidence.



## Public surface

`archctl architecture coverage --json`; overlay contract.

## Modelo y semántica

Unknown ≠ false; coverage siempre declara denominator/exclusions.





## Escenarios Given / When / Then

Given 100 funcs/70 resolved, denominator visible.
Given unsupported language, status unsupported, no 0%.
Given conflict, count visible.

## Plan de implementación

Define metrics → instrument extractors → report → archview overlay.

## Estrategia de pruebas

Known corpus; calibration regression.

## Métricas y SLOs de producto/ingeniería

Denominator no cambia silenciosamente.



## Dependencias y cross-references

ADR-049; capability registry; Explain.


---

<!-- SOURCE: specs/evidence-fusion.md -->

# Spec — Evidence Fusion

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Agregar múltiples observaciones sin perder procedencia ni confidence opaco.

## Scope

Observation identity, producer, supports/contradicts, aggregation, staleness.



## Public surface

`ObservationRepository` + `ClaimEvaluator`; visible mediante explain/coverage.

## Modelo y semántica

Aggregator v1 simple, determinista, order-independent; independencia y contradicción explícitas.





## Escenarios Given / When / Then

Given AST + manifest support, claim 2 supports.
Given contradiction, ambas quedan.
Given producer version cambia, antigua puede marcarse stale.

## Plan de implementación

Schema dual-write → backfill → aggregator → readers → retire legacy.

## Estrategia de pruebas

Migration; idempotency; commutativity; calibration corpus.

## Métricas y SLOs de producto/ingeniería

Aggregation determinista/commutative; 0 provenance loss.



## Dependencias y cross-references

ADR-049; evidence.rs; migrations.


---

<!-- SOURCE: specs/explain-provenance.md -->

# Spec — Explain & Provenance

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Responder por qué existe una entidad/relación/violación sin explicación opaca.

## Scope

Element, relation, claim, violation, projection membership.



## Public surface

`archctl explain <id|selector> --json`; MCP architecture_explain.

## Modelo y semántica

Identity + statement + lineage + source refs + producer/version + confidence derivation + contradictions.



## Seguridad

Source excerpts opt-in y cap.

## Escenarios Given / When / Then

Given A→B, explain lista observation y file:line.
Given conflicto, muestra ambos.
Given no evidence, `unsubstantiated`; nunca inventa.

## Plan de implementación

Existing Evidence primero; Observation/Claim migration transparente.

## Estrategia de pruebas

Lineage graph, cycle guard, missing source, stale evidence.

## Métricas y SLOs de producto/ingeniería

100% machine claims con evidence o explicit reason.



## Dependencias y cross-references

ADR-049; evidence.rs.


---

<!-- SOURCE: specs/filesystem-contract-tests.md -->

# Spec — Filesystem Adapter Contract Tests

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Asegurar LSP entre SystemFilesystem y MemoryFilesystem.

## Scope

read/write/exists/walk/canonicalize/path containment/errors.



## Public surface

`FilesystemContractSuite` reusable con factory/root.

## Modelo y semántica

Contrato explícito para existing/nonexisting/symlink/traversal y ordering.





## Escenarios Given / When / Then

Given nonexistent nested path, ambos adapters mismo resultado/error class.
Given symlink escape, ambos rechazan.
Given walk, ordering determinista.

## Plan de implementación

Formalizar semantics → ejecutar suite → corregir adapters.

## Estrategia de pruebas

Tempdir SystemFS + MemoryFS; cfg platform; property paths.

## Métricas y SLOs de producto/ingeniería

100% scenarios pasan en adapters.



## Dependencias y cross-references

filesystem-port spec; ADR-043.


---

<!-- SOURCE: specs/ladybug-compatibility-doctor.md -->

# Spec — Ladybug Compatibility Doctor

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Detectar incompatibilidad nativa antes de compile/link/open DB failures.

## Scope

Build metadata, runtime probe, schema/migration compatibility.



## Public surface

`archctl doctor --scope storage --json`.

## Modelo y semántica

Tuple: archctl, lbug crate, native version/source digest, target, compiler, stdlib, schema.

## Errores y comportamiento degradado

Unknown no equivale a compatible en release.



## Escenarios Given / When / Then

Given tuple compatible, ok.
Given mismatch, critical.
Given unknown, warning + remediation; release treats unknown as failure.

## Plan de implementación

Pin native; expose metadata; probe; release smoke.

## Estrategia de pruebas

Compatibility table + actual runners + CRUD/migration smoke.

## Métricas y SLOs de producto/ingeniería

Cada Tier-1 release conserva evidencia storage probe.



## Dependencias y cross-references

ADR-048; release; DATA-MODEL-LADYBUGDB.


---

<!-- SOURCE: specs/license-coherence.md -->

# Spec — License Coherence

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Eliminar contradicción README/Cargo/files legales.

## Scope

Root licenses, Cargo license, README, release artifacts.

## Non-goals

No asesoramiento jurídico.

## Public surface

`check-license-coherence` compara expresión SPDX y archivos requeridos.

## Modelo y semántica

El gate valida coherencia textual/estructural, no compatibilidad jurídica.





## Escenarios Given / When / Then

Given Cargo MIT y README MIT OR Apache, fail.
Given dual metadata + ambos license files, pass.

## Plan de implementación

Maintainers eligen licencia efectiva; files + metadata + gate.

## Estrategia de pruebas

Parser TOML + fixture root.

## Métricas y SLOs de producto/ingeniería

0 diferencias SPDX.



## Dependencias y cross-references

README, README-es, Cargo.toml.


---

<!-- SOURCE: specs/plugin-security-hardening.md -->

# Spec — Plugin Security Hardening

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Cerrar bugs de path/staging e introducir supply-chain boundary segura.

## Scope

Spec parsing, install root, download verify, tar extract, activation, trust.



## Public surface

`archctl plugin inspect/install/verify`; identity value objects; manifest v2.

## Modelo y semántica

Resolve → download → verify → safe extract → manifest validate → atomic activate.

## Errores y comportamiento degradado

Fallo limpia staging y no cambia current.

## Seguridad

No seguir symlink escape; limitar expanded size/files; rechazar devices/FIFO.

## Escenarios Given / When / Then

SCN-PLG-01: `../../evil` se rechaza antes de I/O.
SCN-PLG-02: remote sin sha256 falla cerrado.
SCN-PLG-03: tar `../../outside` no escapa staging.
SCN-PLG-04: first install crea root y funciona.

## Plan de implementación

P0 bugs/tests; P1 manifest; P3 signatures/capability enforcement.

## Estrategia de pruebas

Property tests IDs; malicious tar; mocked HTTP; first-install E2E.

## Métricas y SLOs de producto/ingeniería

100% remote installs verificadas; 0 writes fuera de root.

## Rollout y rollback

Legacy readable como `legacy-unverified`.

## Dependencias y cross-references

ADR-046, ADR-004, plugin modules.


---

<!-- SOURCE: specs/pre-merge-ci.md -->

# Spec — Pre-merge CI

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Impedir que fallos reproducibles entren en main sin benchmarks largos en cada PR.

## Scope

pull_request, post-merge, branch protection, release dependency.



## Public surface

Check names estables; fast workflow y heavy evidence workflow.

## Modelo y semántica

Fast deterministic required; perf/real corpus post-merge o opt-in.





## Escenarios Given / When / Then

Given compile fail, PR red.
Given ADR duplicate, PR falla rápido.
Given perf-only regression, post-merge signal visible.

## Plan de implementación

Add trigger/jobs; cache; observar un ciclo; required; release gate.

## Estrategia de pruebas

Workflow scripts + branch protection verification.

## Métricas y SLOs de producto/ingeniería

Fast median target <10 min; docs gates <1 min.



## Dependencias y cross-references

ADR-047; CI scripts.


---

<!-- SOURCE: specs/sanitized-archbundle.md -->

# Spec — Sanitized `.archbundle`

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Compartir conocimiento offline sin compartir repo.

## Scope

Manifest, graph slice, evidence metadata, policies, capabilities, optional diff/snapshot, redaction, checksums.



## Public surface

`archctl bundle export --profile strict`; inspect; archview read-only.

## Modelo y semántica

Container secundario; manifest/schema contrato. Deny-by-default para source/env/absolute paths/credentials.



## Seguridad

Allowlist > blacklist; unknown metadata excluded until classified safe.

## Escenarios Given / When / Then

Given secret fixture, no source bytes.
Given absolute path, relative/pseudonymized.
Given tamper, checksum fails.

## Plan de implementación

Schema → strict sanitizer → scanner → archview open → profiles later.

## Estrategia de pruebas

Secret corpus; path privacy; deterministic bundle; tamper.

## Métricas y SLOs de producto/ingeniería

0 known secret patterns strict; deterministic manifest hash excluding timestamps.



## Dependencias y cross-references

ADR-055; projection/evidence.


---

<!-- SOURCE: specs/task-context-compiler.md -->

# Spec — Task Context Compiler

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Compilar contexto arquitectónico pequeño y verificable para coding agents.

## Scope

Task query, seeds, graph expansion, rank, evidence/policy/ADR, budget/truncation.



## Public surface

`archctl context compile`; MCP; schema `task-context/1`.

## Modelo y semántica

normalize → lexical/symbol seeds → graph expansion → impact → enrichment → scoring → packing → trace.



## Seguridad

Metadata/evidence locations default; source opt-in/capped.

## Escenarios Given / When / Then

Given CheckoutService, exact seed.
Given 12k budget, estimator no excede.
Given disconnected terms, unknowns.
Given same inputs deterministic, hash equal.

## Plan de implementación

Reuse cognitive/context → scorer ports → GraphReadModel → CLI → MCP → preview.

## Estrategia de pruebas

Golden tasks; budget properties; relevance corpus; no-source leakage.

## Métricas y SLOs de producto/ingeniería

Context reduction >80% vs repo text manteniendo known change surface en corpus.



## Dependencias y cross-references

ADR-052; ImpactView; MCP.

## Ejemplos

Ver `../examples/task-context-output.json`.


---

<!-- SOURCE: specs/workbench-moldable-navigation.md -->

# Spec — Moldable Workbench Navigation

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Coordinar vistas según entidad/tarea mediante identidad estable y semantic zoom.

## Scope

Cross-view identity, zoom, breadcrumbs, action palette, lens recommendation, history.



## Public surface

`NavigationTarget { canonicalId, preferredLens?, focus? }`; renderer IDs nunca canónicos.

## Modelo y semántica

Levels combinan jerarquía C4 y code hierarchy mediante relaciones explícitas.





## Escenarios Given / When / Then

Given container double-click, component lens + breadcrumb.
Given function Up, owning module/component si evidence.
Given auto lens, rationale visible y reversible.

## Plan de implementación

Identity bridge → action palette → semantic zoom → recommendation.

## Estrategia de pruebas

UI integration; back/forward; keyboard; 10k perf.

## Métricas y SLOs de producto/ingeniería

<100ms navigation after data loaded; no full reload lens switch.



## Dependencias y cross-references

ADR-056; archview existing views; capability registry.


---

<!-- SOURCE: specs/workbench-session-security.md -->

# Spec — Workbench Session Security

> **Estado:** Propuesta
> **Fecha:** 2026-08-13
> **Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Tipo:** contrato implementable
> **Regla:** los ejemplos de CLI/JSON son diseño propuesto; pueden evolucionar mediante ADR antes de estabilizarse.

## Purpose

Proteger side-effect endpoints loopback sin auth de usuario.

## Scope

Secret lifecycle, bootstrap, auth, Origin/Host, logging.



## Public surface

`ViewSession { token }`; side effect API requiere secret header.

## Modelo y semántica

Secret per process; nunca workspace state; health puede público.



## Seguridad

No query token que pueda filtrar Referer; usar fragment/bootstrap/header.

## Escenarios Given / When / Then

Given missing token POST open-editor →403.
Given restart old token invalid.
Given static asset →works.
Given non-loopback Host →reject.

## Plan de implementación

Inject session → guard endpoints → bootstrap frontend → tests.

## Estrategia de pruebas

Handler tests + browser integration; no persistent token logs.

## Métricas y SLOs de producto/ingeniería

0 side-effect endpoints sin guard.



## Dependencias y cross-references

ADR-051; view.rs.
