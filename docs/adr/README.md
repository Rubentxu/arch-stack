# ADRs — `archctl`

> 7 decisiones operativas. Cada una responde a una idea concreta del
> documento inicial `Skills-para-agentes-IA.md`. Sin especulación.

## ADR-0001 — El producto son diagramas C4 + UML

`archctl` produce **diagramas de arquitectura de un repositorio**,
extraídos por ingeniería inversa:

- **C4**: Context, Container, Component, Deployment, Dynamic.
- **UML**: sequence, class, use case, state, activity, component.

Esto es la pregunta literal del usuario (línea 7 del documento
inicial). Todo lo demás existe para servir a estos diagramas.

## ADR-0002 — Evidencia antes que diagramas

Cada elemento del modelo se sostiene en **evidencias** que apuntan a
archivos y líneas concretas del repositorio. Nada se inventa a partir
de nombres de archivo. Cada evidencia se clasifica como:

- **HECHO**: observado directamente por una herramienta.
- **INFERENCIA**: deducido con una confianza dada.
- **HIPÓTESIS**: explicación provisional pendiente.
- **DESCONOCIDO**: información ausente marcada como hueco.
- **CONFLICTO**: evidencias que se contradicen.

## ADR-0003 — `archctl` no reimplementa, envuelve

`archctl` **no escribe parsers, indexadores ni call graphs propios**.
Las herramientas que ya existen hacen el trabajo; `archctl` orquesta,
normaliza y conserva procedencia:

- Inventario / sintaxis: `ast-grep outline`, Universal Ctags.
- Reglas de framework y políticas: `ast-grep scan` con rule packs.
- Semántica: SCIP, LSP, `jdeps`, `dependency-cruiser`, Doxygen XML.
- Dependencias resueltas: `cargo metadata`, `go list -m -json`,
  `mvn dependency:tree`, `gradle dependencies`.
- IaC: `terraform show -json`, `helm template`, `kubectl get -o json`.
- SBOM: `syft`.

Regla (literal del documento inicial, líneas 2089-2093):

> La herramienta existente extrae hechos; `archctl` conserva la
> procedencia, normaliza los resultados y construye el modelo
> arquitectónico; los agentes interpretan únicamente lo que las
> herramientas no pueden determinar.

## ADR-0004 — Orquestador + subagentes en OpenCode

OpenCode es el **plano de control**. Hay un agente primario y
subagentes especializados con permisos mínimos:

- `architecture-orchestrator` (primario) — planifica, valida contratos,
  aprueba actualizaciones del modelo.
- `repo-cartographer` — inventario inicial del repo.
- `static-semantics-specialist` — SCIP/LSP/jdeps/dependency-cruiser.
- `framework-specialist` — rule packs de `ast-grep`, Semgrep cuando
  haga falta.
- `infrastructure-specialist` — Terraform/Helm/Kubernetes/Syft.
- `architecture-synthesizer` — fusiona el ledger de evidencias en el
  modelo (no lee código, solo consume evidencias).
- `c4-modeler` — genera las vistas C4 desde el IR.
- `uml-modeler` — genera las vistas UML desde el IR.
- `architecture-auditor` — revisa el resultado contra el modelo, busca
  omisiones y elementos sin evidencia.

Cada subagente ve solo su skill y solo las herramientas que necesita.
Los diagramas son **proyecciones** del modelo, no la fuente de verdad.

## ADR-0005 — Structurizr DSL canónico, otros son proyección

Para C4, la fuente canónica es el **`workspace.dsl` de Structurizr**.
PlantUML C4, Mermaid y draw.io son **proyecciones derivadas** que se
regeneran desde el DSL. UML vive en PlantUML. Mermaid queda como
preview ligero (su sintaxis C4 sigue marcada como experimental por la
documentación oficial de Mermaid).

## ADR-0006 — Skills externas versionadas, no copiadas

`archctl` mantiene un **registro propio** de skills externas
(`~/.local/share/archctl/skills/sources/`) en lugar de copiarlas.
Tres modos de uso:

- **`direct`**: symlink a la upstream tal cual (útil para
  `plantuml-skill`, `drawio-skill` ya compatibles).
- **`wrapped`**: la upstream se conserva intacta; añadimos un wrapper
  que impone nuestros contratos (devolver IR, no Mermaid; pedir
  evidencias a `archctl`, no inspeccionar el repo a mano).
- **`patched`**: solo cuando envolver no basta; los parches viven
  fuera del repo, nunca sobre la copia upstream.

Candidatas del documento inicial (líneas 23-30 y 3235-3345):

- `lmammino/c4-codebase-architecture-skill` (wrapped).
- `bitsmuggler/c4-skill` (wrapped, fuente Structurizr).
- `cheriftj/c4-model-skill` (wrapped).
- `Agents365-ai/plantuml-skill` (direct).
- `Agents365-ai/drawio-skill` (direct).

Un `skillset.lock` fija `source`, `commit`, `mode` y `license` para
poder reproducir qué comportamiento generó qué modelo.

## ADR-0007 — Persistencia fuera del repo, renderers locales

Dos consecuencias prácticas del principio "no contaminar el repo":

1. **Persistencia XDG**, fuera del repositorio analizado:
   - `~/.config/archctl/` — config, rule packs, opencode/.
   - `~/.local/share/archctl/projects/<id>/` — modelo, evidencias,
     vistas, snapshots, skills externas.
   - `~/.local/state/archctl/` — runs, checkpoints, locks, events.
   - `~/.cache/archctl/` — índices AST/SCIP/ctags, resultados
     costosos (regenerable).
   - El repo queda limpio: sin `.opencode/`, `.architecture/`,
     `sgconfig.yml` ni `.archctl.yaml` por defecto.

2. **Renderers locales por defecto**: `plantuml.jar`, `structurizr-cli`
   o el viewer `structurizr/lite` local, kroki interno si existe.
   **PlantUML.com y kroki.io públicos quedan bloqueados por defecto**
   (líneas 187-192 del documento inicial). El código y los nombres de
   sistemas no salen de la máquina sin opt-in explícito por run.

La identidad del proyecto y del worktree se calcula sin asumir Git:
`SourceIdentity` es discriminada (`git` con remoto normalizado y root
commit, o `directory` con `realpath` canónico). `projectId` UUIDv4
para bundles export/import portables.