# Roadmap — `archctl`

> Reset: commit `f5e7f83`. Lista plana, sin fases académicas. Lo que
> cuenta es qué ejecuta el usuario mañana.

## 0. Lo que ya está vivo

- CLI `archctl` con extracción estructural (ast-grep, ctags), proyecciones
  Structurizr + PlantUML, renderers locales en podman, fixtures reales
  Rust/TS con SPDX, spike runner.
- Persistencia XDG con `SourceIdentity` y `projectId` portable.
- Tests verdes.

## 1. Cerrar el ciclo de uso (lo que el usuario ve)

- `archctl scan <path>` — inventario del repo (lenguajes, entrypoints,
  módulos, tamaño). Perfil `fast`.
- `archctl extract <path>` — extrae evidencia vía herramientas
  registradas.
- `archctl model build` — fusiona evidencias en un modelo interno.
- `archctl render` — genera diagramas a través del renderer que
  encaje.
- `archctl explain <selector>` — muestra evidencia y confianza de un
  elemento.
- `archctl doctor` — valida entorno (binarios, renderers, XDG).

## 2. Conectar con OpenCode (capa fina, no plataforma)

- Un plugin global en `~/.config/opencode/plugins/archctl.ts` que inyecta
  las env vars `ARCHCTL_PROJECT_*` por `shell.env`.
- Cuatro agentes mínimos: orchestrator, extractor, modeler, auditor.
- Dos skills nativas: `archctl-evidence` (cómo invocar el CLI) y
  `archctl-render` (cómo pedir diagramas).
- Un slash command `/archctl` que orquesta los dos.

## 3. Lo que NO es roadmap (declarado fuera)

- "Gemelo digital temporal" — no es lo que pidió el usuario.
- "Agente falsificador" separado — el auditor basta.
- M3 Rust — el CLI no necesita Rust.
- "Migraciones de IR" — el modelo es v1, no necesita v2 aún.
- Plataforma de plugins en el repo — el plugin va en `~/.config/opencode/`.
