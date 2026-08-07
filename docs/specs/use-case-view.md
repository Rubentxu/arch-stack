# Spec — Use case view (UML actors + use cases)

> **Change**: M39
> **Cycle**: CYC-2026-08-07-m39-use-case-diagrams-end-to-end
> **Branch**: `feat/m39-use-case-shapes-and-e2e-render` @ `<tip>`
> **Status**: in_progress

This delta spec documents the use case view contract that archctl emits when
the view selector is `usecase:*`. It covers the element kinds, the edge
predicate, and the shape mapping for each downstream projection target
(Mermaid, PlantUML).

---

## Element kinds

| Kind | Semantic | Role |
|---|---|---|
| `uml.actor` | A UML actor (human or system outside the boundary) | Rounded rect in Mermaid (closest Mermaid approximation of UML stick figure) |
| `uml.use_case` | A UML use case (ellipse) | Circle in Mermaid (`((name))`); `usecase <name>` in PlantUML |

Both kinds live under the `uml` category (selector `category:uml` would also match them).

## Edge predicate

| Predicate | Semantic |
|---|---|
| `usecase.participates_in` | An actor participates in a use case (UML association) |

The source of the edge MUST be an `uml.actor` and the target MUST be an
`uml.use_case`. Edges that violate this are silently dropped by the
projection (consistent with the rest of the projector's behavior).

## Projection shape mapping

### Mermaid (`--format mermaid`)

```
flowchart TD
    e1(Customer)              # actor: rounded rect
    e2((PlaceOrder))          # use case: circle (closest to UML ellipse)
    e1 --> e2                 # participates_in association
```

| Element kind | Mermaid shape | Rationale |
|---|---|---|
| `uml.actor` | `id(name)` (rounded rect) | Mermaid has no native stick figure; rounded rect is the closest standard shape |
| `uml.use_case` | `id((name))` (circle) | Mermaid circle is the closest approximation of a UML ellipse |
| Association `usecase.participates_in` | `srcId --> tgtId` | Directed arrow with source = actor id, target = use case id |

**Critical Mermaid quirk**: bare `(Label)` syntax is REJECTED by merman.
Every node declaration MUST include an ID prefix. Pre-M39, the projector
emitted bare `(Label)` which silently failed end-to-end (only substring
unit tests passed). M39 fixes this and adds `archctl/tests/usecase_view_e2e.rs`
as the regression test.

### PlantUML (`--format plantuml`)

```
[Customer]                    # actor: bracket shape (SCN-412 design decision)
usecase PlaceOrder            # use case: native PlantUML usecase syntax
Customer --> PlaceOrder       # participates_in association
```

| Element kind | PlantUML shape | Rationale |
|---|---|---|
| `uml.actor` | `[name]` (bracket) | PlantUML native actor would be `(name)` (stick figure) but the project's SCN-412 chose brackets for visual consistency with the rest of the C4 view |
| `uml.use_case` | `usecase name` | Native PlantUML use case syntax |
| Association `usecase.participates_in` | `src --> tgt` | Reference by display name (PlantUML is more lenient than Mermaid) |

PlantUML SVG rendering remains deferred to M40 (graphviz vendor strategy).
The PlantUML text output is correct and parseable.

## End-to-end test

`archctl/tests/usecase_view_e2e.rs` asserts:

1. Projecting a `usecase:*` bundle produces Mermaid source with the documented shapes.
2. The Mermaid source, when rendered via `archctl::render::mermaid::render`, produces
   a valid SVG containing `<svg` root and both actor + use case names as text nodes.
3. An empty use case bundle still renders a minimal valid SVG.

This test is the regression lock for M39: it would have caught the
pre-M39 bug where bare `(Label)` syntax rendered to a parse error.

## Out of scope

- **System boundary boxes** (UML convention to enclose all use cases). Mermaid has
  no native boundary primitive. Defer.
- **Use case descriptions** (notes below the use case name). Defer to a follow-up cycle.
- **PlantUML SVG rendering**. Deferred to M40.
- **`archctl diagram export --view usecase:* --format mermaid`** as a unified one-shot
  command. The current pipeline is `diagram project` → `render` as two calls.