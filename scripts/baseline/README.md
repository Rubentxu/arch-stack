# Frontier Freeze — Baseline 2026-08-13

> **Main:** c3ba8ce (v1.41.2) | **Date:** 2026-08-13
> **Purpose:** establish measurement baselines before the strangler refactor (Wave 1).
> **Principle:** "No mover código aún" — solo medir.

## 1. Scale Baseline

| Metric | Value |
|---|---|
| Total `.rs` files | 112 |
| Total source size | 1,289 KB |
| Files > 30 KB | 10 |
| Files > 50 KB | 4 |
| Largest file | `cli.rs` (96.8 KB) |

## 2. Erosion Hotspots (files > 30 KB)

These files have grown beyond the 30 KB threshold and are primary candidates for decomposition in Wave 1.

| File | Size | Primary concern |
|---|---|---|
| `cli.rs` | 96.8 KB | Parsing + dispatch + business logic mixed |
| `store.rs` | 94.7 KB | Cypher queries + domain types + store impl |
| `code/call_graph.rs` | 82.7 KB | TSG rules + projection + strategy dispatch |
| `code/class_diagram.rs` | 58.7 KB | Extraction + projection + rendering |
| `evidence.rs` | 46.4 KB | Extraction + validation + put logic |
| `code/state_machine.rs` | 46.3 KB | Extraction + projection |
| `scope.rs` | 38.3 KB | Manifest parsing + gate runner + `std::fs` |
| `code/c4_discover.rs` | 35.7 KB | Discovery strategies |
| `view.rs` | 35.5 KB | HTTP server + workspace state |
| `diagram/export.rs` | 34.4 KB | Query + projection + serialization |

## 3. Boundary Violations

### 3a. `std::fs` outside filesystem adapter (3 files)

These modules bypass the `Filesystem` port and use `std::fs` directly.

| File | Risk |
|---|---|
| `scope.rs` | Reads manifest files directly |
| `cognitive/event.rs` | File I/O without port |
| `cognitive/audit/log.rs` | Log writing without port |

**Target:** Route through `StdFilesystem` in Wave 1 (P1-01/P1-02).

### 3b. `Command::new` outside adapters (10 files)

Subprocess invocation scattered across modules instead of centralized in adapters.

| File | What it invokes |
|---|---|
| `doctor.rs` | Scope checks |
| `inventory.rs` | Tool detection |
| `skills.rs` | Skill execution |
| `lifecycle/migration.rs` | Version migration |
| `lifecycle/update.rs` | Self-update |
| `render/plantuml.rs` | PlantUML jar |
| `cognitive/mcp/tools.rs` | Tool execution |
| `view/editor.rs` | Editor launch |
| `code/strategies/cargo.rs` | `cargo metadata` |
| `test_helpers/plantuml.rs` | Test PlantUML |

**Target:** Wrap in adapter trait in Wave 1 (ADR-006 compliance).

### 3c. Cypher/query leaks (0 files)

No Cypher or SPARQL query strings found outside the store adapter and query template files. **This boundary is clean.**

### 3d. Network usage (0 files)

No `reqwest`, `http`, or `ureq` usage in source. ADR-011 (local-only) is respected.

## 4. Golden Outputs

17 golden CLI output files captured at `scripts/baseline/golden-outputs/`:

- `version.txt` — `archctl --version`
- `help.txt` — top-level help
- `help-{command}.txt` — per-subcommand help (15 files)

These serve as regression detectors: if a refactor changes CLI output unexpectedly, `diff` catches it.

## 5. Import Graph

182 cross-module `use` statements captured at `scripts/baseline/import-map.txt`. Key observations:

- `cli.rs` imports from nearly every module — expected for a dispatch entry point, but it also contains business logic (P1-02 target).
- `diagram/` imports from `store`, `graph`, `evidence` — correct dependency direction.
- `cognitive/` imports from `store`, `graph` — correct direction.
- No circular module dependencies detected at the file level.

## 6. Wave 1 Targets (derived from this baseline)

| ID | Work | Files affected | Metric improved |
|---|---|---|---|
| P1-01 | Composition root | `cli.rs`, `main.rs` | `cli.rs` size ↓ |
| P1-02 | CLI → handlers | `cli.rs` | Business logic out of CLI |
| P1-03 | Architecture repositories | `store.rs` | Cypher out of usecases |
| P1-06 | Extractor strategy | `code/call_graph.rs` | Size ↓, carriers common |
| P1-10 | `arch-model` boundary | `graph.rs`, `evidence.rs` | Pure domain model |

## 7. Regeneration

```bash
# Regenerate all baselines
scripts/baseline/regenerate.sh
```

Baselines are point-in-time snapshots. They should be regenerated after each Wave 1 PR to track progress.
