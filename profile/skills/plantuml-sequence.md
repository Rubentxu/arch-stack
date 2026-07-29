---
name: plantuml-sequence
description: Produce a UML sequence diagram from a scenario or a call path. Use when the user asks for a sequence diagram, a runtime trace, or a call path between two elements.
license: MIT
compatibility: opencode
metadata:
  version: "0.1.0"
  maturity: experimental
  output-schema: uml-sequence-spec-v1
---

# Objective

Produce a UML sequence diagram from the canonical graph. The
projection level is selected by the user (system, container,
component, class, operation).

# Required process

1. Receive a `scenario` (or a `from`/`to` pair) from the
   orchestrator.
2. Query the scenario's interactions:
   `archctl scenario interactions <scenario-id>`.
3. Optionally project to a level:
   `archctl scenario project <scenario-id> --level <level>`.
4. Build the view specification as JSON:
   ```json
   {
     "id": "view:sequence:create-order",
     "view_type": "uml-sequence",
     "scenario": "behavior:scenario:orders/create-order/success",
     "projection_level": "component",
     "include_fragments": true,
     "hide_returns": false
   }
   ```
5. Persist via `archctl diagram put specification.json`.
6. Materialize: `archctl diagram materialize <view-id>`.
7. Render: `archctl diagram render <view-id>` (PlantUML via local
   Kroki).
8. Return the render path and the evidence summary.

# Forbidden

- Producing a sequence that hides the structural facts.
- Exposing implementation details that are not in the graph.
- Cross-cutting between unrelated scenarios without justification.
