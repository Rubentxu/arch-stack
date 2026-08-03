# Estado de `archctl`

> Snapshot del estado real del repo. Refreshed al cierre de cada ciclo
> para reflejar la verdad del código, no la planificación aspiracional.
> Última actualización: 2026-08-02, post-`v0.13.0`.

## Estado del trunk

| Field | Value |
|---|---|
| Branch principal | `main` |
| Tip | `65ac6a2` |
| Versión | `v0.13.0` |
| Tests | 254 pasan, 0 fallan, 5 ignorados |
| Working tree | clean, en sync con `origin/main` |
| MSRV | sin documentar explícitamente (código usa `?` en main, `let else`, `impl Trait`) |

## Capacidades shipped (v0.x)

| Tag | Fecha | Capacidad |
|---|---|---|
| `v0.1.0` – `v0.3.0` | 2026-07-30 | LadybugDB graph, evidence, renderers, source-evaluation types |
| `v0.4.0` – `v0.6.0` | 2026-07-30 | Manifest gates (23 scopes), schema migration runner (ADR-017) |
| `v0.7.0` | 2026-07-31 | Cambios en diagram export pipeline |
| `v0.8.0` / `v0.8.1` | 2026-07-31 | `archctl code call-graph` (PR1, con TSG rules fix) |
| `v0.9.0` | 2026-07-31 | `archctl code sequence` (PR2) |
| `v0.9.1` / `v0.9.2` | 2026-07-31 | Refactors: m9-debt-cleanup + store-port-seams |
| `v0.10.0` | 2026-08-01 | M20 bench suite (criterion harness, 3 datasets) |
| `v0.11.0` | 2026-08-01 | m9-renderers-local (F1 security: renderers locales estrictos) |
| `v0.12.0` | 2026-08-01 | m9-relations-decision (audit F2-F7 + M1-M3 + M5) |
| `v0.12.1` | 2026-08-02 | refactor/bench-seed-decomposition (audit M5 follow-up) |
| `v0.13.0` | 2026-08-02 | M12 class-diagram extraction (tree-sitter CST walk) |
| `v0.13.1` | 2026-08-02 | refactor/clippy-fmt-cleanup + composes edges (closes M12 W4) |

## Capacidades en backlog

| Pendiente | Estado | Prioridad |
|---|---|---|
| M13 (workbench actions: drift detection, impact analysis) | Sin iniciar | Baja — enterprise feature, no target developer |
| M14 (versionado, rollback, snapshots) | Sin iniciar | Baja — defer a 1.x |
| M15 (herramientas semánticas opcionales) | Sin iniciar | Baja — defer a 1.x |
| M16 (endurecimiento 1.0) | Sin iniciar | Media — depende de M17 |
| M17 (archview workbench, separate repo) | Sin iniciar | **Alta — próximo ciclo grande** |
| LSP-based extraction (ADR-012 follow-up) | Diferido a fase 2 M12 | Media |
| Cross-file inheritance + composition/aggregation | Diferido a fase 2 M12 | Media |

## Deuda técnica activa

| ID | Descripción | Bloquea gate | Estado |
|---|---|---|---|
| `code::apply` repetition | `code::call_graph::apply` + `code::c4_discover::apply` + `code::class_diagram::apply` comparten ~150 LOC de open-default + init + counters + ApplyReport construction. No extraction hecha — error types y semánticas difieren por dominio. | No | Defer v0.14.x; cada `apply` mantiene su firma pública estable |
| `diagram::apply` 3-tier | `run_apply` (CLI) → `apply_changeset` (high) → `apply_to_store` (core). Algunas validaciones duplicadas entre niveles. | No | OK por ahora; el core está extraído para testabilidad |

**Sin deuda bloqueante activa** — `doctor --scopes code` corre en <1s con 0 findings.

**Cerrado en v0.13.1**:
- W4 composes edges (F1.2) ✅
- Pre-existing 56 clippy warnings (F1.1) ✅
- Pre-existing 137 rustfmt violations across 48 files (F1.1) ✅
- lbug infra gap (F3.3) ✅ — gate ahora cuenta `#[test]` annotations en lugar de ejecutar `cargo test`

## Plan vigente

**Post-v0.13.0 stabilization plan** (obs-5524): TODO CERRADO.

```
Fase 1 ✅ v0.13.1 released (commit 7738b2d)
├── F1.3 branches + STATE.md ✅
├── F1.1 refactor/clippy-fmt-cleanup ✅
└── F1.2 feat/code-class-diagram-composes ✅

Fase 2 ✅
├── F2.1 roadmap M13-M15 trim ✅
├── F2.2 M17.0 archview designado como próximo ciclo grande ✅
└── F2.3 audit manifests/code.toml coverage ✅

Fase 3 ✅
├── F3.1 jurisprudence discoverable (obs-5518) ✅
├── F3.2 fmt-staged.sh + AGENTS.md nota + pre-commit hook ✅
└── F3.3 lbug infra service ✅ (annotation counter, no cargo test subprocess)
```

## Repo satelital: `archview`

- **Path**: `/var/home/rubentxu/Proyectos/agentesIA/archview` (separate repo, no remote yet)
- **Tag**: **`v0.21.3`** (verified — pnpm test 67/67 + build OK)
- **Status**: **M17 series complete** (8 cycles, v0.14.0 → v0.21.0)
  - M17.0 scaffold (v0.14.0): bundle loader + G6 canvas + sidebar
  - M17.1 C4 semantic zoom (v0.15.0): hierarchical + drill-down
  - M17.2 call graph view (v0.16.0): focus + BFS + blast radius
  - M17.3 sequence diagram view (v0.17.0): lifelines + arrows
  - M17.4 class diagram view (v0.18.0): UML compartments
  - M17.5 package diagram view (v0.19.0): modules + cycles (DFS)
  - M17.6 drift detection (v0.20.0): declared vs actual C4 diff
  - M17.7 impact analysis (v0.21.0): blast radius for changes
- **Próximo**: M18 reactive runtime (defer 1.x) o M19 wgpu renderer (2.0).
  Alternativa: refactor cycles (`refactor/extract-code-apply-helpers`,
  archctl-side) o archview-side enhancements (resize observer,
  G6 dagre layout, persist UI state).

## Archivos clave para retomar

- `archctl/src/lib.rs` — declaración de módulos y `pub use`.
- `archctl/src/diagram/` — bounded context C4 (export, apply, validate).
- `archctl/src/code/` — bounded context extracción (call_graph, sequence, class_diagram, c4_discover).
- `archctl/src/store.rs` — `GraphStore` port + `LbugStore` adapter (fs2 flock).
- `archctl/src/evidence.rs` — extracción y validación de evidencias (ADR-005).
- `archctl/src/skills.rs` — 3-modo skill loader (ADR-003).
- `archctl/src/migrations.rs` — schema migration runner (ADR-017).
- `manifests/code.toml` — manifest gate para bounded context `code` (incluye M12 symbols + must_hold).
- `docs/specs/code-class-diagram/spec.md` — spec synced de M12.
- `sddk/m12-class-diagram/` — 10 artefactos del último ciclo.

## Comandos de verificación

```bash
# Estado del repo
cd /var/home/rubentxu/Proyectos/agentesIA/archctl
git log --oneline -10
git tag --sort=-creatordate | head -5
git status

# Tests
cd archctl
cargo build --quiet
cargo test --quiet
cargo clippy --quiet --all-targets  # NOTA: hoy 56 pre-existing warnings; target post-F1.1 = 0
cargo fmt --check                    # NOTA: hoy 3 pre-existing bench issues; target post-F1.1 = 0
```

## Próxima acción del usuario

Ejecutar Fase 1 del plan post-v0.13.0 (`refactor/clippy-fmt-cleanup` + `feat/code-class-diagram-composes`) → release `v0.13.1`. Luego Fase 2 (decisión trim M13-M15 + arranque M17.0 archview).
