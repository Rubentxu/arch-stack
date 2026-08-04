---
name: use-cases-from-graph
description: Derive UML use cases from actors and confirmed goals in the graph. Use when the user asks for use cases, the system landscape needs actor mapping, or a class view requires its collaborators first. Wraps `c4-model`.
license: MIT
compatibility: opencode
metadata:
  version: "0.1.0"
  maturity: experimental
  wraps: c4-model
  output-schema: uml-usecase-spec-v1
---

# Objective

Identify actors and use cases, distinguish candidates inferred from
the codebase from use cases confirmed by tests or docs, and relate
each use case to the scenarios it is realised by.

# Required process

1. Enumerate actors via Cypher:
   ```
   archctl graph query "MATCH (e:Element) WHERE e.kind_id = 'uml.actor' RETURN e.id, e.current_name"
   ```
2. For each actor, enumerate goal candidates via Cypher:
   ```
   archctl graph query "MATCH (a:Element {id: '<actor-id>'})-[r]-(e:Element) WHERE r.predicate_id CONTAINS 'participates' RETURN e.id, e.current_name"
   ```
3. Classify each candidate:
   - `confirmed`: at least one scenario in the graph references it.
   - `inferred`: only static evidence (file, doc, symbol).
4. Reject candidates with zero evidence.
5. Project to PlantUML with
   `archctl diagram project --view usecase:<scope> --format plantuml`.
6. Render via local PlantUML (Kroki) — never `plantuml.com`.

# Forbidden

- Promoting an `inferred` candidate to `confirmed` without explicit
  user/test evidence.
- Hiding actors behind a UI shorthand (the diagram is the projection,
  not the UI).
- Mixing `c4-component` kinds into a use-case view.
