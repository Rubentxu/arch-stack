# Changelog

All notable changes to `archview` are documented here. The format
loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [v0.18.0] — 2026-08-02 — M17.4 class diagram view

### Added
- **ClassDiagramView**: `src/views/ClassDiagramView.tsx` renders
  class-diagram bundles as UML class diagrams. Each class /
  interface / trait / enum is a card with three compartments:
    1. Header: stereotype (`<<interface>>` / `<<trait>>` /
       `<<enum>>`) + class name + language + file:line
    2. Attributes (`member_kind == "field"`)
    3. Methods (`member_kind == "fn"` or `"method"`)
  Relations panel at the bottom groups edges by predicate:
  `extends` (blue), `implements` (orange), `composes` (red).
- **App.tsx Switch routing**: class-diagram bundles go to
  `ClassDiagramView` (more specific than fallback GraphView).
- **Sample bundle**: `public/samples/class-diagram-rich.json` —
  6 elements (Drawable, Serializable interfaces; Shape, Circle,
  AuditEntry classes; Loggable trait) + 5 edges exercising all
  3 predicate styles.
- **Tests**: 17 → 19. New coverage: `members[]` preserved in node
  meta for the diagram render, extends/implements/composes
  predicates distinguished.

### Refs
- Cycle: `m17.4-archview-class-diagram`
- ROADMAP M17.4 (post-v0.13.0 stabilization plan F2.2)

## [v0.17.0] — 2026-08-02 — M17.3 sequence diagram view

### Added
- **SequenceView**: `src/views/SequenceView.tsx` renders sequence
  bundles as UML sequence diagrams — participant header (lifelines)
  + interaction rows in time order. Each row: source → arrow → target
  with kind-specific styling:
    - `SyncCall` — solid line + arrow head
    - `AsyncCall` — dashed line (repeating-linear-gradient) + arrow head
    - `Reply` — dotted line, no arrow head
  Click on a participant to focus the corresponding node in sidebar.
- **Loader preserves raw interactions**: `SequenceInteraction`
  type + `interactions` field on `GraphBundle` for sequence bundles.
  `normalizeInteraction` helper for schema-tolerant parsing.
- **App.tsx Switch routing**: sequence bundles route to
  `SequenceView` (more specific than call-graph's
  `CallGraphView`).
- **Sample bundle**: `public/samples/sequence.json` — 7 interactions
  showing a request/response flow (server → handle_request → auth
  → db → reply, with sync/async/reply message kinds).
- **Tests**: 14 → 17. New coverage: raw interactions preserved,
  unique participant extraction (file:name pair dedup), missing
  optional fields handled.

### Refs
- Cycle: `m17.3-archview-sequence`
- ROADMAP M17.3 (post-v0.13.0 stabilization plan F2.2)

## [v0.16.0] — 2026-08-02 — M17.2 call graph view

### Added
- **CallGraphView**: `src/views/CallGraphView.tsx` renders call-graph
  and sequence bundles with focus-driven BFS expansion. Focus node
  selector, direction (callees / callers / both), depth 1-5 with
  +/− controls.
- **Async flow visualization**: edges tagged `AsyncCall` get a
  distinct orange treatment vs the default blue (SyncCall) via
  `.kind-sync` / `.kind-async` CSS classes.
- **Blast radius counter** in the sidebar: shows total reachable
  functions at the current depth + direction. Emitted via `onStats`
  prop from CallGraphView.
- **App.tsx Switch routing**: call-graph and sequence bundles route
  to CallGraphView. C4 still routes to C4View. Other shapes fall
  through to GraphView (M17.0).
- **Sample bundle**: `public/samples/call-graph-deep.json` — 6 nodes
  (server, handle_request, auth, query_db, log, metric) + 7 edges.
  Used to verify blast radius expansion.
- **Tests**: 8 → 14. New coverage: BFS diamond expansion
  (1 level, 2 levels, dedup, terminate early), direction symmetry
  (callees, callers, both), async flow preservation.

### Refs
- Cycle: `m17.2-archview-callgraph`
- ROADMAP M17.2 (post-v0.13.0 stabilization plan F2.2)

## [v0.15.0] — 2026-08-02 — M17.1 C4 semantic zoom

### Added
- **C4 hierarchical view**: `src/views/C4View.tsx` renders C4 bundles
  with semantic zoom (Context → Container → Component → Code).
  Drill-down: click a System → see its Containers; click a Container
  → see its Components. Breadcrumb navigation to climb back.
- **C4 enrichment in loader**: `c4LevelForKind()` helper maps
  `Person`/`SoftwareSystem` → L1, `Container` → L2, `Component` → L3,
  `Code` → L4. `parentId` extracted for drill-down. C4 metadata
  (description, technology) preserved in `meta`.
- **Sidebar C4 enrichment**: shows `L1`–`L4` tag, technology,
  description (multi-line), parent id.
- **Switch routing in App.tsx**: C4 bundles go to C4View; all other
  bundle shapes (call-graph, sequence, class-diagram) go to the
  existing G6-based GraphView.
- **Sample bundles**: `public/samples/c4-context.json` (3 systems +
  1 person), `public/samples/c4-container.json` (1 system + 3
  containers, archctl real example).
- **Tests**: 4 → 8. New coverage for C4 normalization
  (level + parentId + meta) and `c4LevelForKind` (all C4 kinds +
  instance variants + unknown fallback).

### Refs
- Cycle: `m17.1-archview-c4-zoom`
- ROADMAP M17.1 (post-v0.13.0 stabilization plan F2.2)

## [v0.14.0] — 2026-08-02 — M17.0 scaffold

### Added
- **Bundle loader**: `src/bundle/loader.ts` normalizes 4 archctl bundle
  shapes (call-graph, sequence, class-diagram, C4) to a uniform
  `GraphBundle` interface. Schema-tolerant: accepts any JSON with
  `nodes`+`edges` or `elements`+`relations`.
- **G6 5.x renderer**: `src/renderer/g6.ts` wraps
  `@antv/g6` (WebGPU) for pan/zoom + drag interactions. Dark theme
  palette aligned with `archctl` deliverable.
- **Sidebar evidence inspector**: `src/components/Sidebar.tsx` shows
  evidence `file:line` references for the selected node.
- **App shell**: `src/App.tsx` orchestrates topbar (bundle loader) →
  canvas (renderer) → sidebar. URL input accepts `file://` and
  `http://` bundles.
- **Vite + SolidJS scaffold**: `package.json`, `vite.config.ts`,
  `tsconfig.json`, `index.html`. Dev server on port 18080.
- **Sample bundles**: `public/samples/{call-graph,class-diagram}.json`
  for quick exploration without generating from `archctl`.
- **Tests**: 4 unit tests in `src/__tests__/loader.test.ts`, one per
  bundle shape (call-graph, sequence, class-diagram, C4).
- **Documentation**: `CONTEXT.md`, `README.md`, `AGENTS.md`.

### Refs
- Cycle: `m17.0-archview-scaffold`
- ROADMAP M17.0 (post-v0.13.0 stabilization plan F2.2)
- ADR-020 (renderer stack: G6 5.x WebGPU + SolidJS + Rust/WASM)
- ADR-019 (performance budget — relaxed to <10k nodes for MVP)
