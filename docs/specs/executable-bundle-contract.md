# Spec: executable-bundle-contract (H0)

> **Horizon:** H0 — Ejecutable / verdad verificable
> **Cycle:** `p-38e02210a9f14317/h0-executable-bundle-contract`
> **Status:** **promoted** — stub → full spec (R-DOC)

## Purpose

The `viewer-bundle` JSON is the cross-language executable contract between
`archctl` and `archview`. `schemas/diagram-projection.schema.json` is the
single source of truth; Rust DTOs (`export_types.rs`) and TypeScript types
(`archview/src/types.ts`) must be field-aligned. The bundle is produced by
`archctl diagram export` and served via `GET /api/export` with an optional
`?selector=` query parameter.

## Public surface

| Surface | Description |
|---|---|
| `archctl diagram export --format viewer-bundle` | CLI produces 5-file bundle |
| `GET /api/export` | HTTP endpoint, default selector `container:*` |
| `GET /api/export?selector=<view>` | HTTP endpoint, configurable selector |
| `GET /api/health` | Health check (unaffected by query strings) |
| `schemas/diagram-projection.schema.json` | JSON Schema, single source of truth |

## Capability declaration

The executable bundle contract requires:

1. **Configurable selector**: `GET /api/export?selector=<view>` routes the URL
   through a guard match arm that strips the query string and passes `selector`
   to `handle_api_export`. Invalid selectors return HTTP 400 with JSON error.
2. **Contract alignment**: Field names in the schema (camelCase:
   `viewSelector`, `canonicalKey`, `evidenceRefs`, `schemaVersion`) must match
   Rust DTO serialization and TypeScript loader ingestion exactly.
3. **Deterministic output**: A `FixedClock` with the same instant always produces
   the same `baseRevision` hash, enabling reproducible bundles.

## Requirements

### R-SELECTOR — configurable view selector

**Narrative**: As an `archctl view` caller, I can specify `?selector=<view>` to
request a specific C4 view so that the bundle contains only the elements I need.

| Scenario | Given | When | Then |
|---|---|---|---|
| S1 — valid selector | `project_dir` is set | `GET /api/export?selector=c4-context:myapp` | HTTP 200 + bundle filtered to `c4-context:myapp` |
| S2 — no selector, default | `project_dir` is set | `GET /api/export` | HTTP 200 + bundle filtered to `container:*` |
| S3 — invalid selector | `project_dir` is set | `GET /api/export?selector=bogus` | HTTP 400 + `{"error": "invalid view selector: <detail>"}` |
| S4 — query string health | — | `GET /api/health?x=1` | HTTP 200 + `{"status": "ok"}` (query string ignored) |

### R-ALIGN — field name alignment

**Narrative**: As a consumer of the bundle, I trust that field names are
consistent across schema, Rust DTO, and TypeScript loader so I can parse the
bundle without field-name surprises.

| Scenario | Given | When | Then |
|---|---|---|---|
| A1 — manifest camelCase | Valid bundle | Serialized JSON inspected | `viewSelector`, `schemaVersion`, `baseRevision`, `generatedAt` present |
| A2 — node camelCase | Valid C4 bundle | `projection.nodes[0]` inspected | `canonicalKey`, `evidenceRefs` present (not snake_case) |
| A3 — schema zero-violations | Any bundle | `jsonschema::validate(schema, bundle)` | No validation errors |
| A4 — TS loader no-throw | Schema-valid fixture | `normalizeBundle(fixture, src)` called | No exception; `nodes.length > 0`; `schemaVersion` populated; `rawKind` is known value |

## Non-goals

- This spec does NOT cover the `archctl diagram export --format` CLI output format
  (5 separate files on disk); only the JSON envelope served via HTTP.
- The spec does NOT guarantee bundle contents — only that the selector controls
  filtering. Element counts and structure are determined by the graph.
- Schema evolution (adding new optional fields) is out of scope for H0.

## Traceability matrix

| Requirement | Implementations | Tests |
|---|---|---|
| R-SELECTOR S1 | `view.rs:154` guard arm + selector extraction | `view.rs:426` `export_with_selector_splits_path` |
| R-SELECTOR S2 | `view.rs:78-82` default `container:*` | `view.rs:440` `export_without_selector_uses_default` |
| R-SELECTOR S3 | `view.rs:83` `selector::parse()` + 400 mapping | `view.rs:451` `export_invalid_selector_returns_400` |
| R-SELECTOR S4 | `view.rs:143` health guard arm | `view.rs:464` `health_unaffected_by_query_string` |
| R-ALIGN A1 | `export_types.rs:107-121` `#[serde(rename = "...")]` | `contract_alignment.rs:68` |
| R-ALIGN A2 | `export_types.rs` + schema | `contract_alignment.rs:83` |
| R-ALIGN A3 | `contract_alignment.rs:51` `jsonschema::validate` | `contract_alignment.rs:44` |
| R-ALIGN A4 | `loader.contract.test.ts` | `loader.contract.test.ts:30-57` |

## Cross-references

- [Index](index.md) — all specs
- [ADR-038](adr/ADR-038-one-product-five-invariants.md) — one product identity
- [ADR-019](adr/ADR-019-performance-budget.md) — TTFP budget
- [ADR-024](adr/ADR-024-c4-hierarchy-namespacing.md) — C4 kind/category separation
- `schemas/diagram-projection.schema.json` — canonical field definitions
- `archctl/src/diagram/export_types.rs` — Rust DTO field names
- `archview/src/types.ts` — TypeScript bundle types
