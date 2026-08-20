# Research Synthesis

## Moldable Development / GToolkit
Muchas vistas contextuales por objeto, acciones contextuales, exploraciones encadenadas y pequeñas herramientas específicas. Aplicación: `InspectorRegistry`, `LensDefinition`, investigation trail y Architecture Stories.

## Visual Analytics
Overview first, zoom/filter, details on demand, coordinated multiple views y brushing+linking. Aplicación: Graph + DSM + Source + Timeline compartiendo selección.

## C4 / Structurizr / IcePanel
El modelo existe independientemente de cada vista; los flows explican comportamiento sobre el mismo modelo. C4 es una lente, no el modelo canónico.

## Concept Maps
Adecuados para intención, decisiones, rationale y constraints. Aplicación: Intent Map.

## DSM
Node-link pierde legibilidad en grafos densos. DSM revela capas, clusters y ciclos.

## CodeScene / software visualization
Topología estable + overlays de churn, health, ownership, riesgo y coverage.

## Graphify
Separar extracción estructural e inferencia, provenance/confidence, comunidades, delta/impact. No sustituir evidencia determinista por inference.

## ActiveGraph
Eventos, behaviours, causalidad, scopes/views, replay y fork/diff. Adoptar el patrón de journal/causalidad gradualmente; no reemplazar Ladybug ni hacer event sourcing aún.

## CodeSpeak
Intent recovery, requisitos atómicos, intent diff, grounding, intent coverage y human refinement. No adoptar “specs instead of code” ni specs como única verdad.

## Twip
Timeline causal de interacción agentic + estados Git. Aplicación: HumanPrompt → AgentTurn → Tool → Mutation → GraphRevision → Finding → Feedback.

## tldraw
Interesante para Thinking Canvas agentic, pero posterior y nunca source of truth.
