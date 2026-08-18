# ADR-016 — Encaje de `activegraph-packs` en `archctl`

**Estado:** Investigación cerrada. Decisiones por bloque:
- **B1 (Evidence graph model — Source → Evidence → ...):** Decidido y embodied en [ADR-017 §Schema migration runner](ADR-017-schema-migration-runner.md) (que a su vez implementa el B1 source/eval types via `archctl/src/migrations.rs`). Ver también P2-09a compat carriers (v1.58.0).
- **B2 (Manifest + content_hash + static gates por scope):** Pendiente. Reopen trigger: ≥1 scope con gate estático rompiendo en CI real OR per-scope manifest-versioning requerido por Wave 3.
- **B3 (Trust-by-origin en extractor — `archctl::evidence::extract`, `archctl::tsg`):** Pendiente. Reopen trigger: ≥1 disclosed extractor origin-confusion CVE OR ≥1 UAT report demostrando que `validate_identifier` no es suficiente.
**Fecha:** 30 de julio de 2026 (investigación original); 2026-08-18 (Status clarification).
**Investigado:** `https://github.com/yoheinakajima/activegraph-packs` rev `main`
(143 commits, 28 packs, 9 docs, Apache-2.0).
**Fuente única consolidada:** [docs/STATE.md](STATE.md) describe el estado de
`archctl` al cierre de la sesión anterior.

---

## TL;DR

`activegraph-packs` NO se adopta como dependencia. Lo que aporta — modelo de
event-sourced object graph con packs débilmente acoplados, gates estáticos
para self-modification con provenance, manifest-versioned surface, y un
threat model "trust by origin, never by content" — es **fuente de ideas**
para tres bloques de mejoras concretas en `archctl`:

1. **B1 — Evidence graph model**: tratar `archctl` como ingester
   `Source → Evidence → ...` estilo ActiveGraph Core. No requiere
   código externo; reorganiza lo que ya tenemos.
2. **B2 — Manifest + content_hash + static gates**: dar a
   `archctl` un `manifest.toml` versionado por scope (M0–M16), con
   gate estático que verifique que el código cumple el contrato del
   manifest. Esto es directamente compatible con el principio
   operativo del usuario: *"No leo el código que escriben mis agentes.
   Los rodea de controles como tests, análisis de mutaciones y métricas
   de calidad."*
3. **B3 — Trust-by-origin para el extractor**: el threat model de
   `archctl::evidence::extract` y `archctl::tsg` debe clasificar el
   input por origen (cwd vs archivo del proyecto vs config del usuario
   vs salida de tool externo), no por contenido. Hoy el código confía
   en que `validate_identifier` es suficiente; el threat model de
   ActiveGraph muestra que eso no escala.

El reporte también documenta **lo que NO se aplica** y por qué, para que
la decisión quede registrada y no haya que redescubrirla.

---

## 1. Qué es `activegraph-packs` (resumen ejecutivo)

Es una **librería de packs** sobre el runtime [ActiveGraph](https://pypi.org/project/activegraph/)
(grafo de objetos reactivo en Python, con event sourcing). Cada pack es
un módulo Python autocontenido con:

- **Tipos de objeto y relación** propios (Pydantic).
- **Behaviors** que reaccionan a eventos del grafo.
- **Tools** que las behaviors pueden invocar.
- **Settings** validados con defaults.
- **Prompts** para LLMs.
- **Fixtures** deterministas ejecutables sin API key.
- **Manifest TOML** declarativo con `content_hash` sha256.

28 packs en el repo, organizados en tres tiers:

| Tier | Packs | Función |
|---|---|---|
| **Core** | `core` | 7 tipos universales (`source`, `observation`, `task`, `action`, `artifact`, `memory_candidate`, `evaluation`) y 7 relaciones. El "API mínima" entre packs. |
| **Infrastructure** | `tool_gateway`, `secrets`, `memory_gateway`, `identity_auth`, `agent_profile`, `entity`, `schedule` | Capacidades cross-cutting que cualquier asistente necesita. |
| **Communication** | `communication`, `chat`, `telegram`, `whatsapp`, `email` | Capa de mensajería. |
| **Domain** | `research`, `codebase`, `team_ops`, `meeting` | Verticales específicos. **No son agénticos sobre otros packs**: consumen infra, no la duplican. |
| **Bridge** | `bridges/diligence_core_bridge` | Mapea outputs de un pack third-party a tipos Core. |

Tres invariantes son los que sostienen el modelo (citados literalmente
desde `docs/concepts.md`):

> 1. **Core stays small** — only the 7 universal primitives, forever.
> 2. **Packs compose through graph state, not function calls** — no direct
>    calls, no central coordinator.
> 3. **Packs degrade gracefully** — hard-require only what you truly need;
>    everything else is `integrates_with`.

Lo más distintivo: **la coordinación es emergente**. Una behavior en un
pack escribe un objeto; esa escritura es un evento; ese evento dispara
una behavior en otro pack. No hay orquestador. Esto se llama **"event-
sourced object graph"** — el log de eventos es la fuente de verdad, y
replayear el log reconstruye el grafo.

Tres demos lo demuestran:

- **Inspector UI** (React + Vite) que muestra el grafo en tiempo real.
- **API server** (Express) que proxy + spawn al runtime Python.
- **Demo server** (Python standalone HTTP) que carga packs desde entry
  points y expone el grafo por REST.

Hay un Evolution Pack que permite al assistant **modificarse a sí
mismo** con provenance completa (gates estáticos + fork trial + owner
approval + bundle hash pin). Esto es la pieza más interesante para
`archctl` porque encaja exactamente con nuestro principio operativo.

---

## 2. Por qué `archctl` no se mete dentro de `activegraph-packs`

Tres razones de fondo:

### 2.1 Stack distinto

`activegraph-packs` corre sobre Python (`activegraph` es una lib
Python). `archctl` es Rust. Adoptar el modelo conceptual es viable;
adoptar el código como dependencia es virtualmente imposible sin
reescribir ActiveGraph en Rust (lo cual es un proyecto de un año, no
un sprint).

### 2.2 Caso de uso distinto

ActiveGraph es un **runtime reactivo con agentes LLM** que reciben
chat y producen actions en el mundo. `archctl` es una **CLI sidecar
estática** que ingiere un codebase y produce un grafo C4/UML. El
runtime reactivo no encaja — `archctl` no escucha eventos, no tiene
behaviors que reaccionan, no tiene schedule pack.

### 2.3 Madurez

`activegraph-packs` rev `main` tiene 143 commits, 7 stars, 1 fork,
0 issues, 0 PRs abiertos. Es un proyecto individual de un autor
(Yohei Nakajima) en etapa temprana. Adoptarlo como dependencia nos
ataría a un roadmap que el usuario no controla. El Evolution Pack
admite en su `replit.md` que *"the agent authors pack behind static
gates"* — el propio proyecto reconoce que es experimental.

**Decisión**: `activegraph-packs` es **fuente de patrones**, no
dependencia. Lo que se toma es el modelo conceptual y los principios
de diseño. Lo que se deja es el código.

---

## 3. Bloque B1 — Evidence graph model (reorganización)

### 3.1 El modelo Core de ActiveGraph mapea casi 1:1 a `archctl`

ActiveGraph Core tiene 7 tipos. `archctl` ya tiene varios análogos:

| Core type | `archctl` actual | Gap |
|---|---|---|
| `source` | (implícito en `cwd` / path argumento) | No hay objeto `Source` explícito en el grafo. `inventory::tree` emite entries pero no se persisten como nodos. |
| `observation` | `Evidence` | Sí existe. Pero no hay lifecycle (`drafted → accepted → superseded`). |
| `task` | `Task` (ROADMAP) | Solo en docs. |
| `action` | `evidence::put` log de queries | Implícito, no hay objeto `Action` persistido. |
| `artifact` | `archctl diagram` outputs | Sí existe, pero en filesystem, no en grafo. |
| `memory_candidate` | (no existe) | **Gap real**. Hoy `evidence::put` escribe directamente. No hay evaluation step. |
| `evaluation` | (no existe) | **Gap real**. No hay quién diga "esto es válido" antes de persistir. |

### 3.2 Lo que esto implica

`archctl` tiene los **datos** del modelo Core pero no su **lifecycle**.
Hoy:

```rust
evidence::put() → MERGE (e:Evidence ...) → graph
```

Mañana (modelo Core):

```rust
1. source: created          (file walked, captured as Source node)
2. observation: extracted   (tsg/ast-grep matches → Evidence node)
3. evaluation: created      (does this evidence meet the threshold?)
4. observation: accepted    (Evidence becomes part of the canonical graph)
```

**Impacto en `archctl`**: M9 (renderers) y M14 (versionado) se
benefician de tener un `Source` explícito en el grafo. Hoy una
re-extracción borra evidencia previa por accidente porque no hay
trazabilidad al source original.

**Acción concreta propuesta (no bloqueante)**: introducir dos tipos
en el schema (`Source`, `Evaluation`) en el próximo milestone que
toque el grafo. No es una refactor — es una evolución del schema.

### 3.3 Lo que NO tomar

- **No** tomar la idea de "el log de eventos es la fuente de verdad,
  replay reconstruye el grafo". `archctl` no necesita event sourcing
  — los diagramas se reconstruyen re-corriendo el extractor, no
  re-leyendo un log. La complejidad operativa del event sourcing no
  compensa cuando la fuente de verdad es el repo del usuario (que
  ya tiene su propio control de versiones).
- **No** tomar la idea de "todos los packs escriben en el mismo
  grafo centralizado". `archctl` emite bundles JSON para que
  `archview` los consuma. La separación producer/consumer es buena
  (ver ADR-013).

---

## 4. Bloque B2 — Manifest + content_hash + static gates

### 4.1 Por qué esto encaja con el principio operativo del usuario

> *"No leo el código que escriben mis agentes. En su lugar, los
> rodea de controles como tests, análisis de mutaciones y métricas
> de calidad."*

El Evolution Pack de ActiveGraph resuelve exactamente este problema
para el caso donde un LLM escribe código Python que se carga en un
runtime vivo. El mecanismo es un `manifest.toml` declarativo + once
gates estáticos:

1. `static:reserved_paths` — el agente no puede escribir archivos protegidos.
2. `static:file_set` — solo el conjunto fijo de archivos permitido.
3. `static:trial_driver` — el driver de trial es verbatim, byte-for-byte.
4. Manifest validity (parse + schema + version + runtime range).
5. Hash integrity (content_hash + bundle_hash).
6. Declared-vs-actual (todo lo declarado en el manifest existe; nada declarado existe).
7. Import allow-list (stdlib corto + pydantic + intra-pack).
8. Banned constructs (`exec`, `eval`, `__import__`, dunder access).
9. Reserved namespaces (no chocar con `tool_gateway.*`).
10. Size caps (bytes totales y por archivo).
11. Injection scan sobre todo el source.

### 4.2 Mapeo a `archctl`

`archctl` no se auto-modifica, pero **tiene scope equivalente**:
un agente (humano o LLM) escribe `archctl/src/*.rs` + `docs/adr/*.md`
y debe cumplir contratos que hoy son implícitos.

**Propuesta concreta**: introducir un `manifest.toml` por scope que
declare:

```toml
[scope]
id = "M3-evidence-pipeline"
version = "0.1.0"
description = "Evidence extraction, TSG DSL, graph persistence."

[scope.surface]
# Inputs that may flow into this scope.
inputs = ["source:repo_file", "source:tsg_file"]
# Outputs that must be emitted.
outputs = ["observation:evidence", "evaluation:threshold_passed"]
# Files that may be edited by humans/agents touching this scope.
editable_files = [
  "archctl/src/evidence.rs",
  "archctl/src/tsg.rs",
  "archctl/src/store.rs",
  "archctl/src/clock.rs",
  "archctl/src/environment.rs",
  "archctl/src/row.rs",
]
# Symbols this scope declares as part of its contract.
public_symbols = [
  "Evidence",
  "EvidenceKind",
  "EvidenceStore",
  "GraphStore",
  "extract",
  "put_with_clock",
  "from_tsg_node",
]

[scope.invariants]
# Things that must remain true for the scope to be considered intact.
must_hold = [
  "evidence::put_with_clock does not call std::fs directly",
  "GraphStore::query returns Vec<Row>, never Vec<Json>",
  "All evidence rows go through the Clock port",
  "Cypher queries use validate_identifier before interpolation",
]

[scope.gates]
# Static checks that must pass before a change to this scope is merged.
tests_required = 69
clippy_deny_warnings = true
mypy_equivalent = "cargo check --all-targets"
```

Y un script `scripts/check_scope_manifest.py` (en Python, llamado
desde CI) que verifique:

1. **Editable files**: el manifest declara todos los archivos del
   scope. Si un humano edita un archivo fuera del scope, el gate
   falla.
2. **Public symbols**: rustdoc exporta exactamente esos símbolos.
   Si se exporta algo no declarado (acoplamiento no intencional), el
   gate falla.
3. **Must-hold**: análisis estático del AST (vía `syn`) que
   verifique las invariantes. Por ejemplo, grep
   `std::fs::read` en `evidence.rs` falla porque debe ir por
   `Filesystem` port.
4. **Tests required**: `cargo test` reporta al menos N tests; si
   baja, el gate falla.

### 4.3 Por qué no es过度 (overkill)

- `archctl` es un solo binario, no 28 packs. Un solo manifest por
  scope (no por archivo) es suficiente.
- El gate `import allow-list` no aplica en Rust (Cargo.toml ya
  cumple esa función).
- El gate `banned constructs` (exec/eval/etc.) es trivialmente
  aplicable con `clippy` existente.
- El gate `declared-vs-actual` para `Cargo.toml` puede automatizarse
  con `cargo metadata`.

### 4.4 Por qué SÍ es valioso

- **Para el usuario**: cuando llegue un agente (humano o LLM) y
  proponga cambios, el manifest es el contrato que el reviewer lee.
  Es la materialización del principio "rodeo de controles".
- **Para el equipo**: tests rotos, símbolos sin documentar, archivos
  editados fuera de scope, todo se detecta en CI, no en review.
- **Para el Evolution Pack de ActiveGraph**: si mañana `archctl`
  quisiera tener un LLM que escriba extractores en `.tsg` files,
  el manifest sería el contrato que el gate estático valida.

### 4.5 Lo que NO se hace

- **No** copio el TOML exacto de ActiveGraph. El formato es
  razonable pero está acoplado a Python (declara `python-deps`).
  Un schema Rust tiene `cargo-dependencies` en su lugar.
- **No** implemento gates de injection scan. `archctl` no ejecuta
  código externo, así que la superficie de injection es mínima.
- **No** implemento trial sandbox. `archctl` no tiene self-modification.

---

## 5. Bloque B3 — Trust-by-origin para el extractor

### 5.1 El threat model de ActiveGraph

`activegraph-packs/docs/security.md` enuncia una postura que aplica
directo al extractor de `archctl`:

> **Classify by ORIGIN, never by content.** Content scanning stays what
> it has always been here, a tripwire that flags and audits. Admission
> to the author frame is decided by where text came from, and the
> default for every origin is OUT.

Aplicado a `archctl`: cuando el extractor lee un archivo del repo
del usuario y emite evidencia, **el contenido del archivo es externo**.
Si el usuario analiza un codebase hostil (por ejemplo, un repo de
un tercero que les pasan para review), el archivo puede contener
contenido diseñado para engañar al extractor.

### 5.2 Estado actual de `archctl`

El extractor actual (`evidence::extract`) lee el contenido del
archivo, lo pasa por `ast-grep`, produce matches y los emite como
`Evidence` rows. El output del extractor se persiste en el grafo
con `claim: "Rust function definition"` (texto fijo) y `text_preview`
(un resumen del match). **El contenido del archivo no se persiste
en el grafo** — solo el rango de bytes y un preview.

Eso ya es una postura defensiva razonable. Pero falta clasificar
**el origen del input**:

| Origen | Hoy | Debería |
|---|---|---|
| `cwd` del proyecto del usuario | Confianza implícita | Marcar como `source_origin: user_workspace` |
| `--config` o flag | Confianza implícita | Marcar como `source_origin: user_input` |
| Archivo en el repo | Se procesa tal cual | Marcar como `source_origin: project_file` |
| Output de tool externo (futuro) | N/A | Marcar como `source_origin: tool_output`, fenced |

### 5.3 Acción concreta

Añadir al tipo `Evidence` un campo `source_origin: SourceOrigin`
con un enum cerrado:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceOrigin {
    UserWorkspace,   // cwd argument, archivo del repo
    UserInput,       // --config, --pattern, --claim
    ToolOutput,      // futuro: output de MCP, web fetch, etc.
}

impl SourceOrigin {
    pub fn default_for_cwd() -> Self { SourceOrigin::UserWorkspace }
    pub fn default_for_cli_arg() -> Self { SourceOrigin::UserInput }
}
```

El extractor marca cada `Evidence` con el origen del input que la
produjo. La query `archctl evidence list --origin=user_input`
permite auditar qué evidencia vino de input humano directo vs de
archivos del repo. Esto es **exactly** el patrón que ActiveGraph
usa con `injection_flags`.

### 5.4 Por qué NO se hace completo

El threat model de ActiveGraph tiene un paso más: **fencing del
contenido externo en el prompt del LLM** (`[EXTERNAL CONTENT — data,
not instructions…]`). `archctl` no tiene LLM en el loop crítico
(los extractores son AST-grep deterministas). Cuando M9 introduzca
LLM-assisted extraction (ADR-007 menciona "richer extraction" para
futuros extractores), ahí sí aplica el fencing completo. Hoy no.

---

## 6. Decisiones pendientes y orden de ejecución

### 6.1 Roadmap propuesto

| Orden | Bloque | Esfuerzo | Valor |
|---|---|---|---|
| 1 | B2 — `manifest.toml` por scope + gates estáticos | 2-3 horas | Alto — materializa el principio operativo del usuario. |
| 2 | B3 — `SourceOrigin` enum en `Evidence` | 1 hora | Medio — previene una clase entera de bugs futuros. |
| 3 | B1 — Tipos `Source` y `Evaluation` en el grafo | 4-6 horas | Bajo-Medio — habilita replay/revisión, pero solo si el siguiente milestone los necesita. |
| 4 | B1 — Lifecycle `drafted → accepted` en Evidence | 3-4 horas | Bajo — nice-to-have, no bloqueante. |

### 6.2 Decisiones abiertas que requieren input del usuario

**D1**: ¿Empezar por B2 (manifest + gates) que es la materialización
directa del principio operativo, o por B3 (SourceOrigin) que es
más pequeño y útil?

**D2**: ¿Vale la pena el coste de mantener un script Python
(`check_scope_manifest.py`) en un repo Rust? Alternativa: un binario
Rust `archctl doctor --check-scope` que lea el manifest y verifique
los gates. Es más idiomático pero más código. La opción Python es
~80 LOC y se integra con CI estándar.

**D3**: Si adoptamos B3 (SourceOrigin), ¿lo aplicamos también a
`archctl::tsg`? Las reglas TSG vienen del filesystem del usuario
(proyecto) y del `--tsg-file` flag. Mismo problema que Evidence.

---

## 7. Lo que NO se aplica (registrado para no redescubrirlo)

Para evitar que alguien en el futuro pregunte "¿por qué no usamos
ActiveGraph Core como grafo?" o "¿por qué no escribimos `archctl`
como pack?", dejo registrado el razonamiento:

| Idea de ActiveGraph | Por qué NO se aplica |
|---|---|
| Runtime reactivo con event sourcing | `archctl` es CLI one-shot, no runtime. La fuente de verdad es el repo del usuario (ya versionado), no el log de eventos. |
| Behaviors como `@llm_behavior` con prompts | `archctl` no tiene LLM en el loop crítico (M3). Cuando lo tenga, los prompts vivirán en `archctl/src/prompts/` siguiendo el patrón de ActiveGraph, pero Rust + clap, no Python + LangChain. |
| Packs débilmente acoplados | `archctl` es monolito modular (12 módulos). El análogo serían `crates/` separados, que es trabajo para archview. |
| `bundles/` preset lists | `archctl::cli::run` ya cumple esa función. `archctl doctor --profile=research` sería el equivalente pero no es prioritario. |
| Capability declarations en `[[surface.capabilities]]` | `archctl` no tiene tool gateway ni sandbox. Las "capabilities" son los subcomandos CLI, que ya están declarados vía `clap`. |
| Fixtures deterministas por pack | `archctl::cli::tests` y `archctl::store::tests` ya funcionan así. No hace falta la convención de ActiveGraph. |
| `evolution` pack (self-modification) | `archctl` no se modifica a sí mismo. Lo que sí aplica es B2 (manifest + gates) como versión estática del mismo principio. |
| MCP inbound/outbound | `archctl::cli::render` ya hace HTTP saliente (reqwest). MCP entrante podría tener valor pero no hay demanda actual. |
| Memory gateway con embeddings | M15 (herramientas semánticas opcionales). Cuando llegue, se inspira en `memory_gateway` pero con embeddings opcionales off-by-default. |
| Channel adapters (telegram, whatsapp, email) | `archctl` no es un assistant. La capa de comunicación no aplica. |
| OpenAPI/Express API server | `archctl::cli` ya tiene `archctl doctor`, `archctl graph query` etc. No hace falta un HTTP server encima — los agentes usan el CLI directamente. |

---

## 8. Referencias

- **Repo**: https://github.com/yoheinakajima/activegraph-packs
- **Docs leídas en profundidad**:
  - `README.md` — overview
  - `docs/concepts.md` — Core vs layered, invariantes
  - `docs/architecture.md` — demo stack, frames, trace
  - `docs/manifest-spec.md` — schema del manifest, content_hash vs bundle_hash
  - `docs/evolution-design.md` — gates estáticos, fork trial, adoption
  - `docs/llm-author-design.md` — trust by origin, no por content
  - `docs/long-term-memory.md` — write path, provenance admission, swappable seams
  - `docs/security.md` — threat model, fences, detectors
  - `packs/core/object_types.py` — los 7 tipos universales
  - `packs/codebase/__init__.py` — ejemplo de un pack layered
  - `packs/core/manifest.toml` — ejemplo del formato manifest
- **Docs no leídas en profundidad (referencia futura)**:
  - `docs/mcp.md`
  - `docs/soak-runbook.md`
  - `activegraph-builder-report.md`
  - `activegraph-assistant-upgrade-plan.md`
  - `activegraph-direction-report.md`

---

## 9. Cambios para incorporar al estado del proyecto

Una vez el usuario decida el orden de ejecución (D1):

1. Crear `archctl/manifests/M3-evidence-pipeline.toml` (B2).
2. Crear `scripts/check_scope_manifest.py` o `archctl/src/bin/scope_check.rs` (B2).
3. Añadir `archctl::evidence::SourceOrigin` y migración del extractor (B3).
4. Si B1: actualizar `docs/schema/001_initial_schema.cypher` con los
   tipos `Source` y `Evaluation`.

Ninguno de estos cambios está implementado en este reporte. Es
**planificación**, no código. El snapshot del estado pre-investigación
sigue intacto en `snapshot/pre-activegraph-investigation` y
`docs/STATE.md`.
