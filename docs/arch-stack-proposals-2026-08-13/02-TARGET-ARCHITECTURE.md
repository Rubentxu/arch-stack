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
