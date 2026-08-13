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
