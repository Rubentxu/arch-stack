# Propuesta: Plataforma de Inteligencia Arquitectónica — Plugin-First / No-Rust-First

> **SDDK propose · Context C0 (greenfield) · Entropy method: heuristic (Protocol B) · Auto-grill: ejecutado · Idioma: español**
> Cambio: `architecture-intelligence-platform`
> Fuente: `Skills-para-agentes-IA.md` (3721 líneas) + `explore-report.md` + esquema oficial OpenCode (`https://opencode.ai/config.json`).

---

## 0. Resumen Ejecutivo (lead-with-answer)

**Tesis de producto:** Convertir el insight más valioso del documento fuente —*los diagramas no son la fuente de verdad, son proyecciones de un modelo arquitectónico persistente y verificable*— en un sistema **útil desde el día uno** y **reversible en cada paso**.

**Decisión clave:** Plugin-first / no-Rust-first. El núcleo Rust queda **condicional** a una validación medible. Un fallo de hipótesis deja skills y schemas útiles, no meses de inversión perdidos.

**Correcciones críticas al informe de exploración** (basadas en evidencia externa independiente y esquema oficial):

| # | Corrección | Evidencia |
|---|---|---|
| C1 | **OpenCode usa `mcp` (no `mcpServers`).** La recomendación del investigador independiente de usar `mcpServers` es **falsa** para OpenCode actual. | Esquema oficial `config.json`: `McpLocalConfig { type: "local", command: string[], environment }` bajo clave top-level `mcp`. |
| C2 | **`experimental.session.compacting` no es una clave de config.** Es un *hook de plugin*. La configuración de compacción vive en `compaction` (top-level: `auto`, `prune`, `tail_turns`, `preserve_recent_tokens`, `reserved`). | Esquema oficial: `experimental` no contiene `session` ni `compacting`. Existe `compaction` top-level. |
| C3 | **Structurizr Lite está end-of-life/deprecated.** No usar la imagen Docker `structurizr/lite`. Usar **Structurizr `local`** — la herramienta self-hosted mantenida para **visualizar** workspaces — como herramienta de visualización interactiva. Para validación/export **headless** (CI, pipelines), usar un comando/herramienta Structurizr **soportado actualmente y pineado**, y seguir (track) vNext. No llamar a `local` una "distribución" genérica. | Instrucción del usuario + evidencia externa. `structurizr/lite` está deprecated/EOL. `local` es la herramienta self-hosted mantenida para viewing. Headless requiere tooling pineado soportado. |
| C4 | **Mermaid C4 sigue siendo experimental.** No usar como representación canónica. | Documentación oficial de Mermaid. |
| C5 | **`subagent_depth`, `skills`, `references`, `agent`, `plugin`, `permission`, y custom tools** confirmados directamente contra el esquema oficial en vivo. | Esquema oficial `https://opencode.ai/config.json` fetchado directamente + docs oficiales (última actualización 2026-07-27). Esto es **confirmado para la versión live actual**, no "unverified". La implementación debe pinear un release y ejecutar schema-contract tests en CI para prevenir drift futuro. Las claves existen; las APIs exactas (firmas de hooks, destructuring de eventos) requieren version-pin y schema-test en tiempo de build. |
| C6 | **Skills externas requieren verificación de licencia, pinning por commit/SHA, sandboxing y política de cadena de suministro.** | No se asume confianza en repos comunitarios. |
| C7 | **Los papers citados validan C4 desde texto/visualización, no recuperación fiable desde repos reales grandes.** La hipótesis central sigue sin prueba. | explore-report claim #10 (🔴 unsupported). |

---

> ⚠️ **HIPÓTESIS CENTRAL NO VALIDADA — LEER ANTES DE CONTINUAR**
>
> **La recuperación fiable de arquitectura desde repositorios reales arbitrarios (especialmente grandes y multilenguaje) es una hipótesis NO validada.** Es la afirmación de mayor riesgo del proyecto. El spike (Phase 1) y el MVP (Phase 2) son **experimentos diseñados para falsar esta hipótesis**, no para asumirla.
>
> Los papers citados (arXiv:2510.22787 multi-agent C4, arXiv:2605.24453 Code2UML) **apoyan patrones** (multi-agent + IR intermedio + validación determinista), **pero no prueban la recuperación fiable desde repos reales**: 2510.22787 toma como entrada un *system brief* (texto), no un codebase; Code2UML es visualización. Ninguno demuestra RE fiable a escala.
>
> **Cada decisión de inversión posterior está condicionada al gate de validación.** Si la hipótesis se falsa, Phase 0 (skill-only) queda como producto útil. No se asume éxito.

---

## 1. Tesis de Producto y Propuesta de Valor

**Propuesta de valor (una frase):** Recuperar la arquitectura de un repositorio de software mediante evidencia trazable, producirla como modelo neutral, y proyectarla a C4/UML/draw.io — **sin inventar elementos, sin contaminar el repo, y sin Rust hasta validar que funciona.**

**Por qué importa:** Cada herramienta existente que "dibuja Mermaid" inventa arquitectura. El documento fuente identifica correctamente el fallo estructural: falta separación entre evidencia e inferencia. Nuestra aportación no es analizar código (ya hay CLIs para eso) sino **fusionar evidencias heterogéneas, conservar su procedencia y convertirlas en un modelo verificable**.

**Pensamiento lateral — el truco del valor residual:**

> Diseñamos el MVP para que, **incluso si la hipótesis central falla** (RE fiable en repos grandes), el resultado siga siendo útil: skills de diagramación con disciplina de evidencia, schemas reutilizables, y un resolver de identidad de proyecto. No apostamos todo a una sola hipótesis.

---

## 2. Usuarios Objetivo, Jobs-to-be-Done y Non-Goals

### Usuarios objetivo

| Usuario | Job-to-be-done | Fricción actual |
|---------|----------------|-----------------|
| Arquitecto/a | Justificar cada elemento de un diagrama C4 con evidencia | Herramientas inventan relaciones; no hay trazabilidad |
| Equipo de plataforma | Mantener diagramas sincronizados con código sin contaminar el repo | `.architecture/` ensucia Git; diagramas obsoletos |
| Developer onboarding | Entender la arquitectura de un repo desconocido rápidamente | "Lee el repo y dibuja" = alucinación |

### Jobs-to-be-done (JTBD)

1. **Recuperar** la arquitectura de un repo con evidencia fuente (path/líneas/revision) por elemento. *Revision = Git commit SHA (modo `git`) o content snapshot/hash (modo `directory`).*
2. **Proyectar** el modelo a C4 (Structurizr/PlantUML) y UML (PlantUML), con Mermaid solo para preview.
3. **Persistir** el conocimiento fuera del repo (XDG), recuperable tras interrupciones, sin depender del historial de chat.
4. **Validar** que el modelo no contiene afirmaciones sin evidencia (unsupported-claims = 0 para hechos de alta confianza).

### Non-goals explícitos

- ❌ **No construir parsers/indexadores multilenguaje propios.** Reutilizar CLIs (ast-grep, ctags, build tools).
- ❌ **No invertir en Rust antes de validar la hipótesis central.**
- ❌ **No requerir telemetría runtime para el MVP.** Solo estático + declarado.
- ❌ **No hacer de Mermaid la representación canónica de C4.**
- ❌ **No almacenar conocimiento duradero solo en el historial de conversación.**
- ❌ **No ejecutar análisis profundo (Joern/CodeQL) por cambio.** Solo bajo demanda.
- ❌ **No usar `structurizr/lite` (EOL).** Usar Structurizr `local` (herramienta self-hosted para visualización) + comando/herramienta pineado soportado para headless.
- ❌ **No usar `mcpServers` como clave de config.** Usar `mcp`.

---

## 3. Alcance: Discovery Spike → MVP → Productización Condicional

### Phase 0 — Skill-Only Baseline (días, no semanas)

**Objetivo:** Valor inmediato con riesgo casi cero. El fallback deliberadamente simple.

- 1 `SKILL.md` que impone disciplina de evidencia + regla "Structurizr como proyección canónica".
- Instalar/adaptar 1-2 skills existentes (lmammino c4, plantuml-skill) con pinning de commit + verificación de licencia.
- Sin plugin, sin resolver, sin IR, sin Rust.

**Si todo lo demás falla, esto queda.**

### Phase 1 — Discovery Spike (1-2 semanas)

**Objetivo:** Validar o falsar la hipótesis central en 2 repos reales.

**Gate cero — Spike end-to-end de la hipótesis de carga (2-3 días, antes de cualquier otra inversión):**

> Antes de comprometerse al enfoque de skill orchestration más amplio, se ejecuta un spike que prueba la **hipótesis de carga** de forma barata y falsable. Dos partes:

**Parte A — Compatibilidad OpenCode/runtime/skill:**
> Verificar que la skill externa (lmammino c4-codebase-architecture-skill) se carga en OpenCode, funciona estructuralmente, y produce output válido contra la versión pineada de OpenCode. Si fracasa, se reconsidera toda la estrategia de skill wrapping antes de invertir más.

**Parte B — Falsación barata de la hipótesis central:**
> Ejecutar la skill adaptada contra un **fixture mínimo de 5 ficheros, sin Git** (modo `directory`) con un **gold set etiquetado manualmente** (elementos y relaciones esperadas, hand-authored). El pipeline debe:
> 1. Normalizar el output de la skill al IR minimal.
> 2. Proyectar el IR a `workspace.dsl`.
> 3. Renderizar localmente con Structurizr `local`.
> 4. Medir: matches semánticos contra el gold set y unsupported claims.

> **El gold es hand-authored; el IR producido NO es hand-authored.** El fixture es deliberadamente minúsculo para aislar la hipótesis de carga del ruido de repos grandes. Si la Parte B muestra que el loop evidence→IR→proyección→render no funciona ni en 5 ficheros, la hipótesis central se falsa temprano y baratamente.

- Resolver SourceIdentity de proyecto (XDG, BLAKE3, modo `git` | `directory`).
- Evidence ledger v1 (JSONL: path/lines/revision/extractor/confidence/classification). *Revision = commit SHA (modo `git`) o content hash (modo `directory`).*
- Architecture IR v1 minimal (elements, relationships, confidence, evidence-refs, `schemaVersion`).
- Perfil `fast` solo: Git + ast-grep outline + ctags + 1 build-tool.
- Auditor básico (refutación, no falsificador separado).

**Gate de validación (falsable):** medir en 2 repos reales (1 Rust pequeño, 1 TS mediano). `unsupported_claims_high_confidence = 0` es un **HARD FAIL global en todos los repos de test**, sin excepciones ni carve-out. Cualquier claim `fact`/`inference` con `confidence ≥ 0.9` y cero evidence-refs bloquea el pipeline. Claims de confianza media/baja sin evidencia se registran como `unknown`/`hypothesis` (visibles, auditables, no bloquean). Si el HARD FAIL se dispara → hipótesis en duda → reconsiderar antes de Phase 2.

### Phase 2 — MVP Plugin-First (3-5 semanas, condicional a gate)

- Capability router + adapter registry (formato YAML declarativo).
- Perfil `fast` completo + inicio de `semantic`.
- Plugin OpenCode: `shell.env` (resolver) + `tool.execute.before` (write-guard).
- Skills C4/UML envueltas (direct/wrapped/patched).
- Structurizr `local` (herramienta self-hosted para viewing) + PlantUML local para renderizado offline. Validación/export headless con comando Structurizr pineado y soportado.

### Phase 3+ — Productización Condicional (solo si Phase 2 valida)

- Rust core (si y solo si el overhead de normalización TS justifica tipado fuerte + performance).
- OpenCode control-plane plugin completo (eventos, checkpoints, compaction hook).
- Perfil `semantic` completo (SCIP/LSP).
- Modelo temporal (Phase 4). Drift diff static/declared (Phase 4). Falsifier agent (Phase 4). Observed graph (Phase 5).

---

## 4. Arquitectura de Tres Niveles

### Nivel 1 — Skill-Only Baseline (Phase 0)

```
OpenCode agente nativo
   ↓ carga bajo demanda
SKILL.md (disciplina evidencia + Structurizr como proyección)
   ↓ invoca
Skills existentes (lmammino c4, plantuml) + CLIs via bash
   ↓ produce
Diagramas Structurizr/PlantUML (sin IR persistente, sin ledger)
```

- **Irreversibilidad:** Muy baja. Se elimina borrando skills.
- **Valor residual si falla todo:** Skills útiles para diagramación manual con mejor disciplina.

### Nivel 2 — Plugin-First MVP (Phase 1-2)

```
OpenCode agente primario (TS plugin: shell.env resolver + write-guard)
   ↓ delega a
subagentes (markdown) llamando CLIs vía bash + skills envueltas
   ↓ producen
Evidence ledger (JSONL) → Architecture IR (JSON) → proyección Structurizr/PlantUML
   ↓ persisten en
XDG (~/.local/share/archctl/projects/<id>/)
```

- **Irreversibilidad:** Baja. TS plugin + JSON schemas. Sin compilación.
- **Seam crítico:** el contrato entre el plugin TS y la API de OpenCode. Mitigado por version-pin + schema-test en CI.

### Nivel 3 — Rust Extraction Boundary (Phase 3+, CONDICIONAL)

```
archctl (Rust): capability-router + normalizer + IR + state-machine
   ↔ IPC (JSON over CLI/stdio)
adaptador TypeScript (fino) ↔ OpenCode plugin
```

- **Condición de activación:** Phase 2 valida la hipótesis Y el overhead de normalización TS supera el coste de mantener un binario Rust + IPC.
- **Irreversibilidad:** Media-Alta. Se compromete a mantener un binario, IPC contract, y schema evolution más rígida.
- **Si no se activa:** el sistema vive indefinidamente en Nivel 2. No es un fracaso — es una decisión de simplicidad.

---

## 5. Matriz Build / Buy / Adapt

| Capacidad | Decisión | Razón | Detalle |
|-----------|----------|-------|---------|
| Símbolos/patrones estructurales | **Adapt** (ast-grep) | Reutilizar; no reinventar parser | `ast-grep outline` + rule packs externos (`--config`) |
| Inventario universal fallback | **Adapt** (ctags) | Cobertura multilenguaje ultrarrápida | `--output-format=json` |
| Resolución semántica | **Adapt** (SCIP/LSP) | Autoridad de referencias; bajo demanda | Solo cuando exista indexador |
| Dependencias Rust/Go/Java | **Adapt** (build tools) | El gestor de build conoce el grafo resuelto | `cargo metadata`, `go list`, `jdeps` |
| Dependencias JS/TS | **Adapt** (dependency-cruiser) | Motor de políticas + JSON output | Mejor que parser propio |
| IaC (Terraform/Helm/K8s) | **Adapt** (CLI nativos) | Sin parsers propios | `terraform graph`, `helm template`, `kubectl -o json` |
| SBOM | **Adapt** (Syft) | Detección de tecnologías empaquetadas | CycloneDX/SPDX JSON |
| Análisis profundo | **Adapt** (Joern/Semgrep) | Bajo demanda, no por cambio | CodeQL opcional (licencia) |
| Renderer C4 canónico | **Adapt** (Structurizr `local` + headless tool pineado) | Model-as-code, versionable, C4 nativo | **NO `structurizr/lite` (EOL)**. `local` = herramienta self-hosted para visualización. Headless = comando Structurizr pineado soportado, track vNext. |
| Renderer UML | **Adapt** (PlantUML local) | UML formal, offline | PlantUML local o Kroki interno |
| Renderer preview ligero | **Adapt** (Mermaid) | Documentación, PRs | **C4 experimental — no canónico** |
| Renderer editable | **Adapt** (draw.io skill) | Presentación, edición humana | Adaptador de presentación |
| Skills C4 (lmammino/cheriftj/bitsmuggler) | **Adapt** (wrapped) | Imponer contrato evidencia-IR sobre skill upstream | Pinning commit + licencia + sandbox |
| Evidence ledger | **Build** | Es la aportación diferencial —nadie hace esto | JSONL minimal, schemaVersion |
| Architecture IR | **Build** | Núcleo del valor: modelo neutral, no Structurizr directo | JSON minimal, forward-compatible temporal |
| Project resolver (SourceIdentity) | **Build** | Identidad discriminada `git` \| `directory`; Git es adaptador opcional, no prerrequisito | `git`: repository_id + worktree_id; `directory`: directory_id (local-only). Portable projectId en export bundles, re-bound al importar. |
| Capability router | **Build** (patrón) | OCP: añadir adapters sin tocar router | Registry YAML declarativo |
| Plugin OpenCode control-plane | **Build** | shell.env + write-guard + (Phase 3) compaction hook | TS plugin fino |

---

## 6. Métricas de Éxito/Fallo y Gates Falsables

### Gates por fase

| Fase | Gate | Métrica | Umbral de aprobación | Umbral de rechazo |
|------|------|---------|----------------------|-------------------|
| Phase 1 Spike | unsupported-claims | Elementos sin evidencia (alta confianza) | **= 0** en todos los repos de test (sin excepciones, sin carve-out) | > 0 → HARD FAIL; hipótesis en duda → reconsiderar antes de Phase 2 |
| Phase 1 Spike | evidence-coverage | % elementos con ≥1 evidencia | ≥ 0.90 | < 0.70 |
| Phase 1 Spike | render-success | Structurizr/PlantUML renderiza sin error | = 100% | < 80% |
| Phase 1 Spike | stability | Mismos elementos entre 2 ejecuciones (mismo commit) | ≥ 0.95 Jaccard | < 0.80 |
| Phase 2 MVP | semantic-precision (muestra) | Elementos correctos / elementos producidos (muestra manual) | ≥ 0.85 | < 0.70 |
| Phase 2 MVP | semantic-recall (muestra) | Elementos encontrados / elementos esperados (fixture) | ≥ 0.80 | < 0.60 |
| Phase 2 MVP | cost-per-recovery | Tokens + tiempo por recuperación de arquitectura | < 50k tokens, < 5 min (repo mediano) | > 200k tokens |
| Phase 2 MVP | lead-time | Tiempo desde `archctl` hasta primer diagrama útil | < 10 min (repo mediano, perfil fast) | > 30 min |

### Política de unsupported claims (nivel propuesta)

> La máquina de estados detallada (transiciones, timeouts, reintentos) se define en spec. Aquí solo el contrato de severidad:

| `confidence` del claim | ¿Tiene evidencia? | Severidad | Acción |
|------------------------|--------------------|-----------|--------|
| **Alta** (fact, 0.9+) | No | 🔴 **HARD FAIL** | El pipeline no puede aprobar. `unsupported_claims_high_confidence > 0` = fallo inmediato. |
| **Media** (0.6-0.89) | No | 🟡 **Unknown explícito** | Se registra como `unknown` en el ledger. Requiere auditoría. Escalación humana opcional según `impact × uncertainty × cost_of_error`. |
| **Baja** (< 0.6) | No | 🟢 **Hypothesis** | Se registra como `hypothesis`. Aparece en el informe pero no bloquea. Re-auditable. |

**Regla de oro:** un elemento clasificado como `fact` o con `confidence ≥ 0.9` sin al menos una referencia de evidencia es una contradicción ontológica — el pipeline lo rechaza.

### Invariante de calidad (no negotiable)

```yaml
quality_gates:
  unsupported_claims_high_confidence: 0    # HARD FAIL si > 0
  render_must_succeed: true                # Structurizr/PlantUML no falla
  forbidden_elements: []                   # Elementos inventados = 0
  schema_version_present: true             # Todo IR tiene schemaVersion
  # unsupported_claims_medium → se registran como unknown (no bloquean)
  # unsupported_claims_low → se registran como hypothesis (no bloquean)
```

---

## 7. Seguridad, Privacidad y Cadena de Suministro

### Postura offline-first

- **Renderizado local por defecto.** PlantUML local o Kroki interno. Structurizr `local` (herramienta self-hosted para visualización interactiva). Headless: comando Structurizr pineado y soportado.
- **No enviar código, nombres de sistemas ni diagramas a servicios públicos** (Kroki público, PlantUML server público).
- `store-source-snippets: false` por defecto (configuración por proyecto). Solo path/líneas/hash.

### Cadena de suministro de skills externas

| Medida | Implementación |
|--------|----------------|
| **Pinning** | `skills.lock.json`: commit/SHA por skill |
| **Licencia** | Verificación explícita antes de activar (MIT/Apache preferidos) |
| **Sandboxing** | Skills envueltas no escriben fuera de XDG; write-guard via plugin |
| **Reproducibilidad** | `archctl skills verify` valida hash del upstream |
| **Actualización** | Cambios de skill requieren re-test en fixtures antes de promoción |

### Reglas de OpenCode (schema-oficial, nunca adivinar)

| Clave | Valor canónico | Nota |
|-------|----------------|------|
| Servidores MCP | `mcp` (top-level) | **NO `mcpServers`**. `type: "local"`, `command: string[]`, `environment` |
| Permisos | `permission` | `read/edit/bash/task/skill/lsp` + custom tool patterns via `additionalProperties` |
| Delegación | `subagent_depth` | Default 1; el MVP usa 2 (orquestador → especialista → sub-tarea acotada) |
| Skills | `skills.paths` + `skills.urls` | Directorios adicionales y URLs well-known |
| Referencias | `references` | `{path/repository}` objects; `reference` está deprecated |
| Plugins | `plugin` | Array de strings o `[string, options]` |
| Compacción (config) | `compaction` (top-level) | `auto/prune/tail_turns/preserve_recent_tokens/reserved` |
| Compacción (hook) | `experimental.session.compacting` | **Hook de plugin**, NO clave de config. Inyectar punteros, no grafo completo. |

---

## 8. Política de Persistencia: XDG vs In-Repo

### Decisión: XDG runtime state + export bundle explícito

> **Recomendación:** Estado runtime en XDG por defecto. Export bundle explícito para compartir. **Sin dependencia oculta del historial de chat.**

### Resolución del conflicto del documento fuente

El documento fuente tenía **dos diseños de almacenamiento no reconciliados**:
1. In-repo `.architecture/` (primera mitad) — contamina Git.
2. XDG `~/.local/share/archctl/` (segunda mitad) — limpio pero no compartible.

**Resolución:**

| Modo | Cuándo | Dónde |
|------|--------|-------|
| **Privado local (default)** | Desarrollo individual | `~/.local/share/archctl/projects/<id>/` |
| **Export bundle** | Compartir puntual | `archctl project export --output bundle.tar.zst` (modelo + evidencias sin código sensible + skillset.lock) |
| **Sidecar repo** | Equipos | Repo separado `proyecto-architecture` (Git, curado) |

### Regla de destrucción selectiva

```
Eliminar ~/.cache/archctl       → no se pierde conocimiento (regenerable)
Eliminar ~/.local/state/archctl → se pierden ejecuciones en curso, no el modelo
Eliminar ~/.local/share/archctl → se pierde la memoria arquitectónica persistente
```

### Identidad de proyecto — SourceIdentity discriminada

> Git es un **adaptador de capacidad opcional**, no un prerrequisito universal. El plugin resuelve la identidad en el inicio de sesión (ADR-0003).

**Modo `git`** (cuando hay repositorio Git):

```
repository_id = BLAKE3(normalized_remote + root_commit)   # estable y compartible entre máquinas
worktree_id   = BLAKE3(repository_id + realpath(show_toplevel))
# La rama NO forma parte de la identidad (un worktree puede cambiar de rama).
```

**Modo `directory`** (cuando NO hay Git):

```
directory_id  = BLAKE3(canonical_realpath)                 # local-only; NO portable entre hosts
# Sin remote ni root_commit disponibles → estabilidad local únicamente.
```

**Portable project id** (para compartir):

```
# El export bundle lleva un UUID estable (portable_project_id).
# Al importar en otra máquina, se re-bound a la SourceIdentity local
# (el anchor machine-specific difiere; el projectId permanece).
```

| Modo | Estable entre máquinas | Revision de evidencia |
|------|----------------------|----------------------|
| `git` | ✅ Sí (repository_id compartible) | Git commit SHA |
| `directory` | ❌ No (local-only sin rebind explícito) | Content snapshot/hash (BLAKE3 del árbol o fichero) |

### Sin dependencia del chat

El conocimiento duradero vive en XDG (JSONL/JSON/texto). El historial de OpenCode es solo auditoría. Tras compacción, el plugin inyecta **punteros** al estado, no el grafo completo.

---

## 9. ADR Propuestos

> Los 8 ADRs siguientes existen en `docs/adr/`. Todos están en estado **Proposed** (ver `docs/adr/README.md`).

| ADR | Título (canónico) | Estado | Decide |
|-----|---------------------|--------|--------|
| [ADR-0001](../../docs/adr/0001-plugin-first-no-rust-first.md) | Plugin-First / No-Rust-First with Conditional Rust Extraction Gate | Proposed | Defer Rust hasta que la hipótesis central valide |
| [ADR-0002](../../docs/adr/0002-neutral-ir-truth-structurizr-projection.md) | Neutral Architecture IR as Truth; Structurizr as C4 Projection | Proposed | IR es la fuente; Structurizr es proyección (no dual-truth) |
| [ADR-0003](../../docs/adr/0003-xdg-runtime-state-export-bundle.md) | XDG Runtime State + Explicit Export Bundle | Proposed | Sin `.architecture/` in-repo; XDG por defecto + export explícito |
| [ADR-0004](../../docs/adr/0004-evidence-ontology-confidence-provenance.md) | Evidence Ontology and Confidence Provenance | Proposed | fact/inference/hypothesis/unknown/conflict + procedencia + gate de unsupported-claim |
| [ADR-0005](../../docs/adr/0005-renderer-routing-local-first.md) | Renderer Routing / Local-First Policy | Proposed | Structurizr `local` (no lite EOL); offline-first; Mermaid no canónico |
| [ADR-0006](../../docs/adr/0006-reuse-over-rebuild-capability-adapters.md) | Reuse-over-Rebuild and Capability Adapter Contract | Proposed | Sin parsers propios; seam uniforme de Adapter (declarative ShellAdapter) |
| [ADR-0007](../../docs/adr/0007-opencode-version-pin-schema-contract-minimal-topology.md) | OpenCode Version Pin / Schema-Contract and Minimal Agent Topology | Proposed | `mcp` (no mcpServers); version-pin + CI schema-test; máximo 4 roles |
| [ADR-0008](../../docs/adr/0008-supply-chain-pinning-sandbox.md) | Supply-Chain Pinning / Sandbox Policy | Proposed | skills.lock.json pinning + licencia + sandbox + write-guard |

### Deferred Decisions (no son ADRs — se promueven a ADR solo cuando maduren)

| Decisión | Cuándo se aborda | Nota |
|----------|------------------|------|
| Rust core condicional | Phase 3, solo si Phase 2 valida Y el overhead TS lo justifica | Gobernada por ADR-0001 |
| Modelo temporal (validFrom/validTo) | Phase 4+. Diseñar IR forward-compatible ahora (campos reservados) | No construir history store aún |
| Drift diff (declared vs static) | Phase 4 | Requiere grafo declarado completo |
| Grafo observado (telemetría runtime) | Phase 5. MVP = static + declared only | Requiere telemetría OpenTelemetry |
| Falsifier agent separado | Phase 4 | El MVP usa auditor básico (refutación, no falsificador) |
| CI/CD drift gates | Phase 4 | Tuning de falsos positivos |

---

## 10. ROADMAP con Kill Gates

```text
Phase 0 — Skill-Only Baseline [días]
  ├─ SKILL.md disciplina evidencia
  ├─ 1-2 skills adaptadas (pinning + licencia)
  └─ EXIT: diagramas C4 básicos con Structurizr producidos

Phase 1 — Discovery Spike [1-2 sem]
  ├─ GATE CERO (2-3 días, antes de todo lo demás):
  │    ├─ A: Skill externa carga en OpenCode pineado → output válido
  │    └─ B: Fixture 5-ficheros sin Git + gold hand-authored → IR → workspace.dsl → render local
  │         └─ SI FALLA → hipótesis de carga falsada baratamente; mantener Phase 0
  ├─ Resolver SourceIdentity (XDG, BLAKE3, git|directory)
  ├─ Evidence ledger v1 (JSONL)
  ├─ Architecture IR v1 minimal
  ├─ Perfil fast (Git + ast-grep + ctags + 1 build-tool)
  ├─ Auditor básico
  └─ KILL GATE: unsupported_claims_high_confidence=0 en TODOS los repos de test (HARD FAIL global)
       └─ SI FALLA → mantener Phase 0, documentar, no avanzar a Phase 2

Phase 2 — MVP Plugin-First [3-5 sem, condicional]
  ├─ Capability router + adapter registry (YAML)
  ├─ Plugin OpenCode (shell.env + write-guard)
  ├─ Skills C4/UML envueltas (direct/wrapped/patched)
  ├─ Structurizr `local` (viewing) + headless tool pineado + PlantUML local
  ├─ Perfil fast completo + inicio semantic
  └─ EXIT GATE: precision≥0.85, recall≥0.80, render=100%, cost<50k tokens
       └─ SI FALLA → iterar perfil fast; no avanzar a Phase 3

Phase 3 — Productización Condicional [solo si Phase 2 valida]
  ├─ DECISION GATE: ¿overhead TS justifica Rust?
  │    ├─ SÍ → Rust core (archctl binario + IPC)
  │    └─ NO  → mantener TS, documentar simplicidad como feature
  ├─ Control-plane plugin completo (eventos, checkpoints)
  └─ Perfil semantic (SCIP/LSP)

Phase 4 — Evolución [condicional]
  ├─ Modelo temporal (validFrom/validTo)
  ├─ Drift diff (static vs declared)
  ├─ CI/CD gates
  └─ Falsifier agent

Phase 5 — Gemelo Arquitectónico [ambicioso]
  ├─ Grafo observado (telemetría OpenTelemetry)
  ├─ Deep analysis (Joern/CodeQL on-demand)
  └─ Headless SDK orchestration
```

---

## 11. Entropy Protocol B (Heuristic) + Auto-Grill

### Entropy Budget Prediction (Protocol B)

**Method: heuristic · Confidence: low (greenfield, design document, no code)**

| Métrica | Estimate (bits) | Threshold | Status |
|---------|-----------------|-----------|--------|
| H(Δ_existing) | 0 (greenfield) | < 1.0 | ✅ Nada que modificar |
| H(Δ_new) | ~5-6 (nueva plataforma: IR + ledger + skills + plugin + resolver) | > 0 | ✅ Esperado para greenfield |
| Pares de connascence nuevos | ~3-4 críticos | < 3 | ⚠️ Yellow — ver abajo |
| OCP compliant? | Sí para router; No para IR schema (hub central) | yes | ⚠️ Mixed |

**Pares críticos introducidos:**

| Component A | Component B | Connascence | I(bits) | Mitigación |
|-------------|-------------|-------------|---------|------------|
| Architecture IR schema | Todos los agentes/skills/renderers | Name + Type | ~3.5-4.5 | Versionar (`schemaVersion`); mantener minimal; schema evolution first-class |
| Evidence ledger schema | Synthesizer + auditor + IR | Type + Meaning | ~2.5-3.0 | JSONL flexible; `schemaVersion` |
| Plugin TS ↔ OpenCode API | Hook semantics | Meaning (hidden) | ~1.5-2.0 | Version-pin OpenCode; CI schema-test |
| SourceIdentity resolver ↔ Ledger + IR | Discriminated identity contract | Value | ~1.5 | API estable; test de identidad (git + directory variants); rebind de portable id |

**Verdict:** 🟡 YELLOW — El IR schema hub es el hotspot de entropía. Mitigado por versionado y minimalismo. El router es genuinamente OCP-compliant (el punto fuerte del diseño). Al recortar alcance drásticamente vs el documento fuente, el DQS real de lo construido debería ser ~0.55-0.65 (mejor que el ~0.45 estimado del diseño completo).

### Auto-Grill de la Propuesta

| # | Afirmación/Decisión | Resolución | Confianza |
|---|---------------------|------------|-----------|
| G1 | "Plugin-first es la opción más reversible" | **Auto-resuelta.** Skills + TS plugin se eliminan sin ciclos de compilación. Rust requiere build infra. | 0.95 |
| G2 | "La hipótesis central (RE fiable desde repos reales) está sin validar" | **Auto-resuelta y declarada prominentemente.** Papers citados (2510.22787, 2605.24453) apoyan *patrones* (multi-agent + IR + validación), no prueban RE fiable desde repos reales grandes. El spike/MVP es un experimento para falsar esta hipótesis. | 0.9 |
| G3 | "`mcp` no `mcpServers`; schema confirmado" | **Auto-resuelta.** Esquema oficial en vivo fetchado directamente + docs (2026-07-27). Confirmado para versión live actual. No es "unverified" — es "confirmado ahora, version-pin + schema-test para prevenir drift futuro." | 1.0 |
| G4 | "Structurizr Lite EOL; usar `local`" | **Auto-resuelta.** `structurizr/lite` está deprecated/EOL. `local` es la herramienta self-hosted mantenida para **visualización**. Headless requiere comando Structurizr pineado y soportado, track vNext. No es una "distribución" genérica. | 0.95 |
| G5 | "XDG resuelve la contaminación del repo" | **Auto-resuelta conceptualmente.** Depende de `OPENCODE_CONFIG_DIR` + `shell.env` hook funcionando. | 0.7 |
| G6 | "Mermaid C4 no es canónico" | **Auto-resuelta.** Documentación oficial: experimental. | 0.9 |
| G7 | Timing de Rust | **Resuelta por instrucción del usuario.** Plugin-first. Rust condicional. | 1.0 |
| G8 | Store canónico (IR vs Structurizr) | **Resuelta.** IR es verdad; Structurizr es proyección. | 0.9 |
| G9 | Persistencia (XDG vs in-repo) | **Resuelta.** XDG + export bundle. | 0.85 |

**Estado del grill:** `all_resolved` — 9 afirmaciones auto-resueltas o resueltas por instrucción. Ningún asunto cosmético escalado. No hay decisiones hard-to-reverse sin resolver.

---

## 12. Riesgos, Unknowns, Assumptions y Experimentos

### Riesgos

| Riesgo | Likelihood | Impact | Mitigación |
|--------|------------|--------|------------|
| 🔴 El loop de RE no es fiable en repos reales grandes | Med-High | Crítico | Spike primero (Phase 1); medir en 2 repos; gate falsable |
| 🔴 Build-before-validate si se avanza sin gate | Low (con esta propuesta) | Crítico | Phase 0 fallback; kill gates explícitos |
| 🟡 OpenCode version drift rompe hooks/config | Medium | High | Pin release; CI schema-test contra `config.json` |
| 🟡 Skills Claude-Code no adaptan limpiamente a OpenCode | Medium | Medium | Validar 1 skill end-to-end antes de comprometer registry approach |
| 🟡 Carga operacional del toolchain (10+ CLIs) | Medium | Medium | Perfiles progresivos (fast/semantic/deep); container bootstrap |
| 🟢 Modelo temporal over-engineering | Low (deferido) | Low | Phase 4+; IR forward-compatible, no history store ahora |
| 🟡 Supply-chain de skills externas | Medium | High | Pinning commit/SHA + licencia + sandbox + write-guard |

### Unknowns (explícitos)

- **Confianza no calibrada:** cómo asignar/validar `confidence` numérica. Sin método validado todavía. → Experimento en Phase 1.
- **Compatibilidad exacta Claude-Code ↔ OpenCode:** el skill-registry local confirma descubrimiento parcial en `.claude/skills/`, pero las APIs de commands/plugins pueden diverger. → Validar 1 skill end-to-end.
- **APIs de plugin OpenCode:** los hooks existen (`shell.env`, `tool.execute.before/after`) pero las firmas exactas difieren entre versiones. → Pin + schema-test.
- **SCIP governance 2026:** citado a blog de Sourcegraph, no confirmado. Bajo impacto en MVP.

### Assumptions

- OpenCode mantendrá `mcp` como clave canónica — **confirmado** en el esquema oficial en vivo (`config.json`, docs actualizadas 2026-07-27) para la versión actual. La implementación pinea un release y ejecuta schema-contract tests en CI para detectar drift.
- `subagent_depth` permite orquestación 2-nivel sin problemas de contexto — confirmado en esquema; comportamiento real se valida en el spike.
- ast-grep `outline` + rule packs cubren suficientes frameworks para el perfil `fast`.
- Structurizr `local` (herramienta self-hosted para visualización) renderiza workspace.dsl sin infraestructura compleja. Headless requiere comando Structurizr pineado y soportado (no `structurizr/lite` que está EOL). Track vNext.

### Experimentos

| Experimento | Hipótesis | Métrica | Duración |
|-------------|-----------|---------|----------|
| **Gate Cero** (Parte A: compatibilidad) | Skill externa se carga en OpenCode pineado y produce output estructuralmente válido | skill funciona end-to-end | **1 día** |
| **Gate Cero** (Parte B: hipótesis de carga) | Loop evidence→IR→workspace.dsl→render funciona en fixture 5-ficheros con gold set hand-authored | matches semánticos ≥ 0.80 vs gold; unsupported_claims_high_confidence = 0 | **1-2 días** |
| Spike 2 repos | RE fiable produce modelo sin afirmaciones sin evidencia (alta confianza) | `unsupported_claims_high_confidence = 0` | 1-2 sem |
| Plugin TS write-guard | `tool.execute.before` bloquea escrituras fuera de XDG | escrituras bloqueadas = 100% | 1 día |

---

## Capabilities (CONTRATO con sddk-spec)

> Greenfield: no existen specs previos. Todas son capacidades nuevas.

### New Capabilities

- `architecture-project-resolution`: Resolución de identidad discriminada SourceIdentity (`git` mode: repository_id/worktree_id; `directory` mode: directory_id local-only), almacenamiento XDG espejo, portable projectId para export/reimport, wrapper `archctl`/`OPENCODE_CONFIG_DIR`. Git es adaptador opcional. **Phase 1.**
- `architecture-evidence-ledger`: Recolección, clasificación (fact/inference/hypothesis/unknown/conflict), procedencia (path/lines/revision/extractor/confidence) y almacenamiento de evidencias. Revision = commit SHA (modo `git`) o content snapshot/hash (modo `directory`). **Phase 1.**
- `architecture-ir`: Modelo arquitectónico neutral (elements, relationships, confidence, evidence-refs, `schemaVersion`), forward-compatible con campos temporales reservados. **Phase 1.**
- `c4-projection`: Proyección del IR a vistas C4 (Structurizr `local` como herramienta de visualización canónica; headless con comando pineado soportado; PlantUML, Mermaid preview), control de niveles de abstracción. **Phase 1-2.**
- `architecture-skill-orchestration`: Wrapping/adaptación de skills externas (direct/wrapped/patched), `skills.lock.json`, verificación de licencia/pinning, router de capacidades con adapters YAML declarativos. **Phase 2.**

### Modified Capabilities

None — greenfield, no existen specs previos.

---

## Rollback Plan

- **Phase 0 fallback:** Si todo falla, mantener el skill-only baseline. Las skills adaptadas (lmammino c4, plantuml) siguen siendo útiles para diagramación manual con mejor disciplina.
- **Phase 1 rollback:** Eliminar `~/.local/share/archctl/` + plugin TS + skills. El repo analizado queda intacto (XDG = no contaminación).
- **Phase 2 rollback:** Revertir a Phase 1 (plugin + ledger + IR sin capability router completo). El capability router es aditivo.
- **Phase 3 (Rust) rollback:** Si Rust se activa y resulta prematuro, mantener el binario `archctl` como CLI opcional y seguir usando el camino TS. El IPC contract puede degradarse a "no-op fallback to TS path."

**Reversibilidad por diseño:** XDG garantiza que borrar `~/.local/share/archctl/` elimina toda la memoria arquitectónica sin tocar el repo fuente. Skills y plugins se eliminan sin compilación.

---

## Dependencies

- **OpenCode** (versión pinneada, schema-validada contra `config.json`).
- **ast-grep** (outline + rule packs + JSON output).
- **Universal Ctags** (fallback multilenguaje).
- **Structurizr** `local` (herramienta self-hosted para visualización) + comando Structurizr pineado soportado para headless (NO `structurizr/lite` EOL).
- **PlantUML** local (jar o Kroki interno).
- **Build tools nativos** según lenguaje (cargo, go, mvn, gradle, jdeps).
- **dependency-cruiser** (JS/TS).
- **Skills externas** (lmammino c4, plantuml-skill) con pinning + licencia verificada.
- **Git** (adaptador de capacidad opcional para modo `git`; no es prerrequisito universal — modo `directory` funciona sin Git).

---

## Success Criteria

- [ ] Phase 0: diagramas C4 básicos producidos con Structurizr desde 1 repo real.
- [ ] Phase 1 — **Gate Cero Parte A:** skill externa carga en OpenCode pineado y produce output estructuralmente válido (1 día).
- [ ] Phase 1 — **Gate Cero Parte B:** loop evidence→IR→workspace.dsl→render funciona en fixture 5-ficheros sin Git con gold set hand-authored; matches semánticos ≥ 0.80 (1-2 días). *El gold es hand-authored; el IR NO.*
- [ ] Phase 1: `unsupported_claims_high_confidence = 0` — HARD FAIL global en todos los repos de test (sin carve-out, sin excepción).
- [ ] Phase 1: evidence-coverage ≥ 0.90 en repo Rust pequeño.
- [ ] Phase 1: Structurizr + PlantUML renderizan sin error (render-success = 100%).
- [ ] Phase 2: precision ≥ 0.85 (muestra manual) en repo TS mediano.
- [ ] Phase 2: recall ≥ 0.80 (fixture) en repo TS mediano.
- [ ] Phase 2: repo analizado permanece limpio (cero ficheros añadidos).
- [ ] Phase 2: `skills.lock.json` con commit/SHA por skill externa.
- [ ] Phase 2: `unsupported_claims_medium` se registran como `unknown` en el ledger (no bloquean, son auditables).
- [ ] Todo IR lleva `schemaVersion`.
- [ ] Plugin write-guard bloquea escrituras fuera de XDG.

---

## Standard Envelope

```yaml
status: success
executive_summary: >
  Propuesta decision-grade para plataforma de inteligencia arquitectónica.
  Resuelve el fork Rust-now vs Rust-later a favor de plugin-first/no-Rust-first
  (más reversible, mejor matches utilidad/facilidad). Corrige el explore-report:
  mcp (no mcpServers), Structurizr local (no lite EOL), compaction es config no
  experimental.session.compacting. Diseña Phase 0 skill-only fallback para que
  un fallo de hipótesis deje skills y schemas útiles.
  AMENDMENT 1.3: hipótesis central RE declarada prominentemente como no
  validada; schema OpenCode confirmado contra live config.json (no unverified);
  spike de adaptación 1-skill como gate cero; política de unsupported_claims
  definida (high=FAIL, medium=unknown, low=hypothesis); Structurizr Lite EOL
  confirmado: `local` es la herramienta self-hosted para viewing, headless requiere tool pineado.
artifacts:
  - "sddk/architecture-intelligence-platform/proposal.md"
capabilities:
  new: 5
  modified: 0
risk_level: Medium
next_recommended: sddk-spec
risks:
  - "RE loop reliability on large repos (UNVALIDATED core hypothesis — experiment to falsify)"
  - "OpenCode version drift (mitigated by version-pin + schema-contract tests)"
  - "Claude-Code skill adaptation surface (mitigated by gate-cero 1-skill spike)"
  - "Supply-chain of external skills"
context_quality: C0
taxonomy:
  dominant_axes:
    - build_vs_buy
    - boundary_seam
    - coupling_connascence
    - mvp_scope
    - temporal_evidence
lenses_used:
  - entropy-sdd Protocol B (heuristic)
  - auto-grill (text-mode, Spanish)
  - cognitive-doc-design (progressive disclosure)
  - official OpenCode schema validation (live config.json fetched directly)
skill_resolution:
  - sddk-propose (executed, amended per coherence gate 1.3)
  - entropy-sdd Protocol B (heuristic, executed)
  - auto-grill (text-mode, Spanish, executed — 9 questions, all resolved)
  - cognitive-doc-design (applied to proposal structure)
coherence_gate:
  version: "final-synthesis"
  must_fixes_applied:
    - "§9: exactamente 8 ADRs reales ADR-0001..ADR-0008 matching docs/adr/README.md; ghost ADRs 0009-0012 eliminados; deferred decisions en tabla separada"
    - "unsupported_claims: HARD FAIL global sin carve-out (≤2 eliminado, >5 eliminado); medium/low = unknown/hypothesis visibles"
    - "Gate Cero reforzado: Parte A (compatibilidad) + Parte B (fixture 5-ficheros sin Git + gold hand-authored → IR → workspace.dsl → render → medir); gold NO es produced IR"
    - "ADR IDs formato 4 dígitos (ADR-0001..ADR-0008) en todo el documento"
    - "Structurizr: Lite EOL; `local` = herramienta self-hosted para viewing (no 'distribución'); headless = comando pineado soportado, track vNext"
    - "Idioma español preservado; progressive disclosure preservado"
```
