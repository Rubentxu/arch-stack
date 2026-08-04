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
2. Query classes via Cypher:
   ```
   archctl graph query "MATCH (e:Element) WHERE e.kind_id = 'uml.class' AND e.current_name STARTS WITH '<scope>' RETURN e.id, e.current_name"
   ```
3. Filter: drop classes with zero relationships inside the scope,
   unless they are interfaces declared as public contracts.
4. Resolve attributes and operations via graph query:
   ```
   archctl graph query "MATCH (e:Element {kind_id: 'uml.class'})-[r]-(a:Element {kind_id: 'uml.attribute'}) WHERE e.current_name = '<class>' RETURN a.id, a.current_name"
   ```
5. Project to PlantUML with `archctl diagram project --view class:<scope> --format plantuml`.
6. Render via local PlantUML (Kroki).

# Forbidden

- Pulling classes from outside the scope to "explain" a relationship.
- Showing implementation bodies; this is structure, not source.
- Renaming a class to fit a naming style — the graph name is the
  canonical name.
- Editing the upstream `mermaid-skill` — this wrapper stands alone
  (mermaid is optional).
- `archctl class members` — not available; use graph query instead.
- `archctl diagram put` / `archctl diagram materialize` — not available;
  use `diagram project` instead.
- `archctl graph query --kind` — not available; use Cypher directly.
