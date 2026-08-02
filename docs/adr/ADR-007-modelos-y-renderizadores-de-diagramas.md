# ADR-007 — Diagramas como proyecciones del grafo + split render estático / viewer ortogonal

**Estado:** Aceptado (sustituido por [ADR-013](ADR-013-viewer-ortogonal.md) en la sección de rendering; **ViewEdge diferido** — revisado 2026-08-01, ver `docs/audits/2026-08-01-archctl-adr-vs-impl.md` §F5)
**Fecha:** 29 de julio de 2026
**Última revisión:** 31 de julio de 2026 (pivot a Code Knowledge Graph Workbench; ver [ADR-020](ADR-020-renderer-stack.md) para el stack performance-first)
**Última revisión (F5):** 1 de agosto de 2026
**Refuerza:** ADR-005 (LadybugDB como grafo canónico), ADR-013 (viewer ortogonal)

## Contexto

C4 y UML tienen metamodelos diferentes. Mermaid, PlantUML, Structurizr y draw.io son formatos de representación, no fuentes de verdad completas. Los diagramas son **proyecciones** del grafo canónico en LadybugDB, no al revés.

Adicionalmente, el rendering tiene dos modos distintos:

1. **Estático**: `archctl render` produce un artefacto SVG/DOT/PUML viewable en cualquier visor, committable, embeddable en Markdown, generable en CI sin browser.
2. **Interactivo**: drill-down Context → Container → Component, pan/zoom, hover sobre evidencias, comparación temporal, edición visual. Requiere un viewer dedicado.

Estos dos modos no conviven en el mismo binario. El modo estático es sidecar (un comando, una salida, termina); el modo interactivo es una aplicación separada que consume bundles generados por el sidecar. Ver [ADR-013](ADR-013-viewer-ortogonal.md).

## Decisión

### Grafo canónico

LadybugDB conserva identidades, relaciones, escenarios y evidencias. Los diagramas son **proyecciones** derivadas; cualquier mutación al modelo pasa por el grafo, no por el render.

### Modos de salida

| Modo | Quién produce | Output | Cuándo |
|---|---|---|---|
| Estático (archctl) | `archctl render <source>` | `.svg`, `.puml`, `.dsl` | CI, agentes, embedding en Markdown, revisión sin browser |
| Bundle para viewer (archctl) | `archctl diagram export <id>` | directorio `diagram-bundle/` con `manifest.json`, `projection.json`, `evidence.json`, `styles.json`, `assets/` | Cuando el usuario abre el viewer interactivo |
| Interactivo (archview, proyecto separado) | Carga bundle desde disco | HTML+SVG interactivo en browser local | **Code Knowledge Graph Workbench** — drill-down C4, call graph, sequence, class, package, drift detection, impact analysis |

### Pivot a "Code Knowledge Graph Workbench" (revisión 2026-07-31)

Tras re-evaluación del roadmap contra `docs/Librerías-visualización-grafos-BI.md`, el target de usuario no es BI dashboard sino **developer/architect code intelligence**. La consecuencia arquitectónica es:

- `archview` no es "static viewer" → es un **workbench** con 5 vistas coordinadas (C4 contextual, call graph, sequence, class, package).
- El grafo completo nunca es el diagrama. Cada vista es una **proyección calculada, limitada y explicable** del mismo grafo de conocimiento.
- Las proyecciones se calculan en **Rust → WASM** (algorithms, centralities, layouts) y se renderizan con **G6 5.x + WebGPU** o **cosmos.gl** (massive graphs). El workbench es zero-jank a cualquier nivel de complejidad. Ver [ADR-020](ADR-020-renderer-stack.md) para el stack completo y [ADR-019](ADR-019-performance-budget.md) para el contrato de rendimiento.
- La performance es **prioridad #1** del workbench: hard contract (TTFP <1s, pan/zoom 60 FPS, filter <50ms, memory <500MB para 100k nodos). El grafo puede tener millones de relaciones; el workbench no se atasca.

### Formatos preferentes por tipo de diagrama (modo estático)

| Tipo | Formato preferente | Crate Rust |
|---|---|---|
| C4 (Context, Container, Component, Dynamic, Deployment) | Structurizr DSL → SVG via dagre-rs + svg crate | (M9) |
| UML Use Case | PlantUML via `plantuml-little` | (M9) |
| UML Class, Sequence, Activity, State, Component | PlantUML via `plantuml-little` | (M9) |
| Mermaid | Mermaid via `merman` | (M9) |
| draw.io | draw.io XML (output estático) | — (no prioritario) |

### Formato para el viewer (modo interactivo)

`DiagramProjection` JSON versionado independientemente del binario. Ver [ADR-013 § Contrato](ADR-013-viewer-ortogonal.md#contrato-diagramprojection-bundle).

### Vista persistida

Cada diagrama se representa en el grafo mediante:

```text
view.diagram
view.member
view.edge
view.group
```

Los miembros referencian elementos canónicos. Las aristas de vista referencian relaciones canónicas. El layout automático (Rust en M9, ELK.js en archview) no se persiste por defecto; solo los **overrides manuales** que el usuario marque explícitamente.

## Regla de calidad

Un diagrama debe ser:

1. **Renderizable**: archctl produce `.svg` o un bundle válido; archview abre el bundle sin errores.
2. **Apropiado para su notación**: C4 con Structurizr, UML con PlantUML, vista exploratoria con Cytoscape.js (en archview).
3. **Sustentado**: cada elemento tiene al menos una evidencia o es explícitamente marcado como `inferred`.
4. **Legible**: layout no solapa, etiquetas dentro de los nodos o adyacentes.
5. **Acotado**: una vista tiene un scope claro (Context, Container, Component, etc.) y un propósito.
6. **Reproducible**: el mismo grafo + la misma vista + el mismo layout producen el mismo artefacto.

## Consecuencias

- Un cambio visual no altera el modelo. La edición visual pasa por `archctl diagram apply --changes` (ADR-013 § Cambios de vuelta).
- C4 mantiene consistencia entre vistas (mismos IDs en Context, Container, Component).
- UML conserva expresividad.
- El mismo escenario produce UML Sequence (PlantUML) y C4 Dynamic (Structurizr).
- `archview` no conoce LadybugDB ni el repositorio; solo bundles.
- `archctl render` no abre browser, no mantiene conexiones, no sirve HTML.

## Split con ADR-013

| Aspecto | ADR-007 (este) | ADR-013 |
|---|---|---|
| Modelo canónico | ✓ (LadybugDB) | — (asume) |
| Tipos de diagrama | ✓ (C4, UML, Mermaid, draw.io) | — (asume) |
| Modo estático | ✓ (`archctl render`) | — |
| Modo interactivo | referencia | ✓ (archview) |
| Cambio visual → grafo | referencia a `archctl diagram apply` | ✓ (ChangeSet) |
| Bundle contract | referencia a schema versionado | ✓ (formato y JSON schema) |

## Revisión (1 de agosto de 2026) — **ViewEdge diferido**

`docs/audits/2026-08-01-archctl-adr-vs-impl.md` §F5 flaggeó que el schema
de view-persistence (003_view_nodes.cypher) declara solo
`Diagram`, `ViewMember`, `ViewGroup` y sus edges `MEMBER_OF`,
`RENDERS`, `GROUP_CONTAINS`. La `view.edge` (overrides a nivel de
arista) declarada en este ADR (§Vista persistida) **no está
implementada**.

Decisión: **diferir `ViewEdge` a 1.x (M17.x archview)**.

Razones:

- La pipeline `archctl diagram apply` actual tiene solo 3 commands:
  `move-member`, `collapse-group`, `set-label`. No hay
  `add-edge` / `edit-edge` / `remove-edge`. Implementar `ViewEdge`
  requiere:
  - nueva `ViewEdge` table + rel table (`VIEW_EDGE` o similar)
  - 3 nuevos commands en `Command` enum
  - nuevos métodos en `GraphStore::put_view_edge` /
    `link_view_edge` / `unlink_view_edge`
  - expansión de `Command::apply()` para los 3 nuevos variants
  - schema migration (v4)
- El caso de uso principal de `ViewEdge` (decoradores, badges,
  highlighting de aristas en el workbench) es de **archview**, no
  de `archctl`. El bundle que `archctl diagram export` produce
  incluye los miembros con sus `r.props` (decoradores planos);
  archview puede extender visualmente sin requerir el row
  `ViewEdge` en el grafo canónico.
- El costo de implementar + mantener `ViewEdge` no compensa
  mientras el `archctl diagram apply` cubre los 3 escenarios
  principales (mover, colapsar, etiquetar).

### Cuándo revocar esta deferral

- `archview` necesita **filtros persistentes sobre aristas** (no solo
  decoradores; ej. "ocultar todas las relaciones con confianza < 0.5
  en esta vista").
- El equipo decide implementar un editor visual completo (M17+).

