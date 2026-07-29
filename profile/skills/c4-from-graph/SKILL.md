---
name: c4-from-graph
description: Produce a C4 view (Context, Container, Component, Dynamic, Deployment) from the canonical graph. Use when the user asks for any C4 diagram at a known level and root. Wraps `c4-architecture`.
license: MIT
compatibility: opencode
metadata:
  version: "0.1.0"
  maturity: experimental
  wraps: c4-architecture
  output-schema: c4-view-spec-v1
---

# Objective

Project a C4 view from the canonical graph. The diagram is a
projection; the graph is the source of truth.

# Required process

1. Receive `softwareSystem`, level, and purpose from the orchestrator.
2. `archctl graph neighbours <system-id>` to gather members and
   relationships.
3. Coverage gate:
   - Each member has at least one evidence record.
   - Each relationship has at least one evidence record.
   Fail closed and surface the missing evidence; never invent.
4. Build the view specification as JSON:
   ```json
   {
     "id": "view:system:checkout.context",
     "view_type": "c4-context",
     "root": "c4:system:checkout",
     "selectors": [
       { "predicate": "core.uses", "direction": "both" },
       { "predicate": "core.depends_on", "direction": "both" }
     ],
     "exclude_kinds": ["c4.container", "c4.component", "uml.class", "uml.operation"]
   }
   ```
5. `archctl diagram put <spec>` then `archctl diagram materialize
   <view-id>`.
6. Render with `archctl render <materialized.dsl>` (Structurizr, local
   Structurizr CLI / Lite).
7. Hand back the render path, the spec id, and the evidence summary.

# Forbidden

- Adding elements the graph does not contain.
- Including Components or Classes in a Context view.
- Inventing relationships to make the diagram "look complete".
- Rendering against `kroki.io` / `plantuml.com` (ADR-011).
