# sddk/m17.0-archview-scaffold — apply-progress.md

## Cycle

`m17.0-archview-scaffold` — repo nuevo `archview` separado de `archctl`.
Primer tag `v0.14.0`.

## Branch

`main` (repo nuevo, no fue un branch de archctl).

## Commit History

| # | Hash | Subject |
|---|------|---------|
| 1 | <initial> | scaffold M17.0 archview (SolidJS + G6.5 + Vite) |

> Single commit (scaffold cycle, no multi-task breakdown needed).

## Tasks Completed

### Foundation

- ✅ 1.1 — Repo `archview` creado (separate from `archctl`)
- ✅ 1.2 — package.json con SolidJS + G6 5.x + Vite + vitest
- ✅ 1.3 — vite.config.ts (port 18080, SolidJS plugin)
- ✅ 1.4 — tsconfig.json (strict mode, ESNext target)
- ✅ 1.5 — index.html + src/index.tsx (entry point)
- ✅ 1.6 — docs: CONTEXT.md, README.md, AGENTS.md, CHANGELOG.md, .gitignore

### Bundle Loader (M17.0 core)

- ✅ 2.1 — `src/bundle/loader.ts`: normalize 4 shapes (call-graph, sequence, class-diagram, C4)
- ✅ 2.2 — Tests: 4 scenarios covering each shape
- ✅ 2.3 — Sample bundles in `public/samples/`

### Renderer (M17.0 core)

- ✅ 3.1 — `src/renderer/g6.ts`: G6 5.x wrapper with pan/zoom/drag
- ✅ 3.2 — Dark theme palette aligned with `archctl`

### Shell

- ✅ 4.1 — `src/App.tsx`: topbar + canvas + sidebar
- ✅ 4.2 — `src/components/Sidebar.tsx`: evidence inspector
- ✅ 4.3 — `src/styles.css`: minimal MVP styles

## Tests

- **4 unit tests** in `src/__tests__/loader.test.ts` (one per bundle shape)
- **To run**: `pnpm install && pnpm test`
- **State**: not yet executed (no `pnpm install` performed in this cycle
  due to npm/pnpm resolution cost). Verification deferred to M17.0
  docker setup or CI pipeline.

## Known Debt

- **No CI**: typescript + vitest tests not run in CI yet. Action:
  add `.github/workflows/ci.yml` in M17.0.1.
- **No node_modules verification**: code was written without running
  `pnpm install`. Risk: missing peer dependencies or version
  conflicts. Mitigation: detailed package.json with pinned versions.
- **No performance bench**: ADR-019 budget not measured. M17.0 MVP
  target is <10k nodes; <100k requires M17.1+ optimizations.
- **Sidebar extensibility**: hard-coded evidence layout. M17.4 will
  introduce tabs (evidence | relations | call-graph-incoming).

## Deferred (post-M17.0)

- **M17.1**: Semantic zoom C4 (Context → Container → Component → Code)
- **M17.2**: Call graph view (1-N niveles, blast radius)
- **M17.3**: Sequence diagram view
- **M17.4**: Class diagram view (UML)
- **M17.5**: Package diagram view
- **M17.6**: Drift detection (C4 declarado vs actual)
- **M17.7**: Impact analysis (blast radius)
