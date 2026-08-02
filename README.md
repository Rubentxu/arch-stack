# `archview` — Code Knowledge Graph Workbench

> Workbench interactivo para visualizar los bundles emitidos por
> `archctl`. Stack: SolidJS + Vite + G6 5.x (WebGPU). Consume
> call-graph, sequence, class-diagram y C4 como JSON.

## Quickstart

```bash
pnpm install
pnpm dev   # http://localhost:18080
```

## Estructura

```
src/
├── index.tsx          # entry point
├── App.tsx            # shell: topbar + canvas + sidebar
├── styles.css
├── bundle/
│   └── loader.ts      # JSON bundle normalizer (4 shapes → uniform GraphBundle)
├── renderer/
│   └── g6.ts          # G6 5.x wrapper (pan/zoom, draw)
├── components/
│   └── Sidebar.tsx     # evidence inspector
└── __tests__/
    └── loader.test.ts # 4 shape tests
public/
└── samples/
    ├── call-graph.json
    └── class-diagram.json
```

## Cómo se usa

1. **Generar el bundle** desde `archctl`:

   ```bash
   cd ../archctl
   cargo run -- code call-graph --json --cwd /path/to/repo > bundle.json
   ```

2. **Cargar el bundle** en `archview`:
   - Click "Sample call-graph" → usa `public/samples/call-graph.json`
   - O pegar URL `file:///path/to/bundle.json` en el input del topbar
   - O servir el bundle vía http y pegar la URL

3. **Explorar**: pan/zoom con drag/scroll, click en nodo → sidebar
   muestra evidencia (`file:line`).

## Performance budget (ADR-019)

- TTFP <1s para <10k nodos
- Pan/zoom 60 FPS
- Filter <50ms
- Memory <500MB para 100k nodos

M17.0 MVP NO está optimizado para 100k — solo para 10k. M17.1+
introduce layout jerárquico (ELK), virtualización de DOM, y
WebGPU compute shaders.

## Roadmap

- **M17.0 (v0.14.0)** ← este MVP: bundle loader + pan/zoom + sidebar
- **M17.1**: Semantic zoom C4 (Context → Container → Component → Code)
- **M17.2**: Call graph view (1-N niveles, blast radius)
- **M17.3**: Sequence diagram view
- **M17.4**: Class diagram view (UML)
- **M17.5**: Package diagram view
- **M17.6**: Drift detection (C4 declarado vs actual)
- **M17.7**: Impact analysis (blast radius)

## Stack

- **SolidJS**: fine-grained reactivity, sin virtual DOM
- **Vite**: dev server + build
- **@antv/g6 v5** (WebGPU): renderer de grafos
- **vitest**: tests

## Comandos

```bash
pnpm install          # setup deps
pnpm dev              # dev server (HMR)
pnpm build            # build → dist/
pnpm test             # vitest run
pnpm test:watch       # vitest watch
pnpm format:check     # prettier check
pnpm format           # prettier write
```

## Próximos pasos

El branch actual (`main`) tiene M17.0 listo. Para M17.1+:
- Layout jerárquico ELK.js (Web Worker)
- Virtualización de nodos en DOM
- C4 semantic zoom (drill-down por scope)
- Sidebar con tabs de evidence vs relations
