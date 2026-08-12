# Spec: cosmetic-changeset-roundtrip (H2)

> **Horizon:** H2 — Editor visual
> **Cycle:** `m69-arch-stack-product-roadmap-convergence`
> **Status:** **promoted** — stub → full spec (M81)
>
> **M81 update (2026-08-12):** Closed by `m81-h2-contract-cosmetic-node-fields`.
> D1: `Command::MoveMember` now preserves `ViewMember.label` prior to upsert.
> D2: `Node` exposes `x`/`y`/`collapsed`/`labelOverride`; `build_bundle`
> LEFT JOINs `ViewMember` rows by `element_id`. `base_revision` changes after
> cosmetic edit (R3 ≠ R1). Schema bump 1.0 → 1.1. Seven scenarios verified
> PASS (7/7). Round-trip bundle-side is now closed.

## Purpose

A cosmetic ChangeSet (move-member / collapse-group / set-label) round-trips
through `archctl diagram apply` with `baseRevision` integrity validation.
Undo/redo works via inverse ChangeSets applied in sequence.

## Public surface

- `archctl diagram apply --changes <file>`
- `baseRevision` field in ChangeSet JSON
- Inverse ChangeSet application for undo

## Requirements

### R-COSMETIC — cosmetic changeset round-trip

A cosmetic ChangeSet (move-member / collapse-group / set-label) round-trips
through `archctl diagram apply` with `baseRevision` integrity. The round-trip
is closed end-to-end: apply persists cosmetic state to `ViewMember` rows;
export reads them back via LEFT JOIN and includes them in the bundle.

#### Scenario: MoveMember preserves prior label

- GIVEN a `ViewMember` already carrying `label == "X"`
- WHEN a `Command::MoveMember { x:240, y:160 }` is applied
- THEN `store.get_view_member(member_id).label == "X"` (unchanged)
  AND `m.x == 240`, `m.y == 160`

#### Scenario: Export emits cosmetic fields from ViewMember

- GIVEN a `ViewMember` with `x:240, y:160, collapsed:true, label:"X"`
- WHEN `build_bundle` runs over that diagram
- THEN the emitted `Node` carries `x == 240`, `y == 160`,
  `collapsed == true`, `labelOverride == Some("X")` AND the
  manifest `schemaVersion == "1.1"`

#### Scenario: Round-trip flips revision on cosmetic edit

- GIVEN an initial export producing `base_revision R1`
- WHEN the user applies `set-label("X")` then `move-member(x=240, y=160)` and re-exports
- THEN the new `base_revision R3` differs from `R1` (`R3 != R1`)

#### Scenario: Stale pre-m81 revisions rejected

- GIVEN a `Diagram` whose stored `revision` was computed before the field additions
- WHEN the user applies a new `ChangeSet`
- THEN `apply` MUST reject with `baseRevision mismatch`

#### Scenario: archview loader resolves labelOverride over name

- GIVEN a 1.1 bundle node `{ id, name:"Old", labelOverride:"New", x:1, y:2, collapsed:true }`
- WHEN `bundle/loader.ts` normalises the bundle
- THEN `RendererNode.label == "New"`, `x === 1`, `y === 2`, `collapsed === true`
- WHEN `labelOverride` is absent the loader MUST fall back to `name`

#### Scenario: Schema 1.1 accepts cosmetic fields

- GIVEN a bundle JSON with `manifest.schemaVersion == "1.1"` and
  one node carrying `x:240, y:160, collapsed:true, labelOverride:"X"`
- WHEN validation runs against `schemas/diagram-projection.schema.json`
- THEN validation succeeds and all four values round-trip

#### Scenario: Schema 1.1 still accepts 1.0 bundles

- GIVEN a bundle JSON with `manifest.schemaVersion == "1.0"` and
  a node carrying only `id, type, name` (no cosmetic fields)
- WHEN validation runs against the 1.1 schema
- THEN validation succeeds (defaults: `x==0, y==0, collapsed==false`,
  `labelOverride == None`) — backward compatible

## Schema version

The projection schema is at **v1.1** (`schemas/diagram-projection.schema.json`).
The schema bump from 1.0 to 1.1 is additive and backward compatible.
See `projection-schema-v1_1` capability in `sddk/m81-h2-contract-cosmetic-node-fields/spec.md`.

## Cross-references

- [ADR-013](adr/ADR-013-viewer-ortogonal.md) — ChangeSet contract
- [ADR-038](adr/ADR-038-one-product-five-invariants.md) — invariant 4 (cosmetic-only apply)
- [ADR-019](adr/ADR-019-performance-budget.md) — performance budget
- [`sddk/m81-h2-contract-cosmetic-node-fields/`](../../../sddk/m81-h2-contract-cosmetic-node-fields/) — M81 cycle that closed this spec
- [`executable-bundle-contract.md`](executable-bundle-contract.md) — bundle schema v1.1
