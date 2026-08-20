# Visual Architecture

## Visual Compiler
```text
Question / VisualRequest → Lens resolver → Query → Projection → Encoding → Layout → Renderer
```

El agente no elige coordenadas, colores arbitrarios ni HTML/JS.

## Renderer partition
- G6: C4, call graph, impact, evidence path, causal graph.
- ELK: layered/hierarchical layout.
- Matrix Canvas: DSM.
- D3 utilities: treemap/system map/timeline calculations.
- SolidJS: DOM, state, accessibility, composition.

## Core services
WorkspaceController, SelectionBus, LensRegistry, InspectorRegistry, RendererRegistry, VisualGrammar, RevisionStore e InvestigationTrail.

## Stability
Style/metadata delta no relayout. Topology delta usa relayout mínimo y preserva selection/viewport.
