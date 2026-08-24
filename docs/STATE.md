# Estado de `arch-stack`

> Snapshot del estado real del repo. Refreshed al cierre de cada ciclo
> para reflejar la verdad del código, no la planificación aspiracional.
> Última actualización: 2026-08-23, post-ciclo `cognitive-layer-coverage-v2`
> (v1.89.0) — M34 cycle CLOSED + M34 HIGH debt (DecisionPriority stub
> variants collapse) + cognitive-layer-coverage-v2 sprint (3 PRs, +26
> tests sobre `context.rs`, `dispatcher/event_dispatcher.rs`,
> `mcp/gateway.rs`). Latest shipped: v1.88.0 M34 cycle; v1.89.0 cierra
> el sprint v2. Sin ADR, sin migration, sin port changes.

## Estado del trunk

| Field | Value |
|---|---|
| Branch principal | `main` |
| Tip | `b034152` (test(cognitive): cover mcp/gateway — policy gate integration paths (v2 PR 3 of 3) (#323)) |
| Versión | `v1.89.0` (cognitive-layer-coverage-v2 + M34 closure + M34 HIGH debt) |
| Tests | full-suite `@ v1.89.0` (lib 424 cognitive::subset @ 31 gateway + 37 context + 50 dispatcher, +26 vs v1.88.0); doctest green; clippy clean |
| Working tree | clean (`main`); tags v1.65.0–v1.89.0 verificados en origin |
| MSRV | `1.91` (`rust-version` en `archctl/Cargo.toml`); CI pin `1.97.1` |
| LOC src | 59,046 + ~1216 (cognitive-layer-coverage-v2) ≈ 60,262 (`find archctl/src -name "*.rs" \| xargs wc -l` @ v1.89.0; incluye unit tests inline) |
| LOC tests | 17,502 (`archctl/tests/**/*.rs`, integration, 56 ficheros) |
| LOC benches | 903 (`archctl/benches/**/*.rs`) |
| Vault milestones | 37 + Wave 2 + Wave 3 parcial + M21 culling-lod + M23 perf-ci-gate + T0 TRUST-001..008 + M34 cognitive-context-compression + cognitive-layer-coverage-v1 (v1.87.3) + cognitive-layer-coverage-v2 (v1.89.0) — ciclos archivados en `~/.sddk-knowledge/arch-stack/changes/` |
| Tags | 163 (v0.1.0 → v1.89.0; gap `v1.46.0` never tagged — see v1.47.0 nota; v1.88.0 + v1.89.0 added) |

## Capacidades shipped (v1.x — post-v1.0.0)

| Tag | Cycle | Capacidad |
|---|---|---|
| `v1.1.0` | M30 | Go call-graph extraction |
| `v1.2.0-m32` | M32 PR1 | Apply writer transaction wrap (D1) |
| `v1.3.0-m32-pr2` | M32 PR2 | Bulk UNWIND import (D2) |
| `v1.4.0-m32-d5` | M32-D5 | Sibling writers (class_diagram + state_machine) |
| `v1.4.1` | BREAK-1 | Remove `seed_writes` lying API |
| `v1.5.0` | M31 | Unified empty envelope for `diagram export` |
| `v1.5.1` | M31-FU1 | Tracing → stderr redirect |
| `v1.6.0` | M34 | Call-graph strategy consolidation (~240 LOC) |
| `v1.7.0` | M35 | Java call-graph (5th language) |
| `v1.8.0` | M36 | Kotlin call-graph (6th language) |
| `v1.9.0` | M38 | Mermaid → SVG local render via merman |
| `v1.10.0` | M37 | Pure `--json` stdout mode + reusable bundle builder |
| `v1.11.0` | M39 | Use case diagrams end-to-end (fix mermaid node-id bug) |
| `v1.12.0` | M40 | PlantUML via user-installed backend (Java CLI / docker / custom) |
| `v1.13.0` | M41 | State + C4 Mermaid e2e (fix projector bug) |
| `v1.14.0` | M43 | Use case PlantUML e2e verify |
| `v1.15.0` | M45 | Sequence edge labels from `edge.props["label"]` |
| `v1.16.0` | M46 | Manifest `public_symbols` cleanup (26 stale entries) |
| `v1.17.0` | M47 | CHANGELOG backfill (v1.4.1 → v1.16.0) + docs/README index |
| `v1.18.0` | M48 | Sequence PlantUML e2e verify |
| `v1.19.0` | M49 | State PlantUML e2e verify |
| `v1.20.0` | M50 | C4 PlantUML e2e + vanilla syntax fix |
| `v1.21.0` | M51 | Prepared statements + parameter binding (M32 D3) |
| `v1.22.0` | M52 | M32 D4 doc fixes (3 stale claims) |
| `v1.23.0` | M53 | M32 D5 audit (sequence is read-only) |
| `v1.24.0` | M54 | Session close (CHANGELOG backfill v1.18.0–v1.23.0) |
| `v1.25.0` | M55 | Codebase state study + 11 prioritized proposals |
| `v1.26.0` | M56 | DRY skip-on-missing-backend helper |
| (no tag) | M59 | Close stale PR #32 (M23 Phase 1/6) |
| `v1.27.0` | M60 | Resolve 2 TODO markers (Dockerfile OCI label + Python class methods) |
| `v1.28.0` | M57 | CONTRIBUTING.md (248 lines, manifest hygiene) |
| (no tag) | M58 | docs/specs/index.md (13 specs, audience-grouped) |
| `v1.29.0` | M61 | Cognitive policy tests (22 unit tests; 111 → 133) |
| `v1.30.0` | M69 | Arch-stack product roadmap convergence (ADR-038/039/040 + ROADMAP H0–H3) |
| `v1.31.0` | M70 | H0 executable bundle contract |
| `v1.32.0` | M71 | H1 durable workspace state |
| `v1.33.0` | M72 | Post-M71 debt paydown |
| `v1.34.0` | M73 | Archctl self lifecycle (multi-version + self-update + .arch-version) |
| `v1.35.0` | M75 | IDE adapters (OpenCode/ZCode/Claude Code/Codex) |
| `v1.36.0` | M76 | Plugin tap + assets-stack content + W3/W4 fixes (**H4 CLOSED**) |
| `v1.37.0` | M77 | Plugin download + extract with SHA256 verify + Homebrew formula + `archctl/permissions.yaml` bootstrap + `archctl stack` deprecation |
| `v1.37.1` | M77a | Claude Code + Codex `config_root` HOME-relative hotfix |
| `v1.37.2` | M77a | Lifecycle fix (`unsupported manifest_key`) + version bump |
| `v1.38.0` | M79 | `c4-discover` nested workspace: D1 build-dir blocklist pruning (7 directories); D2 nested manifest fallback for cargo/npm/npm_single/components strategies (`find_manifests` helper, depth ≤ 3) |
| `v1.38.1` | M80 | Cosmetic ChangeSet round-trip e2e coverage: `apply_with_matching_base_revision_succeeds` + `apply_round_trips_export_revision` integration tests in `diagram_apply.rs`; baseRevision optimistic-concurrency token verified end-to-end |
| `v1.39.0` | M81 | Projection schema v1.1 cosmetic fields: D1 `Command::MoveMember` preserves prior `ViewMember.label` (was resetting to empty string); D2 `Node` exposes `x`/`y`/`collapsed`/`labelOverride` (LEFT JOIN via `HashMap<element_id, &ViewMember>`, ADR-019 O(1)); archview renders `labelOverride ?? name`; backward-compat with 1.0 bundles; `cosmetic-changeset-roundtrip` spec promoted from stub to full |
| `v1.39.1` | M82 | `npm-single` pnpm-workspace-path detection: 1-argument swap in `archctl/src/code/strategies/npm_single.rs:67` so `pnpm_workspace_declares_packages` reads from `pkg_json.parent()` instead of `project_root`. Aligns with the sibling convention at `archctl/src/code/strategies/npm.rs:52`. Fixes mis-classification of vueuse-style monorepos (`apps/web/{package.json,pnpm-workspace.yaml}`). 1 regression test added. |
| `v1.40.0` | M83 | `archctl stack` removal: hard delete of the CLI surface (Command::Stack, StackAction, dispatch arm), the dead-code module (`archctl/src/stack.rs` 229 LOC), and the orphaned manifest gate (`manifests/stack.toml`). Migrated 3 e2e contracts (`e2e/install_e2e.sh`, `e2e/human_loop_sandbox.sh`, `e2e/HUMAN_LOOP_TEST.md`) + `docs/specs/e2e-installation.md` + the embedded `stack-management` skill to `archctl ide install <ide>` / `archctl ide doctor <ide>`. Breaking change — semver minor. ADRs untouched (historical truth preserved). |
| `v1.41.0` | M80b | Arrows export adapter: `archctl diagram export <selector> --format arrows` produces a deterministic `.arrows` JSON document (Arrows.app v0.8 shape). Pure serializer over `BundleEnvelope { projection, styles }` (`archctl/src/diagram/arrows.rs` 423 LOC, 7 inline unit tests). Case-insensitive `--format` dispatch wired via `ExportFormat::parse()` in `cli.rs::diagram_export_cmd`. Default output path derived from the selector (replaces `:` and `/` with `_`). `--json` envelope includes `unplaced_count` for cosmetic audit. 4 integration tests in `archctl/tests/diagram_arrows_export.rs`. Manifest gate updated (`editable[] += arrows.rs`, `must_hold[] += "pub fn serialize("`, `minimum_tests: 58 → 60`). M69 stub (`docs/specs/arrows-compatibility-adapter.md`) realigned to reflect export-only-public-surface; import marked as phase 2 deferred until a real consumer trigger fires. Path A-lite, no new ADR, no new port, no new bounded context. |
| `v1.41.6` | P0-12 | CI pre-merge fast gate workflow for PRs (`pr.yml`, PR #173) — Wave 0 item 6. |
| `v1.42.0` | p0-ladybug-doctor-v2 | `archctl doctor --scope storage [--json]`: LadybugDB (lbug) availability, crate/native alignment, schema initialization, and CRUD smoke probe. New `doctor/` module with `DoctorScope`, `LbugStorageProbe`, `NativeProbe`, and smoke gate runner. JSON output follows 5-axis envelope (ADR-048). Tier-1 CI smoke gate wired in `pr.yml` + release gate in `release.yml`. 9 integration tests covering all 7 spec scenarios (PR #174) — Wave 0 item 5. |
| `v1.43.0` | Wave 0 item 7 + Wave 1 batch | **p0-03** native release runners (darwin on `macos-13`/`macos-14`, linux aarch64 on `ubuntu-24.04-arm`, PR #177 — Wave 0 7/7 closed) + **p1-09** dep-fitness baseline + **p1-01** composition root (`CliContext` + factories) + **p1-03** repository ports (PRs #178–#180) — Wave 1 items 8/9/11. |
| `v1.44.0` | p1-04 | RawGraphQuery admin-only boundary (tokenized `is_read_only_query` guard, ADR-059) — Wave 1 item 13. |
| `v1.44.1` | p1-04 patch | RawGraphQuery admin-query guard follow-up (PR #182). |
| `v1.45.0` | p1-05 | UnitOfWork port + `Transaction<'a>` session newtype; 5 apply pipelines wrapped (PRs #184–#185). |
| `v1.47.0` | M32 PR2 | UNWIND bulk import extended to `state_machine` (3 nesting levels) + `c4_discover` writers; ADR-036 amendment (D2 re-ship, D3 deferral). PR #188. **Nota: PR1 (call_graph + class_diagram UNWIND, PR #187) quedó SIN tag — gap `v1.46.0` documentado; CHANGELOG `[1.46.0]` describe su contenido.** |
| `v1.47.1` | M32 remediation r1 | `class_diagram` version-id mismatch (blake3 compartido) + `apply_common` port bypass (batch helpers → `ElementRepository` port, ADR-059) fixed; cross-writer CURRENT_VERSION regression suite added (PR #189+#190). |
| `v1.48.0` | p1-08 | CapabilityRegistry: 8 categories, 79 entries; `archctl capabilities --format json\|markdown` + `--check`; bidirectional alignment invariants in `alignment.rs`; generated `docs/CAPABILITIES.md`; staleness gates in `verify-local.sh` + `test-ci-gates.sh`; ADR-045 accepted; call-graph schema enum fix 3→6 langs (PR #191) — Wave 1 items 15+16, Wave 1 completa. |
| `v1.49.0` | p2-01 | Snapshot metadata MVP: `RepositoryIdentity`, `extractor digest`, `SnapshotRepository`, `archctl architecture snapshot` CLI command (PR #194). |
| `v1.50.0` | p2-01 follow-up | Address 7 WARNINGs from `p2-01` snapshot MVP (PR #196) — warning surface cleanup. |
| `v1.51.0` | P2-02 | Architecture diff MVP: pure read-side diff projection (no lbug write), diff schema, `archctl diff` CLI surface. |
| `v1.52.0` | P2-03 | Explain / provenance MVP — answer "why does this element look this way?" with evidence walk. |
| `v1.53.0` | P2-04 | Coverage metrics MVP — confidence and evidence coverage across the graph. |
| `v1.54.0` | P2-05 | Policy metamodel MVP — closed rule set (6 rules per ADR-054): `forbid_dependency`, `require_dependency`, `forbid_cycle`, `max_fanout`, `evidence_required`, `confidence_min`; `archctl architecture policy check [--policy <file>] [--json] [--fail-on ...]`. |
| `v1.55.0` | P2-06 | Fitness evaluator — SARIF 2.1.0 + JUnit XML output formats; `--format {json,sarif,junit}` (existing `--json` preserved as deprecated alias). |
| `v1.56.0` | P2-07 | Context relevance engine MVP — `archctl architecture relevance` with deterministic scoring (exact-id/name/canonical_key/multi-token + BFS expansion × confidence decay, ASCII-fold + ES/EN stopword drop). |
| `v1.57.0` | P2-08 | Task context compiler MVP — `archctl architecture context --task "..." [--budget-tokens N] [--top N] [--json]` with truncation and dangling-relation closure. |
| `v1.58.0` | P2-09a | Observation / Claim compatibility carriers derived 1:1 from `EvidenceEntry`; `archctl architecture observe --version-id <VID>` read-only projection; preserves existing `Evidence` carrier contract. |
| `v1.59.0` | P2-10 | Intent vs Reality MVP — `archctl architecture intent check --intent <file>` with 4-class delta (DeclaredAndPresent / DeclaredButMissing / ObservedUndeclared / KindMismatch); TOML intent format; self-dogfood via `archctl-intent.toml` declaring 17 bounded contexts. |
| `v1.60.0` | fusion-engine-followups | Item 27 follow-ups: persistencia FusedClaims (migración v6), `ClaimEvaluator` (MaxMember + StalenessWeighted), `--persist`/`--evaluator`, explain 1.1 + coverage 1.1 (PR #214). |
| `v1.61.0` | item-28-strict-archbundle | `--profile strict` en `diagram export` (paths relativos, checksum SHA-256, `manifest.strict`) + archview read-only para strict bundles (Item 29) — ADR-055 reopened via ADR-061 (PRs #215–#222). |
| `v1.62.0` | item-27-residual | Fuse persiste por defecto (`--no-persist` opt-out) + `--expire-stale` GC + fix `parse_observed_at` (PR #223). |
| `v1.63.0` | adr055-phase2-secret-scanner | `redact.rs` zero-dep: AWS/GitHub/Slack/JWT/private-key/URL/credenciales genéricas → `[REDACTED:<kind>]` (PR #224). |
| `v1.64.0` | p2-09b-backfill-timestamp | Fix backfill v5 (filas pre-upgrade): normalizar `parse_observed_at` + wrap `timestamp()` (PR #225). |
| (docs) | changelog-formal | CHANGELOG secciones por release v1.60.0–v1.64.0 (PR #226). |
| `v1.65.0` | fuse-on-write | `recompute_fused_for_versions` persiste tras cada write de evidencia (seams `c4_discover`/`call_graph`) + limpieza de superseded (PR #227). |
| `v1.66.0` | fusion-params | `--cutoff-days` configurable + `StalenessWeightedEvaluator::new` + evaluador configurable en seam (PR #228). |
| `v1.67.0` | adr055-phase3-entropy | Detección por entropía Shannon (≥4.0 bits/char, len ≥32) + allowlist documentada (PR #229). **ADR-055 CERRADO** (fases 1–3). |
| `v1.68.0` | wave-3-workbench-ux | Workbench UX parcial (ADR-062, items 31–33): NavigationTarget + pila con breadcrumbs/back/forward, action palette (copy id, zoom C4, explain vía `GET /api/explain`, relations), semantic zoom Context↔Container↔Component por re-export. Strict bundles degradan. Fixes: flock flakiness diagram_export + version drift (PR #231). |
| `v1.69.0` | d2-deprecated-sweep | Barrido deprecated (deuda D2 auditoría): `diagram::queries` eliminado (13 call sites → `crate::graph`), `evidence::put`/`extract_with_system_clock` eliminados, manifests sync. Release pipeline reparado (PRs #235–#244) + self-update. |
| `v1.70.0` | uat-smoke-fixes | UAT multi-lenguaje (axum-rust + echo-go, sandbox Podman): `sanitize_identifier` para canonical keys con `@` (13 sitios), `batch_link_of_type` propaga errores, schema 1.1.1 (`EvidenceEntry.status`), prefixes go/java/kotlin/javascript en `parse_from_selector`, categoría `code` en relevance/coverage/explain. Harness: `bench/smoke-matrix.sh` + `bench/build-in-sandbox.sh` (PR #245). |
| `v1.71.0` | uat-vueuse-paths | Paths de evidence/source como DATA (UAT vueuse, PR #247): repos con `@` en rutas (snapshots npm scoped, patches) fallaban `call-graph --apply` con `write_source_artifact` error. Se reemplaza charset-validation por quote-escaping en 5 sitios de `store.rs`; vueuse aplica 1239 elementos / 13878 relaciones. |
| `v1.72.0` | uat-vueuse-pnpm | Detección de workspaces pnpm + sanitización de ids C4 (PR #249): NpmWorkspace parsea pnpm-workspace.yaml y expande globs `/*` (scoped + exclusiones `!`); components ignora dirs ocultos; ids `c4:container:@vueuse/core` sanitizados. Vueuse: 12 containers, export container:* = 12 nodes. |
| `v1.73.0` | uat-consistency-sprint | Sprint de consistencia post-UAT (PR #252): verify-local.sh ahora usa el binario real vía `CARGO_TARGET_DIR`/config.toml · bench/datasets.sh --populate-self-dogfood rsyncea el checkout local al cache · smoke-matrix.sh accept_cell falla con 0 evidences (gate no-vacuo) · e2e/human_loop_sandbox.sh Fase 9.2 path check correcto · c4_discover batch_link_of_type error incluye sample_id · doc state-machine corregido (rust/typescript/python, no kotlin). |
| `v1.74.0` | m17-workbench-redesign | Rediseño del workbench archview (4 PRs apilados #255→#256→#257→#258, sprint M17): Sprint A reescribe las 7 vistas (C4/CallGraph/ClassDiagram/Sequence/Impact/Package/Drift) sobre `@antv/g6 ^5.0.50` con dagre layouts, único punto de render y de decode; Sprint B introduce design system unificado (Inter Variable, OKLCH tokens con clamp+rem, light mode override, primitives Button/EmptyState/Tag reusables); Sprint C arregla el loader para bundles reales (EndpointIndex resuelve mismatch `:line-of-declaration` vs `:line-of-reference` del `gold.json`, wall-clock fallback para `loadedAt`); Sprint C2 colapsa el topbar saturado (7 sample buttons → `<select>`) y tokeniza G6 labels (`--g6-label-font-size: var(--fs-sm)` con `readCssVarNumber` para resolver `clamp()` correctamente). 160/160 tests, 0 lint errors, build OK. |
| `v1.75.0` | m18-c4-semantic-zoom | Semantic-zoom pill bar para C4 view (PR #260, M18): barra de pills sobre el canvas (`All levels` + 1 por nivel C4 con ≥1 nodo, badge de conteo) que filtra el visible set globalmente. Re-click o `All levels` toggle off. Persistencia en localStorage `archview.c4.lastLevel`. Helpers nuevos en `C4Graph.ts` (`nodesAtLevel`, `levelCounts`, `visibleNodesWithLevel`); UI con tokens del design system. Sample multi-nivel `c4-semantic-zoom.json` (3+4+5 nodes, 14 edges) entra en `SAMPLE_BUNDLES`. 173/173 tests, 0 lint errors, build OK. |
| `v1.76.0` | m19-elk-worker-layout | ELK layered layout en Web Worker (PR #262, M19): sustituye el dagre built-in de G6 por `elkjs 0.12` corriendo en worker via `workerUrl` (Vite `?url`). 4 archivos nuevos (`layout-presets.ts` con TB/LR/RL_LAYERED, `layout-client.ts` con `LayoutService` interface + `ElkLayoutService` real, `preset-layout.ts` con custom G5 v6 layout no-op) + 1 refactor de `g6.ts` (setData internamente async, DI seam `layoutService`, generation counter anti-race) + 7 vistas migradas (cada `layout: { type: "dagre", rankdir }` → `layoutOptions: TB_LAYERED | LR_LAYERED`). 184/184 tests pass (+11: 5 layout-presets + 6 layout-client), lint 0 errors, 7/7 vistas verificadas con Playwright. |
| `v1.78.0` | m21-g6-culling-lod | G6 viewport culling + zoom LOD (PR #266, M21): reduce overdraw en bundles 1000+ nodos. Dos capas: (1) Zoom LOD always-on — labels ocultos a zoom<0.5, edges a zoom<0.25 via `setElementVisibility` post-render; (2) Viewport culling opt-in — `CullingService` DI seam con `isInViewport` predicate, debounce 100ms en `wheel`/`drag-canvas:end`. `optimize-viewport-transform` behavior appended a G6 config (free FPS win para los 8 views). M18 orthogonality guard: C4View solo activa culling cuando `levelFilter === null` (sin pill activo). `c4-stress-1k.json` asset commiteado (1221 nodos / 3920 edges, hub con 500 incoming). 225/225 archview tests pass (+29: 26 CullingService unit + 3 C4View culling integration). Perf gate `bench/perf-cull.mjs` manual pre-PR; TTFP y FPS validados. Nota: `enableCulling` = false por defecto en CallGraph/Impact (gate-gated post-perf-gate). |
| `v1.79.0` | m22-sidebar-tabs | Sidebar tabs (evidence vs relations) con ARIA tablist (PR #1, M22): nuevo primitive `<TabBar>/<TabPanel>` en `components/primitives/Tabs.tsx` — automatic activation, ArrowRight/Left/Home/End keyboard nav, Space/Enter, badge, disabled. Sidebar.tsx integra el primitive con `activeTab` signal reset per node; Evidence panel conserva `<SourceDrawer>`. +14 tests (6 unit + 4 integration + 2 M20 compat + 2 sidebar-actions compat). 239/239 archview tests pass. Cierra el sprint M17.1 (último item: "Sidebar con tabs"). |
| `v1.80.0` | m23-perf-ci-gate | ADR-019 enforcement para archview (PRs #274–280, M23): post-merge CI job `perf-cull` en `ci.yml` — compara TTFP y FPS vs previous main con threshold 10%; new `scripts/bench-compare-archview.sh` (mirrors `bench-compare.sh` precedent); refactored `archview/bench/perf-cull.mjs` con JSON output + bug fixes (L47 hardcoded path → `__dirname`, L172 undefined timestamps → `window.__perfTimestamps[]`); +66 LOC contract tests en `scripts/test-ci-gates.sh` §11; ADR-019 §enforcement actualizado con implementation status per repo; archview/AGENTS.md "Perf budget enforcement (M23)" section added. Debt: lighthouse score gate (ADR-019 L65) y 10k+100k datasets out of scope. |
| `v1.81.0` | t0-trust-001-eventlog-reopen | **TRUST-001**: `EventLog::open` no longer truncates existing journal (cycle `t0-trust-001-eventlog-reopen`): `File::create` reemplazado por `OpenOptions::new().create(true).append(true)` per los precedents in-tree `cognitive/audit/log.rs:151-154` y `store.rs:961-967`. Adds 4 regression tests covering reopen-with-content, append-after-reopen, first-open-non-existent-path. Invariant: "Abrir journal existente nunca trunca" (spec-40:10). Preventive fix — `EventLog` solo consumido por tests hoy; `SyncDispatcher` lo cableará en T6 (REQ-P11). |
| `v1.82.0` | t0-trust-002-event-ids-causation | **TRUST-002**: `EventEnvelope` gains `eventId` (UUID v7 per RFC 9562), `correlationId`, `causationId`, `processed` (cycle `t0-trust-002-event-ids-causation`). Per-consumer checkpoint infra (`<log>.checkpoint.<id>.seq`). `EventLog::append` (7-arg) auto-assigns `eventId` and `timestamp`; `EventLog::consumer_checkpoint(id)` / `set_consumer_checkpoint(id, seq)` added; `append_serialized` preserves old API. Schema `event-envelope.schema.json` bumped 1.0 → 1.1 (legacy JSONL deserializes with `Uuid::nil()` + warning). 6 new regression tests + 17 total in `cognitive::event::tests`. Foundation for ADR-P11 causal journal; `SyncDispatcher` wiring deferred to T6. |
| `v1.83.0` | m25-authority-execution-classes | **Closes the first live breach of ADR-P02** (TRUST + Determinism + Authority typology + canonical-write gate). Three chained PRs (#287 docs +159, #288 code +738, #289 verify +407); diff `cbce2d3..d8c4a6a`. New `archctl/src/trust.rs` (`ExecutionClass`, `AuthorityClass`, `TrustClassification`, `classify()`, `canonical_write_allowed()`, `canonical_promotion_allowed()`, `TrustViolation`). ADR-063 accepted — hardens ADR-021 escalera into type-enforced invariant. New `archctl/tests/uat_06_false_agent_claim.rs` (2 active + 9 `#[ignore]` skeletons pending TRUST-005 + spec-35); UAT-06 critical gate `false_canonical_promotions == 0` green. `SourceOrigin` gains `ModelInference` variant (always `Suggested`; cannot transit to `Accepted` via `accept_evidence`). spec-30 bumped to v1.1. |
| `v1.84.0` | trust-005-observation-fusion | **TRUST-005**: epistemic plumbing gap closed. 5 chained PRs (#283 + apply + verify). New bounded contexts `archctl/src/feedback.rs` (`Feedback`, `FeedbackVerdict { Accept, Reject, Uncertain, Supersede, Correct }`, `FeedbackError`) and `archctl/src/reconciliation.rs` (`Reconciliation`, `PlaneEvidence`, pure `compute()`). New `archctl/src/fusion_bridge.rs` — trust-gated `recompute_status()` seam consumed by both `fuse_observations_with` and `FeedbackRepository::put_feedback`. `Observation` gains `evidence_origin`, `confidence`, `status: ObservationStatus`, `written_via_backfill`. v7 migration adds `(:Observation).status STRING`, `(:FusedClaim).pending_adjudication_event BOOLEAN`, `(:Feedback)` / `(:Reconciliation)` tables + typed edges. UAT-06 steps 7/9/13/14/15 un-ignored. `ModelInference × Suggested × Accept` lands as `"drafted"` via `trust::canonical_promotion_allowed`. ADR-064 accepted. |
| `v1.85.0` | trust-006-context-bundle | **TRUST-006**: UAT-06 steps 16/17/19/20 un-ignored (cycle `p-38e02210a9f14317/trust-006-context-bundle`). 2 chained PRs (#299 bundle verification, #300 AgentContext). New `FeedbackSummary` carrier (read-only view of `Feedback` excluding pipeline-internal fields). `AgentContext.feedback_history: Vec<FeedbackSummary>` additive with `#[serde(default)]`. New `cognitive::test_support` module (`test-fixtures` feature) — `FeedbackAwareMockAgent` + `MockOutcome` for deterministic tests. Bundle projection helpers `seed_bundle_fixture`, `assert_no_canonical_fact_in_bundle`, `assert_has_canonical_fact_in_bundle`. Fixture fix: `seed_orders_stripe_fixture` ahora stores `SourceOrigin::ModelInference.as_str()` (snake_case) — antes caía a `UserWorkspace` y enmascaraba el trust-first invariant. +442/-26 across 14 files; 843/843 tests; UAT-06 11/11 active. REQ-T06-003 deferred a TRUST-007. |
| `v1.86.0` | trust-007-feedback-port | **TRUST-007**: closes REQ-T06-003 deferred from TRUST-006 (cycle `p-38e02210a9f14317/trust-007-feedback-port`). 7 chained PRs (#303-#309). New `FeedbackRepository::summaries_for_claims(&[&str]) -> Result<Vec<FeedbackSummary>>` port method (no default impl, sealed-in-practice). `LbugStore` impl con single Cypher `MATCH (f:Feedback)-[:VERDICTS_ON]->(c:FusedClaim) WHERE c.id IN $claim_ids RETURN …` + deterministic ordering `(c.id ASC, f.revision ASC, f.timestamp ASC, f.id ASC)`; empty-input short-circuits. New `AgentContext::with_feedback_history` constructor. 8-site doc pass with `// REQ-T06-003: feedback_history plumbing` comment. SCN-T07-002b fix: validation loop dropped `let _ = …` → `?` propagation. `archctl/tests/feedback_summaries_port.rs` (4 tests: empty, ordering, exclusion, invalid-id). +431/-9 across 12 files; 846/846 tests. Trust-first invariant (ADR-P02) data-plane enforced. |
| `v1.87.0` | trust-008-m30-bridge-promotion | **TRUST-008**: **m30 bridge is now a hard fail** + Adjudication bounded context (closes REQ-M25-006, deferred from TRUST-005, named in TRUST-007 archive-manifest:79). 6 chained PRs (#312-#317) + verify-fixes (#318 squash `bfdd172`). New `archctl/src/adjudication.rs` BC: `AdjudicationEvent` (8 fields), `AdjudicationDecision` (Promote/Reject/Defer), `AdjudicationRepository` trait (3 methods) + `LbugStore` impl. `archctl/migrations/v8_adjudication_event_store.cypher` + `v9_fused_claim_evidence_origin.cypher` add `(:FusedClaim).evidence_origin STRING` column. `archctl adjudication { list --pending \| decide --claim --verdict --adjudicator --evidence-refs \| show --claim }` CLI. `AgentContext.pending_adjudications: Vec<AdjudicationEvent>`. New `promotion_requires_adjudication_event(trust, verdict) -> Result<(), TrustViolation>` predicate (fusion_bridge.rs:108) returns `Err(TrustViolation::ModelInferenceWithoutAdjudicationEvent)` for `ModelInference × Suggested + Accept`; `should_warn_pending_adjudication` marked `#[deprecated(since = "v1.87.0")]`. `put_evidence` reads `ev.source_origin.as_str()` (was hardcoded `'evidence_entry_derivation'`, regression carried from TRUST-007 verify). `manifests/adjudication.toml` (new, 44 LOC). 12 `trust::tests` (+2) + 7 `adjudication_events_port.rs` (+2) + 3 `migrations_v8.rs` (rewritten hook-direct). 1201/1201 tests pre-commit + 1204/1204 post no-stubs chain. +1500/-40 across 38 files. |
| `v1.87.1` | sprint-housekeeping-1 | **Sprint housekeeping v1.87.1**: 3 chained PRs. (1) `cli help` para `--lang` en `call_graph` corregido de "one of: rust, ts, py, go, java" a "one of: rust, ts, py, go, java, kotlin" (el código ya tenía 6 variants desde M36 v1.8.0; el help quedó stale). (2) Idem para `class_diagram` + `state_machine` (3 variants, antes decían 6). (3) Pre-push hook migration a `sddk verify` gate (eliminada per ADR-025; redundante con los gates SDDK + tax O(N) por push). 1210/1210 tests. |
| `v1.87.2` | m84-ide-install-root | **M84**: `install_stack` en `IdeAdapter` trait acepta `install_root: Option<&Path>` opcional (4 adapters: OpenCode/ZCode/Claude Code/Codex). Cierra gate donde `archctl ide install <ide>` aceptaba `--install-root` pero el trait no lo propagaba — argumento silenciosamente ignorado. **Cambio de API pública additive** (trait signature change, semver patch); M77a precedent (v1.37.2). CHANGELOG `[1.87.2]` §Changed documenta el additive derive (3 `OutputKind` + `Severity` + `ProposalStatus` + `ApprovalDecision` + `CurrencyUnit` enums) justificado por tests downstream + CHANGELOG `[1.87.2]` §Validation documenta `--lang` help audit + 5e environment hardening. 1245/1245 tests pre-derive + 1245/1245 post. |
| `v1.87.3` | cognitive-layer-coverage-v1 | **Cognitive-layer coverage v1**: 4 chained commits cierran los 2 huecos más profundos del audit M61 (cognitive policy tests) sobre `archctl/src/cognitive/`. (1) `cognitive/mcp/tools.rs` +5 tests — `ToolResult` roundtrip, schema validation paths, default deserialization (completado pre-v1.87.3). (2) `cognitive/mcp/gateway.rs` +11 tests — `McpError` Display contract (5 variants), `McpGateway::default` ≡ `McpGateway::new`, `PolicyGate::default` ≡ `PolicyGate::new`, `PolicyGate::queue()` accessor observa Queue outcomes, `handle_governed` en los 4 paths (Allow/Queue/Deny/ParseError/ToolNotAllowed/MissingProposal). (3) `cognitive/dispatcher/event_dispatcher.rs` +8 tests — `log_seq()` initial 0, `SerializedEvent::from_envelope` `processed=false` default, dispatch con registry vacía, dispatch con Hypothesis output (vs NoAction), seq monotonic + consumer_checkpoint, partial fan-out preserva registration order, erroring observer no rompe el resto, seq sobrevive EventDispatcher reopen. **Cycle summary**: 1279 → 1298 tests; ratios más profundos `gateway.rs` 1.8% → 3.4%, `event_dispatcher.rs` 1.7% → 3.3%. Bonus: 3 `--lang` help string fixes (call_graph 5→6 langs, class_diagram/state_machine 6→3) + `archview/package.json` 1.87.1 → 1.87.3 (lock-step convention established). Sin ADR, sin migration, sin port changes. |
| `v1.88.0` | m34-cognitive-context-compression | **M34 cycle CLOSED end-to-end**: integration spec (`docs/specs/spec-M34-cognitive-context-compression.md`), implementation W1–W7, archive (PR #319, merge commit `a1b6eb6`). Foundation for future context-window-aware cognitive processing; `archctl/src/cognitive/context.rs` exposes `Compress`, `compress_with_strategy`, `CompressionLedger`, `ContextSnapshot`, `ContextView` — wired into `dispatcher/event_dispatcher.rs` M34 W3 with read-only split between dispatch `EventLog` and compression `CompressionLedger`. 1228 tests pre-commit, all green. M34 HIGH debt identified at closure: `DecisionPriority` had 3 stub variants with 0 callers (Anti-roadmap §"no stubs without productive use" violation). |
| `v1.88.1` | m34-high-debt-decision-priority | **M34 HIGH debt closed**: `DecisionPriority` collapsed to `#[non_exhaustive] + RecencyOnly` (1 file +8/-8, PR #320 squash-merged). Removed stub variants `ActionProposalOnly` + `Balanced` had 0 callers (`grep` clean). `archctl/src/cognitive/context.rs:DecisionPriority` is now a non-exhaustive single-variant enum ready for future strategy addition without API breakage. Tests untouched. 11 MEDIUM + 1 LOW items remain in M34 backlog (cycle-scoped debt, not blocking). |
| `v1.89.0` | cognitive-layer-coverage-v2 | **Cognitive-layer coverage v2**: 3 chained PRs (#321 #322 #323, ~1216 LOC total, +26 tests). Locks the compression+policy-gate integration paths M34 wired in v1.88.0. (1) PR #321: `archctl/src/cognitive/context.rs` +10 tests — compression edge cases (`compress_with_budget_tokens_zero_bails`, `compress_with_no_events_returns_empty_view`, `compression_ledger_records_compressed_at_and_count`, `compression_strategy_recency_keeps_recent_events_first`, `compression_cycle_invariant_view_size_does_not_grow`, `compress_then_query_via_recent_events_returns_only_compressed_set`, `compression_ledger_entry_count_matches_compress_invocations`, `compression_ledger_new_is_empty`, `compression_strategy_recency_respects_token_budget_strictly`, `compression_repeated_calls_yield_consistent_view_for_identical_input`) — context.rs tests 27→37. (2) PR #322: `archctl/src/cognitive/dispatcher/event_dispatcher.rs` +8 tests — compression paths in the dispatch fan-out surface (`dispatch_zero_tokens_compression_bails_but_fan_out_continues`, `dispatch_with_empty_compression_ledger_populates_empty_recent_events`, `dispatch_log_seq_monotonic_across_compression_cycles`, `dispatch_with_compression_log_reads_only_from_compression_ledger`, `dispatch_with_compression_log_does_not_write_to_compression_ledger`, `dispatch_fan_out_preserves_registration_order_with_compression`, `dispatch_erroring_observer_does_not_break_others_with_compression`, `dispatch_with_compression_log_but_no_budget_does_not_read_compression_ledger`) — event_dispatcher.rs tests 22→30. (3) PR #323: `archctl/src/cognitive/mcp/gateway.rs` +8 tests — `PolicyGate` integration paths (`gateway_policy_gate_audit_logger_grows_per_check`, `gateway_policy_gate_queue_accumulates_distinct_proposal_ids`, `gateway_graph_query_with_malformed_args_returns_error_response`, `gateway_schema_validate_with_malformed_args_returns_error_response`, `gateway_two_policy_gates_have_independent_queues`, `gateway_handle_raw_always_returns_valid_json_for_error_cases`, `gateway_handle_governed_unknown_tool_returns_tool_not_allowed_error`, `gateway_handle_governed_missing_proposal_field_returns_parse_error`) — gateway.rs tests 23→31. **Cycle summary**: gateway.rs 3.4% → 6.3%, event_dispatcher.rs 3.3% → 5.9%, context.rs coverage extended on compression surface (no ratio reported — ratio baseline v1.87.3 already covered compression dispatch path). All gates green (`cargo build`, `cargo test --features test-fixtures --tests`, `cargo clippy --features test-fixtures --all-targets -- -D warnings`, `cargo fmt --check`, `archctl doctor --scopes cognitive`). Sin ADR, sin migration, sin port changes — pure test coverage deepening. |

## Capacidades shipped (v0.x — historical)

| Tag range | Período | Highlights |
|---|---|---|
| `v0.1.0` – `v0.3.0` | 2026-07-30 | LadybugDB graph, evidence, renderers, source-evaluation types |
| `v0.4.0` – `v0.6.0` | 2026-07-30 | Manifest gates (23 scopes), schema migration runner (ADR-017) |
| `v0.7.0` | 2026-07-31 | Diagram export pipeline changes |
| `v0.8.0` / `v0.8.1` | 2026-07-31 | `archctl code call-graph` (PR1) |
| `v0.9.0` | 2026-07-31 | `archctl code sequence` (PR2) |
| `v0.10.0` | 2026-08-01 | M20 bench suite (criterion harness) |
| `v0.11.0` | 2026-08-01 | m9-renderers-local (F1 security) |
| `v0.12.0` – `v0.13.1` | 2026-08-01 / 02 | M12 class-diagram extraction + refactors |
| `v0.14.8` – `v0.14.10` | 2026-08-05 | M26 C4 contract integrity + vertical validation |
| `v0.22.0` | 2026-08-06 | M27 sandbox + benchmarks |

## Capacidades en backlog (priority order)

| Item | Status | Notes |
|---|---|---|
| **M56–M68** (11 proposals from M55 study) | See `docs/sessions/2026-08-07-codebase-state-study.md` | Recommended trio: M56 (done) + M59 (done) + M62 (this file) |
| call_graph → prepare/execute migration (M51 deferred) | Deferred — needs typed bindings or CAST | 2-5x perf on top of M32 D1+D2 |
| archview M18 (reactive runtime) | Deferred — ver anti-roadmap (ADR-039) | Reopen trigger: ≥2 third-party consumers |
| archview M19 (wgpu renderer) | Deferred — ver anti-roadmap (ADR-039) | Reopen trigger: benchmark p99 fails ADR-019 budget |
| SparrowDB adapter (ADR-014) | Optional — no SparrowStore exists | port is ready |
| M13/M14/M15 (workbench actions, versionado, semantic tools) | Defer to 1.x (per ROADMAP) | not enterprise target |
| M23 (Action Proposal & Policy Engine) | Deferred — ver ADR-040 | Reactivation: real HITL workflow required |
| LSP-based extraction (ADR-012 follow-up) | Deferred to phase 2 M12 | |

## Anti-roadmap (deferred decisions con reopen triggers)

> Ver [ADR-039](adr/ADR-039-renderer-reality-anti-roadmap.md) §anti-roadmap para el detalle completo.

| Decisión | Status | Reopen trigger |
|---|---|---|
| WGPU renderer | Deferred | Benchmark p99 > ADR-019 budget AND JS/Worker insufficient |
| Rust/WASM compute layer | Deferred | ≥2 third-party consumers needing shared compute |
| Apache Arrow | Deferred | Bundle size >10MB AND JSON parsing bottleneck measured |
| cosmos.gl (>100k nodos) | Deferred | Node count >100k AND G6 canvas FPS <30 |
| [ADR-051](adr/ADR-051-loopback-workbench-session-security.md) loopback workbench session security | Deferred (ADR-051 accepted with Deferido 2026-08-18) | ≥1 disclosed loopback-session hijack vector (CVE or reproducible PoC) OR feature parity claim requiring per-session permission scoping |
| [ADR-056](adr/ADR-056-moldable-architecture-workbench.md) moldable architecture workbench (LensSpec) | **Aceptado (parcial)** — 2026-08-19 via ADR-062: items 31–33 shipped (v1.68.0); P3-05 sigue deferida | ≥2 consumers with LensSpec-translatable duplication OR a measured need (≥3 users reporting the same lens problem OR perf p99 breach traceable to view-strategy variance) |
| SceneGraph abstraction | Deferred | ≥3 view types need shared scene model |
| WIT Plugin SDK | Deferred | ≥1 third-party consumer registered |
| Event sourcing/replay | Deferred | Temporal diff is shipped requirement |
| Architecture Lab (forks) | Deferred | ≥3 user requests |
| Full 9-agent catalog | Deferred | 2/2 deployed agents >50% adoption |
| Desktop shell (Tauri) | Deferred | Browser-only is blocker for ≥1 user segment |

## Deuda técnica activa

**Doctor:** 34/34 scopes pass. No findings.

**Pending**:
- **`archctl/src/store.rs`** (5,435 LOC @ v1.87.3; +54 % desde el cierre M54): módulo más grande del crate. M63 propuso splitearlo; el trabajo se mantiene diferido porque (a) el coste de refactor supera al valor en este momento y (b) los tests de apply ya pasan al suite completo sin que el tamaño impacte el perf budget de export.
- **Cognitive-layer test coverage** (`archctl/src/cognitive/`, 14 sub-módulos): v1.89.0 cerró la fase v2: `mcp/gateway.rs` 3.4% → 6.3%, `dispatcher/event_dispatcher.rs` 3.3% → 5.9%, `context.rs` extendida sobre compression surface. Cycle summary: 1298 tests (v1.87.3) → 1298 + 26 (cognitive-coverage-v2) = 1324 tests @ v1.89.0. Próximos candidatos con ratio < 6% restantes mapeados pero sin trigger formalizado — siguiente fase cognitive-coverage-v3.

> **Removed by drift** (resumidos para auditoría; los detalles viven en el CHANGELOG bajo las versiones citadas — ya no aportan al lector actual): M37–M56 deuda cerrada in-session · 2 obsolete `feat/p1-04-raw-graph-query-boundary` stashes dropped (2026-08-18) · M32 remediation cerrada en v1.47.0/v1.47.1 · "POSIX-only symlink" debt removida (apuntaba a `stack.rs` eliminado en v1.40.0/M83).

## Plan vigente

**T0 Trust cerrado end-to-end** (v1.81.0 → v1.87.0): 8 ciclos encadenados cubren desde el reopening trivial de `EventLog` (TRUST-001) hasta la promoción `Err(TrustViolation)` del m30 bridge (TRUST-008). El horizonte T0 ya no genera nuevos tickets; el `SyncDispatcher` wires en T6 queda como ticket de backlog dentro de T3 (Live Revision Loop, ADR-P11). Historia completa y decisiones locked por ciclo en `sddk/p-38e02210a9f14317/trust-00X/` y en el CHANGELOG bajo cada `[1.8X.0]` header.

**Wave 0/1/2 cerrado, Wave 3 parcial** (catálogo `docs/arch-stack-proposals-2026-08-13/09-IMPLEMENTATION-PR-PLAN.md`):

- **Wave 0 (remediation)** — 7/7 DONE (v1.42.0–v1.43.0).
- **Wave 1 (architecture scaffolding)** — 8–16 ALL DONE (v1.43.0–v1.48.0).
- **Wave 2 (intelligence)** — 10/10 DONE (v1.49.0–v1.59.0, gated under `archctl architecture …`).
- **Wave 3 (platform)** — parcial: items 19 (P2-09b), 22 (ide doctor), 27 (fusion engine), 28+29 (strict ArchBundle + archview read-only), 31–33 (workbench UX: NavigationTarget, action palette, semantic zoom C4) **+ el sprint M17.1 (M21 G6 culling/LOD v1.78.0, M22 sidebar-tabs v1.79.0, M23 ADR-019 perf-ci-gate v1.80.0)** CERRADOS v1.60.0–v1.80.0. Restantes: item 30 (session token, gated por ADR-051) y item 34 (P3-05 lens recommendation, gated por ADR-056/062).

**Próximo candidato natural** (ROADMAP + STATE §Anti-roadmap):
- **M35.1** (debt cleanup de `cognitive/scoring.rs`) — 4 findings P2 + test de precedencia del cierre M35 (v1.90.0); backlog detallado en `sddk/m35-severity-scoring-pipeline/archive-manifest.md`.
- **cognitive-layer-coverage-v3** — siguiente fase del audit M61; módulos con ratio < 6% restantes identificados en el cierre de v1.89.0.
- **Bump lbug 0.18.3 → 0.19.1** (deferido; ver pendientes menores).

> **M35 cerrado** (v1.90.0, 2026-08-24): `severity_for()` función pura en `archctl/src/cognitive/scoring.rs` cableada en `ArchitectureAgent`; bins discretos ≥0.9/≥0.7/≥0.4 con overrides; verify PASS 23/23 REQs; debt PASS_WITH_WARNINGS. Detalle del ciclo en ROADMAP §`m35-severity-scoring-pipeline`. Nota: la etiqueta M35 colisiona con el milestone histórico del vault (`M35-java-call-graph`, v1.7.0).

## Comandos de verificación

```bash
# Estado del repo
git log --oneline -10
git tag --sort=-creatordate | head -5
git status

# Tests
cd archctl
cargo build --quiet
cargo test --features test-fixtures --tests   # 1298/1298 (lib 953 + integration 345 + doctest)
cargo clippy --quiet --all-targets --features test-fixtures -- -D warnings
cargo fmt --check

# Verificación completa (cheap mode)
cd /var/mnt/DiscoChino2-fast/Proyectos/agentesIA/arch-stack
bash scripts/verify-local.sh

# 34/34 doctor scopes
cargo run --bin archctl -- doctor --scopes $(ls ../manifests | sed 's/.toml//' | tr '\n' ',' | sed 's/,$//') --cwd ..

# Capability registry staleness
cargo run --quiet --bin archctl -- capabilities --check
```

## Próxima acción del usuario

**T0 Trust — cerrado end-to-end** (v1.81.0 → v1.87.0): TRUST-001..008 todos shipped; horizonte deja de generar tickets. Próximos pasos viven en los horizontes T1–T11 del paquete `docs/arch-stack-architecture-feedback-workbench-2026-08-20/` y en M34/M35 del ROADMAP canónico.

Wave 3 parcial cerrado (v1.60.0 → v1.80.0): items 19, 22, 27, 28+29, 31–33 + sprint M17.1 (M21/M22/M23) — todos taggeados. Restantes: **item 30** (session token, gated por ADR-051) y **item 34** (P3-05 lens recommendation, gated por ADR-056/062). Detalle completo en `docs/arch-stack-proposals-2026-08-13/`.

Pendientes menores out-of-scope:
- Report de redacciones en strict bundles (XDG).
- Persistir cutoff de staleness por proyecto (XDG).
- **Bump lbug 0.18.3 → 0.19.1** (decisión 2026-08-22): workaround en `archctl/src/migrations.rs:344-350` gana; bump deferido a ciclo de mantenimiento futuro. ~10 líneas economizadas no compensan el riesgo de regresión sobre los 1298 tests.

Candidatos post-T0 (reopen triggers en `docs/STATE.md` §Anti-roadmap):
- **ADR-051** (loopback session security) — abre solo con hijack vector disclosed.
- **B2/B3 ADR-016** (activegraph packs) — pendientes con sus propios triggers.