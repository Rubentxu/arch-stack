# `archview` — Agent Guidelines

> Documento operativo para agentes de IA y contribuidores humanos
> que trabajen sobre el repo `archview`. Las decisiones de
> arquitectura viven en `docs/adr/` (cross-link con `archctl/docs/adr/`).

## Project Intent

`archview` renderiza los bundles JSON emitidos por `archctl` como
grafos navegables de alta performance. Es el **frontend visual**
del pipeline; `archctl` es el backend generador. Nunca se analiza
código en `archview` — siempre consume el output de `archctl`.

- **No invasive**: read-only sobre `archctl` bundles.
- **Local-first**: renderiza en navegador local, sin servers.
- **Performance-first**: hard budget per ADR-019.
- **Stack ortogonal**: ciclos de release independientes de `archctl`.

## Core Architecture

```
src/
├── bundle/loader.ts                  # Único punto de decode/normalize
├── renderer/g6.ts                    # Único punto de render
├── lib/                              # Hooks reutilizables (e.g. useWorkspaceState)
│   └── workspace.ts                  # H1 durable state (ADR-041)
├── components/
│   ├── Sidebar.tsx                   # Inspect selected node + bundle metadata
│   └── SourceDrawer/                 # Read-only source preview + editor handoff (H1)
├── App.tsx                           # orquesta bundle→renderer→sidebar
├── index.tsx
└── styles.css
```

### Reglas de dependencia

- `bundle/loader.ts` → no imports de `renderer/`, `components/` ni `lib/`
- `renderer/g6.ts` → no imports de `bundle/`, `components/` ni `lib/`
- `lib/workspace.ts` → no imports de `bundle/`, `renderer/` ni `components/`
  (pure hook + types; talks to backend over HTTP)
- `components/Sidebar.tsx` → depende de `bundle/`, `components/SourceDrawer/`,
  y `lib/` (recibe `fetchSource`/`openInEditor` handlers via props; no llama
  `fetch` directamente)
- `App.tsx` → orquesta todo: bundle → renderer → sidebar → drawer

## Commands

```bash
# Setup
pnpm install

# Dev
pnpm dev              # Vite dev server en http://localhost:18080

# Build
pnpm build            # → dist/

# Test
pnpm test             # vitest run (one-shot)
pnpm test:watch       # vitest watch mode

# Lint/format
pnpm format:check     # prettier check
pnpm format           # prettier write
```

## Definition of Done

- `pnpm build` exit 0
- `pnpm test` exit 0 (>= 4 tests passing, one per bundle shape)
- `pnpm lint` exit 0
- `pnpm format:check` exit 0
- No `console.error` en dev (revisar DevTools console)
- Para cambios en `loader.ts`: tests cubren los 4 shapes
  (call-graph, sequence, class-diagram, c4)

## Conventions

- **Reactivo**: SolidJS usa `createSignal` para state. NO uses
  `useState` (no existe en SolidJS); el equivalente es `createSignal`.
- **JSX**: `class` (no `className`). `for` (no `htmlFor`).
- **Imports**: relativos entre carpetas `bundle/`, `renderer/`,
  `components/` con imports explícitos `../`.
- **Tests**: junto al código bajo `__tests__/<name>.test.ts`.
  4 tests baseline (uno por bundle shape).
- **Conventional commits**: `<type>(<scope>): <subject>`.
  - `feat(renderer): add edge bundling`
  - `fix(loader): handle missing schemaVersion`
  - `docs(readme): document sample bundle format`
- **No `Co-Authored-By` en commits**
- **No cierres en rama compartida**: este repo tiene 1 autor.

## Conventions

- **Tabs pattern**: see `<TabBar>` in `components/primitives/Tabs.tsx` for reusable two-or-more panel switching with ARIA APG keyboard nav.

## Ciclo M17.1 — ship done (M22)

- Semantic zoom C4 (Context → Container → Component → Code)
- Layout jerárquico ELK.js en Web Worker
- Virtualización de DOM para >1k nodos
- Sidebar con tabs (evidence vs relations) ✅ shipped v1.79.0

## Perf budget enforcement (M23)

ADR-019 §enforcement declares a post-merge CI gate for archview perf regressions.
M23 implements it.

### CI job: `perf-cull`

Runs after every push to `main` (post-merge only, NOT PR-gated per ADR-025/ADR-047).

- **Location**: `.github/workflows/ci.yml` — job `perf-cull`
- **Baseline**: previous main SHA (`github.event.before`)
- **Dataset**: `c4-stress-1k.json` (1 221 nodes / 3 920 edges)
- **Threshold**: 10% regression
  - TTFP increase > 10% → **FAIL**
  - FPS decrease < 10% (i.e. `pr < main * 0.90`) → **FAIL**

### Local reproduction

```bash
# Fake mode (no real benchmark — fast, deterministic)
scripts/bench-compare-archview.sh --fake-ttfp-regression 11 --fake-fps-regression 11
# EXIT 1 = regression detected (expected)

scripts/bench-compare-archview.sh --fake-ttfp-regression 5 --fake-fps-regression 5
# EXIT 0 = within threshold

# Real mode (requires playwright)
cd archview && pnpm build && node bench/perf-cull.mjs --output perf.json --warmup 1
```

### Investigating a failure

1. **Check the CI run**: `gh run list --workflow=ci.yml` → find the failing `perf-cull` job
2. **Get baseline JSON**: download `perf-baseline.json` from the baseline SHA artifact
3. **Get head JSON**: download `perf-head.json` from the head artifact
4. **Compare manually**: `jq '.ttfp_ms, .fps' perf-baseline.json perf-head.json`
5. **Reproduce locally**: run the bench on a local checkout of the failing SHA

### Out of scope (M23 debt)

- Lighthouse score gate (ADR-019 L65) — not implemented
- 10k + 100k benchmark datasets (ADR-019 §benchmarking) — not generated
- Re-enabling `enableCulling: true` in CallGraphView/ImpactView — deferred
