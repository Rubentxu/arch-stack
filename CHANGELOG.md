## [Unreleased]

### Added
- **Secret redaction fase 3** (ADR-055) — detección por entropía:
  tokens alfanuméricos ≥ 32 chars con Shannon entropy ≥ 4.0 bits/char
  se redactan como `[REDACTED:high-entropy]` (API keys custom sin
  prefijo conocido). Hashes hex (blake3/sha256 ≈ 3.3 bits/char) e
  identificadores cortos sobreviven. Allowlist de campos documentada
  en `redact.rs` (strings escaneados vs numéricos/hashes/timestamps
  seguros por construcción).
- **Fusion params configurables** (Item 27 follow-up):
  - `architecture fuse --cutoff-days N` — cutoff de staleness configurable
    (evaluador staleness-weighted + `--expire-stale`; default 90).
  - `StalenessWeightedEvaluator` con `new(cutoff_days)` / `Default` (90).
  - `recompute_fused_for_versions` acepta evaluador (seam usa MaxMember
    por defecto).
- **Fuse-on-write** (Item 27 residual) — `recompute_fused_for_versions`
  persiste FusedClaims automáticamente tras cada write de evidencia
  (seams en `c4_discover::write_evidence` y `call_graph`): ya no hace
  falta `architecture fuse --persist` manual para mantener el layer de
  fusion sincronizado. Best-effort (ADR-049 D4), idempotente, con
  limpieza de claims superseded (los ids cambian al crecer el set de
  observaciones). `manifest.diagram.toml`/`architecture.toml`:
  `recompute_fused_for_versions` declarado.

## [1.60.0] — 2026-08-18

### Added
- **Fusion engine follow-ups** (Wave 3 Item 27) — persistence + evaluators + surfacing:
  - `ClaimEvaluator` trait with `MaxMemberEvaluator` (v1, byte-equal) and
    `StalenessWeightedEvaluator` (v2: 90-day cutoff, ×0.5 stale confidence);
    `FusedClaim.stale` flag; `fuse_observations_with`.
  - Schema migration `v6-fusion-persistence` creates `(:FusedClaim)` +
    `CONTRADICTS` + `FUSED_FROM` tables; `architecture fuse --persist`
    materializes claims idempotently; `--evaluator` selects strategy.
  - `architecture explain` surfaces intersecting fused claims (report 1.1);
    `architecture coverage` adds `byFusedClaims` buckets (report 1.1).
- **P2-09b persistent Observation + Claim tables** (Wave 3 Item 19) —
  schema migration `v4-p2-09b-create-obs-clm-tables` creates `(:Observation)`
  + `(:Claim)` tables (12 + 5 columns including `source_origin`,
  `written_via_backfill`, `evidence_ids`). `EvidenceOps::put_evidence`
  extended to return `PutEvidenceResult { evidence_rows, observation_rows,
  claim_rows }` and to dual-write Observation + compat Claim on every
  Evidence write (best-effort per ADR-049 D4). Backfill migration
  `v5-p2-09b-backfill-obs-clm-from-evidence` (PR-B wiring) populates
  the new tables from pre-upgrade Evidence rows via a `rust_hook` on
  the `Migration` struct (idempotent on re-run; bounded by
  `SAFETY_CAP = 100_000`).

### Changed
- `archctl/src/observation_claim.rs` mod-level docstring rewritten to
  reflect P2-09b: canonical tables are persistent; the read path
  prefers canonical with compat fallback for partial states.
- `docs/adr/ADR-049-evidence-observation-claim-confidence-model.md`
  Status header flipped from `Aceptado (parcial)` to
  `Aceptado — full scope closed`; body notes P2-09b cycle.

### Fixed

## [1.61.0] — 2026-08-18

### Added
- **Strict ArchBundle export** (Wave 3 Item 28, ADR-055 via ADR-061) —
  `archctl diagram export --profile strict` sanitizes bundles for sharing:
  - `ExportProfile` enum (`default` | `strict`); `manifest.strict: bool` +
    `manifest.checksum` (SHA-256 over bundle excluding `generatedAt`).
  - Evidence paths relativized to project root (no absolute filesystem
    paths leak); strict bundles open in **read-only mode** in archview
    (badge + disabled source preview / editor handoff).
  - Schema `diagram-projection.schema.json` v1.1: optional `strict` +
    `checksum` fields (backward compatible).

## [1.62.0] — 2026-08-18

### Added
- **Fusion engine residual** (Item 27 follow-up) — fused claims lifecycle:
  - `architecture fuse` **persists by default** (MERGE upsert idempotente);
    `--no-persist` preserva el modo stdout-only previo.
  - `architecture fuse --expire-stale [--dry-run]` — GC de FusedClaims stale
    (cutoff 90 días, single source con `StalenessWeightedEvaluator`).
  - Fix: `parse_observed_at` normaliza el formato de timestamps de
    LadybugDB (`"2026-08-15 0:00:00.0 +00:00:00"`, no RFC 3339) — sin este
    fix todo claim persistido se marcaba stale y el evaluador
    staleness-weighted era inútil con datos reales.

## [1.63.0] — 2026-08-18

### Added
- **Secret redaction en strict bundles** (ADR-055 fase 2) — scanner
  deny-by-default zero-dep (`archctl/src/diagram/redact.rs`) aplicado en
  `--profile strict`: AWS access keys (AKIA…), GitHub/Slack tokens,
  private keys (BEGIN…PRIVATE KEY), JWTs, URLs con credenciales y
  asignaciones genéricas (`api_key=`/`token=`/`secret=`/`password=`)
  se reemplazan por `[REDACTED:<kind>]`. El default profile no redacta
  nada (0 regression). Determinista (mismo input → mismo output).

## [1.64.0] — 2026-08-18

### Fixed
- **P2-09b backfill timestamp** — el backfill v5 saltaba silenciosamente
  filas pre-upgrade: lbug no hace implicit cast STRING→TIMESTAMP y el
  readback round-tripped (`"2026-08-15 0:00:00.0 +00:00:00"`) rompía
  `timestamp()`. Fix: normalizar con `parse_observed_at` (RFC 3339
  estricto) + wrap `timestamp()` en las columnas TIMESTAMP
  (`written_at`), literal en la columna STRING (`observed_at`).
  Test de regresión con filas pre-upgrade reales.

## [1.59.0] — 2026-08-18

### Added
- **P2-10 intent vs reality MVP** — `archctl/src/architecture/intent.rs` with
  `IntentDeclaration`, `DeclaredElement`, `DeclaredRelation`, `IntentDelta`,
  `IntentReport`, `IntentError`, and pure `check_intent(&dyn DiagramRepository,
  &IntentDeclaration, &str) -> IntentReport` use case. Four-class delta:
  `DeclaredAndPresent`, `DeclaredButMissing`, `ObservedUndeclared` (informational),
  `KindMismatch`. Deltas sorted by id ASC. No new Cargo dependencies.
  `archctl/src/architecture/intent_loader.rs` with `load_intent(&Path) ->
  Result<IntentDeclaration, IntentError>` (TOML via `toml = "0.8"`).
  `schemas/architecture-intent-report.schema.json` (schemaVersion `1.0`,
  capability `architecture-intent-mvp`).
  `archctl architecture intent check --intent <file> [--json] [--fail-on
  error|warning|info]` CLI command: validates `--fail-on` before graph read,
  loads TOML, calls `check_intent`, prints JSON or human per-class summary.
  Exit 1 when `DeclaredButMissing` or `KindMismatch` (drift) at `--fail-on
  = error`; `ObservedUndeclared` never triggers non-zero alone.
  `IntentAction::Check` subcommand and `ArchitectureAction::Intent` variant
  added to `cli.rs`. Unit tests S1–S7 in `intent.rs`; integration tests
  in `archctl/tests/architecture_intent.rs`. Self-dogfood:
  `archctl-intent.toml` at repo root declaring 17 bounded contexts.
  `manifests/architecture.toml` updated (intent.rs, intent_loader.rs in
  editable; check_intent, IntentDeclaration, IntentError, IntentReport,
  load_intent in public_symbols and must_hold). `manifests/cli.toml` updated
  (IntentAction in must_hold).

## [1.58.0] — 2026-08-17

### Added
- **P2-09a observation/claim carriers MVP** — `archctl/src/observation_claim.rs`
  with `Observation` and `Claim` carriers, `observation_from_evidence()`,
  `compat_claim_from_evidence()`, and
  `observations_and_claims_for_version(&dyn DiagramRepository, &str)` use case.
  `Observation` maps 1:1 from `EvidenceEntry` with `obs:<evidence_id>` id
  namespace. `Claim` carries `clm:compat:<evidence_id>`, `fused: false`,
  status mirrors `EvidenceEntry.status`, confidence defaults (1.0 accepted,
  0.0 others). Both types re-exported at `architecture::`. No new Cargo
  dependencies.
  `archctl architecture observe --version-id <VID> [--json]` CLI command:
  validates `version_id` via `graph::validate_identifier` (exit 1 on bad id),
  calls `observations_and_claims_for_version()`, prints JSON
  `{"observations": [...], "claims": [...]}` or human summary
  (`N observation(s), M claim(s)` + per-claim `id status confidence fused`).
  `ArchitectureAction::Observe` variant added to `cli.rs`.
  `manifests/architecture.toml` updated (observation_claim.rs in editable;
  Observation, Claim, observation_from_evidence, compat_claim_from_evidence,
  observations_and_claims_for_version in public_symbols and must_hold).
  `manifests/cli.toml` updated (ArchitectureAction::Observe in public_symbols
  and must_hold). Unit tests S1–S9 in observation_claim.rs; integration tests
  S8, S8b, empty version in `archctl/tests/observation_claim.rs`.

## [1.57.0] — 2026-08-17

### Added
- **P2-08 task context compiler MVP** — `archctl/src/architecture/task_context.rs`
  with `TaskContextReport` carrier (schema `task-context/1`, schemaVersion
  `1.0`, capability `architecture-task-context-mvp`), `BudgetInfo`,
  `ContextElement`, `ContextRelation`, `ContextError { EmptyTask, InvalidBudget,
  Store }`, and pure `compile_task_context(&dyn DiagramRepository, &str,
  budget_tokens, top) -> Result<TaskContextReport, ContextError>` use case.
  Delegates to P2-07 `relevance()` for ranking, enriches with evidence via
  `list_evidence_for_versions`, packs elements under budget (token estimate
  serialized_json_len / 4, ceiling division), relation closure (drops dangling
  endpoints). Sort: (score DESC, id ASC) for elements, (sourceId ASC,
  targetId ASC, predicateId ASC) for relations. Determinism: BTreeMap + sorted
  Vec — byte-equal JSON across runs. 9 unit tests in task_context.rs.
  `manifests/architecture.toml` updated (task_context.rs in editable;
  compile_task_context, TaskContextReport, ContextElement, ContextRelation,
  ContextError in public_symbols and must_hold).
- **P2-08** — `archctl architecture context --task <text> [--budget-tokens N]
  [--top N] [--json]` CLI command: opens project store, calls
  `compile_task_context()`, prints JSON (`TaskContextReport`) or human summary.
  Defaults: `--budget-tokens 4000`, `--top 10`. Exit 0 on success (incl.
  empty results), exit 1 on `ContextError`. `schemas/task-context.schema.json`
  shipped. No new Cargo dependencies.
- **P2-08** — `archctl/tests/architecture_task_context.rs` integration tests:
  S1 happy path with evidence, S2 budget truncation, S3 relation closure
  (dangling dropped), S4 empty/whitespace task → EmptyTask, S5 budget 0 →
  InvalidBudget, S6 empty graph → empty report exit 0, S7 JSON valid +
  human readable, S8 evidence per retained element, S9 schema validates
  report. No new Cargo dependencies.

## [1.56.0] — 2026-08-17

### Added
- **P2-07 context relevance engine MVP** — `archctl/src/architecture/relevance.rs`
  with `RelevanceReport` carrier (schema `relevance-report/1`, schemaVersion
  `1.0`, capability `architecture-relevance-mvp`), `RelevanceOptions { top,
  max_hops }`, `RelevanceError { EmptyQuery, Store }`, and pure
  `relevance(&dyn DiagramRepository, &str, &RelevanceOptions) -> RelevanceReport`
  use case. Scoring: exact-id → 1.0 × max(0.1, confidence); name/canonical_key
  substring (case-insensitive, ASCII-folded) → 0.8 × max(0.1, confidence);
  multi-token → proportional; BFS expansion 0.5^hop × confidence; relations
  0.5 × min(srcScore, tgtScore) when source/target in shortlist. Sort (score
  DESC, id ASC). ASCII-fold: ñ→n, á→a, é→e, í→i, ó→o, ú→u, ü→u. Stopword
  drop (EN+ES). `manifests/architecture.toml` updated (relevance.rs in
  editable; relevance symbols in public_symbols and must_hold).
- **P2-07** — `archctl architecture relevance --query <text> [--top N]
  [--max-hops N] [--json]` CLI command: opens project store, calls
  `relevance()`, prints JSON (`RelevanceReport`) or human shortlist.
  Exit 0 on success (incl. empty results), exit 1 on `RelevanceError`.
  `schemas/relevance-report.schema.json` shipped. No new Cargo dependencies.

## [1.55.0] — 2026-08-17

### Added
- **P2-06 fitness evaluator** — `archctl/src/architecture/report_formats.rs`
  with `to_sarif(&PolicyReport) -> SarifLog` projector (SARIF 2.1.0) and
  `to_junit_xml(&PolicyReport) -> String` projector (JUnit XML). Severity
  mapping: `Error → "error"`, `Warning → "warning"`, `Info → "note"`.
  SARIF output: `archctl://graph/<subject.id>` URI per violation. JUnit
  output: `<testsuites>` root with `tests/failures/skipped` attributes,
  `<testcase classname="<rule>" name="<subject.id>">` per violation,
  `<failure>` for error/warning, `<skipped/>` for info. Hand-rolled XML
  (no external crate). 7 unit tests in `report_formats.rs` and 7
  integration tests in `archctl/tests/architecture_policy_report_formats.rs`.
- **P2-06** — `archctl architecture policy check --format {json,sarif,junit}`
  CLI flag (`PolicyReportFormat` enum: `Json`, `Sarif`, `Junit`). Existing
  `--json` flag preserved as deprecated alias for `--format json`. Output
  via projectors in `architecture_policy_check_cmd`. `manifests/architecture.toml`
  updated (report_formats.rs in editable; `to_sarif`/`to_junit_xml` in
  public_symbols and must_hold).

## [1.54.0] — 2026-08-17

### Added
- **P2-05 architecture-policy MVP** — `archctl/src/architecture/policy.rs` with
  the ADR-054 closed rule set (6 rules): `forbid_dependency`,
  `require_dependency`, `forbid_cycle` (iterative Tarjan SCC), `max_fanout`,
  `evidence_required`, `confidence_min`. Carriers: `PolicyRule` (serde-tagged
  JSON), `Waiver` (expiry recomputed at evaluation time), `Violation`,
  `PolicySummary`, `PolicyReport` (schema `architecturePolicyReport/1`).
  Selector glob: `*` suffix match with separator boundary (prefix ending in
  `:`/`.` matches any remainder; bare prefix requires a separator next).
  Waivers suppress matching violations while active; expired waivers stay in
  `waivers[]` with `expired: true` and do NOT suppress.
- **P2-05** — `archctl architecture policy check [--policy <file>] [--json]
  [--fail-on error|warning|info]` CLI command: reads the policy document
  (`{"rules": [...], "waivers": [...]}`, file or stdin) BEFORE any graph
  read, validates `--fail-on`, evaluates over the live graph, prints human
  summary or JSON `architecturePolicyReport/1`. Exit 1 when any remaining
  violation severity >= the `--fail-on` threshold (default `error`).
- **P2-05** — 47 unit tests in `archctl/src/architecture/policy.rs` and 6
  integration tests in `archctl/tests/architecture_policy.rs` (S1–S5 +
  confidence_min). `manifests/architecture.toml` updated (policy.rs in
  editable; policy symbols in public_symbols and must_hold).

## [1.53.0] — 2026-08-17

### Added
- **P2-04 architecture-coverage MVP** — `archctl/src/architecture/coverage.rs` with
  `CoverageReport` carrier (schema `coverageReport/1`), `CoverageError
  { Store }`, and pure `coverage(&dyn DiagramRepository, &dyn Clock)
  -> Result<CoverageReport>` use case. Four bucket axes: `byConfidence`
  (high≥0.9/medium≥0.7/low≥0.5/unknown<0.5), `byEvidenceStatus`
  (accepted/drafted/superseded), `byConflict` (always 0 with warning),
  `byStaleness` (fresh≤90d/stale>90d). Live-graph scan over Element,
  SemanticRelation, and Evidence tables. No store writes.
- **P2-04** — `archctl architecture coverage [--json]` CLI command:
  scans live graph, computes coverage metrics, prints human summary or
  JSON `coverageReport/1`. `ArchitectureAction::Coverage` variant added.
- **P2-04** — `EvidenceEntry.status: Option<String>` added to support
  coverage metrics; `EvidenceBundle` export filters to accepted evidence
  only per ADR-005. `manifests/architecture.toml` updated (new `coverage.rs`
  in editable, `coverage`/`CoverageReport`/`CoverageError` in public_symbols
  and must_hold).
- **P2-04** — 6 unit tests in `archctl/src/architecture/coverage.rs`
  (empty graph, mixed confidence, all high/accepted, drafted evidence,
  schema/capability invariants, staleness warning) and 5 integration tests
  in `archctl/tests/architecture_coverage.rs`.

## [1.52.0] — 2026-08-17

### Added
- **P2-03 architecture-explain MVP** — `archctl/src/architecture/explain.rs` with
  `ExplainReport` carrier (schema `explain-report/1`), `ExplainError
  { SubjectNotFound, RelationNotFound, Store }`, and pure
  `explain(&dyn DiagramRepository, &str) -> ExplainReport` use case.
  Routes by id prefix: `rel:*` → relation path, all others → element path.
  Element path reads `Element` + `ElementVersion` + `SUPPORTED_BY→Evidence`.
  Relation path reads `SemanticRelation` + `RelationVersion` +
  `SUPPORTED_BY→Evidence`. Honesty principle: unsubstantiated subjects
  (no evidence) return `unsubstantiated: true` with a warning, never silent
  omission.
- **P2-03** — `archctl architecture explain <id> [--json]` CLI command:
  validates id via `graph::validate_identifier` (exit 1 on bad id), calls
  `architecture::explain()`, prints human summary or JSON `explain-report/1`.
  `ArchitectureAction::Explain` variant added to `cli.rs`.
- **P2-03** — `From<ExplainError> for SnapshotError` conversion in
  `archctl/src/architecture/errors.rs`; `manifests/architecture.toml` updated
  (new `explain.rs` in editable, `explain`/`ExplainReport`/`ExplainError`/
  `ExplainSubject`/`ExplainProvenance` in public_symbols and must_hold).
- **P2-03** — 11 unit tests in `archctl/src/architecture/explain.rs`
  (happy path, unsubstantiated, error cases, schema/capability, routing)
  and 9 integration tests in `archctl/tests/architecture_explain.rs`
  (element/relation happy path, unsubstantiated honesty, unknown ids, routing,
  null version). Store primitives for relation read: `RelationRow` struct,
  `DiagramRepository::read_relation_by_id` and
  `DiagramRepository::list_evidence_for_relation_versions` added to store.rs
  trait and `LbugStore` implementation.

## [1.51.0] — 2026-08-17

### Added
- **P2-02 architecture-diff MVP** — `archctl/src/architecture/diff.rs` with
  `ArchitectureDiffReport` carrier (schema `architecture-diff-report/1`),
  `DiffError { InvalidIdentifier, SnapshotNotFound }`, and pure
  `diff_snapshots(&Snapshot, &Snapshot) -> ArchitectureDiffReport` use case.
  Compares 7 field groups (`commitHash`, `schemaVersion`, `extractorDigest`,
  `repoIdentity`, `label`, `pinned`, `createdAt`); emits `compatibility.schema
  = "same" | "different"` with reason. No store writes.
- **P2-02** — `archctl architecture diff <id_a> <id_b> [--json]` CLI command:
  validates both ids via `graph::validate_identifier` (exit 1 on bad id),
  loads both snapshots via `SnapshotRepository::get_snapshot`, calls
  `diff_snapshots`, prints human table or JSON `architecture-diff-report/1`.
  `ArchitectureAction::Diff` variant added to `cli.rs`.
- **P2-02** — `From<DiffError> for SnapshotError` conversion in
  `archctl/src/architecture/errors.rs`; `manifests/architecture.toml` updated
  (new `diff.rs` in editable, `diff_snapshots`/`ArchitectureDiffReport`/
  `DiffError` in public_symbols and must_hold).
- **P2-02** — 4 unit tests in `archctl/src/architecture/diff.rs` (identical
  snapshots, different commit_hash, different schema_version, different
  extractor_digest) and 4 integration tests in `archctl/tests/architecture_diff.rs`
  (`--json` round-trip, identical diff, invalid id rejection, snapshot-not-found).

## [1.50.0] — 2026-08-17

### Added
- **p2-01 follow-up** — closes 7 WARNINGs from `p2-01` verify-report (commit
  `8e6c434`, PR #196; squash-merged `feat/p2-02-followup`). Cycle
  `p-38e02210a9f14317/p2-02-followup`. 12 files touched (471 insertions,
  56 deletions); 666/666 tests + 2 new regression tests; verify
  PASS_WITH_WARNINGS; debt-verify PASS_WITH_WARNINGS. Highlights:
  `manifests/cli.toml` `ArchitectureAction` registered (closes WARNING #3);
  `archctl architecture` capability entry regenerated `docs/CAPABILITIES.md`
  (closes WARNING #2); new spec
  `docs/specs/architecture-cli-snapshot-surface-deviation.md`; new fixture
  `archctl/tests/fixtures/capability_markdown_golden.txt`; CLI surface
  extended (`archctl/src/cli.rs` +28 lines, `archctl/src/capability/source_cli.rs`
  +8 lines); cross-feature sequence id uniqueness regression test
  (closes WARNING #4 + WARNING #7); renderer/IDE bump stability regression
  test (closes WARNING #6); `archctl/src/identity.rs` `find_deepest_commit`
  semantics tightened (closes WARNING #8); `--dry-run` default-true +
  `--keep-last` clamp 1..=1000 (closes WARNING #10); spec
  `docs/specs/architecture-snapshots.md` clarified.


## [1.49.0] — 2026-08-17

### Added
- **P2-01 snapshot metadata MVP** — new bounded context `archctl/src/architecture/`
  with `Snapshot` carrier, `create()`, `list()`, `gc()` and `SnapshotError`. New
  port method surface on `GraphStore` via `SnapshotRepository`
  (`create_snapshot`, `get_snapshot`, `list_snapshots`, `label_snapshot`,
  `pin_snapshot`, `update_snapshot_props`, `delete_snapshots`); LbugStore
  implementation over the `Snapshot` table; `MockGraphStore` updated.
- **P2-01** — `RepositoryIdentity` carrier in `archctl/src/identity.rs` with
  `resolve_repository_identity()` resolver; stable
  `blake3("repo|{normalized_remote}|{first_commit}")` formula
  (`find_deepest_commit` fallback to HEAD currently documented in verify-report
  WARNING #8; follow-up cycle `p2-02` planned).
- **P2-01** — `extractor_set_digest()` in `archctl/src/architecture/digest.rs`:
  renderer/IDE-stable digest over sorted
  `(lang_id, lang_ver, view_strategy_id, project_strategy_id, evidence_extractor_id)`
  from `source_code` + `source_cargo` only. `blake3:` prefix, deterministic,
  `code.sequence` excluded. 4 unit tests green.
- **P2-01** — CLI `archctl architecture {create,list,gc}` (`--json --label
  --keep-last --dry-run --yes`); end-to-end CLI smoke (`archctl architecture
  create --cwd <git-repo>` → `Created snapshot blake3:91480afa... (sequence 1)`).
- **P2-01** — manifest gate NEW `manifests/architecture.toml` (must_hold for
  `pub fn create/list/gc` + `UnitOfWork::begin_transaction`); MOD
  `manifests/store.toml` (SnapshotRepository symbols). Follow-up cycle to add
  `Command::Architecture` / `ArchitectureAction` to `manifests/cli.toml` (verify
  WARNING #3) and `cli.architecture` to capability registry (verify WARNING #2).
- **P2-01** — GC retention: `(pinned) ∪ (last N by created_at)` with
  `--keep-last N` (default 10), `--dry-run` (opt-in), `--yes` confirms.
  4 GC integration tests green.
- **P2-01** — `SnapshotError::NotGitRepository` on non-Git cwd; no row written;
  1 integration test asserts both error and side-effect absence.
- **P2-01** — schema version metadata: `props.schema_version` (full semver) +
  `props.schema_compatibility` ("1.0"); `INT64 schema_version` column = major.
  1 integration test asserts both.

### Changed
- **P2-01** — `archctl/src/store.rs` gains `SnapshotRepository` trait at
  `store.rs:438` and `LbugStore` impl at `store.rs:1680+`; `GraphStore`
  supertrait extended with `+ SnapshotRepository` at `store.rs:207`.
  `archctl/src/diagram/export.rs:656` updated for `MockGraphStore`.
- **P2-01** — `archctl/src/lib.rs`: `pub mod architecture;` added.
- **P2-01** — `archctl/src/capability/mod.rs`: minor wiring for new bounded
  context (no capability registry entry added; follow-up cycle planned).

### Out-of-scope (next cycle)
- `ElementVersion`/`RelationVersion` ↔ `Snapshot` materialization (P2-03).
- Diff between snapshots (P2-03).
- Non-Git worktree overlay (P3-01).
- `--ref <git-rev>` flag on `create` (proposal §Design; verify WARNING #9).
- `--dry-run` default-true and `--keep-last N` clamp 1..=1000 (verify WARNING #10).
- `find_deepest_commit` actual implementation (verify WARNING #8).
- True concurrent-create test (verify WARNING #7); current test is sequential.
- Renderer/IDE bump stability regression test (verify WARNING #6).
- `use crate::store::LbugStore` removal from `architecture/snapshot.rs` (verify
  WARNING #4; depends on `UnitOfWork` trait-generic refactor).
- `manifests/cli.toml` `ArchitectureAction` registration (verify WARNING #3).
- `cli.architecture` capability registry entry + `docs/CAPABILITIES.md`
  regeneration (verify WARNING #2; #5 optional).
- T5.2 spec docs (`docs/specs/{architecture-snapshot,snapshot-repository,
  architecture-cli}.md`) — deferred per launch plan; will land as a follow-up
  cycle.

See `docs/ROADMAP.md` cycle log row
`p-38e02210a9f14317/p2-01-snapshot-mvp` for the full cycle audit trail.

## [1.48.1] — 2026-08-16

### Changed
- **docs-state-refresh-v148** — `docs/STATE.md` full refresh to v1.48.0 reality (Wave 0 7/7 DONE — was "6/7 falta item 7"; Wave 1 items 8–16 ALL DONE); corrected version-table rows for v1.41.6 (CI fast gate), v1.42.0 (ladybug doctor — was mislabeled "Wave 0 item 7"), v1.43.0 (batch: p0-03 native runners + p1-09 + p1-01 + p1-03), v1.45.0 (UnitOfWork, dropped bogus "item 15"), v1.47.0 (M32 PR2 UNWIND extension — was mislabeled remediation), v1.47.1 (remediation r1); real counts (118 tags, ~41.1K LOC src, ~11.6K tests, ~900 benches); fixed wrong `/var/home/...` path; "Próxima acción" → Wave 2 (items 17–18).
- **docs-state-refresh-v148** — `docs/ROADMAP.md` M32 row: corrected the `v1.46.0` tag claim (PR1 merged as PR #187 but tag never created; documented gap).
- **docs-state-refresh-v148** — `docs/specs/capability-registry.md` W1 wording fix (13 → 8 categories, 79 entries).

### Fixed
- **docs-state-refresh-v148** — `manifests/scope.toml`: `archctl/src/doctor.rs` → `archctl/src/doctor/` (file became directory in 2026-07-30; the editable_files_exist gate was failing silently in the full-suite run — STATE.md "30/30 scopes" was actually 28/30).
- **docs-state-refresh-v148** — `manifests/distribution.toml` + `Formula/archctl.rb`: restored the cross-link between the Homebrew formula and `docs/maintainers/HOMEBREW_FORMULA.md` (dropped when the doc moved in 2026-08-11; must_hold invariant was silently failing since then). Doctor full suite now genuinely 30/30 OK.
- **docs-state-refresh-v148** — `archctl/src/cli.rs`: removed dead `_ctx: &CliContext` parameter from `capabilities_cmd` (debt-verify OE-2); relocated the orphaned `row_to_json` doc comment stranded above `capabilities_cmd` during the p1-08 insertion and gave `capabilities_cmd` its own doc comment.

## [1.48.0] — 2026-08-16

### Added
- **P1-08 Wave 1 (items 15+16)** — `archctl capabilities` CLI (`--format json|markdown`, `--check`): typed capability registry as single source of truth (ADR-045, promoted accepted) with 79 entries across 8 categories (extractors per language, strategies, renderers, views, doctor scopes, IDE adapters, MCP tools, CLI, plugins); bidirectional alignment tests (incl. IDE adapter id derivation); generated `docs/CAPABILITIES.md` + staleness gates in `verify-local.sh`/`test-ci-gates.sh`; `schemas/capability-registry.schema.json` (schemaVersion "1"); `docs/specs/capability-registry.md` (8 requirements, 17 G/W/T scenarios).

### Fixed
- **P1-08** — `schemas/call-graph-report.schema.json` stale language enum 3 → 6 (rust, typescript, python, go, java, kotlin): Go/Java/Kotlin reports previously failed schema validation; `schemaVersion` field correctly camelCased per spec; stale language matrices removed from README/MANUAL (now pointer to `docs/CAPABILITIES.md`); `SUPPORTED_LANGUAGES` stale const removed from `cli.rs`.
- **P1-08** — Registry IDE ids derive from `ide::builtin_adapters()` ids (`ide.claude-code`, was drifted `ide.claude_code`) — debt-verify CP-5.
- Pre-existing clippy `--all-targets` errors fixed (unused imports in `tests/code_state_machine.rs` from the M32 refactor; doc-lint in `benches/call_graph_apply.rs`) — surfaced by this cycle's verify-local gate.

## [1.47.1] — 2026-08-16

### Fixed
- **M32 remediation r1** — `class_diagram::apply` version-id mismatch (data
  corruption): `Element.current_version_id` and `ElementVersion.id` were two
  independent `uuid::Uuid::new_v4()` calls that never matched, breaking the
  CURRENT_VERSION integrity invariant (`e.current_version_id = v.id` returned
  0 rows). Both now share one deterministic `blake3` version id, mirroring
  `call_graph`/`state_machine`/`c4_discover`. Found by M32 debt-verify (FAIL),
  fixed forward on `fix/m32-debt-remediation-r1`.
- **M32 remediation r1** — Port boundary restored (ADR-059):
  `batch_upsert_element`/`batch_upsert_element_version` moved from free
  functions taking `&mut LbugStore` (with `session_mut_inner()` raw Cypher)
  into the `ElementRepository` port trait; Cypher generation now lives in the
  adapter. New `batch_link_of_type` port method batches the remaining
  per-element OF_TYPE loops in `call_graph`.
- **M32 remediation r1** — UNWIND row escaping: all interpolated string fields
  now go through `escape_cypher_string` (previously only 3 of 9 fields were
  escaped in `batch_upsert_element`, 4 of 11 in the version helper).
- **M32 remediation r1** — `BATCH_SIZE=500` is now real: batch helpers chunk
  via `batch.chunks(BATCH_SIZE)` (constant was declared but unused).

### Changed
- **M32 remediation r1** — `state_machine::apply` single-walk restructure:
  the collect phase now emits `TransitionEdge` tuples so the edge-linking
  phase no longer re-walks the entire `report.machines` (~270 LOC, was 3
  phases with a redundant third re-walk).
- **M32 remediation r1** — Silent `let _ =` on edge-link `Result`s replaced
  with explicit `.ok()` best-effort semantics in production writers.

### Added
- **M32 remediation r1** — Cross-writer CURRENT_VERSION integrity regression
  suite (`archctl/tests/code_writer_current_version.rs`): applies fixtures
  through all four writers and asserts `element.current_version_id =
  version.id` for every CURRENT_VERSION edge. This is the test that would
  have caught the original UUID mismatch.

## [1.47.0] — 2026-08-16

### Changed
- **M32 PR2** — Extend UNWIND bulk import (ADR-036 §D2) to `state_machine` and
  `c4_discover` apply writers. `state_machine::apply` now batch-inserts machines,
  states, and transition nodes in 3 UNWIND passes; `c4_discover::apply` batch-inserts
  containers and their ElementVersion nodes in 2 UNWIND passes.
- **M32 PR2** — ADR-036 amendment: D2 re-ship documented (was regressed by P1-04 T3
  commit `599c863`); D3 (prepared + param binding) stays deferred per M51 decision;
  class_diagram N+1 fix on `existing_canonical_keys` hoisted out as D2 batching
  prerequisite.

## [1.46.0] — 2026-08-16

### Changed
- **M32 PR1** — Re-introduce UNWIND bulk import on `call_graph` and `class_diagram`
  apply writers (ADR-036 §D2); `BATCH_SIZE=500` constant lives in `apply_common`.
  D2 was regressed by P1-04 T3 commit `599c863`. Sequence writer N/A per M53
  audit (SCN-217 read-only).
- **M32 PR1** — `class_diagram::apply` N+1 query bug fixed: `existing_canonical_keys`
  was called inside the per-node loop; hoisted out as a prerequisite for D2 batching.
  Also fixes version_id: each node now gets its own unique `ElementVersion` id
  (previously all nodes shared ONE version_id).
- **M32 PR1** — Manifest drift fixed: `archctl/src/code/state_machine.rs` added to
  `editable` array in `manifests/code.toml`.

### Fixed
- **M32 PR1** — `class_diagram::apply` N+1 query regression (ADR-036 §D2 trade-off:
  pre-D2 batching, the hoist is a prerequisite for any batching to work correctly).

## [1.45.0] — 2026-08-15

### Added
- **P1-05** — `pub trait UnitOfWork: Send + Sync` + `pub struct Transaction<'a>`
  session newtype (Option γ, primitive-borrower pattern) in `store.rs`. Five apply
  pipelines (call_graph, state_machine, class_diagram, c4_discover, diagram::apply_to_store)
  now use `Transaction::commit()` / `Transaction::rollback()` with implicit Drop-rollback.
  `begin` internalised — only `commit`/`rollback` exposed to callers.
- **P1-05** — `test-fixtures` Cargo feature (`test-fixtures = []`) enabling the
  `execute_raw_cypher_for_test` escape hatch in bench fixtures and integration tests.

### Changed
- **P1-05 (A-W1)** — `+ RawGraphQuery` supertrait removed from `GraphStore`
  (`store.rs:204`); `RawGraphQuery::query` remains reachable only via
  `impl RawGraphQuery for LbugStore` on concrete `&self` (no `dyn` dispatch).
- **P1-05 (C-W1)** — `pub fn session_mut` and `pub fn execute_raw_cypher_for_test`
  in `store.rs` now compiled only under `#[cfg(any(test, feature = "test-fixtures"))]`;
  production builds do not expose these symbols.

### Fixed
- **P1-05** — `link_with_merge_fallback` (`store.rs`) is now transaction-safe:
  replaced two-query MERGE+fallback pattern with a single idempotent
  `OPTIONAL MATCH ... WHERE r IS NULL ... CREATE` query. Fixes Kùzu 0.18.3
  jurisprudence: any query error inside a Kùzu transaction causes auto-revert of
  the entire transaction. The fallback now never throws a duplicate-PK error,
  avoiding spurious auto-reverts that left no active transaction for COMMIT.

## [1.44.1] — 2026-08-15

### Fixed
- **P1-04 (patch)** — `LbugStore::open_raw` now initializes the schema before
  first query: raw graph reads against a fresh project returned empty results
  (regression caught by UAT, invisible to 831 unit tests).
- **P1-04 (patch)** — `is_read_only_query` write-keyword guard tokenized:
  substring matching let `MERGE` slip through when a token started with a
  write keyword; the guard now splits on word boundaries before rejecting
  MERGE/CREATE/DELETE/SET/REMOVE. (PR #182, UAT-driven)

## [1.44.0] — 2026-08-15

### Added
- **P1-04** — `RawGraphQuery` admin-only trait: the **only** entry point for raw
  Cypher execution in `store.rs`. The `LbugStore` implementation enforces
  `is_read_only_query` guard (rejecting MERGE/CREATE/DELETE/SET/REMOVE) on
  every call. Application code must use typed repository traits instead.
  `execute_raw_cypher_for_test` provided as a test-only escape hatch.
- **P1-04** — `SemanticEdgeRepository` trait (`link_semantic_edge`,
  `link_call_edge_with_resolution`) as the write port for semantic edge
  creation, replacing raw Cypher in `call_graph`, `class_diagram`,
  `state_machine` apply pipelines.
- **P1-04** — `ElementRepository::ensure_metatype` for metatype pre-seeded
  existence guarantees in `c4_discover` and `call_graph` apply paths.
- **P1-04** — `SemanticEdgeRepository::link_call_edge_with_resolution` handles
  the full call-edge creation including name-resolution and `CALL_EDGE` label
  population in `call_graph::apply`.

### Changed
- **P1-04** — `diagram::queries` reads wired to `DiagramRepository` directly
  (the four `query_*` free functions removed); call sites updated in
  `diagram/export.rs`, `diagram/project/*`. Deprecation re-exports of
  `ElementRow`, `SemanticEdgeRow`, `VersionPropsRow` added with
  `#[deprecated(since = "1.43.0")]` guidance.
- **P1-04** — `call_graph`, `class_diagram`, `state_machine` apply pipelines
  rewired from raw Cypher writes to `ElementRepository::upsert_element`,
  `upsert_element_version`, and `SemanticEdgeRepository` methods.
- **P1-04** — `c4_discover` apply pipeline rewired to repository methods
  (`ensure_metatype`, `upsert_element`).

### Removed
- **P1-04** — ~140 lines of dead `call_graph` apply scaffolding: the old
  `apply` function body replaced by direct repository calls via
  `SemanticEdgeRepository::link_call_edge_with_resolution`.
- **P1-04** — `diagram::queries::query_elements`,
  `diagram::queries::query_semantic_edges`,
  `diagram::queries::query_evidence_for_versions`,
  `diagram::queries::query_version_props` free functions removed;
  callers now use `DiagramRepository` typed reads.

## [1.43.0] — 2026-08-15

### Added
- **P1-03** — Architecture repositories: introduce `ElementRepository`,
  `EvidenceRepository`, `SourceRepository`, `EvaluationRepository`,
  `DiagramRepository` as siblings of `GraphStore` in `store.rs`.
  `LbugStore` implements all five; the four `code/*` apply pipelines
  (`call_graph`, `class_diagram`, `state_machine`, `c4_discover`) plus
  the four `diagram::queries` reads plus the 12 `evidence::tests`
  raw queries now consume the typed port. `archctl/src/graph.rs` no
  longer imports `lbug` (domain→lbug paydown of dep-fitness baseline
  finding #1; baseline ratchet 4→3). The `Session`/`create_db_session`
  lifetime-transmute trick and the `escape_cypher_string` helper moved
  to the `LbugStore` adapter. `manifests/graph.toml` now requires
  `use lbug` absence and the five repository types in `must_hold`;
  `manifests/store.toml` exposes the new traits in `public_symbols`.

- **P1-01** — `GraphStoreFactory` trait + `LbugStoreFactory` adapter as the
  composition root for store initialisation (ADR-010 single-writer flock).
  `CliContext` extended with `clock: Arc<dyn Clock>` and
  `store_factory: Arc<dyn GraphStoreFactory>`; all 9 store call sites and
  8 clock literals rewired through context. Eliminates ad-hoc `LbugStore::open`
  scattered across handlers.
- **P1-09 (Wave 1 item 8)** — `scripts/check-dep-fitness.sh`: architectural
  dependency fitness check implementing the self-dogfood rules from the
  2026-08-13 plan (`domain !-> lbug/reqwest`, `application !-> tiny_http/
  std::process`, `projection !-> cli`, `analysis !-> view`). Report-only
  with a baseline ratchet (`scripts/dep-fitness-baseline.txt`, 4 legacy
  findings documented with paydown paths); `--strict` mode for the future
  CI-blocking DoD; `--json` for tooling. Wired into `verify-local.sh`
  (cheap tier) and `scripts/test-ci-gates.sh` (6 new gate assertions).
- **P0-03** — `release.yml` native runners per target: darwin binaries
  built on macOS (`macos-13`/`macos-14`), linux aarch64 on `ubuntu-24.04-arm`;
  plus assets-stack bootstrap before the release build.

### Changed
- **P1-03** — `LbugStore::open` + `init` replaces `crate::store::open_and_init`
  in `code/call_graph.rs::apply`, `code/c4_discover.rs::apply`,
  `code/class_diagram.rs::apply`, and `code/state_machine.rs::apply`.
  `Box<dyn GraphStore>` no longer threads through these handlers — they
  hold `LbugStore` directly so the repository traits (which `Box<dyn
  GraphStore>` cannot expose via dynamic dispatch) are reachable.

### Removed
- **P1-03** — Public `graph::Session` struct and `graph::open_session`
  function removed; the lbug session lifetime is now private to
  `LbugStore::LbugSession`. `GraphStore::query` retained on the trait
  (admin boundary per `02-TARGET-ARCHITECTURE.md`) but no longer
  reachable from apply paths.

## [1.42.0] — 2026-08-14

### Added
- **P0-ladybug-doctor** — `archctl doctor --scope storage [--json]` checks
  LadybugDB (lbug) availability, schema initialization, and basic
  read/write operations. The new `doctor/` module (`archctl/src/doctor/`)
  provides `DoctorScope` enum, `LbugStorageProbe`, `NativeProbe`,
  and smoke gate runner. CLI dispatch via `--scope` flag on the existing
  `doctor` subcommand. JSON output follows a 5-axis envelope
  (`archctlVersion`, `lbugCrateVersion`, `native`, `targetCompilerStdlib`,
  `findings[]`) per ADR-048. Integrates with Tier-1 CI smoke gate in
  `pr.yml` and release gate in `release.yml`.

### Fixed
- **CI red on main (100+ runs)** — `ci.yml` and `pr.yml` now bootstrap
  `archctl/assets-stack/` via `scripts/embed-stack.sh` before building
  (the rust-embed folder is gitignored; same gap M33 fixed for the
  pre-push hook was never applied to CI workflows).
- Version sync guard failure — `archctl/Cargo.toml` bumped 1.41.0 →
  1.42.0 to match tag `v1.42.0` (tag was pushed without the bump).

## [1.41.6] — 2026-08-14

### Added
- **P0-12** — Pre-merge CI workflow (`.github/workflows/pr.yml`): fast
  deterministic checks on every PR (build, test, clippy, fmt, doctor,
  script gates + ADR integrity). Benchmarks remain post-merge in `ci.yml`.

## [1.41.5] — 2026-08-13

### Fixed
- **P0-06/07/08** — Plugin security hardening: identifier validation,
  SHA256 checksum verification, safe tar extraction.

## [1.41.4] — 2026-08-13

### Fixed
- **P0-04/05** — Plugin install: XDG root path resolution +
  `create_dir_all` on first install.

## [1.41.3] — 2026-08-13

### Added
- Frontier freeze baseline: golden outputs + size/import map
  (`chore(baseline)`, PR #170).

## [1.41.2] — 2026-08-13

### Fixed
- **P0-11** — License coherence: Cargo.toml updated from `MIT` to
  `MIT OR Apache-2.0` (matching README). Added `LICENSE-MIT` and
  `LICENSE-APACHE` files. Fixed broken LICENSE link in README badge.
  Added `"license"` field to `archview/package.json`.

## [1.41.1] — 2026-08-13

### Added
- **P0-10** — `scripts/check-adr-integrity.sh` validates ADR directory
  integrity (unique IDs, filename↔H1 match, valid status, index
  consistency, broken links, gap info). Supports `--json` for CI.
  Test fixtures in `scripts/fixtures/adr/`.

### Changed
- **P0-09** — Resolved duplicate ADR IDs:
  `ADR-040-archctl-versioned-distribution` → ADR-057,
  `ADR-041-self-update-github-releases` → ADR-058.
  All cross-references updated (11 files).
- Landed H5-H8 consolidation pack at `docs/arch-stack-proposals-2026-08-13/`
  (4 horizons, 14 ADRs 043-056, 18 specs, 34-PR plan).

## [1.41.0] — 2026-08-12

### Added
- **M80b** — `archctl diagram export --format arrows` exports a deterministic
  `.arrows` JSON document (Arrows.app v0.8 shape). The serializer is a pure
  function over `BundleEnvelope { projection, styles }` — no I/O, no lbug
  access. Default output path is derived from the selector (replaces `:` and
  `/` with `_`). The `--json` envelope includes `unplaced_count` for cosmetic
  overlap auditing.

## [1.40.0] — 2026-08-12

### Removed
- **M83** — `archctl stack` subcommand removed (deprecated since
  v1.35.0, stub since v1.37.0). Use `archctl ide install <ide>
  --install-root X` and `archctl ide doctor <ide>` instead. Migration
  script: see `e2e/install_e2e.sh`.

## [1.39.1] — 2026-08-12

### Fixed
- **M82** — `npm-single` strategy now resolves `pnpm-workspace.yaml` relative to the manifest it actually read (parent directory), instead of always checking `project_root`. Prevents mis-classifying monorepos whose `package.json` lives in a subdirectory next to a sibling `pnpm-workspace.yaml` (vueuse-style `apps/web/...` layouts).

## [1.39.0] — 2026-08-12

### Added
- **M81 D2** — Projection schema v1.1: `Node` exposes cosmetic view fields
  (`x`, `y`, `collapsed`, `labelOverride`). `build_bundle` LEFT JOINs
  `ViewMember` rows (one query + HashMap lookup, ADR-019). `archview`
  renders `labelOverride ?? name`. Backward-compatible: bundles v1.0 keep
  validating (optional fields with defaults).

### Fixed
- **M81 D1** — `Command::MoveMember` preserves the existing `ViewMember.label`
  instead of resetting it to an empty string.

### Changed
- `baseRevision` of existing diagrams is invalidated one-time: the hash now
  covers cosmetic fields, so pre-1.39.0 revisions are stale by design. Apply
  rejects them with `baseRevision mismatch`; re-export regenerates. No
  migration needed (ADR-017 not triggered).

## [1.38.1] — 2026-08-12

### Added
- **M80** — Cosmetic ChangeSet round-trip e2e coverage: `apply_with_matching_base_revision_succeeds` and `apply_round_trips_export_revision` integration tests in `diagram_apply.rs`. Verifies baseRevision optimistic-concurrency token works across the v1.38.0 cosmetic apply plumbing.

## [1.38.0] — 2026-08-12

### Fixed
- **M79 D1** — `inventory::tree()` now prunes `target/`, `node_modules/`,
  `dist/`, `build/`, `.git/`, `.venv/`, `__pycache__/`, `.gradle/` by exact
  directory name regardless of `.gitignore`. Previously, a subdir walk without
  a local `.gitignore` entry for `target/` would traverse thousands of build
  artefact files, causing timeouts on large repos (root `.gitignore` was not
  inherited because `parents(false)` is intentionally set).

### Added
- **M79 D2** — Nested manifest fallback for all four C4 strategies:
  - `cargo-workspace`: when `Cargo.toml` is absent at project root, finds the
    nearest nested `Cargo.toml` (depth ≤ 3) and resolves the full workspace
    via `cargo metadata --manifest-path`. Works for arch-stack's own layout
    (`archctl/` at depth 1).
  - `npm-workspace`: root-first `package.json` + `workspaces` detection
    unchanged; fallback now finds first nested `package.json` with `workspaces`
    and re-bases glob patterns to the manifest's parent directory.
  - `npm-single`: first nested `package.json` within depth ≤ 3 (previously
    returned empty for all subdir repos).
  - `components`: `collect_container_dirs` now falls back to `find_manifests`
    for both Cargo and npm, correctly skipping sibling service directories.

### Changed
- archctl version 1.37.2 → 1.38.0.
- README.md (`en` + `es`) + MANUAL.md + STATE.md refreshed for v1.38.0.
- `archctl stack` deprecation carried forward (M77): removal deferred to
  v1.39.x (slipped from this cycle; tracked in M79 next-up).

## [v1.37.1] — 2026-08-11

### Fixed
- **M77a hotfix** — Claude Code + Codex adapters now write to `~/.claude/` and
  `~/.codex/` respectively (HOME-relative) instead of `~/.config/claude/`
  and `~/.config/codex/` (XDG-correct but invisible to the IDEs). Discovered
  during user-side install verification of v1.37.0.

## [v1.37.0] — 2026-08-11

### Added
- M77 — `archctl plugin install` download + extract with SHA256 verify:
  - `install_plugin()`: download, verify, extract to `~/.config/archctl/plugins/`.
  - `download_plugin()`: HTTP GET with 120s timeout + user-agent.
  - `verify_plugin_sha256()`: SHA256 hash verification (bail on mismatch).
  - `extract_plugin()`: tar.gz extraction via flate2+tar.
  - `parse_plugin_spec()`: parses `author/name@version` or `author/name` (latest).
  - `archctl plugin install <spec>` now wires to `install_plugin()` and resolves
    `latest` to highest semver in the tap.
- M77 — Homebrew formula (`Formula/archctl.rb`) for `brew install`.
- M77 — `archctl/permissions.yaml` for SDDK011 permissions gate bootstrap.

### Changed
- M77 — `archctl stack` now exits with error code 2 and a deprecation
  message pointing to `archctl ide install`. The subcommand will be
  removed in v1.38.0.
- archctl version 1.36.0 → 1.37.0.
- archview version 1.36.0 → 1.37.0.

### Removed
- M77 — `archctl stack` subcommand body removed (stub remains with deprecation
  error). Use `archctl ide install` instead.

## [v1.36.0] — 2026-08-11

### Added
- M76 — `archctl plugin` subcommand with tap model (ADR-057 §4):
  - `Tap` and `PluginEntry` types for plugin distribution.
  - `fetch_tap()` to retrieve plugin list from a URL.
  - `archctl plugin install <name>@<version> [--tap <url>]` (stub: M77 adds download).
  - `archctl plugin list [--tap-url <url>]` — lists plugins from a tap.
- M76 — `archctl ide` now loads real skills/agents/plugins from embedded
  `assets-stack/` via `rust-embed`. `IdeAction::Install` uses
  `current_stack_payload()` instead of empty payload.

### Fixed
- M76 W4 — migration script sandbox: replaced `sh -c` shell injection risk
  with `Command::new(script_path)` + path validation (no `..`, no `/` prefix).
  `run_sandboxed_script()` uses `env_clear()` + restricted `PATH=/bin:/usr/bin`.

### Changed
- archctl version 1.35.0 → 1.36.0.
- archview version 1.35.0 → 1.36.0.

### Tests
- M76 W3 — `lifecycle_update_e2e.rs` with `tiny_http` mock server for
  GitHub API testing (avoids tokio+blocking-reqwest incompatibility).

## [v1.35.0] — 2026-08-11

### Added
- M75 — `archctl ide` subcommand with adapter abstraction (ADR-042):
  - `IdeAdapter` trait + 4 built-in adapters (OpenCode, ZCode, Claude Code, Codex).
  - `archctl ide install <ide>` — installs stack payload to the IDE's
    native discovery path (e.g. `~/.config/opencode/skills/` for OpenCode,
    `~/.claude/plugins/arch-stack/skills/` for Claude Code,
    `~/.codex/prompts/<name>.toml` for Codex).
  - `archctl ide list [--installed]` — lists supported IDEs + which are installed.
  - `archctl ide doctor <ide>` — diagnostic specific to one IDE.
  - `archctl ide remove <ide>` — removes our payload (preserves user files).
  - `archctl ide update <ide>` — re-install (alias for install).
  - `archctl stack install opencode` kept as alias deprecated for one cycle.

### Changed
- archctl version 1.34.0 → 1.35.0.
- archview version 1.34.0 → 1.35.0.

### Migrations
- M75 migration-manifest.json: schema version 1.35.0 — new `ide` bounded
  context; no breaking changes to existing IDE discovery paths.

## [v1.34.0] — 2026-08-10

### Added
- **M73 T1 — archctl self install**: multi-version install in
  `~/.local/share/archctl/installs/<version>/`, shim binary generation,
  `archctl self install [--version X]` (latest stable by default).
- **M73 T2 — archctl self list/use/uninstall**: `archctl self list [--json]`,
  `archctl self use <version>`, `archctl self uninstall [--purge]` with
  per-project `.arch-version` pin via `ARCHCTL_VERSION` env var.
- **M73 T3 — archctl self update**: GitHub Releases API client (`reqwest`
  blocking), SHA256SUMS verification, migration manifest execution,
  `archctl self update [--channel stable|rc|nightly] [--check]`.
- **M73 T4 — manifests/self.toml**: manifest gate for lifecycle scope
  with 30 minimum tests, must_hold invariants (ARCHCTL_HOME, current,
  installs/v, .arch-version, migration-manifest.json, SHA256SUMS),
  and must_not_contain unsafe_code.

### Changed
- **archctl**: version bumped 1.33.0 → 1.34.0.
- **archview**: version bumped 1.33.0 → 1.34.0 (synchronized per ADR-038).

## [v1.30.0] — 2026-08-10

### Changed
- **M69 — Arch-stack product roadmap convergence**: documentation-only
  cycle consolidating validated product decisions into canonical ADRs.
  PR #142 (squash `b00e063`).

  - **3 new ADRs**: ADR-038 (one product, five invariants), ADR-039
    (renderer reality + anti-roadmap with measurable reopen triggers),
    ADR-040 (cognitive layer conditional activation).
  - **5 ADRs annotated** (header-only, bodies preserved verbatim):
    ADR-013 and ADR-020 SUPERSEDED; ADR-021/022/023 conditional.
  - **6 new spec stubs** for H0–H3 horizons: `executable-bundle-contract`,
    `durable-workspace-state`, `source-drawer-read-only`,
    `cosmetic-changeset-roundtrip`, `arrows-compatibility-adapter`,
    `lens-spec-entry-criteria`.
  - **ROADMAP restructure**: H0–H3 outcome-driven horizons replace the
    milestone-aspiration mix. M17–M23 anchors preserved with
    `→ superseded by H{n}` redirects.
  - **Identity correction**: STATE/README/specs-index/manifest reflect
    arch-stack as one product (`archctl` + `archview` coupled via
    rust-embed), not archctl alone.
  - **20 files changed**, +768/-30, docs-only (zero source modifications).

## [v1.29.0] — 2026-08-07

### Added
- **M61 — Cognitive policy tests**: 22 unit tests for `cognitive/policy/{context,decision}`
  (the 0-test gap from M55 study). Side-fix: `PolicyResult` derives `PartialEq`
  (test convenience). Cognitive test count 111 → 133. PR #140.

## [v1.28.0] — 2026-08-07

### Added
- **M57 — CONTRIBUTING.md**: 248 lines with cycle workflow, manifest hygiene
  conventions, bounded contexts, testing rules, and a "what-not-to-do" list.
  Cross-referenced from AGENTS.md. PR #136.

## [v1.27.0] — 2026-08-07

### Fixed
- **M60 — Resolve 2 TODO markers from M55 study**:
  - `code/strategies/dockerfile.rs:139` — OCI LABEL parser
    (handle multi-label and quoted values).
  - `code/class_diagram.rs:1067` — Python class method extraction
    (handle decorated methods correctly).
  - 12 new unit tests; 1 golden fixture regenerated. PR #134.

## [v1.26.0] — 2026-08-07

### Changed
- **M56 — DRY skip-on-missing-backend helper**: extracted
  `archctl::test_helpers::plantuml::backend_available` to deduplicate
  the "skip if no PlantUML backend" pattern across 5 e2e files.
  Net **-7 LOC** across the test files. PR #130.

## [v1.25.0] — 2026-08-07

### Added
- **M55 — Codebase state study + 11 prioritized improvement proposals
  (M56–M68)**: post-session study at v1.24.0 documenting current state,
  active debt, and a prioritized backlog. Reference:
  `docs/sessions/2026-08-07-codebase-state-study.md`. PR #128.

## [v1.24.0] — 2026-08-07

### Added
- **M54 — Session close**: CHANGELOG backfill (v1.18.0 → v1.23.0 entries
  below) + Engram session summary. 21 cycles closed in this session
  (v1.4.1 → v1.24.0, 42 PRs, 70+ commits, 25 tags).

## [v1.23.0] — 2026-08-07

### Added
- **M53 — M32 D5 sequence writer audit**: audit verdict: N/A.
  `archctl/src/code/sequence.rs` is a READ-ONLY projector per SCN-217.
  No apply/writer function exists. M32 D5 migration pattern does not
  apply. PR #125.

## [v1.22.0] — 2026-08-07

### Fixed
- **M52 — M32 D4 doc fixes**: removed 3 stale "no parameter binding"
  claims in `archctl/src/` (queries.rs, graph.rs, plus M51 already
  fixed store.rs:420). ROADMAP + bench criterion were already correct
  from prior cycles. PR #123.

## [v1.21.0] — 2026-08-07

### Added
- **M51 — Prepared statements + parameter binding** (M32 D3 / ADR-036):
  added `GraphStore::prepare` + `GraphStore::execute` port methods +
  `PreparedStatementHandle` + `Params` types. LbugStore implements both
  via `Connection::prepare/execute`. 3 unit tests for round-trip +
  empty params + i64/String params. PR #121.

  **Known limitation** (documented in test): lbug wraps String params
  as `Value::Json` which doesn't match typed String properties in
  WHERE. Migration of `call_graph::apply` to use prepare/execute is
  deferred until typed bindings or `CAST()` are introduced.

## [v1.20.0] — 2026-08-07

### Fixed
- **M50 — C4 PlantUML e2e + vanilla syntax fix**: pre-M50 the C4 view
  PlantUML projector emitted lowercase Structurizr keywords (`person
  "X" { }`, `container "Y" { }`) inside `@startuml`/`@enduml` —
  syntax rejected by vanilla Java PlantUML. Fixed to emit
  `actor "Name" as Name` and `rectangle "Name" as Name`. Added
  `c4_view_plantuml_e2e.rs`. Closes the verification triangle.
  PR #119.

## [v1.19.0] — 2026-08-07

### Added
- **M49 — State PlantUML e2e verify**: `state_view_plantuml_e2e.rs`
  exercises state projector + M40 backend + SVG (per ADR-026
  transition join semantics). 2 e2e tests, skip-on-missing-backend.
  PR #117.

## [v1.18.0] — 2026-08-07

### Added
- **M48 — Sequence PlantUML e2e verify**: `sequence_view_plantuml_e2e.rs`
  exercises sequence projector (with M45 labels) + M40 backend + SVG.
  2 e2e tests, skip-on-missing-backend. PR #115.

## [v1.17.0] — 2026-08-07

### Added
- **M47 — CHANGELOG backfill + docs/README index**: 14-cycle session summary
  (v1.4.1 → v1.16.0) consolidated into this changelog. `docs/README.md`
  now indexes all 12 view specs under `docs/specs/`. No code changes.

## [v1.16.0] — 2026-08-07

### Fixed
- **M46 — stale `public_symbols` in 8 manifests**: removed 26 entries that
  could not validate (enum variants like `Plantuml`, struct fields like
  `project_id`, removed functions like `from_str`). `archctl doctor
  --scopes <all 26>` now reports 0 findings (was 8). PR #111.

## [v1.15.0] — 2026-08-07

### Added
- **M45 — sequence edge labels**: `archctl diagram project --view sequence:*`
  now reads optional message labels from `edge.props["label"]`. Backward-
  compatible: absent / empty / non-string values fall through to bare arrow.
  Both Mermaid and PlantUML projectors. New `archctl/tests/sequence_view_e2e.rs`.
  PR #109. Spec: `docs/specs/sequence-view-labels.md`.

## [v1.14.0] — 2026-08-07

### Added
- **M43 — use case PlantUML e2e verify**: new
  `archctl/tests/usecase_view_plantuml_e2e.rs` exercises the full chain
  M39 (projector) → M40 (backend) → SVG. SKIP-on-missing-backend. Closes
  the verification loop. PR #107. No code changes (test only).

## [v1.13.0] — 2026-08-07

### Fixed
- **M41 — state + C4 Mermaid projector bug**: state and C4 views emitted
  bare `[Label]` / `(Label)` syntax that merman silently rejected, same
  bug class as M39 found in use case view. Now emit `id([Name]):::state`
  for states and `id(name)` / `id([name])` for C4 persons / systems /
  containers / components. Edges reference node IDs. New e2e tests:
  `archctl/tests/state_view_e2e.rs`, `archctl/tests/c4_view_e2e.rs`.
  Every Mermaid view now renders end-to-end. PR #105. Spec:
  `docs/specs/state-and-c4-views.md`.

## [v1.12.0] — 2026-08-07

### Added
- **M40 — PlantUML local render via user-installed backend**:
  `archctl render --format plantuml` now delegates to a user-installed
  PlantUML engine (Java CLI / docker plantuml/plantuml / custom
  `archctl-puml-backend` binary) in PATH. archctl itself does NOT link
  graphviz or open network connections (ADR-011, ADR-006). The
  `plantuml-little` crate was explored and REJECTED because it hard-links
  `graphviz-anywhere` at compile time. New `archctl/tests/plantuml_render_e2e.rs`
  with skip-on-missing-backend. PR #103. Spec: `docs/specs/plantuml-render.md`.

## [v1.11.0] — 2026-08-07

### Fixed
- **M39 — use case diagrams end-to-end + mermaid node-id bug**: use case
  view (`usecase:*`) now renders end-to-end via merman to valid SVG. Fixed
  pre-existing bug where bare `(Label)` mermaid syntax was rejected by
  merman (masked by substring unit tests for 10+ cycles). Now emits
  `id(Name)` for actors, `id((Name))` (circle) for use cases, edges
  reference node IDs. New `archctl/tests/usecase_view_e2e.rs`. PR #101.
  Spec: `docs/specs/use-case-view.md`.

## [v1.10.0] — 2026-08-07

### Added
- **M37 — public JSON Schema + pure `--json` stdout mode**:
  `archctl diagram export --json` now emits the full bundle envelope
  (manifest + projection + evidence + styles) to stdout without writing
  5 files when `--output` is omitted. `archctl diagram export
  --json --output <dir>` writes both. The envelope validates against
  `schemas/diagram-projection.schema.json` (round-trip test added).
  Refactor: extracted `build_bundle` + `build_export_envelope` helpers.
  PR #98. Spec: `docs/specs/diagram-projection-bundle.md`.

## [v1.9.0] — 2026-08-07

### Added
- **M38 — Mermaid → SVG local render via merman**: pure-Rust renderer
  using `merman-core = "0.8.0-alpha.3"` + `merman-render`. No graphviz,
  no network. Supports sequence, flowchart, class, state diagrams.
  PlantUML rendering deferred to M40 (graphviz vendor strategy).
  Drive-by: enabled `serde_json/preserve_order` feature toggle in row.rs.
  PR #96.

## [v1.8.0] — 2026-08-07

### Added
- **M36 — Kotlin call-graph extraction**: `archctl code call-graph`
  now supports Kotlin (6th language: rust, typescript, python, go, java,
  kotlin). `tree-sitter-kotlin-sg = "0.4"` for parsing. Mirrors the Java
  pipeline from M35.

## [v1.7.0] — 2026-08-07

### Added
- **M35 — Java call-graph extraction**: `archctl code call-graph`
  now supports Java (5th language). `tree-sitter-java = "0.23"` for
  parsing. Class declarations, method declarations, method invocations.

## [v1.6.0] — 2026-08-07

### Changed
- **M34 — call-graph strategy consolidation**: removed dead code
  (`InvalidLanguage` enum variant), removed vacuous test
  (`test_confidence_per_language`), deduplicated `extract_fn` across
  strategies. ~240 LOC reduction.

## [v1.5.1] — 2026-08-07

### Fixed
- **M31-FU1 — tracing → stderr redirect**: archctl tracing output
  now goes to stderr (not stdout) so `--json` stdout mode is clean
  for agent piping. One-line fix in `tracing-subscriber` init.

## [v1.5.0] — 2026-08-07

### Added
- **M31 — unified empty envelope for `archctl diagram export`**:
  consistent shape whether the project exists, has nodes, or has no
  matching selector. Always emits `{manifest, projection, evidence,
  styles, empty, warning}`.

## [v1.4.1] — 2026-08-07

### Fixed
- **BREAK-1 — remove `seed_writes` lying API**: removed a function that
  claimed to "seed" the graph but did nothing observable. PR #84 + #85.

## [v1.1.0] — 2026-08-07

### Added
- **M30 Go call-graph extraction (ADR-035)**: `archctl code call-graph` ahora
  soporta Go de verdad vía tree-sitter-go (bundled en `ast-grep-language`
  0.45.0 builtin-parser). MVP languages pasa a `{rust, typescript, python,
  go}`. Implementado:
  - Go function extraction: `function_declaration` → `FunctionNode`,
    `method_declaration` → `MethodNode`, `func_literal` no genera nodo
    propio (calls se atribuyen a la named function envolvente — ADR-035 D3).
  - Go call-edge extraction: direct calls (`identifier`) + method calls
    (`selector_expression` → `field_identifier`).
  - Help text `--lang` lista Go en los 3 clap doc-strings.
  - `Language::Go` value_enum variant + 2 tests deterministas nuevos en
    `archctl/tests/code_call_graph.rs` (5 nodes + 6 edges; `--lang rust`
    en proyecto Go → 0 filesScanned).
  - ADR-035 (`docs/adr/ADR-035-go-call-graph-extraction.md`) documenta las
    5 decisiones (parsing engine, confidence 0.85, func_literal attribution,
    MVP language list, smoke fixture strategy).
  - Smoke `smoke_echo()` assert extracción Go real (rápido) +
    `smoke_go_apply_fixture` (apply-path sobre fixture pequeño
    `tests/fixtures/go_callgraph/`, 6 elements + 6 relations).
  - Human loop Fase 6 + Fase 9.2 actualizadas (Go soportado, extracción
    rápida; apply-path cubierto por el smoke fixture).
  - Error message MVP lista actualizado: `{rust, typescript, python, go}`.

### Amendment (2026-08-06, ver M32)
El writer `--apply` es lento a escala (~0.43s/elemento; zustand 212 el →
92s, echo 1307 el → 483s) — problema preexistente expuesto por el soporte
Go. Por eso el smoke y el human loop usan extracción rápida + fixture
pequeño para el apply-path. El fix del writer se trackea en M32 (apply
writer performance: batching de transacciones).

### ROADMAP follow-ups (M30 además)
- **M31**: semántica unificada de `diagram export` sin proyecto/grafo.
- **M32**: apply writer performance — batching de transacciones.
- **M33**: pre-push hook — bootstrap `assets-stack/` en worktree fresco.
- **M34**: call-graph strategy consolidation (W3 dead-code
  `InvalidLanguage`, W4 vacuous `test_confidence_per_language`, D2
  extract_fn duplication, D3 inline GO_SAMPLE vs fixture) — cierra la
  deuda debt-report `sddk/m30-call-graph-go-support/debt-report.md`.

### Validation
- **Spec scenarios**: 20/20 cubiertos (4 con test runnable, 14 con
  static inspection, 2 con smoke `#[ignore]`).
- **Tests**: 525 pasan, 0 fallan (523 baseline + 2 nuevos Go).
- **Gates**: `cargo clippy -- -D warnings` clean, `cargo fmt --check`
  clean, smoke `smoke_echo()` + `smoke_go_apply_fixture` 2/2 PASS,
  human loop sandbox 8/8 phases PASS.
- **Verify verdict**: PASS_WITH_WARNINGS (debt-verify: 0 CRITICAL, 5 WARN
  backloqueados a M34 + 1 preexistente OCP `match-lang`).
- **Cycle**: `m30-call-graph-go-support`. PR URL en release notes.

## [v1.0.2] — 2026-08-06

### Added
- **M29 E2E coverage expansion (ADR-034)**: 4 suites versionadas que cierran
  los gaps de instalación, despliegue, render y multi-lenguaje:
  - `e2e/install_e2e.sh` (29 checks): instalación del producto contra HOME
    aislado — stack install, drift none, idempotencia, doctor, view health,
    frontmatter SKILL.md
  - `e2e/render_e2e.py` (20 checks): playwright asserts DOM por tipo de
    bundle (C4/sequence/class/call-graph) + bundles reales multi-lenguaje
    (axum 4, ripgrep 11, zustand 1, express 1) + 0 errores JS + screenshots
  - `bench/sandbox-e2e.sh` (6 checks): sandbox reproducible — build image,
    compilar archctl in-container, vertical C4 con asserts, veredicto JSON
  - `smoke_real_projects.rs` v2 (6/6): vertical completo por lenguaje
    (c4+accept rust/js/ts, call-graph go, class-diagram python), XDG
    aislada por repo, zustand añadido
- **M29.5 integración**: verify-local.sh --full ejecuta install+render+
  sandbox; release.yml añade post-release binary verification (version +
  view + stack status del binario subido)

### Fixed
- **CORS entre origins localhost** en el diseño del render E2E (navegar al
  origin del server por-repo)
- **Smoke no determinista**: XDG compartida hacía `--apply` reportar
  "Applied: 0" por grafos previos — aislada por repo
- **`bash -c` positional arg** en sandbox-e2e (dataset se convertía en $0)

### Validation
- ADR-034 aceptado, M29 completo en ROADMAP
- 523 tests + 6 smoke + 29 install + 20 render checks verdes

## [v1.0.1] — 2026-08-06

### Added
- **`archctl view` (ADR-033)**: embedded archview workbench server. One-shot
  HTTP on 127.0.0.1 serving the archview dist embedded via rust-embed (gzip).
  Endpoints: `/`, `/assets/*`, `/api/health`, `/api/export`. COOP/COEP/CORP
  headers per ADR-020/011. `scripts/embed-view.sh` (sourcemaps excluded).
- **`archctl stack install|update|status` (ADR-033)**: the arch-stack product
  (binary + workbench + skills) installed/versioned/updated as ONE unit.
  Embeeds 9 skills + 5 agents + env plugin; installs into OpenCode/ZCode
  discovery paths idempotently; `status` reports drift.
- **Skills refrescadas al CLI real**: architecture-discovery, c4-from-graph,
  class-view-from-graph, sequence-from-scenario, use-cases-from-graph,
  diagram-review + nuevas evidence-lifecycle, workbench-view, stack-management.
  Comandos verificados contra `archctl --help` (removidas referencias a
  `graph aggregate` inexistente, etc).
- `scripts/embed-stack.sh`, `manifests/view.toml`, `manifests/stack.toml`
  (scope gates), release.yml ahora embebe workbench + skills antes del build.

### Fixed
- **M28: strategy `npm-single`** — detecta package.json raíz como container
  para repos npm single-package (zustand, express). Maneja el edge case de
  `pnpm-workspace.yaml` con solo `allowBuilds:`. 7 tests.
- **M28: conteo FP/FN corregido** — solo metatype `mt.container` cuenta;
  candidates de `components` excluidos (ADR-032). clap FP 27.3% → 0%.
- **dockerfile strategy self-detection** — `dockerfile.rs` matcheaba
  `starts_with("dockerfile.")` → FP "docker". Whitelist de entornos.
- **archview detectKind** — call-graph con `language` se clasificaba como
  class-diagram; check de function ahora gana.
- **embed-view.sh** — preserva el README tracked del assets-view.

### Validation
- FP/FN manual review: **7/7 datasets PASS** (FP <20%, FN <30%) →
  **v1.0 desbloqueado**.
- 523 tests archctl + 102 tests archview; clippy/fmt clean.
- Benchmark M28 re-run: Gate OPEN, 11/11.

### Cycle
- stack-distribution: ADR-033 (view + stack), M28 (npm-single + gate fixes)
- E2E real: sandbox Quadlet (discover→accept→export→validate in-container),
  archctl view render verificado en Chromium headless (5 tipos de bundle)

## [v1.0.0] — 2026-08-06

### Added
- **M27 Sandbox + Benchmarks (PRs #47-#50)**: podman Quadlet-sandboxed harness for pre-v1.0 release gate. 11 datasets (10 multi-language + archctl self-dogfood), 8 thresholds enforced (5 automated, 3 manual), dated report at `bench/reports/<date>.md`. Methodology in ADR-032. Specs in `docs/specs/bench-{harness,methodology}.md`. Container: ubuntu:24.04 + rustup 1.97.1 (rejected catthehacker/ubuntu:rust-latest, ADR-032 Q2).

### Cycle
- verify-report: PASS (after W1-W5 corrections)
- 17 files, ~1350 LOC
- chain: stacked-to-main (PRs #47, #48, #49, #50)
- benchmark run (2026-08-06): **all automated thresholds PASS**
  - exit_zero_rate: 100% (11/11)
  - c4_discover_time: 311ms max (< 30s threshold)
  - peak_rss: 144MB max (< 500MB threshold)
  - bundle_validity: 7/7 (100%)
  - determinism: 7/7 (100%)
  - FP/FN: manual review pending (non-blocking per ADR-032)
- v1.0 gate: **UNBLOCKED** — automated thresholds satisfied

## [v0.14.10] — 2026-08-05

### Fixed
- **M26.5 C4 Vertical End-to-End Validation (ADR-031)**: 6 bugs discovered by
  smoke-testing the C4 vertical against `tokio-rs/axum` (real Cargo workspace):
  - **B1**: `apply()` wrote to `<cwd>/architecture.lbdb`, `graph_query`/`export`
    read from `<XDG>/.../architecture.lbdb`. Two different DBs. Now `apply()`
    receives `info.project_dir` consistently.
  - **B2**: `query_evidence_for_versions` / `query_version_props` generated
    invalid Cypher (`IN [id1, id2]` without quotes). Wrap IDs in single quotes.
  - **B3**: `write_evidence` silently swallowed schema errors with `.ok()` —
    0 evidences persisted. Remove invalid `status`/`language` columns, propagate
    errors via `?`.
  - **B4**: `version_id = blake3(version_props)` collided — all containers
    shared the same ElementVersion. Include `element_id` in hash input.
  - **B5**: `"status": "Drafted"` (capital) vs `parse_label` lowercase mismatch —
    `evidence accept` was no-op. Change to `"drafted"`.
  - **B6**: Bundle export emitted `type="c4"` and `status="active"`, but schema
    requires `enum:["context",…]` and `enum:["accepted",…]`. Add
    `kind_id_to_type()` and `schema_valid_status()` mappings.

### Added
- **`tests/smoke_real_projects.rs`** (#[ignore]): cached clones of mini-redis,
  echo, express, requests. Run with `cargo test --test smoke_real_projects -- --ignored`.

### Validation
- 402 lib tests + integration tests passing
- tokio-rs/axum: discover→apply→accept→export→validate produces a valid bundle
  (4 containers + 4 evidences detected and accepted)

## [v0.14.9] — 2026-08-05

### Fixed
- **M26 C4 Contract Integrity (ADR-024)**: Fixed `Element.category` semantic
  mismatch that broke the C4 discover→export→archview vertical.
  `export.rs` was deriving `category` from `C4Kind.to_string()` (e.g.,
  `container`), but `c4_discover` writes `category='c4'` (the canonical
  diagram family). Now queries use `category='c4' AND kind_id CONTAINS '{kind}'`
  which correctly matches the `mt.container` format. Unblocks M17 closing
  → M16 closing → v1.0. No data migration required.

## [v0.14.8] — 2026-08-05

### Changed
- **M8 TSG Vestigial Cleanup**: removed dead `tsg.rs` adapter (229 LOC),
  `call_rules/` module + 3 `.tsg` files (~378 LOC), and `from_tsg_node`
  from evidence pipeline. The `basemind-tree-sitter-graph` crate dependency
  was removed. ADR-012 amended to mark TSG as REMOVED. All extractors use
  direct tree-sitter walks since v0.8.1 — no functional change.

## [v0.14.0] — 2026-08-05

### Added
- **M18 Reactive Runtime** (3 PRs): event + delta reactive types (PR1),
  subscriptions + `EventDispatcher` (PR2), integration tests + observer docs (PR3).
- **M21 Cognitive Layer Foundation (PR #27)**: `AgentContext`, `AgentOutput`,
  `ReactiveObserver` types; dispatcher, escalation ladder, MCP gateway;
  `Evidence.properties` field for metadata.
- **M22 Agent Catalog (PR #30)**: `ArchitectureAgent` + `ProjectionAgent`
  as `ReactiveObserver`.
- **M23 PolicyGate + Audit + Governed Invoke** (6 phases):
  `ActionProposal` v1.0 with backward compat, `PolicyEngine` + default rules,
  audit module (JSONL log + HITL queue), `PolicyGate` MCP seam,
  `governed invoke` CLI + xdg `policies_root`, full policy gate flow tests.
- **M24 Diagram Authoring Toolchain**: G1 state machine extraction,
  G3 evidence put for semantic facts, G4 diagram project DSL projection,
  G5 C4 components strategy, G2 metamodel extension
  (`metamodel-core.json`); ADR-026..029; skills SKILL.md realignment.

### Changed
- Release pipeline + CI post-merge hardening (dup-002, dup-006).
- Archview ESLint 9 + Prettier configuration.
- M17 viewer-bundle contract alignment, routing fixes, package-view onselect.

### Refs
- Cycles: `m18-reactive-runtime`, `m21-cognitive-layer`, `m22-agent-catalog`,
  `m23-policygate-audit`, `m24-diagram-authoring`
- All intermediate v0.13.2–v0.13.7 cycles now nominal tags (consolidated below).

## [v0.13.8] — 2026-08-03

### Changed
- **CI post-merge (ADR-025)**: `.github/workflows/ci.yml` triggers only on
  `push` to `main` (removed `pull_request` trigger). Feature branches are
  verified locally instead of on the remote. All four gate groups remain:
  rust (build/test/clippy/fmt/doctor), bench-smoke, bench-compare, web
  (test/build/bundle-cap ≤2MB gzipped).
- **Toolchain pinned exactly**: root `rust-toolchain.toml` pins `1.97.1`
  (profile minimal, rustfmt + clippy); floating `rustup toolchain install
  stable` steps removed. MSRV declared `1.91` in `archctl/Cargo.toml`
  (spec proposed 1.85; dependency tree requires 1.91 — validated at apply).
- **`bench-compare` baseline**: `scripts/bench-compare.sh <ref>` compares
  against an explicit baseline (default `origin/main`). Post-merge CI passes
  `github.event.before`; local `--full` passes `origin/main`. All-zero SHA
  and absent/invalid/unreachable baselines fail with exit 2.
- **Doctor path fixed**: the CI doctor step now runs
  `./target/release/archctl` (from `archctl/`), not `../target/...`.

### Added
- **`scripts/verify-local.sh`**: tiered local verification. Cheap mode
  (default): Rust test/clippy/fmt/doctor. `--full`: adds web
  test/build/bundle-cap, benchmark smoke, and ADR-019 comparison vs
  `origin/main`. Exit 0 = pass, 1 = gate failure, 2 = usage/prerequisite.
- **`.githooks/pre-push`**: runs cheap `verify-local.sh` on every push
  (installed via `core.hooksPath` by `scripts/install-hooks.sh`).
- **`scripts/test-ci-gates.sh`**: deterministic checks for trigger policy,
  preserved jobs, baseline wiring, zero-SHA/error behavior, local verify
  modes, hook wiring, toolchain pin, and MSRV.
- **ADR-025**: decision record for post-merge CI + pinned toolchain +
  local-first prevention, including trade-offs and rollback.

### Refs
- Cycle: `ci-main-gates`

## [v0.13.7]

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

## [v0.13.6]

### Added
- **CI gate (M20 / ADR-019)**: GitHub Actions workflow with 3 jobs —
  rust (build/test/clippy/fmt/doctor), bench-smoke (criterion quick),
  web (vitest + build + bundle cap ≤2MB gzipped). First CI for arch-stack.

### Refs
- Cycle: `m20-ci-gate`
- Partially closes M20 (CI gate slice); regression >10% comparison deferred to backlog

## [v0.13.5]

### Changed
- **`store::open_and_init` promoted to canonical helper**: 8 CLI handlers
  (`graph init/stat/query/neighbours`, `evidence accept/supersede/list`,
  `graph export`) now use the shared `open_default + init` sequence from
  `crate::store` instead of inline duplication. `code/*` apply pipelines
  import it from `crate::store` too. No behavior change; net −7 lines.

### Refs
- Cycle: `refactor/open-and-init-store`
- Closes debt-report suggestion from `source-artifact-id` cycle

## [v0.13.4]

### Changed
- **Test coverage**: unit tests for `c4_language_label` (dockerfile/manifest
  extension derivation) and `existing_canonical_keys` (empty, seeded, and
  no-key-element stores). Closes backlog observations from the
  `source-artifact-id` debt-report. 240 lib tests (+6).

### Refs
- Cycle: `test/apply-common-helpers`

## [v0.13.3]

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

## [v0.13.2]

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

## [v0.6.1] — hygiene

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
