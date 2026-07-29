# ADRs — `archctl`

> Reset: commit `f5e7f83`. Solo decisiones operativas que el código
> refleja. Sin especulación ni "modelos intermedios neutrales".

## ADR-0001 — Reutilizar herramientas, no reimplementar

`archctl` **no** escribe parsers, indexadores ni call graphs propios.
Usa `ast-grep`, `ctags`, `cargo metadata`, `go list`, `dependency-cruiser`,
`jdeps`, `terraform show -json`, `helm template`, `kubectl get -o json`, etc.
La herramienta extrae hechos; `archctl` normaliza, conserva procedencia
y construye el modelo.

## ADR-0002 — Persistencia fuera del repositorio

Por defecto, `archctl` **no escribe absolutamente nada** dentro del
repositorio analizado. El estado vive bajo XDG:

- `~/.config/archctl/` — config, rule packs, opencode/.
- `~/.local/share/archctl/projects/<id>/` — modelo, evidencias, vistas.
- `~/.local/state/archctl/` — runs, checkpoints, locks, events.
- `~/.cache/archctl/` — índices AST, ctags, scip, joern, sbom (regenerable).

El repositorio solo recibe opcionalmente un `.archctl.yaml` mínimo
(declarativo, no generado) si el usuario quiere configurar overrides.

## ADR-0003 — SourceIdentity sin asumir Git

`archctl` no asume que el repositorio analizado tenga Git. La
identidad se calcula así:

- `git` mode: `BLAKE3(normalized_remote + root_commit)`.
- `directory` mode: `BLAKE3(canonical_realpath)`.
- `projectId`: UUIDv4 derivado del contenido para bundles export/import.

Cuando se hace un bundle en una máquina e import en otra, el
`SourceIdentity` local se recalcula y se hace rebind **explícito** (por
defecto, **rechaza y pregunta**).

## ADR-0004 — Renderers locales, no públicos

Por defecto `archctl` solo usa renderers locales:

- Structurizr `local` (viewer) o `structurizr-cli` (headless).
- PlantUML `local` jar o internal Kroki.
- Mermaid en navegador local (no se envía a GitHub).

PlantUML.com y kroki.io quedan **bloqueados** por defecto. Activar
servicios públicos requiere opt-in explícito por run.

## ADR-0005 — Structurizr DSL como fuente canónica C4

Para diagramas C4, la fuente canónica es `workspace.dsl` de Structurizr.
PlantUML C4 y Mermaid son **proyecciones** derivadas. Mermaid C4
queda como preview no canónico (su sintaxis es oficial-experimental).

## ADR-0006 — Skills versionadas, no copiadas

`archctl` mantiene un registro propio de skills externas en
`~/.local/share/archctl/skills/sources/`. Tres modos de uso:

- `direct`: symlink a la upstream compatible.
- `wrapped`: la upstream se conserva intacta; añadimos un wrapper que
  impone nuestros contratos.
- `patched`: solo cuando envolver no basta; parches almacenados
  externamente, nunca en la copia upstream.

El `skillset.lock` registra `source`, `commit`, `mode`, `wrapper`,
`license` para cada skill.

## ADR-0007 — Plugin OpenCode global, no en el repo

El plugin de OpenCode vive en
`~/.config/opencode/plugins/archctl.ts` (global), nunca en el
repositorio analizado. El wrapper `archcode` (lanzador de OpenCode
con la config del plugin) carga la distribución:

```bash
archcode
  ↓
export OPENCODE_CONFIG_DIR=~/.config/opencode/
export ARCHCTL_* (vía shell.env)
exec opencode "$@"
```
