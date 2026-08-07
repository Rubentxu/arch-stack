# Spec — Sequence view edge labels (M45)

> **Change**: M45
> **Cycle**: CYC-2026-08-07-m45-sequence-edge-labels
> **Branch**: `feat/m45-sequence-edge-labels` @ `<tip>`
> **Status**: in_progress

This delta spec documents the optional message label support added to the
sequence view projection. Pre-M45 the projection emitted bare `A->>+B`
arrows with no text; M45 appends `: <label>` when `edge.props["label"]`
is a non-empty string.

---

## Why labels matter

Sequence diagrams without message labels are useless: the whole point is
showing WHAT messages participants send. Pre-M45 the diagrams rendered
correctly but showed only the participants, not the messages between them.

## Label convention

The label is read from `edge.props["label"]` (a `serde_json::Map<String, Value>`):

| `edge.props["label"]` | Behavior |
|---|---|
| `Some("placeOrder()")` | Emit `A->>+B: placeOrder()` |
| `None` (key absent) | Emit bare `A->>+B` (backward-compat) |
| `Some("")` (empty string) | Emit bare `A->>+B` (treated as absent) |
| `Some(non-string Value)` | `as_str()` returns None → bare `A->>+B` |

This is the canonical "label is optional" convention — existing edges without
labels keep rendering exactly as before, no migration needed.

## Mermaid syntax

```
sequenceDiagram
    participant Client
    participant Server
    Client->>+Server: placeOrder()    # labeled
    Client->>+Server                  # unlabeled (backward-compat)
```

## PlantUML syntax

```
@startuml
participant "Client"
participant "Server"
Client -> Server : placeOrder()      # labeled
Client -> Server                     # unlabeled (backward-compat)
@enduml
```

## Code surface

### `archctl/src/diagram/project/mermaid.rs::project_sequence_view`

Reads `edge.props.get("label").and_then(|v| v.as_str())`. Emits
`A->>+B: label` when present and non-empty; emits bare `A->>+B` otherwise.

### `archctl/src/diagram/project/plantuml.rs::project_sequence_view`

Same pattern, PlantUML syntax `A -> B : label`.

### Tests

Unit tests (mermaid.rs::tests):
- `sequence_view_with_edge_label` — asserts labeled arrow appears.
- `sequence_view_without_edge_label` — asserts bare arrow preserved.
- `sequence_view_with_empty_label_treated_as_unlabeled` — asserts empty
  string falls through to bare arrow.

Integration test (NEW): `archctl/tests/sequence_view_e2e.rs`
- `sequence_view_with_label_renders_to_svg` — labeled sequence renders to
  SVG with participants + label text.
- `sequence_view_single_participant_renders_to_svg` — minimal valid bundle.

## Out of scope (deferred)

- **Adding labels to other views** (use case, state, C4). Each has its own
  label conventions; defer per-view.
- **Typed `edge.label()` helper** to replace `props.get("label").and_then(|v| v.as_str())`.
  Defer; the inline JSON-string-key access is fine for now.
- **PlantUML sequence label e2e** (similar to M43 use case verification).
  Defer to M47 if needed.
- **Special character escaping in labels** (`:`, `;`, `[`, `]`). Empirical:
  most labels are method names like `placeOrder()` — no problematic chars.
  If escaping becomes necessary, add per-projection.

## Risks

- Some edges may have `props["label"]` set to non-string values (numbers,
  nested objects). The fix uses `.as_str()` which returns None for non-
  strings; no panic risk.
- Mermaid/PlantUML labels with special characters may need escaping.
  Empirical: most labels are method names without problematic chars.

## Verification matrix

| Check | Status |
|---|---|
| `cargo test --quiet` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo fmt --check` | PASS |
| `bash scripts/verify-local.sh` | PASS |
| `archctl doctor --scopes diagram` | PASS |
| `archctl doctor --scopes render` | PASS |
| Sequence e2e (labeled sequence → merman → SVG) | PASS |