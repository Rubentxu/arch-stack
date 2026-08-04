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
2. Extract ordered interactions:
   `archctl code sequence --from <selector>`.
3. Optionally re-project with a different depth or max_interactions.
4. Group technical calls into meaningful participants (do this in the
   agent; `archctl graph aggregate` is not available).
5. Project to PlantUML with
   `archctl diagram project --view sequence:<id> --format plantuml`.
6. Render via local PlantUML (Kroki).

# Forbidden

- Producing a sequence that hides structural facts.
- Cross-cutting between unrelated scenarios without justification.
- Rendering via `kroki.io` / `plantuml.com` (ADR-011).
- Replacing a participant name with a UI label.
- `archctl scenario` — not available; use `archctl code sequence` instead.
- `archctl graph aggregate` — not available; group in the agent.
- `archctl diagram put` / `archctl diagram materialize` — not available;
  use `diagram project` instead.
