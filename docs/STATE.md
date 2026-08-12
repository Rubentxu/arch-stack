# Estado de `arch-stack`

> Snapshot del estado real del repo. Refreshed al cierre de cada ciclo
> para reflejar la verdad del código, no la planificación aspiracional.
> Última actualización: 2026-08-12, post-release v1.40.0 (M83 — `archctl stack` removal).

## Estado del trunk

| Field | Value |
|---|---|
| Branch principal | `main` |
| Tip | squash commit on `main` post-PR-M83 (M83 — `archctl stack` removal) |
| Versión | `v1.40.0` (latest tag, post-PR-M83 squash) |
| Tests | 785 Rust + 128 archview tests pasan; 15 ignored (env-specific) |
| Working tree | trunk `main` en sync con `origin/main`, post-PR-M83 squash merge |
| MSRV | `1.91` (`rust-version` en `archctl/Cargo.toml`); CI pin `1.97.1` |
| LOC src | ~30,951 + ~30 (M82 unchanged) + (**M83** −303 net: removed `archctl/src/stack.rs` 229 LOC + `manifests/stack.toml` 41 LOC + 37 LOC `cli.rs`/`lib.rs` + e2e migration offset) |
| LOC tests | ~6,560 + ~25 (M82 — `nested_package_json_with_pnpm_workspace_in_same_dir_is_skipped` test) |
| LOC benches | ~790 |
| Vault milestones | 34 (M30–M61 + M69 + M73 + M75 + M76 + M79 + M81 + M82 + M83) |
| Tags | 40 (v1.1.0 → v1.36.0; v1.31.0–v1.35.0 backfilled below; +v1.37.0–v1.40.0) |

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
| SceneGraph abstraction | Deferred | ≥3 view types need shared scene model |
| WIT Plugin SDK | Deferred | ≥1 third-party consumer registered |
| Event sourcing/replay | Deferred | Temporal diff is shipped requirement |
| Architecture Lab (forks) | Deferred | ≥3 user requests |
| Full 9-agent catalog | Deferred | 2/2 deployed agents >50% adoption |
| Desktop shell (Tauri) | Deferred | Browser-only is blocker for ≥1 user segment |

## Deuda técnica activa

**Doctor:** 26/26 scopes pass. No findings.

**Closed in this session** (M37–M56):
- `seed_writes` lying API removed (BREAK-1, v1.4.1)
- Mermaid projector bare-Label bug fixed across 4 views (M39 + M41)
- C4 PlantUML Structurizr-style emit bug (M50)
- 26 stale manifest `public_symbols` removed (M46)
- 3 stale "no parameter binding" doc claims (M52)
- `backend_available()` helper DRY'd (M56, -60 LOC across 5 files)

**Pending** (from M55 study):
- **TODO markers** (2): `code/strategies/dockerfile.rs:139`, `code/class_diagram.rs:1067`. M60 proposes resolving.
- **store.rs** (2,383 LOC): biggest file. M63 proposes splitting.
- **Cognitive-layer test coverage**: 14 sub-modules, minimal tests. M61 proposes audit.

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

**Next horizon (per ROADMAP H0–H3):**
- **H0 — Ejecutable / verdad verificable**: contrato bundle schema ↔ Rust DTO ↔ TS types + selector configurable en `archctl view`. First code cycle of the H0 era.
- H1, H2, H3 — pendiente, ver ROADMAP H0–H3 section.
- **H4 — Distribución & ciclo de vida del CLI**: **CERRADO en v1.36.0 (M73+M75+M76)** — 2026-08-11. Multi-version installs, self-update, IDE adapters (OpenCode/ZCode/Claude Code/Codex), plugin tap model.

## Comandos de verificación

```bash
# Estado del repo
git log --oneline -10
git tag --sort=-creatordate | head -5
git status

# Tests
cd archctl
cargo build --quiet
cargo test --quiet
cargo clippy --quiet --all-targets
cargo fmt --check

# Verificación completa (cheap mode)
cd /var/home/rubentxu/Proyectos/agentesIA/arch-stack
bash scripts/verify-local.sh

# 26/26 doctor scopes
cargo run --bin archctl -- doctor --scopes $(ls manifests | sed 's/.toml//' | tr '\n' ',' | sed 's/,$//') --cwd archctl
```

## Próxima acción del usuario

Tras el cierre de M55–M69, el código y la documentación están en estado estable:

1. **H0 code cycle** — Contrato bundle schema ↔ Rust DTO ↔ TS types + selector configurable.
2. **Otra ronda de housekeeping** (revisar sddk framework, vault, etc.).
3. **Otra tarea o feature** no listada.
4. **Cerrar sesión** y volver con fresh energy.