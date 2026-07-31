# Delta spec — Diagram projection bundle

> **Change**: `m9-archctl-export`
> **Cycle**: A-full (explore → propose → spec || design → tasks → apply → verify → debt-verify → archive → release)
> **Branch**: `feat/m9-archctl-export` @ `<tip>`
> **Status**: Completed and archived

> This file IS the main spec for the diagram-projection bundle contract.
> The full behavior spec lives in `sddk/m9-archctl-export/spec.md` (33 Given/When/Then scenarios).

---

## What the diagram-projection bundle is

A read-side contract between `archctl` and `archview` (per ADR-013 § "Contrato: DiagramProjection bundle"). `archctl diagram export` queries the LadybugDB graph and produces a deterministic, self-contained directory that `archview` consumes without touching the graph or any network.

## Bundle structure (5 entries)

```
<output-dir>/
├── manifest.json          # metadata: schemaVersion, diagramId, baseRevision, source, generatedAt
├── projection.json        # nodes + edges + groups (no layout — archview applies Sprotty/ELK layout)
├── evidence.json          # evidence rows linked from projection.json nodes/edges
├── styles.json            # theme + per-element-type colors
└── assets/                # embedded C4 icons (6 PNG, no external URLs per ADR-011)
```

## Path 2 (stateless projection) — locked design

The cycle shipped under Path 2 (decisión de scope en explore phase):

- **No schema migration.** `Element`/`SemanticRelation`/`Evidence` rows are read as-is.
- **No graph node type added.** `Diagram`/`view.*` nodes were prescribed by ADR-007 but never built — this cycle does NOT add them.
- **No lock.** Export/validate are read-only and pure; ADR-010 lockfile infra deferred to follow-up cycle.
- **`baseRevision` = content-hash blake3** on canonical (sorted-key, sorted-array) JSON of the projected slice. Deterministic → byte-identical re-exports (idempotency per spec SCN-002, SCN-060..062).

## View-selector grammar

```
<c4-kind>:<scope>
```

- `c4-kind ∈ {context, container, component, dynamic, deployment}`
- `scope` is an alphanumeric+hyphen identifier validated via `graph::validate_identifier` (anti-Cypher-injection)
- Examples: `container:orders`, `component:orders-api`, `context:system`

## JSON Schema

Lives at `schemas/diagram-projection.schema.json` (NEW directory). JSON Schema 2020-12. `$defs` for shared types. `additionalProperties: false` for strictness.

`archctl diagram validate <bundle-dir>` validates the bundle against this schema plus 4 internal consistency checks:
1. `manifest.json` + `projection.json` + `evidence.json` + `styles.json` + `assets/` all present.
2. Each `evidence.json` entry is referenced from a `projection.json` node/edge (no dangling).
3. Each `projection.json` reference to an evidence-id resolves in `evidence.json` (no orphans).
4. `baseRevision` is recomputable from the bundle contents (integrity check).

## Out of scope (explicitly deferred)

These items are NOT in this cycle. They belong to follow-up cycles.

- `archctl diagram apply` — full cycle deferred to `m9-archctl-export-apply`.
- ADR-010 lockfile infra — needed for apply but not for export/validate.
- Schema v3 migration with `view.diagram` graph nodes — only if apply's override model requires it.
- `archctl render` kroki POST — separate debt item (ADR-011 violation).
- C4 icon provenance — currently 1×1 PNG placeholders; real icons pending separate asset cycle.

## ADR divergences

- ADR-013 §"baseRevision" example shows `revision:42` (counter). This cycle uses content-hash (blake3). ADR text update owed. Spec and design documents the divergence explicitly.
- ADR-007 §"Vista persistida" prescribes `view.diagram/member/edge/group` graph nodes. They were never built. Path 2 sidesteps this; if apply cycle finds Element.props approach too constraining, the v3 migration becomes necessary.