# Changelog

All notable changes to `archview` are documented here. The format
loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
