# ADR-007 — Diagramas como proyecciones del grafo + split render estático / viewer ortogonal

**Estado:** Aceptado (sustituido por [ADR-013](ADR-013-viewer-ortogonal.md) en la sección de rendering)
**Fecha:** 29 de julio de 2026
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
| Interactivo (archview, proyecto separado) | Carga bundle desde disco | HTML+SVG interactivo en browser local | Drill-down, hover, comparación temporal, edición visual |

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
