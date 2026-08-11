---
name: c4-from-graph
description: Produce a C4 diagram (Context, Container, Component) from the canonical graph. Use when the user asks for a "C4 diagram", "context diagram", "container diagram", "component diagram", or "system architecture". Projects graph → bundle → DSL with `archctl diagram export` + `diagram project`.
license: MIT
compatibility: opencode
metadata:
  version: "1.0.0"
  maturity: stable
  output-schema: c4-view-spec-v1
---

# Objective

Project a C4 view from the canonical graph. The diagram is a
projection; the graph is the source of truth.

# Required process

1. Receive `softwareSystem`, level (context/container/component), and
   purpose from the orchestrator.
2. Selector grammar: `<c4-kind>:<scope>` where scope is `*` (all),
   an exact id, or a path. Examples:
   - `context:*` — whole system context
   - `container:orders` — containers named/related to `orders`
   - `component:checkout` — components inside `checkout`
3. Export the viewer bundle (for archview) and project to editable DSL:
   ```bash
   # Bundle for the workbench
   archctl diagram export container:* --cwd <dir> --format viewer-bundle --output <out-dir>
   # Editable source (PlantUML / Mermaid / Structurizr)
   archctl diagram project --view c4-container:orders --format structurizr --output orders.dsl --cwd <dir>
   ```
4. Validate any bundle before handing it over:
   ```bash
   archctl diagram validate <out-dir> --cwd <dir>
   ```
5. Hand back: the DSL path, the bundle path, and an evidence summary
   (`archctl evidence list --path <id>` for each member).

# Forbidden

- Including Components or Classes in a Context view (the exporter
  filters by C4 kind — don't bypass it with raw Cypher).
- Inventing relationships the graph does not contain.
- Rendering against `kroki.io` / `plantuml.com` (ADR-011). If the user
  needs an image, serve the workbench instead (`workbench-view` skill)
  or use local renderers.
