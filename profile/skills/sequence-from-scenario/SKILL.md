---
name: sequence-from-scenario
description: Produce a UML sequence diagram for a use case, scenario, endpoint, test, or symbol. Use when the user asks for runtime trace, call path, or inter-service choreography. Wraps `plantuml-skill`.
license: MIT
compatibility: opencode
metadata:
  version: "0.1.0"
  maturity: experimental
  wraps: plantuml-skill
  output-schema: uml-sequence-spec-v1
---

# Objective

Project a sequence diagram at the level chosen by the user (system,
container, component, class, operation) from a scenario or a call
path stored in the graph.

# Required process

1. Receive a scenario id (or `from`/`to` pair) from the orchestrator.
2. archctl scenario ... (deferred — no current CLI subcommand) to gather ordered
   interactions.
3. Optionally re-project:
   archctl scenario ... (deferred — no current CLI subcommand).
4. Group technical calls into meaningful participants using
   archctl graph aggregate ... (deferred — no current subcommand).
5. Build the sequence spec:
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
6. `archctl diagram put` + `archctl diagram materialize`.
7. Render via local PlantUML (Kroki).

# Forbidden

- Producing a sequence that hides structural facts.
- Cross-cutting between unrelated scenarios without justification.
- Rendering via `kroki.io` / `plantuml.com` (ADR-011).
- Replacing a participant name with a UI label.
