# Contexto — `archctl`

> Reescrito desde `Skills-para-agentes-IA.md` después del reset (commit `f5e7f83`).
> Este documento es breve a propósito. Para la propuesta completa lee
> `Skills-para-agentes-IA.md`.

## Qué es esto

Una **mini-aplicación CLI local** (`archctl`) que ayuda a un usuario
(generalmente a través de OpenCode y un orquestador + subagentes) a
**diagramar la arquitectura de un repositorio** en C4 y UML.

## Cómo trabaja

- **CLI local** que envuelve herramientas que ya existen
  (`ast-grep`, `ctags`, `cargo metadata`, `go list`, `npm ls`, etc.) sin
  reescribirlas. La herramienta extrae hechos; `archctl` normaliza
  resultados, conserva procedencia y construye el modelo.
- **Persistencia fuera del repositorio analizado** bajo rutas XDG
  (`~/.config/archctl/`, `~/.local/share/archctl/`, `~/.local/state/archctl/`,
  `~/.cache/archctl/`). El repositorio no se contamina.
- **Renderers intercambiables**: Mermaid, PlantUML, Structurizr DSL,
  draw.io. La fuente canónica C4 es Structurizr DSL.
- **Skills externas versionadas** (no copiadas) en un registro propio
  con tres modos de uso: `direct`, `wrapped`, `patched`.
- **Plugin OpenCode** que inyecta `ARCHCTL_PROJECT_*` y hace cumplir el
  write-guard.

## Identidad

- `SourceIdentity` discriminada: `git` o `directory` (no asumimos Git).
- `projectId` UUIDv4 portable para bundles export/import.

## Lo que NO hace

- No dibuja diagramas directamente desde nombres de fichero.
- No contamina el repositorio con `.opencode/`, `.architecture/` ni
  `sgconfig.yml`.
- No usa renderers públicos (PlantUML.com, kroki.io) por defecto; solo
  locales (`structurizr/structurizr`, `kroki`, `plantuml` jar).
- No implementa parsers propios; reutiliza los del ecosistema.
