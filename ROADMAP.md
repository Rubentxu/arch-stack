# Roadmap — `archctl`

> 4 fases cortas, alineadas con `Skills-para-agentes-IA.md`. Lo que
> importa es qué puede ejecutar el usuario mañana, no la cobertura.

## 0. Lo que ya está vivo

- CLI `archctl` con extracción estructural (ast-grep, ctags), modo
  Git y modo `directory`, proyecciones Structurizr + PlantUML,
  fixtures Rust/TS con SPDX, spike runner.
- Persistencia XDG con `SourceIdentity` y `projectId` portable.
- Renderers locales (plantuml jar, structurizr-cli) en podman.
- Tests verdes.

## 1. C4 + UML de punta a punta

Cerrar el ciclo visible para el usuario:

- `archctl scan <path>` — inventario del repo (lenguajes, entrypoints,
  módulos, tamaño).
- `archctl extract <path>` — extracción por herramientas registradas,
  perfil `fast` (ast-grep outline + ctags + gestor de paquetes) y
  perfil `semantic` (SCIP / LSP / jdeps / dependency-cruiser).
- `archctl model build` — fusiona evidencias en el IR arquitectónico.
- `archctl render` — genera vistas C4 (Structurizr DSL) y UML
  (PlantUML: secuencia, clase, casos de uso, estado, actividad,
  componentes).
- `archctl explain <selector>` — muestra la evidencia y la confianza
  de un elemento.
- `archctl doctor` — valida entorno (binarios, renderers, XDG).

Salida esperada: el usuario corre `archctl render` y obtiene C4 +
UML sin tocar Mermaid ni diagramas externos.

## 2. OpenCode: orquestador + subagentes

Capa fina en OpenCode, no plataforma:

- Plugin global en `~/.config/opencode/plugins/archctl.ts` que
  resuelve proyecto/worktree y expone `archctl` como custom tool.
- Wrapper `archcode` que exporta `OPENCODE_CONFIG_DIR` y `ARCHCTL_*`.
- Agente primario `architecture-orchestrator` + los subagentes
  listados en ADR-0004.
- Skills nativas mínimas: `archctl-evidence` (cómo invocar el CLI y
  leer el JSON de vuelta) y `archctl-render` (cómo pedir cada tipo de
  diagrama).
- Slash command `/archctl` que orquesta todo.

El plugin escribe **solo** bajo `~/.local/share/archctl/...` (write
guard). El repositorio analizado sigue limpio.

## 3. Registro de skills externas + drift básico

- `archctl skills sync|verify|build|test|activate` consume el
  `skillset.lock` (ADR-0006).
- Modos `direct`, `wrapped`, `patched` operativos sobre al menos dos
  candidatas reales del documento inicial (p.ej. `plantuml-skill` en
  `direct`, `c4-codebase-architecture-skill` en `wrapped`).
- `archctl model diff` entre commits: cambios añadidos, eliminados,
  contradicciones nuevas.
- `/archctl update` reanaliza solo el subgrafo afectado.

## Fuera del roadmap (declarado, no se hace)

Esto responde al feedback del usuario: el documento inicial no pidió
ninguna de estas piezas, así que no entran.

- "Gemelo digital temporal" con `validFrom`/`validTo` por relación.
- "Agente falsificador" como subagente separado.
- Migraciones de versiones del IR (el modelo es v1).
- Plataforma de plugins dentro del repo.
- Perfiles MCP separados (`fast`/`semantic`/`deep`/`corporate`).
- Núcleo en Rust (el CLI queda en TypeScript).