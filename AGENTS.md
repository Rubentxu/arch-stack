# `archctl` — Agent Guidelines

> Documento operativo para agentes de IA y contribuidores humanos que
> trabajen sobre el repositorio `archctl`. La especificación completa
> vive en [`CONTEXT.md`](CONTEXT.md), [`docs/README.md`](docs/README.md)
> y los [ADRs](docs/adr/README.md). Este `AGENTS.md` no contradice esa
> documentación; si lo hace, gana la documentación detallada.

## Project Intent

`archctl` es una **CLI sidecar local** que asiste a un agente OpenCode
(`diagram-architect` orquestador + cuatro subagentes) a producir
diagramas C4 y UML a partir de un repositorio. **Persiste, consulta,
normaliza y proyecta.** No decide qué diagrama hace falta y no
interpreta la arquitectura por su cuenta.

- **Problema**: un agente IA no puede mantener un grafo de arquitectura
  en contexto; necesita persistencia local, evidencia por nodo y
  proyecciones deterministas sin contaminar el repo del usuario.
- **Usuarios**: agentes OpenCode (consumo programático vía custom
  tools) y humanos que auditan diagramas o ejecutan `archctl` a mano.
- **Outcome principal**: el repositorio del usuario **no contiene**
  `.opencode/`, `.architecture/`, `.archctl.yaml` ni `sgconfig.yml`.
  Toda la persistencia vive en XDG (`~/.local/share/archctl/`,
  `~/.config/archctl/`).
- **Prioridades** (en orden): (1) persistencia fuera del repo;
  (2) renderers locales, nunca públicos por defecto; (3) herramientas
  externas envueltas, nunca reimplementadas; (4) performance
  (ADR-019); (5) evidencia por nodo/relación (ADR-005).
- **Compromisos deliberados**: cosmetic-only apply (ADR-013),
  single-writer flock por proyecto (ADR-010), tres modos de skill
  sin copiarlas (ADR-003), Migraciones de schema con runner
  versionado (ADR-017).

> When making changes, optimize primarily for **reproducibilidad y
> auditabilidad local-first**, preserving the strict separation
> between the semantic graph and the user's repository.

## Core Principles

### 1. Persistencia externa al repo (ADR-004)
- **Intención**: el repo del usuario es sagrado; `archctl` no debe
  contaminarlo con archivos de configuración o estado.
- **Consecuencia práctica**: nada de `.archctl/`, `.archctl.yaml`,
  `.opencode/` o equivalentes en el directorio del proyecto. Estado
  solo en XDG.
- **Ejemplo**: el manifest de un proyecto vive en
  `~/.config/archctl/projects/<hash>/manifest.toml`, nunca en el repo.

### 2. Envuelve, no reimplementa (ADR-006)
- **Intención**: `archctl` orquesta adaptadores; no compite con las
  herramientas de análisis de cada ecosistema.
- **Consecuencia práctica**: usar `ast-grep`, `ctags`, `cargo
  metadata`, `go list`, `dependency-cruiser`, `terraform show -json`,
  `helm template`, `kubectl get -o json`, `jdeps`, Syft, etc. vía
  `std::process::Command`. Sin reinventar parsers.
- **Ejemplo correcto**: el adaptador `astgrep.rs` invoca `ast-grep`
  con `--json=stream`; no parsea código fuente en Rust.

### 3. Renderers locales por defecto (ADR-011)
- **Intención**: el contenido del repo no sale a Internet por
  defecto; `plantuml.com` y `kroki.io` están **bloqueados** sin
  opt-in explícito por run.
- **Consecuencia práctica**: usar PlantUML jar local, Structurizr
  CLI o `structurizr/lite` local, y Kroki interno. El adapter de red
  exige un flag.
- **Ejemplo incorrecto**: hacer un GET a `https://kroki.io/...`
  durante `archctl diagram export`.

### 4. Skills upstream en tres modos (ADR-003)
- **Intención**: las skills externas (de Anthropic, OpenCode, etc.)
  se referencian, no se copian.
- **Consecuencia práctica**: cada skill se declara como `direct`,
  `wrapped` o `patched` en `skills.lock.yaml` con `source`, `commit`
  y `license` fijados.
- **Ejemplo correcto**: `skills.lock.yaml` fija
  `source: github.com/anomalyco/opencode` + `commit: abc1234` para
  la skill `sddk-verify` en modo `direct`.

### 5. Evidencia por nodo y arista (ADR-005)
- **Intención**: una afirmación sin evidencia y con confianza alta
  se rechaza. Cada nodo y cada relación del grafo lleva pointers
  a archivos y líneas.
- **Consecuencia práctica**: no crear `Element`, `SemanticRelation`,
  `ElementVersion` o `RelationVersion` sin al menos una evidencia
  resolvible. La regla aplica también a `apply` (cosmetic-only).
- **Ejemplo correcto**: al insertar un `Container`, el adapter
  resuelve `file:line` desde la herramienta externa y lo añade a
  `Element.evidence_refs`.

### 6. Manifest gates como contrato (ADR-022 base)
- **Intención**: cada módulo declara en `manifests/<id>.toml` qué
  símbolos públicos expone, qué patrones `must_hold`, qué tiene
  prohibido (`must_not_contain`) y el mínimo de tests.
- **Consecuencia práctica**: `archctl doctor --scopes <id>` falla
  la build si el módulo viola su propio contrato. Tratar el manifest
  como un test más.
- **Ejemplo correcto**: añadir `pub fn run_apply` exige actualizar
  `manifests/diagram.toml` con `public_symbols += ["run_apply"]`.

### 7. Conventional commits sin Co-Authored-By
- **Intención**: el historial del repo refleja trabajo humano, no
  atribuciones de IA.
- **Consecuencia práctica**: mensajes de commit con prefijos
  `feat`, `fix`, `chore`, `docs`, `test`, `refactor`, `perf`,
  `style`, `build`, `ci`. **Nunca** añadir `Co-Authored-By:
  <modelo>` ni similares.
- **Ejemplo correcto**:
  `feat(diagram): add apply pipeline (T12)`.

## Scope and Boundaries

| Clasificación | Paths / archivos |
|---|---|
| **Safe to modify** | `archctl/src/**/*.rs`, `archctl/tests/**/*.rs`, `archctl/benches/**/*.rs`, `manifests/**/*.toml`, `schemas/**/*.json`, `docs/README.md`, `docs/specs/**`, `CHANGELOG.md`, `ROADMAP.md` |
| **Modify with caution** | `docs/adr/**` (crear ADR nuevo, no mutar ADRs aprobados sin supersede), `Cargo.toml` (lockfile de dependencias), `archctl/src/lib.rs` (exporta el crate), `archctl/src/main.rs` (entry point CLI), `archctl/src/diagram/mod.rs` (registro de módulos públicos) |
| **Do not modify directly** | `CONTEXT.md` (resumen; la verdad vive en `docs/`), `Cargo.lock` (regenerar con `cargo update` solo si el cambio es deliberado), `sddk/<change>/apply-progress.md` y `sddk/<change>/verify-report.md` (artefactos inmutables post-fase) |
| **Generated or external** | `target/`, `.archctl/`, `~/.local/share/archctl/`, `~/.config/archctl/`, `docs/reports/*.html` (releases), `sddk/reports/*.html` (skill output), `sddk/registry.json` (skill-registry) |
| **Untracked (intencional)** | `docs/Librerías-visualización-grafos-BI.md` — documento de investigación del usuario, **nunca commitear** |

## Repository Structure

```
archctl/                          # workspace root
├── archctl/                      # único crate Rust del workspace
│   ├── src/
│   │   ├── main.rs               # entry point CLI
│   │   ├── cli.rs                # clap subcommands + dispatch
│   │   ├── diagram/              # bounded context: bundle export + apply
│   │   │   ├── mod.rs            # registro de módulos + re-exports
│   │   │   ├── export.rs         # run_export (read-only)
│   │   │   ├── export_types.rs   # Projection, Node, Edge (carriers)
│   │   │   ├── queries.rs        # Cypher templates (read)
│   │   │   ├── validate.rs       # bundle validation
│   │   │   ├── apply.rs          # apply pipeline (write)
│   │   │   ├── apply_queries.rs  # Cypher templates (write, MERGE-fallback)
│   │   │   ├── changeset_types.rs
│   │   │   ├── changeset_schema.rs
│   │   │   ├── view_types.rs     # Diagram, ViewMember, ViewGroup
│   │   │   ├── selector.rs       # parse, ViewSelector, C4Kind
│   │   │   ├── hash.rs           # base_revision (blake3)
│   │   │   ├── assets.rs         # icon_for
│   │   │   └── schema_embed.rs   # include_str! del projection schema
│   │   ├── store.rs              # GraphStore port + LbugStore adapter (fs2 flock)
│   │   ├── graph.rs              # domain types: Element, Relation, Evidence
│   │   ├── evidence.rs           # extracción y validación de evidencias
│   │   ├── render.rs             # renderers (PlantUML jar, Structurizr, Kroki)
│   │   ├── skills.rs             # 3-modo skill loader
│   │   ├── astgrep.rs            # adapter: ast-grep --json=stream
│   │   ├── source.rs             # adapter: ctags, universal-ctags
│   │   ├── tsg.rs                # adapter: tree-sitter-graph
│   │   ├── inventory.rs          # inventory.rs
│   │   ├── evaluation.rs         # evaluation rules
│   │   ├── doctor.rs             # doctor --scopes
│   │   ├── scope.rs              # ScopeManifest (parser + gate runner)
│   │   ├── migrations.rs         # ADR-017 schema migration runner
│   │   ├── clock.rs              # Clock port + SystemClock + FixedClock
│   │   ├── filesystem.rs         # Filesystem port
│   │   ├── project.rs            # resolve_project
│   │   ├── xdg.rs                # XDG path resolution
│   │   ├── environment.rs        # env detection
│   │   ├── identity.rs           # project identity
│   │   ├── telemetry.rs          # opt-in telemetry
│   │   ├── row.rs                # result row types
│   │   └── lib.rs                # crate root
│   ├── tests/                    # integration tests
│   │   ├── diagram_apply.rs      # apply pipeline (T15)
│   │   └── …                     # un test file por bounded context
│   └── Cargo.toml
├── manifests/                    # 23 manifest gates (uno por módulo)
│   ├── diagram.toml              # editable + must_hold + must_not_contain
│   ├── store.toml
│   ├── evidence.toml
│   └── …                         # 23 archivos, ver `ls manifests/`
├── schemas/                      # JSON Schemas versionados
│   ├── projection.schema.json
│   ├── changeset.schema.json     # PR2
│   └── …
├── docs/
│   ├── README.md                 # índice de documentación
│   ├── adr/                      # 23 ADRs (000-022)
│   ├── specs/                    # spec por bounded context
│   ├── reports/                  # reports generados (gitignored)
│   ├── DATA-MODEL-LADYBUGDB.md
│   ├── Skills-para-agentes-IA-v2.md
│   ├── Librerías-visualización-grafos-BI.md  # UNTRACKED, user research
│   └── STATE.md                  # estado actual del proyecto
├── sddk/                         # artefactos SDD kernel por change
│   └── <change-name>/
│       ├── explore-report.md
│       ├── proposal.md
│       ├── spec.md
│       ├── design.md
│       ├── tasks.md
│       ├── apply-progress.md
│       ├── verify-report.md
│       ├── debt-report.md
│       ├── archive-report.md
│       └── reports/              # HTML (gitignored)
├── profile/                      # XDG dirs layout reference
├── scripts/                      # scripts operativos
├── CHANGELOG.md                  # Keep a Changelog
├── ROADMAP.md                    # pivotes + cycle log
├── CONTEXT.md                    # resumen del proyecto
└── README.md
```

## Architecture and Design Rules

### Capas
- **CLI** (`cli.rs`): parse args, dispatch, output format.
- **Application** (diagram/, evidence.rs, evaluation.rs): orquesta
  casos de uso. No I/O directo.
- **Ports** (`store`, `clock`, `filesystem`): traits abstractos
  implementados por adapters.
- **Adapters** (`store::LbugStore`, `clock::SystemClock`,
  `filesystem::StdFilesystem`, adaptadores externos en
  `astgrep.rs`/`source.rs`/`tsg.rs`): I/O concreto.

### Reglas de dependencia (verificables)
- `cli` → `application` (diagram, evidence, evaluation).
- `application` → `ports` (store, clock, filesystem) y `domain` (graph).
- `ports` ← `adapters` (LbugStore implementa GraphStore).
- `domain` no importa de `infrastructure` ni de `cli`.
- `archctl/src/diagram/` no importa de `archctl/src/render.rs` (regla
  `must_not_contain` en `manifests/diagram.toml`).
- `archctl/src/diagram/` no usa `serde_json::Value` sin tipo (regla
  histórica; si reintroduces, documenta).

### Modelo de errores
- **Runtime**: `anyhow::Result<T>` en entry points y orchestration.
- **Ports**: `thiserror`-based errors específicos (`LbugError`,
  `FilesystemError`, etc.).
- **Bail con contexto**: `.with_context(|| format!(...))` para que
  la traza de error apunte al caller, no al adapter.

### Concurrencia (ADR-010)
- `archctl` **no es un daemon** en el MVP. No hay Tokio runtime
  principal; subprocess sí es síncrono.
- Concurrencia entre procesos: `fs2::try_lock_exclusive` sobre el
  archivo `.lbdb` del proyecto (RAII en `LbugStore::open`).
- Si dos `archctl` apuntan al mismo proyecto, el segundo falla con
  mensaje claro (`another archctl holds the lock`).

### Persistencia
- **LadybugDB** (lbug) en `~/.local/share/archctl/projects/<hash>/`.
- **Migraciones**: ADR-017 schema migration runner. Cada migración
  registrada en orden; la versión actual se persiste en una tabla
  de metadatos.
- **Schemas versionados**: las proyecciones tienen `schemaVersion`
  (e.g. `"1.0"`, `"1.1"`). Las changesets también (PR2: `"1.0"`).

### APIs
- **CLI subcommands**: `archctl <group> <action> [flags]`. Cada
  grupo = bounded context. Cada acción = use case.
- **JSON output**: flag `--json` en todos los comandos. Formato
  estable por versión.
- **Custom tools OpenCode**: los agentes consumen `archctl` vía
  custom tools (no crate library).
- **MCP**: si aplica, expuesto vía `archctl mcp` con server stdio.

### Extensiones
- **Skills**: 3 modos (`direct`/`wrapped`/`patched`), declaradas en
  `skills.lock.yaml` con `source`, `commit`, `license`. Ver ADR-003.
- **Adapters**: 1 por herramienta externa. Patrón: `archctl-<tool>`
  adapter en `archctl/src/<tool>.rs` o carpeta.
- **Renderers**: registrados en `manifests/render.toml` con
  política de opt-in para externos (ADR-011).

### Architectural Decision Triggers

| Trigger | Acción |
|---|---|
| Cambio de modelo de grafo (nodos/relaciones nuevos) | ADR + `sddk/<change>/design.md` + migration ADR-017 |
| Nuevo renderer público o de red | ADR-011 check + ADR si habilita red saliente |
| Nuevo `Command` CLI top-level | spec + design + tasks en `sddk/<change>/` |
| Nueva dependencia externa (`Cargo.toml`) | ADR + sección en `docs/decisions/` + evaluar ADR-019 perf budget |
| Cambio de port (`GraphStore`, `Filesystem`, `Clock`) | design + tests de contract + actualizar adapter |
| Cambio de schema lbug (DDL) | Migration ADR-017 + bump en `migrations::len()` test |
| Cambio de manifest gate (must_hold/must_not_contain) | Actualizar con justificación en el commit |
| Nueva regla de evidence o rechazo de baja confianza | ADR-005 update + spec.md scenario |

## Change Strategy

1. **Localizar la fuente de verdad**: si la tarea toca un bounded
   context, lee su spec en `docs/specs/` y los ADRs relevantes.
   Para cambios grandes, busca un ciclo en `sddk/` o crea uno.
2. **Identificar el alcance mínimo**: ¿qué archivos cambian? ¿qué
   tests se rompen? ¿qué manifests necesitan actualización?
3. **Revisar código, tests y docs relacionados**: busca con
   `grep`/`ripgrep` todas las referencias. Verifica el manifest
   gate del scope tocado.
4. **Comprobar restricciones arquitectónicas**: ¿la dependencia
   propuesta respeta la dirección de imports? ¿rompe un
   `must_not_contain`? ¿requiere un ADR?
5. **Realizar el cambio más pequeño que satisfaga el objetivo**:
   un work-unit por task (ver `sddk/<change>/tasks.md`).
6. **Validar el comportamiento**: `cargo test --quiet` + `cargo
   run --bin archctl -- doctor --scopes <id> --cwd <root>` si el
   scope aplica.
7. **Revisar efectos secundarios**: CHANGELOG, ROADMAP, docs/.
8. **Actualizar documentación cuando proceda**: nunca aspiracional;
   describe el comportamiento real.

### Reglas adicionales
- No realizar refactorizaciones no relacionadas.
- No cambiar APIs públicas accidentalmente.
- No añadir abstracciones sin un caso real.
- No corregir silenciosamente otros problemas fuera del alcance.
- Comunicar hallazgos relevantes que no formen parte del cambio
  (en el mensaje del commit o en `sddk/<change>/apply-progress.md`).

## Build Commands

```bash
# Compilar en modo debug
cd archctl && cargo build

# Compilar en modo release
cd archctl && cargo build --release

# Build de un binario específico
cd archctl && cargo build --bin archctl

# Build con todos los features
cd archctl && cargo build --all-features

# Verificar que compila sin warnings de clippy
cd archctl && cargo build --quiet 2>&1 | tail -5

# Artefacto generado
./archctl/target/debug/archctl --version
./archctl/target/release/archctl --version
```

> Advertencia: el primer build descarga e indexa crates.io; usar
> `sccache` si está disponible para reducir tiempo en CI.

## Test Commands

```bash
# Suite completa (lib + integration + doctest)
cd archctl && cargo test --features test-fixtures

# Modo silencioso (sólo summary)
cd archctl && cargo test --features test-fixtures --quiet

# Solo tests de librería
cd archctl && cargo test --lib

# Solo integration tests
cd archctl && cargo test --test <name>

# Test individual por nombre
cd archctl && cargo test --lib <module>::<test_name>

# Filtrar tests con substring
cd archctl && cargo test --features test-fixtures <substring>

# Doctests
cd archctl && cargo test --doc

# Manifest gate (validación de contrato de scope)
cd archctl && cargo run --bin archctl -- doctor --scopes <id> --cwd <repo_root>

# Validación mínima durante desarrollo
cd archctl && cargo test --features test-fixtures --quiet && \
  cargo run --bin archctl -- doctor --scopes <scope> --cwd <repo_root>

# Validación completa antes de PR (sin CI)
cd archctl && cargo test --features test-fixtures --quiet && \
  cargo clippy --quiet --features test-fixtures -- -D warnings && \
  cargo fmt --check && \
  cargo run --bin archctl -- doctor --scopes <id>,<id2> --cwd <repo_root>

# Validación de CI (los mismos comandos que corren en la nube)
cd archctl && cargo test --features test-fixtures && \
  cargo clippy --features test-fixtures --all-targets -- -D warnings
```

## CI Policy — local-first, cloud async

**El gate de verificación es LOCAL, no la nube.**

- **Gate real**: los gates del ciclo SDDK (verify/debt-verify) para trabajo
  de ciclo; `scripts/verify-local.sh` (cheap mode: build + test + clippy +
  fmt + doctor scopes) ejecutado **manualmente** antes de un push fuera de
  ciclo. Si pasa localmente, el push procede. **No esperes runs de GitHub
  Actions para mergear** ni los trates como bloqueantes — son evidencia
  asíncrona.
- **GitHub Actions es post-hoc**: `ci.yml` (post-merge en main) y
  `pr.yml` (informativo en PRs) corren en la nube sin required status
  checks. Si fallan, se abre fix en el siguiente cambio; no se
  reverte ni se bloquea el merge.
- **Ejecutar workflows en local**: `act` (v0.2.89, `/usr/local/bin/act`)
  + podman con `ubuntu-latest` mapeado a `catthehacker/ubuntu:rust-latest`
  (`~/.config/act/actrc`). Ejemplo: `act pull_request -W .github/workflows/pr.yml`.
- **Prohibido**: `gh pr checks --watch`, demorar un merge esperando la
  nube, o "arreglar CI" sin antes reproducir localmente.
- **Historical**: existió un pre-push hook per-commit (ADR-025); eliminado
  2026-08-14 por redundante con los gates SDDK y su tax O(N) por push.
- Nota: los runners de GH exportan `XDG_CONFIG_HOME`; si un test
  depende de la forma de paths bajo `$HOME`, pinea ambas variables
  (ver `archctl/tests/ide_config_root_paths.rs`).

## Validation Matrix

| Tipo de cambio | Build | Test | Lint/Format | Doc | ADR | Notas |
|---|---|---|---|---|---|---|
| Doc solo (markdown) | – | – | – | sí | – | Verificar links relativos |
| Bug fix localizado | sí | sí | sí | CHANGELOG | – | Test de regresión obligatorio |
| Nueva funcionalidad | sí | sí | sí | CHANGELOG + ROADMAP | según trigger | Spec + design + tasks en `sddk/` |
| Refactor (sin cambio de API) | sí | sí | sí | – | – | Mantener diff < 400 líneas o split en chain |
| Cambio de API pública | sí | sí | sí | CHANGELOG | sí | Anunciar deprecation 1 minor antes |
| Cambio de schema lbug | sí | sí | sí | migration | sí | Bump `migrations::len()` test |
| Nueva dependencia | sí | sí | sí | – | sí | Evaluar ADR-019 perf budget |
| Cambio de build (Cargo.toml sin deps) | sí | sí | – | – | – | Justificar en commit message |
| Cambio de CI | – | – | – | CI doc | – | – |
| Modificación de código generado | – | – | – | – | – | No commitear (ver `Do not modify`) |

## Code Style

- **Formatter**: `rustfmt` con `rustfmt.toml` defaults del proyecto
  (si existe). Comando: `cargo fmt` (workspace-wide) o
  `rustfmt <file>` para un archivo específico.
- **Gotcha `cargo fmt`**: `cargo fmt` y `cargo fmt -- <file>` formatean
  **el workspace entero**, no el archivo pasado. Para format
  incremental en archivos staged, usar `scripts/fmt-staged.sh` (check
  mode por defecto, `--apply` para reformatear + re-stage).
- **Check**: `cargo fmt --check` en pre-commit y CI.
- **Linter**: `cargo clippy -- -D warnings` (todos los warnings son
  errores).
- **Análisis estático**: `cargo clippy --all-targets` antes de PR.
- **Convenciones de nombres**:
  - `snake_case` para funciones, variables, módulos, archivos.
  - `CamelCase` para tipos, traits, enums.
  - `SCREAMING_SNAKE_CASE` para constantes.
  - `kebab-case` para flags CLI (`--project-root`, no
    `--project_root`).
  - `PascalCase` para subcommands y bounded contexts.
- **Organización de módulos**:
  - 1 bounded context por carpeta bajo `archctl/src/`.
  - `mod.rs` solo cuando hay sub-módulos.
  - `pub mod` para módulos públicos; sin `pub` para internos.
  - Port traits y adapters en archivos separados.
- **Comentarios**:
  - `///` para rustdoc en items públicos.
  - `//!` para module-level docs.
  - Comentarios inline para explicar el **por qué**, no el qué.
  - `// TODO:` solo si está abierto; `// FIXME:` para bugs
    conocidos.
- **Documentación de APIs**: rustdoc en todos los items `pub`.
  Ejemplos en `///` cuando aporten valor (no ejemplos triviales).
- **Tratamiento de warnings**: cero warnings nuevos. Si un warning
  viene de un dep upstream, aislar en un módulo propio y
  documentar.
- **Código muerto**: prohibido. Si una función pública se elimina,
  deprecar 1 minor antes con `#[deprecated]`. Si es interna,
  eliminar.
- **Supresiones de lint**: prohibidas sin justificación documentada
  en el código (e.g. `// allow: serde renombra el campo para
  compat con bundle v1`).

## Testing Principles

- **Comportamiento observable**: testea la salida del CLI o el
  valor de retorno de la API pública, no el estado interno de un
  módulo.
- **Regresiones**: 1 test mínimo por bug. El test nombra el bug en
  su doc comment o nombre.
- **Casos límite**: empty, single, large, malformed, unicode,
  boundaries. Los tests unitarios cubren los triviales; los
  integration tests cubren los complejos.
- **Errores esperados**: cada rama de error tiene un test que
  verifica el mensaje y la causa (`err.to_string().contains("...")`).
- **Determinismo**: nada de fechas reales; usar `FixedClock`. Nada
  de randomness sin seed fija. Nada de orden de HashMap en
  aserciones de igualdad (usar `BTreeMap` o sorted Vec).
- **Aislamiento**: cada test crea su `TempDir`. Cleanup con RAII
  (`tempfile::TempDir`). No compartir estado entre tests.
- **Fixtures**: en `archctl/tests/fixtures/`, generadas por
  scripts reproducibles, no commiteadas si son grandes.
- **Mocks**: solo para I/O externo (network, clock). El adapter
  lbug se testea con `LbugStore::open` real + TempDir.
- **Tests frágiles**: evitar `sleep`, polling, y aserciones sobre
  strings de error exactos. Preferir `contains` o `starts_with`.
- **Concurrencia**: tests de lock con `fs2::try_lock_exclusive`
  sostenido en un `File` aparte; verificar mensaje de error.
- **Performance**: si aplica, benchmark con `criterion` en
  `archctl/benches/`. No micro-optimizar sin benchmark.

### Cuándo añadir/actualizar tests
- **Añadir nuevo**: cualquier feature nuevo comportamiento o nuevo
  branch de error.
- **Actualizar existente**: cuando el bug fix cambia el
  comportamiento esperado.
- **Evitar mocks**: cuando puedas usar el adapter real con TempDir.
- **Integration test**: cualquier ruta que toque lbug, filesystem,
  o subprocess.
- **Regression test**: obligatorio para todo bug fix.

## Dependencies

- **Comprobar si ya existe**: antes de añadir una dep, buscar en
  `Cargo.toml` y en transitivas. Si ya está, usarla.
- **Preferir stdlib**: cuando el stdlib cubra el caso, no añadir
  deps. Ejemplo: `std::process::Command` en vez de un wrapper.
- **Evaluar antes de añadir**:
  - Mantenimiento: ¿último commit > 2 años? ¿autor único?
  - Licencia: compatible con `MIT OR Apache-2.0` del workspace.
  - Estabilidad: ¿pre-1.0? Semver-respetuoso?
  - Seguridad: ¿CVEs conocidos? ¿auditada?
  - Tamaño: ¿añade 5MB compilados para 200 líneas? Out.
- **Evitar deps para funcionalidades triviales**: nada de
  `chrono` para un solo `Utc::now()` si `std::time` sirve. Nada
  de `serde_yaml` si `serde_json` cubre.
- **Fijar versiones exactas** en `Cargo.toml` con
  `version = "=X.Y.Z"` cuando la estabilidad es crítica. Para
  crates maduros, dejar el resolver elegir.
- **Lockfile**: `Cargo.lock` está commiteado. No regenerar
  manualmente.
- **Compatibilidad**: MSRV documentado en `archctl/Cargo.toml`
  `rust-version`. Sin bump sin ADR.
- **Aprobación requerida**: deps de sistema (lbug, structurizr
  CLI, plantuml jar) requieren opt-in explícito del usuario.

> Do not add a new dependency unless its value exceeds its long-term
> maintenance cost.

## Security and Sensitive Data

- **Secretos**: nunca en el repo. Variables de entorno o prompt.
- **Credenciales**: en `~/.config/archctl/` con permisos `0600`.
- **Tokens**: ephemeral, scoped, revocables.
- **Datos personales**: nunca en fixtures, ejemplos, o tests.
- **Logs**: redactar secrets (`tracing-subscriber` con
  `EnvFilter` + redactor). No loggear paths completos de usuario
  en producción.
- **Archivos de configuración**: en XDG, no en repo. Con permisos
  restrictivos si contienen credenciales.
- **Ejemplos**: datos sintéticos (`example.com`, `user@example.com`).
- **Fixtures**: sintéticos y minimalistas. Sin código real.
- **Telemetría**: opt-in (flag `--telemetry` o `~/.config/archctl/
  telemetry.toml`). Documentar qué se envía.
- **Dependencias**: auditar antes de añadir. `cargo audit` en CI.
- **Comandos destructivos**: requieren `--yes` o scope explícito.
  Loggear siempre qué se borró/modificó.
- **Acceso a red**: bloqueado por defecto (ADR-011). `reqwest` y
  `use reqwest` están prohibidos en `manifests/diagram.toml`.
- **Ejecución externa**: solo vía adapter (ADR-006). El adapter
  declara el comando, los args permitidos, y el timeout. Sin
  shell expansion de input del usuario.

## Performance and Resource Usage

- **Performance budget** (ADR-019):
  - `archctl diagram export <C4 view>`: p99 < 2s para grafos
    < 10K nodos.
  - Cold start del binario: < 100ms.
  - RSS en idle: < 50MB.
  - Bundle export: < 1MB para C4 standard (5 archivos).
- **Memory**: bounded por el modelo de grafo. Evitar cargar el
  grafo completo en memoria si se puede paginar (e.g. iteradores
  sobre queries lbug).
- **Latencia**: cada adapter externo tiene timeout (e.g. 30s para
  `ast-grep`). Si excede, fallar con `anyhow!` claro.
- **Complejidad algorítmica**: O(n) en traversals del grafo. O(n*m)
  aceptable para joins sobre dominios pequeños. O(n²) prohibido.
- **Asignaciones**: pre-allocate con `Vec::with_capacity` cuando
  el tamaño es conocido. Reusar buffers en hot paths.
- **Concurrencia**: Tokio solo donde hay I/O. En CLI síncrono, no
  añadir runtime. Si en el futuro hay daemon, evaluar `tokio` vs
  `smol` con ADR.
- **I/O**: buffered reads (`BufReader`). Sin llamadas `read_to_end`
  sobre archivos de tamaño desconocido sin límite.
- **Tamaño de artefactos**: bundles < 1MB, schemas < 10KB, lockfiles
  < 100KB.
- **Tiempos de build**: `cargo build` incremental < 30s en hardware
  medio. Si supera, evaluar `sccache` y dividir crates.

## Compatibility and Migrations

- **Versiones**: semver estricto. Major = breaking, minor =
  feature, patch = fix.
- **API pública backward compatible** dentro del mismo major.
  Deprecated APIs sobreviven al menos 1 minor con warning.
- **Formato de datos**: `serde_json` con `serde(rename_all =
  "...")` determinista. `schemaVersion` explícito en cada JSON
  top-level.
- **Esquemas JSON Schema**: versionados con `$id` y `$schema`. Un
  bump de major del schema = bump de minor de `archctl`.
- **Protocolos**: si OpenCode MCP, LSP-style con `Content-Length`
  framing.
- **Migraciones**: ADR-017 schema migration runner. Cada
  migración idempotente. La versión actual se persiste en una
  tabla `_meta`. Nunca drop manual; siempre a través del runner.
- **Deprecations**: anuncio 1 minor antes. Marcar con
  `#[deprecated(since = "x.y", note = "...")]` y mantener
  funcionando.
- **Feature flags**: scopes (`doctor --scopes <id>`) para opt-in
  por módulo. Si una feature requiere opt-in, vive tras un scope
  flag.
- **Ventanas de transición**: 2 minor versions. Después, breaking
  change en major.

## Documentation Rules

- **README.md**: mantenido al día. Comandos principales listados.
- **CONTEXT.md**: resumen, no cambia a menudo. Si cambia, es
  porque cambió un ADR.
- **docs/README.md**: índice de docs. Actualizar al añadir docs.
- **docs/adr/**: ADRs con formato
  `ADR-NNN-titulo-corto.md`. Estado: `proposed`, `accepted`,
  `superseded`. Un ADR superseded no se borra; se enlaza al nuevo.
- **docs/specs/**: spec por bounded context. Scenarios Given-When-Then.
- **CHANGELOG.md**: Keep a Changelog format
  (`Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`).
- **ROADMAP.md**: pivotes y cycle log. Un ciclo = un `m<N>` o
  `b<N>` en `sddk/`.
- **Comentarios**: el código documenta el **qué**; los
  comentarios documentan el **por qué**.
- **Diagramas**: actualizados cuando cambia arquitectura.
  Herramientas: Structurizr DSL (C4) o PlantUML (UML), renderizados
  por `archctl` mismo.
- **API ref**: rustdoc generada. `cargo doc` para validar.

> La documentación describe el comportamiento real, no aspiraciones
> futuras. Si el código aún no implementa lo que dice el doc, es
> un bug.

## Git and Change Hygiene

- **Alcance de commits**: 1 task = 1 commit. Si un commit toca
  archivos no relacionados, dividir.
- **Mensajes de commit**: conventional commits.
  - Formato: `<type>(<scope>): <subject>`.
  - Subject en imperativo, sin punto final, < 72 chars.
  - Body con motivación, qué, por qué. Wrap a 72.
  - Footer con `Refs:`, `BREAKING CHANGE:`, o links a issues.
  - Tipos: `feat`, `fix`, `chore`, `docs`, `test`, `refactor`,
    `perf`, `style`, `build`, `ci`.
  - **Nunca** añadir `Co-Authored-By: <IA>` ni equivalentes.
- **Generated files**: nunca commitear
  (`target/`, `.archctl/`, `docs/reports/*.html`,
  `sddk/reports/*.html`, `sddk/registry.json`).
- **Lockfiles**: sí (`Cargo.lock` está commiteado).
- **Format masivo**: commit aparte con `chore(fmt): rustfmt sweep`.
  Nunca dentro de un commit de feature.
- **Merges**: `--no-ff` para PRs de feature. Squash para PRs
  pequeños o single-commit.
- **Rebases**: prohibidos en branches compartidos. Permitidos en
  feature branch personal antes del PR.
- **Cambios no relacionados**: prohibidos en el mismo commit. Si
  un drive-by fix es necesario, commit aparte.
- **History**: no force-push en `main`. No reescribir historia
  compartida.
- **Artefactos temporales**: en `/tmp/opencode/` o como `untracked`
  intencional. Ver `Scope and Boundaries`.

## Definition of Done

Cada item debe responderse con **sí** o **no**:

- [ ] `cargo build --quiet` termina con código 0.
- [ ] `cargo test --quiet` pasa todos los tests (lib + integration
      + doctest).
- [ ] `cargo clippy --quiet -- -D warnings` pasa sin warnings.
- [ ] `cargo fmt --check` pasa.
- [ ] Si el scope aplica: `cargo run --bin archctl -- doctor
      --scopes <id> --cwd <repo_root>` reporta 0 findings.
- [ ] No hay warnings nuevos en el diff.
- [ ] Las reglas arquitectónicas se preservan (no se introducen
      imports prohibidos por manifests).
- [ ] `CHANGELOG.md` actualizado si el cambio es user-facing.
- [ ] `ROADMAP.md` actualizado si el cambio cierra o abre un ciclo.
- [ ] ADR creado si el trigger aplica (ver tabla arriba).
- [ ] Test de regresión añadido si es bug fix.
- [ ] No hay secretos, credenciales, ni archivos temporales
      (`/tmp/`, `.archctl/`, `target/`, `docs/reports/*.html`).
- [ ] El cambio es revisable: < 400 líneas netas, o se spliteó
      en chain de PRs.
- [ ] `docs/Librerías-visualización-grafos-BI.md` no se tocó
      (sigue untracked).
- [ ] El work-unit commit message explica el por qué, no solo el
      qué.

## Failure and Recovery Guidance

- **Build falla**: no avanzar. Reproducir en limpio
  (`cargo clean && cargo build`). Reportar causa raíz en el commit
  o en `sddk/<change>/apply-progress.md`.
- **Test falla**: nunca silenciar con `#[ignore]` o `#[cfg(...)]`
  sin justificación. Si es flaky, documentar y abrir issue.
  Distinguir regresión de test que captura bug pre-existente.
- **Pre-existing fail**: marcar como tal en el commit message
  (`Refs: pre-existing test failure, tracked in #N`). No hacer
  pasar el build silenciando.
- **Herramienta falta**: documentar en `Open Questions` y
  workaroundear. Si es bloqueante, abortar y reportar.
- **Contradicción entre docs**: respetar la más específica. Si es
  entre AGENTS.md y un ADR, el ADR gana (AGENTS.md es un resumen).
- **Doc desactualizado**: abrir issue. No inventar comportamiento.
- **No se puede reproducir**: pedir datos mínimos
  (input, OS, versión de `archctl`, versión de lbug). Si es
  imposible, marcar `validated: <alcance>` en el commit.
- **Entorno no permite validación**: marcar `validated: manual
  only` o `validated: static-only`. No declarar éxito sin
  comprobación.
- **lbug session no comparte uncommitted data entre `LbugStore::open`**:
  usar `apply_to_store(store, changeset)` con un store pre-abierto
  para tests que seedean y aplican en la misma sesión.

## Instruction Precedence

1. Instrucciones explícitas del usuario o mantenedor.
2. Reglas de seguridad (no secretos, no red por defecto, etc.).
3. `AGENTS.md` de scope más cercano (si existiera, p.ej.
   `docs/specs/diagram/AGENTS.md`).
4. `AGENTS.md` del workspace (este archivo).
5. ADRs aprobados en `docs/adr/`.
6. Specs y diseño en `sddk/<change>/` y `docs/specs/`.
7. Convenciones observadas en el código (rustfmt, clippy, git
   log reciente).
8. Preferencias generales del agente.

> Una instrucción más específica prevalece sobre una general,
> salvo que vulnere seguridad o restricciones explícitas.

## Additional Documentation

- [`CONTRIBUTING.md`](CONTRIBUTING.md) — practical handbook for new
  contributors (cycle workflow, manifest hygiene, bounded context
  rules, what NOT to do). **Read this first** if you are starting a
  cycle. Added in M57.
- [`CONTEXT.md`](CONTEXT.md) — resumen ejecutivo del proyecto.
- [`docs/README.md`](docs/README.md) — índice de toda la
  documentación.
- [`docs/adr/`](docs/adr/README.md) — Architecture Decision
  Records (23 ADRs, 000-022).
- [`docs/DATA-MODEL-LADYBUGDB.md`](docs/DATA-MODEL-LADYBUGDB.md) —
  modelo de grafo canónico (lbug).
- [`docs/Skills-para-agentes-IA-v2.md`](docs/Skills-para-agentes-IA-v2.md) —
  sistema de skills en 3 modos.
- [`docs/STATE.md`](docs/STATE.md) — estado actual del proyecto.
- [`docs/ROADMAP.md`](docs/ROADMAP.md) — pivotes y cycle log.
- [`docs/specs/`](docs/specs/) — specs por bounded context.
- [`sddk/`](sddk/) — artefactos SDD kernel por change.
- [`CHANGELOG.md`](CHANGELOG.md) — historial de versiones.
- [`README.md`](README.md) — entrada principal.
- [`Cargo.toml`](archctl/Cargo.toml) — deps y MSRV.
- [`manifests/`](manifests/) — manifest gates (23 archivos).

## Open Questions

- **MSRV de Rust**: no documentado explícitamente. El código usa
  `?` en `main`, `let else`, `impl Trait`, y traits asociadas.
  Probable MSRV ≥ 1.75. **Recomendación**: documentar en
  `archctl/Cargo.toml` `rust-version = "1.75"`.
- **Soporte de Windows**: el código usa `std::os::unix::fs` en
  algunos puntos (vía `fs2`) y subprocess de `java`/`ast-grep`
  en `$PATH`. **Recomendación**: confirmar scope; si Windows es
  out-of-scope, documentar en `README.md`.
- **Política exacta de deprecation de CLI commands**: no escrita.
  **Recomendación**: añadir sección en
  `docs/specs/cli-lifecycle.md` con ventana de 2 minor versions.
- **Cuándo añadir benchmark vs test de performance**: no hay guía.
  **Recomendación**: criterio "cualquier ruta de export que tocare
  > 1K nodos requiere benchmark en `archctl/benches/`".
- **Política de telemetry opt-in vs opt-out**: la sección
  `telemetry.rs` existe pero no hay decisión documentada. **Recomendación**:
  ADR explícito antes de habilitar.
- **Manejo de cambios en `skills.lock.yaml` cuando una skill
  upstream cambia de licencia**: no documentado. **Recomendación**:
  añadir a `docs/Skills-para-agentes-IA-v2.md` sección de
  re-licensing.
