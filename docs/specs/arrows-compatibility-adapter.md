# Spec: arrows-compatibility-adapter (H2)

> **Horizon:** H2 — Editor visual
> **Cycle origin:** `m69-arch-stack-product-roadmap-convergence` (stub)
> **Cycle realisation:** `m80b-arrows-export-adapter` (export-only, Path A-lite)
> **Status:** aligned — export realised in v1.41.0; import deferred to phase 2

## Purpose

An `.arrows` adapter for Arrows.app interchange. The cycle ships the
**export** path only (`archctl diagram export <selector> --format arrows`).
Import is acknowledged as a **phase 2** capability that will be initiated
when a real consumer needs it (e.g. round-trip from an Arrows-app edit into
the canonical graph), not speculatively.

## Public surface (as realised in v1.41.0)

- `archctl diagram export <selector> --format arrows [--output PATH] [--json]`
- `archctl diagram export <selector> --format viewer-bundle` (unchanged)
- `--format` is **case-insensitive** and accepts exactly `viewer-bundle` or
  `arrows`. Any other value exits non-zero with a clear message listing the
  accepted values.

## Capability contract

### Export (realised)

The serializer is a pure function over `BundleEnvelope { projection, styles }`:

| Bundle field | Arrows field | Notes |
|---|---|---|
| `Element.id` | `nodes[].id` and `nodes[].properties["archctl:element"]` | Canonical id, mirrored. |
| `ViewMember.label` (or `Element.name` fallback) | `nodes[].caption` | Caption falls back to `name` when no `ViewMember` row matches. |
| `ViewMember.x` / `ViewMember.y` (or `{0,0}` fallback) | `nodes[].position` | `Position::ZERO` when no `ViewMember` row matches. |
| `Element.kind_id` (e.g. `mt.container`) | `nodes[].properties["archctl:kind"]` | Plus PascalCase label derived from `kind_id` (e.g. `Container`). |
| `Edge.predicate` | `relationships[].type` | The actual field name in `export_types::Edge` (was `predicate_id` in earlier drafts). |
| `Edge.source` / `Edge.target` | `relationships[].fromId` / `relationships[].toId` | |
| `Edge.id` | `relationships[].properties["archctl:relation"]` | Pocket for round-trip. |
| `styles.element_colors.<kind>` | `nodes[].style["node-color"]` | Map covers `context`, `container`, `component`, `dynamic`, `deployment`. |
| `styles.edge_colors.default` | `relationships[].style["arrow-color"]` | Single colour applied to every relationship. |

The serializer is **read-only** with respect to the canonical graph: it
never touches lbug, never calls `apply`, never writes to a changeset.

### Unplaced cosmetic audit

When invoked with `--json`, the envelope documents the count of nodes
without a ViewMember row:

```json
{
  "format": "arrows",
  "document": { "nodes": [...], "relationships": [...], "style": {...} },
  "unplaced_count": 3
}
```

Detection uses `x == 0 && y == 0 && label_override.is_none()` as a proxy
in the absence of a `has_view_member` schema flag. A node genuinely placed
at `{0,0}` with no label override will be (mis)counted as unplaced —
acceptable false-positive for MVP, documented in the verify report.

### Default output path

When `--output` is omitted, the path is derived from the selector by
replacing `:` and `/` with `_` and appending `.arrows`:

| Selector | Derived path |
|---|---|
| `container:orders` | `./container_orders.arrows` |
| `c4:domain/orders` | `./c4_domain_orders.arrows` |
| `context:system` | `./context_system.arrows` |

`--output` always overrides the derivation.

## Phase 2 — Import (NOT in v1.41.0)

Import (`archctl import arrows <file>`) is **deliberately not implemented
in v1.41.0**. It will be initiated in a future cycle when at least one of
the following triggers fires:

1. A user reports a real round-trip need (edit in Arrows.app → merge back
   into the canonical graph via a changeset).
2. An agent tooling workflow requires self-referential diagram feedback
   (e.g. archview → Arrows → archview).
3. An ADR explicitly enables an external source of truth beyond the
   canonical graph.

Until then, the canonical graph remains the single source of truth
(ADR-038, invariant 1), and `.arrows` is a **projection** of the
canonical graph, not an input to it.

## Cross-references

- [ADR-007](adr/ADR-007-modelos-y-renderizadores-de-diagramas.md) — projections
- [ADR-038](adr/ADR-038-one-product-five-invariants.md) — invariant 1 (canonical graph)
- `sddk/m80b-arrows-export-adapter/spec.md` — cycle delta spec (synchronized)
- `sddk/m80b-arrows-export-adapter/verify-report.md` — verification evidence
- `docs/specs/diagram-projection-bundle.md` — bundle contract (schemaVersion 1.1)
- `docs/specs/cosmetic-changeset-roundtrip.md` — adjacent route for graph
  mutations via cosmetic changesets (closed by M81)

## Change log

- **2026-08-12 (M80b / v1.41.0)** — stub realigned to reflect realised
  export-only path. Public surface changed from `archctl export arrows` /
  `archctl import arrows` to `archctl diagram export --format arrows`.
  Import marked as phase 2 (deferred until real consumer need).
- **2026-08-10 (M69)** — stub created during roadmap convergence.
