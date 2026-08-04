---
name: diagram-review
description: Validate a generated diagram against its source spec and the canonical graph. Use as the final gate of any `/diagram` invocation, before the run is closed.
license: MIT
compatibility: opencode
metadata:
  version: "0.1.0"
  maturity: experimental
  output-schema: diagram-review-v1
---

# Objective

Confirm that a render corresponds exactly to its view specification,
that the spec corresponds to the canonical graph, and that no
relationship or element is fabricated. Persist the verdict with the
run.

# Required process

1. Receive the `view-id` and the rendered artefact path.
2. Re-derive the view spec from the graph via Cypher queries:
   ```
   archctl graph query "MATCH (e:Element {id: '<view-id>'}) RETURN e"
   ```
   Then query related elements and edges to reconstruct the spec.
3. Compare the render with the spec:
   - Every member in the spec is present in the render.
   - Every relationship in the spec is present and oriented correctly.
   - Nothing extra is in the render.
4. Compare the spec with the graph:
   - Every spec member resolves to a graph element.
   - Every spec relationship resolves to a graph relation with the
     same predicate and endpoints.
5. Emit the verdict as JSON:
   ```json
   {
     "view_id": "view:...",
     "verdict": "pass | pass_with_warnings | fail",
     "missing_members": [],
     "missing_relationships": [],
     "extra_members": [],
     "extra_relationships": []
   }
   ```
6. Persist verdict with `archctl evidence put --kind semantic --file <json>`.
7. If verdict is `fail`, the orchestrator must NOT proceed to
   archive/release.

# Forbidden

- Approving a render that contains elements absent from the spec.
- Approving a spec that contains elements absent from the graph.
- Skipping the graph-vs-spec comparison.
- Allowing drawio-only deliveries to bypass the structural check
  (drawio is a projection, still reviewed).
