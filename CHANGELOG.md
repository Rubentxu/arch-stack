# Changelog

All notable changes to `archctl` are documented here. The format is
loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [v0.9.2] — 2026-08-01

### Changed
- **`graph::create_db_session` no longer does its own `mkdir`.** Signature changed from `create_db_session(project_dir: &Path)` to `create_db_session(path: &Path)`. Both call sites already handle directory creation at their respective layers (`open_session` via the `Filesystem` port; `open_lbug_session` via `LbugStore::open`'s lockfile mkdir). The helper is now pure — `MemoryFilesystem` test isolation works correctly.
- **Apply pipeline narrowed to `&mut dyn DiagramOps`** (was `&mut dyn GraphStore`). `apply_to_store`, `reexport_view`, and `Command::apply` now take the narrowest sub-trait that covers every method they call. Realises the ISP benefit of the v0.9.1 trait split. `LbugStore` coerces to `&mut dyn DiagramOps` via the super-trait chain — no caller changes needed.
- **`update_view_member_label` no longer writes `updated_at`.** The column was set-but-unread (only `m.label` is hashed for `base_revision`). Removing the `chrono::Utc::now()` call closes the Clock port bypass. The column will be NULL for rows through this path; no semantic loss.

### Notes
- Patch bump from v0.9.1. No DDL change. Zero new dependencies. Zero trait method removals.
- 260 tests pass (baseline preserved; 0 regressions).
- Closes 4 carried WARN items from `refactor-m9-debt-cleanup` debt audit: CP-W1, CP-W2, CP-W3, OE-W1.

## [v0.9.1] — 2026-08-01

### Changed
- **GraphStore trait restructured into 3 domain sub-traits.** The 16-method `GraphStore` is now a super-trait of `EvidenceOps` (5 methods), `SourceOps` (4 methods), and `DiagramOps` (9 methods incl. new `update_view_member_label`). `GraphStore` itself keeps only the 4 cross-cutting methods (`open`, `init`, `stat`, `query`). ISP benefit unlocked — future seams can take narrower `&mut dyn DiagramOps` instead of the full `&mut dyn GraphStore`.
- **`apply::dispatch_command` and `apply_to_store` accept `&mut dyn GraphStore`** (not concrete `LbugStore`). Restores DIP — test mocks implementing the port can drive the apply pipeline. Concrete `LbugStore::open` still used in `apply_changeset` because the `fs2` flock is adapter-bound.
- **`Command::apply` method on the enum.** Per-variant apply logic travels with the data definition. Adding a new `Command` variant now touches 4 places (enum, const, schema, `Command::apply` match arm) instead of 6.
- **Atomic `update_view_member_label` path** replaces the read-modify-write pattern in `Command::SetLabel`. Single `MATCH ... SET ... RETURN` Cypher is atomic with respect to the row.

### Refactored
- `graph::create_db_session` extracted; `open_session` (public, Filesystem-port-aware) and `open_lbug_session` (private, std-fs) both delegate.
- `link_with_merge_fallback` helper extracted; 5 `link_*` methods (`link_extracted_from`, `link_evaluates`, `link_member_of`, `link_renders`, `link_group_contains`) deduplicated.

### Notes
- Patch bump from v0.9.0. No DDL change. Zero new dependencies. Zero new public surface besides the 3 sub-traits and `update_view_member_label`.
- 260 tests pass (vs 259 baseline; +1 new atomic round-trip test).
- Closes 6 carryover debt items from `m9-archctl-export-apply` PR2 release: W-DV-3, W-DV-4, W-DV-5, W-DV2-A1, W-DV2-A3, W-DV2-C2.
- `debt-verify` smoke (overeng + coupling clusters): PASS_WITH_WARNINGS, 0 CRIT, 0 HIGH, 4 non-blocking WARN follow-ups (see `sddk/refactor-m9-debt-cleanup/debt-report.md`).

## [v0.9.0] — 2026-08-01

### Added
- `archctl code sequence --from <selector>` — BFS projection over persisted call-graph edges into the `behavior.interaction` shape. Supports `--depth` (default 5), `--max-interactions` (default 500), `--json`, `--cwd`. `--apply` is accepted but no-op (sequence is read-only per spec SCN-217).

### Notes
- Requires PR1 (v0.8.1) call-graph data (run `archctl code call-graph --apply` first).
- Selector forms: `ByName("foo")`, `ByFileLine { file, line }`, `ByCanonicalKey("rust:src/lib.rs:foo:42")`.
- Cycle detection: marks `cyclic: true` when a callee is already in the visited set.

## [v0.8.1] — 2026-08-01

### Fixed
- `archctl code call-graph` extraction was non-functional in v0.8.0 due to TSG rule patterns incompatible with `basemind-tree-sitter-graph` 0.12. Now uses direct tree-sitter call-edge walk (verified against a smoke fixture: 3 functions + 2 call edges extracted correctly).
- 5 clippy warnings + 10+ fmt diffs in `call_graph.rs` resolved.

### Notes
- Patch bump from v0.8.0. No DDL change. Zero new deps.
- TSG rule files in `archctl/src/code/call_rules/*.tsg` remain as compiled design artifacts (Phase 2 / future use).

## [v0.8.0] — 2026-08-01

### Added
- `archctl code call-graph` — deterministic static call-graph extraction via tree-sitter-graph. MVP languages: Rust, TypeScript, Python. Persists with `--apply` as Element (`code.function`/`method`/`closure`) + SemanticRelation (`code.calls`) + Evidence (`derived` classification). New bounded-context surface `code/call_graph.rs` + `code/call_rules/{rust,typescript,python}.tsg` (compiled via `include_str!`).

### Notes
- No DDL change; new MetaType and Predicate rows are seeded at apply time (idempotent).
- Zero new dependencies; uses existing `tree-sitter` 0.26 + `basemind-tree-sitter-graph` 0.12 + `ast-grep-language` 0.45.
- Documented limitations: no dynamic dispatch, no cross-file resolution, no macro expansion, no async state machine (Phase 2 via LSP/SCIP).
- This is M11 PR1. PR2 (`archctl code sequence`) is v0.9.0.

## [unreleased] — v0.6.1 hygiene

### Added
- `AGENTS.md` (root): repository-level operating guidelines for AI agents
  and human contributors. Captures intent, core principles, scope,
  architecture rules, change strategy, build/test commands, validation
  matrix, code style, testing principles, dependencies, security,
  performance budget, compatibility/migrations, doc rules, git hygiene,
  definition of done, failure/recovery, instruction precedence, and
  open questions.

### Fixed
- `CHANGELOG.md` was missing entries for v0.2.0 through v0.6.0; this
  release backfills the gap (prior entries below).

## [0.6.0] — 2026-07-31 — `archctl diagram apply` (write-side)

### Added
- New CLI surface: `archctl diagram apply --changes <file>` (cosmetic
  view overrides — never touches `Element`/`SemanticRelation`/
  `ElementVersion`/`RelationVersion` nodes per ADR-013).
- Schema v3 migration `docs/schema/003_view_nodes.cypher` introducing
  4 NODE TABLEs (`Diagram`, `ViewMember`, `ViewEdge`, `ViewGroup`) and
  3 REL TABLEs (`MEMBER_OF`, `RENDERS`, `GROUP_CONTAINS`).
- Per-project DB lock via `fs2::try_lock_exclusive` on the `.lbdb`
  file (ADR-010 letter/states gap closed).
- 8 additive `GraphStore` port methods: `put_diagram`, `get_diagram`,
  `put_view_member`, `link_member_of`, `link_renders`,
  `put_view_group`, `link_group_contains`, `get_view_members`.
- ChangeSet format `schemas/changeset.schema.json` (JSON Schema 2020-12,
  `schemaVersion: "1.0"`) with 3 commands:
  - `move-member` (updates ViewMember x/y)
  - `collapse-group` (toggles ViewGroup.collapsed)
  - `set-label` (updates ViewMember label)
- Optimistic concurrency control via blake3 `baseRevision` content-hash.
- New dependency: `fs2 = "0.4"` (POSIX `flock` on Unix, `LockFileEx`
  on Windows).
- Extended `manifests/diagram.toml` with apply substrate entries
  (editable, must_hold, minimum_tests 25 → 56).

### Fixed
- `put_view_group.collapsed` was not persisted through the MERGE SET
  clause (latent bug from PR1 — masked because PR1 only created groups
  with `collapsed=false`). Surfaced by `dispatch_collapse_group_creates_group`
  in PR2 integration tests.
- Dropped 2 misplaced `manifests/diagram.toml` invariants (`try_lock_exclusive`
  belonged in `store.toml`, `use serde_json::Value` ban conflicted with
  pre-existing `validate.rs` usage).

### Security
- DB lock now prevents concurrent mutation of the same project's
  LadybugDB instance (single-writer enforced).

## [0.5.0] — 2026-07-31 — manifest coverage 11/23 → 22/23

### Added
- 11 new scope manifests: `astgrep.toml`, `cli.toml`, `doctor.toml`,
  `graph.toml`, `inventory.toml`, `project.toml`, `render.toml`,
  `row.toml`, `skills.toml`, `telemetry.toml`, `xdg.toml`. Each
  declares public symbols + `must_hold` invariants + `must_not_contain`
  bans. Coverage: 22/23 modules.
- `migrations.toml` deliberately excluded (bootstrap infrastructure,
  not a domain module).

### Notes
- No functional code changes; this release closes a coverage gap in
  the manifest gate (`archctl doctor --scopes <id>`).

## [0.4.1] — 2026-07-31 — v3.3 local-only policy hygiene

### Fixed
- `.ignore` companion file (gitignored itself) added that re-includes
  `sddk/` for opencode tools (grep, glob, read). The `.gitignore`
  documents the cross-reference inline.
- `docs/reports/*.html` added to `.gitignore` so the `sddk-release`
  phase no longer commits HTML closing reports to the remote.

### Notes
- Non-functional, infra-only. No code changes.

## [0.4.0] — 2026-07-31 — `archctl diagram export` (read-side)

### Added
- New CLI surface: `archctl diagram export <view-selector> --format
  viewer-bundle --output <dir>` and `archctl diagram validate
  <bundle-dir>`. Two new subcommands on the existing `Diagram` action.
- 5-file viewer bundle format: `manifest.json`, `projection.json`,
  `evidence.json`, `styles.json`, `assets/` (consumable by `archview`).
- View-selector grammar `<c4-kind>:<scope>` with 5 c4-kinds:
  `context`, `container`, `component`, `dynamic`, `deployment`.
- `baseRevision` = blake3 content-hash of the canonical JSON projection
  (OCC support; deterministic ordering before hashing).
- JSON Schema 2020-12 for bundle validation:
  `schemas/diagram-projection.schema.json`.
- C4 icon set in `archctl/src/diagram/icons/` (1×1 PNG placeholders).
- Bundle contract spec at `docs/specs/diagram-projection-bundle.md`.
- New dependency: `jsonschema` (already present via other paths; used
  here for bundle validation).
- `manifests/diagram.toml` registered with `must_hold` + `must_not_contain`
  + `minimum_tests`.

### Changed
- `archctl/src/diagram/` (9 new files, ~1605 LOC total): `export`,
  `validate`, `queries`, `export_types`, `selector`, `hash`, `assets`,
  `schema_embed`, `view_types`. Module wired into `lib.rs` with
  re-exports.
- `GraphStore::query` port gained a thin wrapper for diagram read
  queries (read-side only; no mutations).

### Fixed
- Icon list unified across `export` + `validate` (round-trip consistency).
- `Node.canonical_key` / `evidence_refs` fields renamed to camelCase
  in projection JSON.

### Notes
- ADR-007 split: `view.*` projection graph nodes deferred to M9-v2
  (now v0.6.0); this release ships **stateless projections** (Path 2).
  Apply (write-side) is deferred to v0.6.0.

## [0.3.1] — 2026-07-31 — extract `cell_to_json_map` helper

### Changed
- New private helper `cell_to_json_map(&Cell) -> serde_json::Map` in
  `archctl/src/store.rs:667-689` ("Internal helpers") replaces 3
  inline duplications in `accept_evidence`, `supersede_evidence`, and
  `list_evidence_by_status`. Net **-41 LOC**.

### Notes
- Mechanical refactor, no behavior change. `manifests/store.toml`
  unchanged, `must_hold` satisfied, `minimum_tests = 13` exceeded.
- 3 homologous inline patterns in test fixtures
  (`store.rs:1064, 1208, 1261`) deferred to a follow-up cycle
  (`refactor-extract-cell-to-json-map-v2`).

## [0.3.0] — 2026-07-30 — Evidence lifecycle (Drafted → Accepted)

### Added
- `EvidenceStatus` enum: `Drafted`, `Accepted`, `Superseded`.
- `status` field on `Evidence`, persisted in `props` (zero migration
  required).
- 3 new `GraphStore` port methods: `accept`, `supersede`,
  `list_by_status`.
- 2 new CLI subcommands: `archctl evidence accept <id>` and
  `archctl evidence supersede <id>`, plus a `--status` flag on the
  existing `archctl evidence list`.

### Docs
- ADR-016 §3.2 closed (lifecycle for evidence status).

## [0.2.2] — 2026-07-30 — fix parallel lbug test races

### Fixed
- Bound lbug buffer pool and DB size to 256 MB each in `archctl/src/graph.rs`
  (`BUFFER_POOL_SIZE = 256 * 1024 * 1024`). With 64 cores × 8 TB
  (lbug `SystemConfig::default()` returns `UINT64_MAX` for mmap size),
  the kernel could not satisfy the virtual memory requirement under
  parallel tests. 256 MB per DB × 64 cores = 16 GiB total, well within
  kernel limits.

### Changed
- Removed `--test-threads=1` workaround from `archctl/src/scope.rs` tests.
  Parallel test execution restored.

### Performance
- `archctl doctor --scopes <id>` per-scope runtime: ~10s (was ~2 min
  when forced serial).

## [0.2.1] — 2026-07-30 — manifest coverage 7/23 → 10/23

### Added
- 3 new scope manifests: `clock.toml`, `environment.toml`,
  `identity.toml`. Coverage: 10/23 modules.

## [0.2.0] — 2026-07-30 — SourceArtifact + Evaluation types

### Added
- `SourceArtifact` and `Evaluation` domain types in the graph.
- 2 new REL TABLEs: `EXTRACTED_FROM` (Evidence → SourceArtifact),
  `EVALUATES` (Evaluation → Element).
- Schema migration runner (`archctl/src/migrations.rs::MigrationRunner`)
  wiring both v1 and v2 init paths.
- Schema migration `docs/schema/002_source_evaluation.cypher` adds
  the new NODE/REL TABLEs.
- 4 new `GraphStore` port methods: `put_source`, `put_evaluation`,
  `link_extracted_from`, `link_evaluates`.
- `put_with_source` wrapper: combine evidence + source provenance
  in a single call.
- `source_origin` field in `Evidence.props` (no schema change; props
  is `serde_json::Value`).

### Docs
- ADR-017 (schema migration runner) + SourceArtifact identity section.

---

## [0.1.0] — first reset (commits `f5e7f83` / `b7b57a6`)

Removed inflated planning artifacts and rewrote CONTEXT, ROADMAP,
ADRs and CHANGELOG from `Skills-para-agentes-IA.md`. Replaced an
8-document roadmap with a flat user-facing list. Kept the existing
MVP: TypeScript CLI with ast-grep + ctags extractors, Structurizr /
PlantUML projections, XDG persistence, local podman renderers, three
fixtures with SPDX-labelled `gold.json`.
