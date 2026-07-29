---
name: c4-context
description: Produce a C4 Context view from the canonical graph. Use when the user asks for a System Context, Container or Component diagram, or after `architecture-evidence` confirms enough elements to project.
license: MIT
compatibility: opencode
metadata:
  version: "0.1.0"
  maturity: experimental
  output-schema: c4-view-spec-v1
---

# Objective

Produce a C4 Context view (Level 1) from the graph. The diagram is a
projection; the graph is the source of truth.

# Required process

1. Receive a `softwareSystem` root from the orchestrator.
2. Query `archctl graph neighbours <system-id>` to gather the
   neighbouring systems and persons.
3. Check evidence coverage:
   - Each neighbouring system has at least one evidence record.
   - Each relationship has at least one evidence record.
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
5. Persist via `archctl diagram put specification.json`.
6. Materialize: `archctl diagram materialize <view-id>`.
7. Render: `archctl diagram render <view-id>` (Structurizr `local`).
8. Hand the result back with the render path and the
   `evidence_summary`.

# Forbidden

- Adding elements that the graph does not contain.
- Including Components or Classes in a Context view.
- Inventing relationships to make the diagram "look complete".
