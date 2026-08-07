# Spec — State machine + C4 view Mermaid shape mapping (M41)

> **Change**: M41
> **Cycle**: CYC-2026-08-07-m41-state-and-c4-e2e
> **Branch**: `feat/m41-state-and-c4-e2e` @ `<tip>`
> **Status**: in_progress

This delta spec extends the M39 use-case-view fix pattern to two more views
that share the same Mermaid projector bug class: bare `[Label]` / `(Label)`
syntax that merman silently rejects.

---

## Affected views (post-M39 audit)

| View | Kinds | Mermaid shape (post-M41) |
|---|---|---|
| Use case | `uml.actor`, `uml.use_case` | `id(name)`, `id((name))` (fixed M39) |
| **State** | `uml.state` | `id([name]):::state` (fixed M41) |
| **C4** | `c4.person` | `id(name)` (fixed M41) |
| **C4** | `c4.software_system`, `c4.container`, `c4.component` | `id([name])` (fixed M41) |
| Class | `uml.class`, `uml.interface` | `Name("Display Label")` — already valid |
| Sequence | `behavior.participant` | `participant Name` — already valid |

After M41, **every view's Mermaid projection produces parseable, renderable SVG**.

## State machine view

### Mermaid shape mapping

| Element kind | Mermaid shape | Rationale |
|---|---|---|
| `uml.state` | `id([name]):::state` | Rectangle with `state` class assignment; style applied via `classDef state ...` (emitted automatically) |

### Edge projection

| Predicate | Shape |
|---|---|
| `behavior.source_state` | `srcId --> tgtId` (node IDs) |

### Example

```
flowchart TD
    e1([Idle]):::state
    e2([Active]):::state
    e1 --> e2
    classDef state fill:#f9f,stroke:#333,stroke-width:2px
```

### PlantUML (unchanged — already valid)

The PlantUML state projection was already correct (`state Name { }` block
syntax, transitions joined via behavior.source_state + behavior.target_state).
The `state_view_produces_valid_plantuml` test passed before M41.

## C4 view

### Mermaid shape mapping

| Element kind | Mermaid shape | Rationale |
|---|---|---|
| `c4.person` | `id(name)` | Rounded rect (Mermaid approximation of UML person; Mermaid has no stick figure) |
| `c4.software_system`, `c4.container`, `c4.component` | `id([name])` | Rectangle — standard C4 box |

### Edge projection

| Predicate | Shape |
|---|---|
| `core.uses`, `core.depends_on` | `srcId --> tgtId` (node IDs) |

### Example

```
flowchart TD
    e1(Customer)
    e2([Orders])
    e3([WebApp])
    e4([Database])
    e1 --> e2
    e2 --> e3
    e3 --> e4
```

### PlantUML (M50 fix — vanilla PlantUML syntax)

**Pre-M50**: the C4 view PlantUML projector emitted lowercase Structurizr
keywords (`person "X" { }`, `container "Y" { }`) inside `@startuml`/`@enduml`.
This syntax is rejected by vanilla Java PlantUML unless the C4-PlantUML stdlib
is loaded via `!include <C4/Container>`. The projector output was effectively
broken for any non-Structurizr renderer.

**M50 fix**: the projector now emits native vanilla PlantUML shapes:

| Element kind | PlantUML shape |
|---|---|
| `c4.person` | `actor "Name" as Name` |
| `c4.software_system`, `c4.container`, `c4.component` | `rectangle "Name" as Name` |

These work with any vanilla PlantUML installation without requiring the
C4-PlantUML stdlib. The dedicated `archctl/src/diagram/project/structurizr.rs`
projector continues to handle the `--format structurizr` path (which uses the
proper Structurizr DSL with `model { ... }` blocks).

### Verification

- `archctl/src/diagram/project/plantuml.rs::tests::c4_container_view_emits_valid_plantuml`
  (NEW, M50): asserts `actor`, `rectangle`, and absent lowercase Structurizr
  keywords.
- `archctl/tests/c4_view_plantuml_e2e.rs` (NEW, M50): end-to-end test that
  verifies the projector output + PlantUML backend + SVG chain. SKIP-on-
  missing-backend.

## Out of scope (deferred)

- **`uml.pseudostate` and `uml.state_machine`** — the Mermaid state view
  currently handles only `uml.state`. PlantUML state view handles all three.
  Defer adding Mermaid support for the additional kinds until a real use
  case emerges.
- **C4 Dynamic and Deployment views** — currently no projection. Defer.
- **Transition labels in Mermaid state edges** — PlantUML state projection
  includes transition labels (e.g. `Idle --> Active : login`). Mermaid state
  projection emits only the `srcId --> tgtId` arrow without a label. This
  is a known limitation of the Mermaid projection; defer.

## Tests

### Unit tests (`archctl/src/diagram/project/mermaid.rs::tests`)

- `state_view_produces_valid_mermaid` — string assertions on
  `e1([Idle]):::state`, `e1 --> e2`, `classDef state`.
- `c4_container_view_produces_valid_mermaid` — string assertions on
  `e1(Customer)`, `e2([WebApp])`, `e1 --> e2`.

### Integration tests (NEW)

- `archctl/tests/state_view_e2e.rs`:
  - `state_view_renders_to_svg_with_names_visible` — Idle + Active + Suspended
    states rendered to SVG via merman.
  - `state_view_empty_bundle_renders_to_svg` — empty bundle still renders
    valid SVG.

- `archctl/tests/c4_view_e2e.rs`:
  - `c4_container_view_renders_to_svg_with_names_visible` — Customer +
    Orders + WebApp + Database rendered to SVG via merman.
  - `c4_container_view_empty_bundle_renders_to_svg` — empty bundle.

These are the regression locks that would have caught the pre-M41 bug class
in the same way M39's `usecase_view_e2e` caught the pre-M39 bug.

## Verification matrix

| Check | Status |
|---|---|
| `cargo test --quiet` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo fmt --check` | PASS |
| `bash scripts/verify-local.sh` | PASS |
| `archctl doctor --scopes diagram` | PASS |
| `archctl doctor --scopes render` | PASS |