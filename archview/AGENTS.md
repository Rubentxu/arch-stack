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
├── bundle/loader.ts    # Único punto de decode/normalize
├── renderer/g6.ts      # Único punto de render
├── components/Sidebar.tsx
├── App.tsx
├── index.tsx
└── styles.css
```

### Reglas de dependencia

- `bundle/loader.ts` → no imports de `renderer/` ni `components/`
- `renderer/g6.ts` → no imports de `bundle/` ni `components/`
- `components/Sidebar.tsx` → depende solo de `bundle/`
- `App.tsx` → orquesta bundle→renderer→sidebar

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

## Próximo ciclo (M17.1)

- Semantic zoom C4 (Context → Container → Component → Code)
- Layout jerárquico ELK.js en Web Worker
- Virtualización de DOM para >1k nodos
- Sidebar con tabs (evidence vs relations)
