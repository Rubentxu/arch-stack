# `archview` — Code Knowledge Graph Workbench

> Resumen breve. La especificación completa vive en
> [`docs/README.md`](docs/README.md), los
> [ADRs](docs/adr/) (enlaces cruzados con `archctl/docs/adr/`),
> y las specs en `docs/specs/`. `CONTEXT.md` no contradice esa
> documentación; si lo hace, gana la documentación detallada.

## Qué es

`archview` es un **workbench interactivo** que consume bundles JSON
emitidos por `archctl` (proyecciones C4/UML de un repositorio) y
los renderiza como grafos navegables. El target primario es
desarrolladores y arquitectos que necesitan entender la estructura
de un codebase con latencia sub-frame (60 FPS).

## Restricciones duras

- **No invasive**: `archctl` lo alimenta con bundles via CLI; no
  reimplementa análisis. Repro: `archctl code call-graph --json`
  genera bundle, `archview` lo carga.
- **Local-first**: renderiza en navegador local, no envía datos
  a servers remotos. Bundles cargados vía `file://` o servidor estático.
- **Performance-first**: Built sobre G6 5.x WebGPU + SolidJS + Rust/WASM
  (ver ADR-020). Performance budget hard (ver ADR-019):
  TTFP <1s, pan/zoom 60 FPS, filter <50ms, memory <500MB para 100k nodos.
- **Stack ortogonal a `archctl`**: ciclo de release independiente.
  `archctl` v0.14.x → `archview` v0.14.0. Co-evolucionan.

## Capacidades del MVP (M17.0 — v0.14.0)

- Bundle loader: parsea cualquier bundle JSON de `archctl` (C4,
  call-graph, sequence, class-diagram).
- View genérica: renderiza nodos + edges con G6 5.x WebGPU.
- Pan/zoom + sidebar de evidencias: click en nodo → muestra
  `evidence_refs` con `file:line` snippets.

## Capacidades post-MVP (M17.1+ — v0.14.x)

- **M17.1**: Semantic zoom para C4 (Context → Container → Component
  → Code).
- **M17.2**: Call graph view (1-N niveles, blast radius, async flow).
- **M17.3**: Sequence diagram view (call chains, async flows).
- **M17.4**: Class diagram view (UML).
- **M17.5**: Package diagram view (dependencias, ciclos, cohesión).
- **M17.6**: Drift detection (C4 declarado vs actual).
- **M17.7**: Impact analysis (blast radius).

## Stack

- **SolidJS**: fine-grained reactivity, sin virtual DOM
  (más rápido para listas con muchos nodos).
- **Vite**: dev server + build.
- **G6 5.x WebGPU**: renderer de grafos (primary).
- **cosmos.gl**: adapter para >100k nodos (defer a M17.2+).
- **petgraph (Rust→WASM)**: algoritmos de grafos en Rust.
- **roaring (Rust→WASM)**: selections/filters bitmaps.

## Lo consume

- `archctl> v0.13.x` con `code {call-graph, sequence, class-diagram} --json`
- `archctl code c4-discover --json` para C4
- `archctl diagram export` para bundles C4 canónicos

## Repos relacionados

- [`archctl`](https://github.com/anomalyco/archctl) — CLI sidecar que
  emite los bundles.
- [`archctl-remote`](https://github.com/anomalyco/archctl-remote) — el
  repo actual `archctl` (mismo remote, distinto alias local).
