# Contexto — `archctl`

> Reconstruido desde `Skills-para-agentes-IA.md`. El pivote del
> proyecto está descrito en detalle en `docs/adr/README.md` y en
> `ROADMAP.md`.

## Qué es

Una mini-aplicación **CLI local** (`archctl`) que ayuda a un usuario
— generalmente desde OpenCode con un orquestador + subagentes — a
**diagramar la arquitectura de un repositorio** en C4 y UML,
extrayendo los hechos con herramientas que ya existen.

## Cómo trabaja

- **Wrapper de herramientas**, no parser propio. `archctl` envuelve
  `ast-grep`, Universal Ctags, `cargo metadata`, `go list`,
  `dependency-cruiser`, `terraform show -json`, `helm template`,
  `kubectl get -o json`, etc. La herramienta extrae hechos;
  `archctl` normaliza, conserva procedencia y construye el modelo.
- **OpenCode como plano de control**: un agente orquestador + 8
  subagentes especializados (ver ADR-0004) que consumen `archctl`
  vía custom tools, no inspeccionan el repo a mano.
- **Persistencia fuera del repositorio analizado** bajo rutas XDG
  (`~/.config/archctl/`, `~/.local/share/archctl/`,
  `~/.local/state/archctl/`, `~/.cache/archctl/`). El repositorio no
  se contamina: ni `.opencode/`, ni `.architecture/`, ni
  `sgconfig.yml`.
- **Renderers intercambiables**. Structurizr DSL es la fuente
  canónica C4; PlantUML es la fuente canónica UML; Mermaid y
  draw.io son proyecciones. Por defecto todo renderizado es local
  (`plantuml.jar`, `structurizr-cli`, kroki interno).
- **Skills externas versionadas** (no copiadas) en un registro
  propio con tres modos: `direct`, `wrapped`, `patched`.

## Lo que produce

- **C4**: Context, Container, Component, Deployment, Dynamic.
- **UML**: sequence, class, use case, state, activity, component.

Cada elemento del modelo lleva **evidencias** que apuntan a archivos
y líneas del repo (HECHO / INFERENCIA / HIPÓTESIS / DESCONOCIDO /
CONFLICTO). Nada se inventa.

## Lo que NO hace

- No dibuja diagramas directamente desde nombres de fichero.
- No contamina el repositorio analizado.
- No usa renderers públicos (PlantUML.com, kroki.io) por defecto.
- No implementa parsers propios: reutiliza los del ecosistema.
- No copia skills externas para modificarlas.