---
name: class-view-from-graph
description: Project a UML class diagram for a bounded context, module, aggregate, or collaboration selected from the graph. Use when the user wants class structure, interface contracts, or aggregate boundaries.
license: MIT
compatibility: opencode
metadata:
  version: "0.1.0"
  maturity: experimental
  output-schema: uml-class-spec-v1
---

# Objective

Select only the classes, interfaces, attributes and operations that
are relevant to the chosen scope and produce a PlantUML class
diagram.

# Required process

1. Receive a scope (module, aggregate, component, collaboration) from
   the orchestrator.
2. `archctl graph query --kind uml.class --scope <scope-id>`.
3. Filter: drop classes with zero relationships inside the scope,
   unless they are interfaces declared as public contracts.
4. Resolve attributes and operations:
   archctl class members ... (deferred — no current subcommand).
5. Build the class spec:
   ```json
   {
     "id": "view:class:orders.aggregate",
     "view_type": "uml-class",
     "scope": "agg:orders",
     "include_private": false,
     "include_operations": true,
     "layout": "sugiyama"
   }
   ```
6. `archctl diagram put` + `archctl diagram materialize`.
7. Render via local PlantUML (Kroki).

# Forbidden

- Pulling classes from outside the scope to "explain" a relationship.
- Showing implementation bodies; this is structure, not source.
- Renaming a class to fit a naming style — the graph name is the
  canonical name.
- Editing the upstream `mermaid-skill` — this wrapper stands alone
  (mermaid is optional).
