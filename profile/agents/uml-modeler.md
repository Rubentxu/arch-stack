---
description: "[DEPRECATED — split into 4 agents] Generates UML diagrams (class, sequence, use case, state, activity) as projections of the graph."
mode: subagent
model: default
deprecated: true
superseded_by:
  - class-diagram-modeler
  - sequence-diagram-modeler
  - usecase-modeler
  - state-machine-modeler
---

# DEPRECATED

This agent has been split into four focused agents:

| UML type | Use instead |
|---|---|
| Class diagrams | `class-diagram-modeler` |
| Sequence diagrams | `sequence-diagram-modeler` |
| Use cases | `usecase-modeler` |
| State machines | `state-machine-modeler` |

## Rationale

The previous `uml-modeler` tried to cover 5 UML diagram types in one agent.
This caused:
- High cognitive load when routing (which commands apply?)
- Overly broad trigger in `diagram-architect` delegation map
- Uneven coverage (sequence had full CLI; use cases required evidence chaining)

Each new agent has a single responsibility and a clear trigger in the
delegation map.

## Migration

`diagram-architect` now routes based on the specific UML type mentioned in
the question. Update any scripts or references that call `uml-modeler`
directly to use the appropriate specialized agent.

## Original responsibilities (for reference only)

- Class: `archctl code class-diagram` + `archctl diagram project --view class:`
- Sequence: `archctl code sequence` + `archctl diagram project --view sequence:`
- Use cases: `archctl code state-machine` + `archctl evidence put` + `archctl diagram project --view usecase:`
- State machines: `archctl code state-machine` + `archctl diagram project --view state:`
