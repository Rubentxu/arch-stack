# Spec — Internal LensDefinition

## Status
Abstracción interna primero; LensSpec externo sólo tras gate.

## Fields
`id, question_types, applicable_entity_kinds, query_strategy, projection_kind, renderer, layout, overlays, inspector_sections, actions, zoom_targets, rationale`.

## Requirements
Deterministic para mismos inputs, explainable recommendation, override humano y canonical selection preservada.
