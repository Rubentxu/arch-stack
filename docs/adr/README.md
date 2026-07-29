# ADRs — `archctl`

> Esta especificación **reemplaza** los 7 ADRs planos que vivían
> anteriormente en este fichero. La consolidación quedó documentada en
> [ADR-000](ADR-000-reinicio-de-alcance.md). Cada ADR vive ahora en su
> propio fichero bajo `docs/adr/` para mantener la trazabilidad de cada
> decisión por separado.

## Índice

| ID | Título | Estado |
|---|---|---|
| [ADR-000](ADR-000-reinicio-de-alcance.md) | Reinicio de alcance | Aceptado |
| [ADR-001](ADR-001-opencode-first-archctl-sidecar.md) | OpenCode primero; `archctl` como sidecar | Aceptado |
| [ADR-002](ADR-002-topologia-de-agentes.md) | Topología mínima de agentes | Aceptado |
| [ADR-003](ADR-003-reutilizacion-y-adaptacion-de-skills.md) | Reutilización y adaptación de skills | Aceptado |
| [ADR-004](ADR-004-persistencia-externa-xdg.md) | Persistencia externa XDG por proyecto y worktree | Aceptado |
| [ADR-005](ADR-005-ladybugdb-grafo-canonico-y-evidencias.md) | LadybugDB como grafo canónico y evidencias | Aceptado |
| [ADR-006](ADR-006-adaptadores-de-herramientas-cli.md) | Adaptadores de herramientas CLI existentes | Aceptado |
| [ADR-007](ADR-007-modelos-y-renderizadores-de-diagramas.md) | Diagramas como proyecciones del grafo | Aceptado |
| [ADR-008](ADR-008-recuperacion-versionado-y-evolucion.md) | Recuperación, versionado y evolución | Aceptado |
| [ADR-009](ADR-009-relaciones-semanticas-reificadas.md) | Relaciones semánticas reificadas y aristas derivadas | Aceptado |
| [ADR-010](ADR-010-concurrencia-ladybugdb.md) | Concurrencia de LadybugDB y procesos `archctl` | Aceptado |
| [ADR-011](ADR-011-renderers-locales-y-bloqueo-de-publicos.md) | Renderers locales y bloqueo de servicios públicos | Aceptado |
| [ADR-012](ADR-012-adopcion-incremental-crates-analisis.md) | Adopción incremental de crates de análisis como librerías | Aceptado |

## Documentos relacionados

- [`docs/README.md`](../README.md) — resumen ejecutivo.
- [`docs/Skills-para-agentes-IA-v2.md`](../Skills-para-agentes-IA-v2.md) — propuesta base revisada.
- [`docs/DATA-MODEL-LADYBUGDB.md`](../DATA-MODEL-LADYBUGDB.md) — modelo de grafo.
- [`docs/ROADMAP.md`](../ROADMAP.md) — milestones M0–M11.
- [`docs/schema/`](../schema/) — `001_initial_schema.cypher`, `metamodel-core.json`.

## Cómo añadir un nuevo ADR

1. Crear `docs/adr/ADR-NNN-slug.md` siguiendo el formato de los existentes.
2. Añadir la fila correspondiente a este índice.
3. Si la decisión **sustituye** a una anterior, indicar `**Sustituye:** ADR-XXX anterior` en la cabecera.
4. Añadir el fichero a `docs/manifest.json`.
5. Commit dedicado (`docs(adr): ADR-NNN ...`).
