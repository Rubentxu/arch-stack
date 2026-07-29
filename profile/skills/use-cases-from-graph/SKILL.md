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

1. `archctl graph query --kind uml.actor` to enumerate the actors.
2. For each actor, `archctl graph query --predicate uml.participates_in
   --from <actor-id>` to enumerate goal candidates.
3. Classify each candidate:
   - `confirmed`: at least one scenario in the graph references it.
   - `inferred`: only static evidence (file, doc, symbol).
4. Reject candidates with zero evidence.
5. Build the use-case spec:
   ```json
   {
     "id": "view:usecase:checkout",
     "view_type": "uml-usecase",
     "system": "c4:system:checkout",
     "include_inferred": false,
     "actors": ["actor:customer", "actor:staff"],
     "use_cases": ["uc:place-order", "uc:apply-coupon"]
   }
   ```
6. `archctl diagram put` + `archctl diagram materialize`.
7. Render via local PlantUML (Kroki) — never `plantuml.com`.

# Forbidden

- Promoting an `inferred` candidate to `confirmed` without explicit
  user/test evidence.
- Hiding actors behind a UI shorthand (the diagram is the projection,
  not the UI).
- Mixing `c4-component` kinds into a use-case view.
