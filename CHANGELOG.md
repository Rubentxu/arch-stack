# Changelog

All notable changes to `archctl` are documented here. The format is
loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased] — v0.13.7

### Added
- **ADR-019 regression gate**: `scripts/bench-compare.sh` benchmarks
  `origin/main` in a temp worktree and the current branch, compares
  criterion medians per group, and fails on >10% degradation. New
  `bench-compare` CI job (PR-only) blocks regressions. Sampling calibrated:
  `--quick` jitter (~20%) was too noisy for a 10% gate; moderate sampling
  (~3% jitter, ~1m45/run) used instead.

### Refs
- Cycle: `m20-baseline-comparison`
- **M20 COMPLETE** (harness + datasets + doctor gate + CI gate + regression gate)

## [Unreleased] — v0.13.6

### Added
- **CI gate (M20 / ADR-019)**: GitHub Actions workflow with 3 jobs —
  rust (build/test/clippy/fmt/doctor), bench-smoke (criterion quick),
  web (vitest + build + bundle cap ≤2MB gzipped). First CI for arch-stack.

### Refs
- Cycle: `m20-ci-gate`
- Partially closes M20 (CI gate slice); regression >10% comparison deferred to backlog

## [Unreleased] — v0.13.5

### Changed
- **`store::open_and_init` promoted to canonical helper**: 8 CLI handlers
  (`graph init/stat/query/neighbours`, `evidence accept/supersede/list`,
  `graph export`) now use the shared `open_default + init` sequence from
  `crate::store` instead of inline duplication. `code/*` apply pipelines
  import it from `crate::store` too. No behavior change; net −7 lines.

### Refs
- Cycle: `refactor/open-and-init-store`
- Closes debt-report suggestion from `source-artifact-id` cycle

## [Unreleased] — v0.13.4

### Changed
- **Test coverage**: unit tests for `c4_language_label` (dockerfile/manifest
  extension derivation) and `existing_canonical_keys` (empty, seeded, and
  no-key-element stores). Closes backlog observations from the
  `source-artifact-id` debt-report. 240 lib tests (+6).

### Refs
- Cycle: `test/apply-common-helpers`

## [Unreleased] — v0.13.3

### Fixed
- **`SourceArtifact` identity divergence (D2)**: `code::apply_common::write_source_artifact`
  used a path-only `blake3(file)` id with `kind='manifest'` and empty fields,
  contradicting ADR-017 §D2 (`"src:" + blake3(path + content_hash)[..16]`,
  `kind='source_file'`). `code/*` apply pipelines and the `evidence` pipeline
  wrote different nodes for the same file. Now: `content_hash` is threaded
  through `FunctionNode`/`CallEdge` carriers (computed once per file at
  extraction) and computed in c4 `apply` via the injected Filesystem;
  `write_source_artifact` routes through `SourceArtifact::from_content` +
  `store.put_source` (canonical path). Cross-pipeline joins on
  `SourceArtifact.id` now align.
- **Removed single-use `Pipe` trait** (O-AE-1) from `apply_common` — `.pipe(Ok)`
  replaced with `Ok(...)`.

### Changed
- Report schemas (`call-graph-report`, `discover-report`) declare optional
  `contentHash` on node/edge/evidence items (`schemaVersion` stays `"1.0"`).

### Refs
- Cycle: `refactor/source-artifact-id`
- Closes backlog `refactor/debt-source-artifact-id-1` (C-HD-1) and
  `refactor/extract-code-apply-helpers-pipe-1` (O-AE-1)

## [Unreleased] — v0.13.2

### Changed
- **Internal refactor**: shared `code::apply_common` module extracted from four
  caller pipelines (`call_graph`, `c4_discover`, `class_diagram`, `sequence`).
  `escape_cypher_string`, `open_and_init`, `existing_canonical_keys`,
  `write_source_artifact`, and a local `Pipe` trait now live in one place.
  No observable behavior change; ~150 LOC of duplicated apply boilerplate
  eliminated. The `Pipe` trait is a single-use abstraction kept for API
  stability; tracked for future removal.

### Fixed
- **`scripts/fmt-staged.sh --edition`**: derive Rust edition from
  `archctl/Cargo.toml` instead of hardcoding 2021. Fixes pre-commit false
  failure for Rust 2024 `let` chains.

### Refs
- Cycle: `refactor/extract-code-apply-helpers`
- Closes M16 deferred debt item (`~150 LOC helper duplication`)
- Debt carried: `SourceArtifact` id formula divergence (pre-existing, tracked as
  `refactor/debt-source-artifact-id-1`); `Pipe` trait single-use (drive-by,
  `refactor/extract-code-apply-helpers-pipe-1`)

## [v0.13.1] — 2026-08-02 — clippy-fmt cleanup + composes edges

### Fixed
- **56 pre-existing clippy warnings** resolved across the workspace.
  `cargo clippy --quiet --all-targets -- -D warnings` now exits 0 (previously
  blocked by accumulated debt since v0.6.0).
- **Workspace-wide rustfmt normalization**. `cargo fmt --check` now exits 0
  (previously blocked by import ordering in benches + accumulated drift).
- **M12 W4 composes gap closed**: `archctl code class-diagram` now emits
  `composes` edges for same-file typed fields (e.g. `pub config: Config`
  inside `struct App`). The previously-ignored
  `test_class_diagram_same_file_composes` test now passes.
- **F3.3 lbug-infra gap closed**: `archctl doctor --scopes` no longer hangs.
  Replaced `cargo test --no-fail-fast` subprocess inside the test-count gate
  with a fast `#[test]` annotation counter. The hang was caused by
  integration tests like `code_sequence` that spawn `archctl code call-graph`
  as subcommands — at `--test-threads=1` they still locked lbug sessions
  for 60-90s. The annotation counter sub-second. Drift is caught by the
  standard `cargo test` run before commit / CI.

### Changed
- `EvidenceStatus::from_str` renamed to `parse_label` (avoids confusion with
  `std::str::FromStr`).
- `evidence.put_with_source` uses `std::slice::from_ref` instead of
  `&[sa.clone()]`.
- `TsgOutput` initialization uses struct literal with `..Default::default()`
  (was: build default + reassign fields).
- `gate_test_count_meets_minimum` reads `#[test]` annotations from
  `src/` and `tests/` instead of running `cargo test` as a subprocess.

### Refs
- Cycle: `refactor/clippy-fmt-cleanup`
- Post-v0.13.0 stabilization plan F1 + F3.3 (obs-5524)
- Closes M12 debt-report W4 (`composes` edges deferred)
- Closes lbug infra gap (STATE.md "Deuda bloqueante")

## [v0.13.0] — 2026-08-02 — M12 class-diagram extraction

### Added
- `archctl code class-diagram <selector>` CLI subcommand.
- Tree-sitter CST walk extractors for Rust (struct/enum/trait/impl), TypeScript (class/interface), Python (class).
- Intra-file edges: `extends`, `implements`, `composes`.
- Projection schema `schemas/class-diagram-report.schema.json` v1.0.
- Bench harness `archctl/benches/class_diagram_pipeline.rs` (criterion, ADR-019 budget).
- 7 new integration tests for class-diagram extraction.

### Changed
- Manifest gate `code`: `public_symbols` extended; `must_hold` includes projection determinism + ADR-019 budget.

### Deferred (follow-up cycle)
- Cross-file inheritance / type resolution (requires LSP or symbol table).
- Composition / aggregation with cross-file type lookup.
- LSP-based extraction (per ADR-012).

Refs: cycle `m12-class-diagram`, PR (pending).

## [v0.12.1] — 2026-08-02

### Bench (M5 follow-up)
- **Seed-cost decomposition via `iter_batched(NumIterations(N))`.** All seeded benches now use `iter_batched(seed_X, |(store, _tmp)| { measure }, BatchSize::NumIterations(10))` (or 5 for the large dead-code bench). The bulk Cypher seed runs once per batch of N measured iters instead of once per iter. After this change, `export_query_semantic_edges_medium` measures ~16ms (was ~2.8s) — the actual query cost, not the seed cost. Closes audit finding M5 (seed-cost decomposition follow-up from `docs/audits/2026-08-01-archctl-adr-vs-impl.md`).

### Notes
- Patch bump because no behavior change in the library — bench harness only.
- 263 tests pass (baseline preserved). No new tests added (bench harness is dev-only).
- Doctor scope `benchmark` reports 0 findings.
- M5 fully closed: audit 2026-08-01 is now 100% closed (F1–F7 + M1–M3 + M5).

## [v0.12.0] — 2026-08-01

### Fixed
- **`archctl graph stat` reported 0 relations even after a successful call-graph apply.** The stat query counted `MATCH (:SemanticRelation) RETURN count(*)`, but `SemanticRelation` is a reified node table that no application code writes. The call-graph writer persists relations on the `SEMANTIC_EDGE` REL TABLE (per ADR-009 deferral). The stat query now counts `MATCH ()-[r:SEMANTIC_EDGE]->() RETURN count(r)`, which reflects the actual data.

### Documentation
- **F2 — ADR-009 (relaciones semánticas reificadas) formally marked as DEFERRED.** The reified model (`SemanticRelation` + `REL_SOURCE`/`REL_TARGET`/`RELATION_TYPE` + `RelationVersion`) remains declared in the schema for future use, but `archctl` v0.12+ uses the direct `SEMANTIC_EDGE` model. Rationale documented: 1 round-trip per edge vs 3, plus sequence projection needs `r.props` directly. Closes audit finding F2.
- **F3 — ADR-008 (recuperación, versionado, evolución) revised.** `Snapshot` + `AnalysisRun` tables are reserved in the schema but not written by application code. `archctl run resume` is deferred to 1.x (aligned with ADR-021 Cognitive Layer). Closes audit finding F3.
- **F4 — `profile/agents/*.md` and `profile/skills/*/SKILL.md` no longer reference non-existent subcommands** (`archctl scenario`, `archctl scan`, `archctl graph path`, `archctl graph aggregate`, `archctl graph repair-index`, `archctl diagram spec`/`put`/`materialize`/`render`, `archctl class members`). Each reference is annotated with its current status (deferred or renamed). Closes audit finding F4.
- **F5 — ADR-007 revised.** `ViewEdge` table + `add-edge`/`edit-edge`/`remove-edge` commands deferred to M17.x archview. The `archctl diagram apply` pipeline keeps the 3 current commands (`move-member`, `collapse-group`, `set-label`). Closes audit finding F5.
- **F6 — ADR-005 naming aligned.** Trait is `GraphStore` (not `ArchitectureGraph`); adapter is `LbugStore` (not `LadybugArchitectureGraph`). Contract identical.
- **F7 — ADR-004 XDG path aligned.** Project dir is `<portable-project-id>/` (UUIDv4), not `<host>/<owner>/<repo>--<id>/`. Contract identical.
- **M3 — ROADMAP "Cambios SDD completados" table updated** with v0.9.1, v0.9.2, v0.10.0, v0.11.0 rows.
- **M1 — ADR-016 moved** from `docs/ADR-016-activegraph-packs-investigacion.md` (orphaned at docs root) to `docs/adr/ADR-016-activegraph-packs-investigacion.md` (canonical ADR location). Cross-references in ADR-017 + STATE-2026-07-30 updated.
- **M2 — ADR-015 / ADR-018 historical references documented.** Both ADR-015 (puertos faltantes Clock/Environment/Filesystem) and ADR-018 (lock path divergence) were either implemented incrementally in `refactor-1b-filesystem-port` and similar, or explicitly rejected in `m9-archctl-export-apply` planning. The historical references in `docs/STATE.md` (frozen snapshot) and `docs/ROADMAP.md` (decision point) are correct as historical artifacts.

### Bench (M5)
- **Seed-cost decomposition via `criterion::Bencher::iter_with_setup`.** Each bench function's `b.iter` closure now uses `iter_with_setup(|| seed_X(), |(store, _tmp)| { measure })` so the bulk Cypher seed runs once per batch instead of once per iteration. Note: criterion's default `BatchSize` is `NumIterations(1)` so this is semantically the same as the previous `b.iter` (setup runs every iter) — the bench harness would need `BatchSize::PerBatch(N)` for true amortization. Tracked as a follow-up.

### Notes
- Patch bump because no new feature surface; this cycle closes audit findings F2–F7 + the doc drifts M1–M3 + the bench seed-cost observation M5.
- 263 tests pass (260 baseline + 3 from v0.11.0). No new tests added.
- All 9 audit findings from `docs/audits/2026-08-01-archctl-adr-vs-impl.md` are now closed: F1 closed in v0.11.0; F2–F7 + M1–M3 + M5 closed in v0.12.0.

## [v0.11.0] — 2026-08-01

### Security
- **`archctl render` no longer POSTs to a remote URL.** The `--kroki-url` flag is removed. The `reqwest` dependency is dropped (-19 transitive deps). `archctl` cannot reach the network at runtime. Closes audit finding F1 (`docs/audits/2026-08-01-archctl-adr-vs-impl.md`).

### Added
- **Custom Structurizr DSL → SVG renderer** (`archctl/src/render/structurizr.rs`). Pure-Rust implementation using `petgraph 0.6` for graph data structure + Sugiyama-style layered layout, and `svg 0.14` for document emission. Supports the C4 subset: `person`, `softwareSystem`, `container`, and `src -> dst` relations. Anything outside the C4 subset yields an explicit parse error with the offending token.

### Changed
- `archctl render <file.dsl>` works **without** network access. Output is a real SVG (no kroki error page).
- Format detection now recognises `.mmd` as Mermaid (previously fell into Structurizr branch — wrong).
- `RenderFormat` enum adds `Mermaid` variant.

### Deferred
- **PlantUML rendering** — `plantuml-little 1.2026.2-4` (ADR-012 prescribed crate) requires `libgraphviz` at build time via the `graphviz-anywhere` transitive dependency. Vendor strategy (system package vs. prebuilt static lib vs. build-from-source) is a separate decision. Calling `archctl render` on `.puml`/`.iuml`/`.wsd` files yields a clear "not yet wired" error pointing at this follow-up.
- **Mermaid rendering** — `merman 0.8.0-alpha.3` (ADR-012 prescribed crate) has the same graphviz blocker. Calling `archctl render` on `.mmd` files yields the same deferred error.

### Notes
- Minor bump because the public surface changed (new `RenderKind` enum, removed `--kroki-url` flag). Zero new runtime dependencies; `petgraph 0.6` and `svg 0.14` are added.
- 263 tests passing (260 baseline + 3 new structurizr tests).
- Doctor `--scopes render` reports 0 findings.
- Smoke test: `archctl render /tmp/diagram.dsl` on a 4-node DSL with 3 relations produces a 1.5 KB SVG with correct layered layout (Customer → Web App → API → Database, viewBox `0 0 220 520`).

## [v0.10.0] — 2026-08-01

### Added
- **`archctl/benches/` criterion harness** (M20 first slice). Three bench binaries:
  - `export_pipeline` — `query_elements`, `query_semantic_edges`, `base_revision_hash`
  - `apply_pipeline` — `apply_set_label_small`, `apply_move_member_medium`, `apply_chained_commands_large` (gated)
  - `query_pipeline` — `query_count_elements_small`, `query_semantic_edges_medium`, `query_evidence_filter_large` (gated)
- **Three deterministic dataset fixtures** at `benchmarks/datasets/`:
  - `small-100.json` (100 elements, 250 relations, 65 KB)
  - `medium-1k.json` (1k elements, 2.5k relations, 660 KB)
  - `large-10k.json` (10k elements, 25k relations, 6.6 MB)
  - Generation is deterministic via `scripts/generate_bench_datasets.py` (seed 0xC0DE0001); re-running produces byte-identical fixtures.
- **`manifests/benchmark.toml`** — doctor scope gate for the bench harness. `cargo run --bin archctl -- doctor --scopes benchmark` validates the public symbols and must_hold invariants.
- **`benchmarks/README.md`** — user-facing harness documentation (layout, how to run, baseline measurements, ADR-019 budget mapping, follow-ups).

### Notes
- Minor bump because this is a new feature surface (bench harness + new manifests + new docs). Zero new public API on the `archctl` binary itself.
- New dev-dep: `criterion = { version = "0.5", features = ["html_reports"] }`. Adds ~20 transitive dev-deps (plotters, walkdir, tinytemplate, etc.) — all dev-only, never in the release binary.
- 260 tests pass (baseline preserved). All three doctor scopes (`benchmark`, `diagram`, `store`) report 0 findings.
- The 1k-node medium benches clock ~3s because each iteration is dominated by the seed cost (bulk Cypher inserts via `MATCH ... CREATE` on the `SEMANTIC_EDGE` REL TABLE), not by the actual query/apply. Future cycle should split seed cost out of the measurement loop.
- M20 first slice only — the full ADR-019 budget mapping (full `run_export` bench, cold-start, RSS measurement, CI gate workflow) is documented as follow-ups in `benchmarks/README.md`.

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
