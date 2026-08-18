# Estado de `arch-stack`

> Snapshot del estado real del repo. Refreshed al cierre de cada ciclo
> para reflejar la verdad del código, no la planificación aspiracional.
> Última actualización: 2026-08-18, post-release v1.59.0 (P2-10 intent vs reality MVP)
> + housekeeping `ed5b6cb` (drop 2 obsolete P1-04 stashes).

## Estado del trunk

| Field | Value |
|---|---|
| Branch principal | `main` |
| Tip | `ed5b6cb` (housekeeping post-v1.59.0; latest release merge = `313f18b` for v1.59.0) |
| Versión | `v1.59.0` (latest tag, P2-10) |
| Tests | baseline `872 @ v1.48.0`; current `cargo test --features test-fixtures --quiet` re-cuenta en cada verify; clippy clean |
| Working tree | clean (`main`, ahead of `origin/main` by housekeeping `ed5b6cb`) |
| MSRV | `1.91` (`rust-version` en `archctl/Cargo.toml`); CI pin `1.97.1` |
| LOC src | 49,200 (`wc -l` sobre `archctl/src/**/*.rs` @ v1.59.0; incluye unit tests inline) |
| LOC tests | 14,348 (`archctl/tests/**/*.rs`, integration) |
| LOC benches | 903 (`archctl/benches/**/*.rs`) |
| Vault milestones | 37 + Wave 2 (v1.49.0–v1.59.0, P2-01 → P2-10) — en `sddk/p-38e02210a9f14317/p2-*` |
| Tags | 130 (v0.1.0 → v1.59.0; gap `v1.46.0` never tagged — see v1.47.0 nota) |

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
| [ADR-055](adr/ADR-055-sanitized-architecture-bundle.md) sanitized architecture bundle | **Abierto** (reopened 2026-08-18 por ADR-061) | Trigger actualizado: ≥1 stakeholder (interno o externo) necesitando compartir sin código fuente |
| [ADR-056](adr/ADR-056-moldable-architecture-workbench.md) moldable architecture workbench (LensSpec) | Deferred (ADR-056 accepted with Deferido 2026-08-18; canonical anchor of ROADMAP §H3) | ≥2 consumers with LensSpec-translatable duplication OR a measured need (≥3 users reporting the same lens problem OR perf p99 breach traceable to view-strategy variance) |
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
4. **Wave 3 (platform)** — pendiente (planning needed).

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

Wave 3 completo: Items 19 (P2-09b), 22 (ide doctor), 27 (fusion engine)
cerrados en `main` (PRs #211/#212/#213, commits `667a706`/`098a96d`/`3365abd`).
El ciclo `fusion-engine-followups` (PR #214) cierra el loop del Item 27:
persistencia de fused claims (migración v6 + `--persist`), evaluadores
pluggables (`--evaluator max-member|staleness-weighted`) y surfacing en
`architecture explain` (1.1) + `architecture coverage` (byFusedClaims, 1.1).

**Item 28 (strict ArchBundle) CERRADO** (v1.61.0, 2026-08-18): `--profile strict`
en `diagram export` (paths relativos, checksum SHA-256, manifest.strict) +
archview read-only para strict bundles. ADR-055 sigue OPEN para fase 2
(pseudonymization, anti-secretos scanner).

**Item 27 residual CERRADO** (v1.62.0 pendiente de tag): `architecture fuse`
persiste por defecto (`--no-persist` para stdout-only) + `--expire-stale
[--dry-run]` para GC de claims stale. Fix incluido: parseo del formato de
timestamp de LadybugDB en `parse_observed_at` (sin él, todo claim persistido
se marcaba stale).

**P2-09b backfill timestamp CERRADO** (v1.64.0 pendiente de tag): el backfill
v5 saltaba filas pre-upgrade (lbug no hace implicit cast STRING→TIMESTAMP).
Fix: `parse_observed_at` + wrap `timestamp()` en columnas TIMESTAMP
(`written_at`), literal en columnas STRING (`observed_at`). Test de
regresión con filas pre-upgrade. Eliminado el helper muerto
`iso_to_lbug_timestamp`.

Candidatos futuros (reopen triggers documentados en Anti-roadmap):
- **ADR-051 loopback session security** — abre solo con hijack vector
  disclosed.
- **ADR-055 sanitized architecture bundle** — **ABIERTO** (reopened 2026-08-18
  por ADR-061). Fase 2: pseudonymization de filenames + scanner anti-secretos.
- **ADR-056 moldable architecture workbench (LensSpec)** — entry criteria
  explícitos en `docs/ROADMAP.md` §H3; ≥2 consumers OR measured need.
- **Fuse-on-write** — persistir fused claims automáticamente desde
  `put_evidence` (best-effort, ADR-049 D4 style); cutoff configurable.

Sigue válido el catálogo de items en
`docs/arch-stack-proposals-2026-08-13/`.