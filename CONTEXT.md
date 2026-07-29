# Contexto — `archctl`

> Resumen breve. La especificación completa vive en
> [`docs/README.md`](docs/README.md),
> [`docs/Skills-para-agentes-IA-v2.md`](docs/Skills-para-agentes-IA-v2.md),
> [`docs/DATA-MODEL-LADYBUGDB.md`](docs/DATA-MODEL-LADYBUGDB.md) y los
> [ADRs](docs/adr/README.md). `CONTEXT.md` no contradice esa
> documentación; si lo hace, gana la documentación detallada.

## Qué es

`archctl` es una **CLI sidecar local** que asiste a un agente OpenCode
(`diagram-architect` orquestador + cuatro subagentes) a producir
diagramas C4 y UML a partir de un repositorio. **Persiste, consulta,
normaliza y proyecta.** No decide qué diagrama hace falta y no
interpreta la arquitectura por su cuenta.

## Restricciones duras

- **Persistencia fuera del repositorio**, en XDG. Por defecto el repo
  no contiene `.opencode/`, `.architecture/`, `.archctl.yaml` ni
  `sgconfig.yml`.
- **Renderers locales** por defecto (PlantUML jar, Structurizr CLI /
  `structurizr/lite` local, Kroki interno). `plantuml.com` y `kroki.io`
  bloqueados sin opt-in explícito por run. Ver ADR-011.
- **Herramientas existentes envueltas, no reimplementadas.**
  `archctl` orquesta adaptadores para `ast-grep`, ctags, `cargo
  metadata`, `go list`, `dependency-cruiser`, `terraform show -json`,
  `helm template`, `kubectl get -o json`, `jdeps`, Syft, etc. Ver
  ADR-006.
- **Skills upstream en tres modos** (`direct`, `wrapped`, `patched`)
  sin copiarlas. `skills.lock.yaml` fija `source`, `commit`, `license`.
  Ver ADR-003.

## Pivote arquitectónico

- **OpenCode es el plano cognitivo.** Un agente primario +
  cuatro especialistas, todos consumiendo `archctl` vía custom tools.
  Ver ADR-002.
- **LadybugDB** es el grafo canónico de C4 y UML con versionado por
  snapshot y relaciones reificadas. Ver ADR-005, ADR-008 y ADR-009.
- **`archctl` no es un daemon en el MVP.** La concurrencia se resuelve
  con lockfiles por proyecto. Ver ADR-010.

## Lo que produce

- **C4**: Context, Container, Component, Dynamic, Deployment.
- **UML**: casos de uso, clases, secuencia, actividad, estado,
  componentes.

Diagramas son **proyecciones** del grafo. Structurizr DSL es la fuente
canónica de C4; PlantUML es la canónica de UML; Mermaid y draw.io
entran como proyecciones alternativas (Mermaid C4 no canónico por
limitación oficial-experimental). Ver ADR-007.

Cada elemento y cada relación del grafo lleva evidencias que apuntan a
archivos y líneas del repo. Una afirmación sin evidencia y con confianza
alta se rechaza (regla de auditoría local por nodo/arista).
