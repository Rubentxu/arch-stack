# Estado de `arch-stack`

> Snapshot del estado real del repo. Refreshed al cierre de cada ciclo
> para reflejar la verdad del código, no la planificación aspiracional.
> Última actualización: 2026-08-20, post-release v1.82.0 (TRUST-002
> shipped: event IDs + causation/correlation + per-consumer checkpoint infra;
> v1.81.0 was TRUST-001 EventLog reopen fix).

## Estado del trunk

| Field | Value |
|---|---|
| Branch principal | `main` |
| Tip | `70c8fbf` (PRs #274–280 squash — M23 perf-ci-gate) |
| Versión | `v1.80.0` (latest tag, M23 perf-ci-gate) |
| Tests | baseline `872 @ v1.48.0`; último full-suite `1074 @ fusion-engine-followups` (v1.60.0); archview `239 @ m23-perf-ci-gate`; re-cuenta en cada verify; clippy clean |
| Working tree | clean (`main`); tags v1.65.0–v1.80.0 verificados en origin |
| MSRV | `1.91` (`rust-version` en `archctl/Cargo.toml`); CI pin `1.97.1` |
| LOC src | 52,576 (`wc -l` sobre `archctl/src/**/*.rs` @ v1.67.0; incluye unit tests inline) |
| LOC tests | 15,613 (`archctl/tests/**/*.rs`, integration) |
| LOC benches | 903 (`archctl/benches/**/*.rs`) |
| Vault milestones | 37 + Wave 2 (v1.49.0–v1.59.0) + Wave 3 parcial (items 19/22/27/28+29, v1.60.0–v1.67.0) + M21 culling-lod + M23 perf-ci-gate — ciclos archivados en `~/.sddk-knowledge/arch-stack/changes/` |
| Tags | 152 (v0.1.0 → v1.82.0; gap `v1.46.0` never tagged — see v1.47.0 nota) |

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

**Doctor:** 30/30 scopes pass. No findings.

**Closed in this session** (M37–M56):
- `seed_writes` lying API removed (BREAK-1, v1.4.1)
- Mermaid projector bare-Label bug fixed across 4 views (M39 + M41)
- C4 PlantUML Structurizr-style emit bug (M50)
- 26 stale manifest `public_symbols` removed (M46)
- 3 stale "no parameter binding" doc claims (M52)
- `backend_available()` helper DRY'd (M56, -60 LOC across 5 files)

**Closed in this housekeeping pass** (2026-08-18, post-v1.59.0):
- **2 obsolete `feat/p1-04-raw-graph-query-boundary` stashes dropped** (WIP from the
  pre-#181 branch). Their bases (`0b75778`, `63e2200`) are reachable from `main`
  and the work was already merged via PR #181, then reshaped by `58f5150`
  (P1-05 RawGraphQuery supertrait), `2731800` (move helpers into
  `ElementRepository` port), and `24e2eb8`/`3ab707c` (M32 D2 UNWIND bulk
  import). Re-applying on a fresh branch from `main` produced conflicts in
  12 files; every conflict was "both sides did the same change, `main` is the
  cleaner final form". Drop is intentional and irreversible — rederivation
  lives in those cited commits.

**M32 remediation (closed in v1.47.0/v1.47.1)**:
- class_diagram UUID mismatch fixed; port bypass corrected; cross-writer `CURRENT_VERSION` regression suite added.

**Accepted debt (per proposal + cycle retrospective)**:
- **Registry introspection v2**: registry sources are catalog-mirrors, not runtime introspection. Follow-up deferred to registry-introspection-v2 proposal.
- **Parallel `Vec<Element>` + `ElementVersion`** (~120 LOC, M32 era): `apply.rs` processes elements in parallel; noted as debt in p1-08 retrospective.
- **D4 throughput**: measured 96.57 ms/element vs ≤30 ms/element budget (ADR-019); accepted as small-N edge case.
- *Removed (2026-08-18, post-v1.59.0 refresh)*: "POSIX-only symlink: stack.rs-era
  symlink bootstrap" — `archctl/src/stack.rs` was deleted in v1.40.0 (commit
  `c2d65c3 refactor(cli): remove deprecated stack subcommand`, M83); the debt
  pointed to a file that no longer exists.

**Pending**:
- **store.rs** (3,540 LOC): biggest file. M63 proposes splitting (still pending).
- **Cognitive-layer test coverage**: 14 sub-modules, minimal tests. M61 audit partially done (cognitive policy tests added), full coverage deferred.

## Plan vigente

**Marathon session closed at M54** (v1.24.0). Post-session work:
- M55 (v1.25.0): state study + 11 proposals (M56–M68) — DONE
- M56 (v1.26.0): DRY helper — DONE
- M59: close stale PR #32 — DONE
- M60 (v1.27.0): resolve 2 TODO markers — DONE
- M57 (v1.28.0): CONTRIBUTING.md — DONE
- M58 (no tag): docs/specs/index.md — DONE
- M61 (v1.29.0): cognitive policy tests — DONE
- M62 (no tag): STATE.md refresh (was this cycle) — DONE
- M69 (v1.30.0): arch-stack product roadmap convergence — DONE
- **t0-trust-001-eventlog-reopen** (TRUST-001, v1.81.0): shipped. Latent bug closed
  preventively (EventLog only used in tests today; `SyncDispatcher` wires it in T6).
- **t0-trust-002-event-ids-causation** (TRUST-002, v1.82.0): shipped. EventEnvelope
  extended with eventId (UUID v7) + correlationId/causationId + per-consumer checkpoint
  infra. Foundation for causal journal (ADR-P11). `SyncDispatcher` wiring deferred to T6.
  Next: TRUST-003 (AuthorityClass/ExecutionClass mapping) or TRUST-006 (T6 SyncDispatcher).

Trio tidy-up (M56 ✅, M59 ✅, M62 ✅) y la saga M55–M69 están cerradas.

**2026-08-13 plan (Wave 0/1/2/3)** — ver
`docs/arch-stack-proposals-2026-08-13/09-IMPLEMENTATION-PR-PLAN.md`.

Estado de la wave:

1. **Wave 0 (remediation)** — **7/7 DONE** (PRs #168–#175 + #177): plugin tests,
   plugin hardening, ADR integrity, license coherence, Ladybug doctor
   (v1.42.0), PR CI fast gates, **native release runners (v1.43.0)**.
2. **Wave 1 (architecture scaffolding)** — **8–16 ALL DONE** (v1.43.0–v1.45.0
   + v1.48.0): dependency-fitness baseline (v1.43.0, p1-09), composition
   root (v1.43.0, p1-01), repositories (v1.43.0, p1-03), RawGraphQuery
   boundary (v1.44.0+v1.44.1, p1-04), UnitOfWork (v1.45.0, p1-05),
   filesystem contracts, doctor/diagram migrations via CliContext, **capability
   registry (v1.48.0, p1-08, PR #191)**.
3. **Wave 2 (intelligence)** — **10/10 DONE** (v1.49.0–v1.59.0): snapshot
   metadata (P2-01), diff (P2-02), explain (P2-03), coverage (P2-04),
   policy metamodel (P2-05), fitness evaluator (P2-06), context relevance
   (P2-07), task context (P2-08), observation/claim carriers (P2-09a),
   intent vs reality (P2-10). All under `archctl architecture ...`.
4. **Wave 3 (platform)** — parcial: items 19 (P2-09b), 22 (ide doctor), 27 (fusion engine), 28+29 (strict ArchBundle + archview read-only) y 31–33 (workbench UX: NavigationTarget, action palette, semantic zoom C4) CERRADOS v1.60.0–v1.68.0; restantes: item 30 (session token, gated por ADR-051) y item 34 (lens recommendation, P3-05, gated por ADR-056/062).

## Comandos de verificación

```bash
# Estado del repo
git log --oneline -10
git tag --sort=-creatordate | head -5
git status

# Tests
cd archctl
cargo build --quiet
cargo test --features test-fixtures --quiet   # 872/872 baseline
cargo clippy --quiet --all-targets --features test-fixtures -- -D warnings
cargo fmt --check

# Verificación completa (cheap mode)
cd /var/mnt/DiscoChino2-fast/Proyectos/agentesIA/arch-stack
bash scripts/verify-local.sh

# 30/30 doctor scopes
cargo run --bin archctl -- doctor --scopes $(ls ../manifests | sed 's/.toml//' | tr '\n' ',' | sed 's/,$//') --cwd ..

# Capability registry staleness
cargo run --quiet --bin archctl -- capabilities --check
```

## Próxima acción del usuario

**T0 Trust — progress:**
- ✅ **TRUST-001 EventLog reopen** (cycle `t0-trust-001-eventlog-reopen`, v1.81.0): shipped. Latent bug closed preventively.
- ✅ **TRUST-002 Event IDs + causation/correlation** (cycle `t0-trust-002-event-ids-causation`, v1.82.0): shipped. Causation infra ✅; `SyncDispatcher` wires in T6.
- 🎯 **Próximo PR sugerido**: TRUST-006 (T6 SyncDispatcher → EventLog wiring) or TRUST-003 (AuthorityClass/ExecutionClass mapping).

Wave 3 parcial CERRADO: Items 19 (P2-09b), 22 (ide doctor), 27 (fusion
engine), 28+29 (strict ArchBundle + archview read-only) y **31–33
(workbench UX, v1.68.0)** — v1.60.0 a v1.68.0, todos taggeados y
verificados en origin.

**Workbench UX parcial (ADR-062)**: cross-view identity (NavigationTarget
sobre IDs canónicos), action palette (copy id, zoom, explain vía
`/api/explain`, relations), semantic zoom C4 (Context↔Container↔Component
por re-export). Strict bundles degradan explain.

Restante Wave 3 (catálogo `docs/arch-stack-proposals-2026-08-13/`):
- **Item 30 (session token)** — gated por ADR-051 (hijack vector
  disclosed).
- **Item 34 (lens recommendation)** — P3-05, XL, gated por ADR-056/062
  (≥2 consumers OR measured need).
- **Nivel "Code" (C4→class-diagram)** — reopen trigger propio (ADR-062):
  ≥1 consumidor con necesidad real.
- **ELK layout + virtualización >1k nodos** (M17.1 opcionales) — solo si
  el zoom los exige.

Candidatos futuros (reopen triggers documentados en Anti-roadmap):
- **ADR-051 loopback session security** — abre solo con hijack vector
  disclosed.
- **B2/B3 ADR-016** — pendientes con reopen triggers en
  `docs/adr/ADR-016-activegraph-packs-investigacion.md`.

Pendientes menores out-of-scope:
- Report de redacciones en strict bundles.
- Persistir cutoff de staleness por proyecto (XDG).
- Bump lbug (implicit cast STRING→TIMESTAMP o workaround documentado).

Sigue válido el catálogo de items en
`docs/arch-stack-proposals-2026-08-13/`.