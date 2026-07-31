# ADR-020 — Renderer stack: G6 5.x WebGPU + SolidJS + Rust/WASM

**Estado:** Aceptado
**Fecha:** 31 de julio de 2026
**Aplica a:** `archview` (Code Knowledge Graph Workbench)
**Refuerza:** ADR-013 (viewer ortogonal), ADR-019 (performance budget)
**Sustituye a:** Sprotty + Cytoscape.js (propuesta original en ADR-013 §"Stack de `archview`")

## Contexto

`archview` necesita renderizar grafos de conocimiento de código a **velocidad máxima**, en cualquier nivel de complejidad. La propuesta original en [ADR-013](ADR-013-viewer-ortogonal.md) (Sprotty + ELK.js + Cytoscape.js) no cumple el [performance budget](ADR-019-performance-budget.md):

- **Sprotty** está orientado a statecharts, no a grafos grandes. El doc `Librerías-visualización-grafos-BI.md` lo marca explícitamente: "no escala para grafos grandes".
- **Cytoscape.js** tiene performance media/alta pero no aprovecha GPU.
- **ELK.js** solo en Web Worker es lento para grafos >5k nodos sin acceleration GPU.
- **React/Svelte** introducen virtual DOM que añade latencia innecesaria en paneles con muchos elementos.

La re-evaluación del roadmap (julio 2026) confirmó que el target no es BI sino un workbench para developers/arquitectos. Este público es **extremadamente sensible a la latencia** y opera con grafos de 5k-100k nodos como tamaño típico (repos reales).

## Decisión

`archview` se construye sobre el siguiente stack, optimizado para el performance budget:

```text
┌─────────────────────────────────────────────────────────┐
│ Renderer (graph)                                          │
│   G6 5.x con WebGPU backend (primary)                     │
│   cosmos.gl (specialized adapter para >100k nodos)        │
│   ELK.js (Web Worker) — para layouts jerárquicos pesados  │
└─────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│ Compute layer (Rust → WebAssembly)                        │
│   petgraph (grafos, algoritmos)                            │
│   roaring (selections, filters)                            │
│   wasm-bindgen (bindings) + js-sys + web-sys              │
│   rayon (paralelismo CPU; defer en WASM)                  │
└─────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│ Data transport                                            │
│   Apache Arrow (columnar, zero-copy)                       │
│   TypedArrays (Float32Array, Uint32Array)                 │
│   MessagePack (data interchange, no JSON)                  │
└─────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│ UI framework                                               │
│   SolidJS (fine-grained reactivity, no virtual DOM)        │
│   + Web Components (para Sprotty-style custom elements)    │
│   + D3 modules (escalas, color)                           │
└─────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│ Threading                                                  │
│   Main thread: render + interaction                       │
│   Web Worker: layout, indexing, heavy algorithms           │
│   SharedArrayBuffer: zero-copy data share (when COOP/COEP) │
│   OffscreenCanvas: off-thread rendering (when supported)   │
└─────────────────────────────────────────────────────────┘
```

### Capa por capa: rationale

#### 1. Renderer (graph)

- **G6 5.x con WebGPU backend** (primary): soporta hierarchical (C4) + force (call graph) + WebGPU acceleration. MIT license. Activo. Probado hasta ~100k nodos.
- **cosmos.gl** (specialized adapter): para grafos >100k nodos donde G6 empieza a tener jank. Computa layout y rendering en GPU. MIT.
- **ELK.js** (Web Worker): para layouts jerárquicos complejos donde G6's dagre no basta. Ejecuta en Web Worker para no bloquear main thread.

**Descarte explícito**:
- ❌ **Sprotty** — orientdo a statecharts, no escala.
- ❌ **Cytoscape.js** (como renderer primario) — no usa GPU, performance media.
- ❌ **Sigma.js** — bueno para análisis pero BI-leaning; G6 cubre el caso de uso.

#### 2. Compute layer (Rust → WASM)

- **petgraph** (Rust): algoritmos de grafos, centralidades, layouts CPU. WASM-compiled para 5-50x speedup vs JS.
- **roaring** (Rust): bitmaps comprimidos para selections, filtros, comunidades. 10-100x compression para sets dispersos.
- **wasm-bindgen** + **js-sys** + **web-sys**: bindings Rust ↔ JS. Tipos primitivos zero-copy.
- **rayon**: defer a 1.x. WASM threads tienen restricciones (COOP/COEP, toolchain). Para v1, single-threaded WASM.

**Descarte explícito**:
- ❌ **DuckDB-Wasm** — wrong abstraction (es para BI cross-filter, no para nuestro target).

#### 3. Data transport

- **Apache Arrow** (columnar, zero-copy): tablas transferidas entre Rust y JS sin serializar. `Float32Array`, `Uint32Array` directos a GPU buffers.
- **TypedArrays**: node position, size, color, flags como `Float32Array` / `Uint32Array`. No per-node JS objects.
- **MessagePack**: para interchange binario de bundles desde disco. Alternativa a JSON; más compacto y rápido.

**Descarte explícito**:
- ❌ **JSON** para datasets >1000 elementos (mata el TTFP).

#### 4. UI framework

- **SolidJS**: fine-grained reactivity (signals), sin virtual DOM. 2-5x más rápido que React en paneles con muchos elementos. Bundle pequeño (~7KB gzipped).
- **Web Components** (custom elements): para encapsular Sprotty-style custom nodes si fuera necesario (no para v1, defer).
- **D3 modules** (escalas, color): para visual encoding (tamaños, colores por atributo). Tree-shakable.

**Descarte explícito**:
- ❌ **React** — virtual DOM diff es innecesario para nuestro use case; 2-5x overhead.
- ❌ **Svelte** (propuesta original) — compilado a JS, pero SolidJS es más rápido en runtime.
- ❌ **Vue** — overhead de reactividad.
- ❌ **vanilla TS** — viable pero costoso de mantener para paneles complejos.

#### 5. Threading

- **Main thread**: render (canvas), interaction (mouse, keyboard, scroll), UI panel updates. 60 FPS budget.
- **Web Worker**: layout computation, indexing, full-dataset filters. Off-thread para no bloquear render.
- **SharedArrayBuffer**: zero-copy data share entre threads (requiere COOP/COEP headers en server).
- **OffscreenCanvas**: si soportado, render en Worker; main thread libre.

### Performance contract linkage

Este stack cumple el [ADR-019 performance budget](ADR-019-performance-budget.md):

- **TTFP <1s** (10k nodos): bundle JS+WASM <2MB, Arrow deserializa en <100ms, SolidJS no re-render global.
- **Pan/zoom 60 FPS**: G6 WebGPU render, layout pausado durante pan/zoom.
- **Filter <50ms**: RoaringBitmap bitwise ops, Arrow columnar scan.
- **Layout <2s** (10k): G6 dagre en Web Worker; para force-directed, GPU compute via WebGPU.
- **Memory <500MB** (100k): TypedArrays (no per-node objects), Arrow no copia, Rust WASM heap propia.

### Dependencias iniciales (workbench)

```json
{
  "dependencies": {
    "@antv/g6": "^5.1.1",
    "@antv/hierarchy": "^0.7.0",
    "elkjs": "^0.9.0",
    "cosmos.gl": "^0.5.0",
    "solid-js": "^1.8.0",
    "@solidjs/router": "^0.14.0",
    "d3-array": "^3.2.0",
    "d3-color": "^3.2.0",
    "d3-format": "^3.1.0",
    "d3-scale": "^4.0.0",
    "apache-arrow": "^15.0.0",
    "msgpackr": "^1.10.0",
    "comlink": "^4.4.0",
    "wasm-feature-detect": "^1.8.0"
  },
  "devDependencies": {
    "vite": "^5.4.0",
    "vite-plugin-solid": "^2.10.0",
    "vite-plugin-wasm": "^3.2.0",
    "rollup-plugin-visualizer": "^5.12.0",
    "typescript": "^5.5.0"
  }
}
```

Y para el lado Rust (`graph-wasm/` crate):

```toml
[dependencies]
petgraph = "0.8"
roaring = "0.10"
bitvec = "1"
hashbrown = "0.14"
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = "0.3"
serde = { version = "1", features = ["derive"] }
serde-wasm-bindgen = "0.6"
```

## Consecuencias

### Positivas

- Stack moderno, todas las piezas activas en 2026.
- Cumple el performance budget para 10k-100k nodos (target principal del workbench).
- Separación clara: renderer / compute / data / UI / threading. Cambiar una capa no requiere reescribir las otras.
- SolidJS + G6 + WebGPU es la combinación más rápida disponible en JS para grafos interactivos.
- Rust → WASM compute permite algoritmos pesados (centralities, projections) sin bloquear el render.

### Negativas

- **WebGPU availability**: WebGPU está disponible en Chrome 113+, Edge 113+, Firefox 121+ (2024), Safari 17+ (2023). Para browsers antiguos, fallback a WebGL2 (más lento).
- **SharedArrayBuffer** requiere COOP/COEP headers; sin ellos, WASM multi-thread no funciona. El workbench puede servir single-threaded pero con menos performance.
- **SolidJS ecosystem** es más pequeño que React/Svelte. Menos componentes pre-hechos.
- **cosmos.gl** + **G6** coexisten: si se cambia el renderer, hay que mantener la abstracción. Coste de portabilidad.
- **WASM bundle size** puede crecer: petgraph + roaring + bitvec + wasm-bindgen = ~500KB-1MB WASM. Hay que tree-shake.

### Métricas de éxito

- El workbench abre un bundle de 10k nodos en <1s en hardware objetivo.
- 0 frames por debajo de 60 FPS durante interacción continua.
- Bundle JS+WASM gzipped <2MB.
- Lighthouse Performance ≥90.
- 0 regresiones detectadas por usuarios en los primeros 3 meses.

## Cómo revertir

| Decisión | Reversión |
|---|---|
| G6 → cosmos.gl primary | Reorden de prioridades. G6 sigue siendo el primero. cosmos.gl es fallback. |
| G6 → Sigma.js | Sigma es para análisis puro, no visualización rica. G6 lo supera en jerárquicos + force. |
| SolidJS → Svelte | Svelte compila a JS. SolidJS es más rápido en runtime. Más trabajo. |
| SolidJS → vanilla TS | Viable si SolidJS no cumple. Más trabajo de mantenimiento. |
| WASM compute → JS algorithms | Degradaría performance budget. Solo acceptable si WASM build es problemático. |
| Apache Arrow → JSON | Degradaría TTFP. Solo si Arrow no se puede integrar. |
| cosmos.gl → Sigma → Cytoscape | Cada uno es más débil que el anterior. cosmos.gl es el máximo rendimiento disponible. |

## Referencias

- `docs/Librerías-visualización-grafos-BI.md` — investigación que sustenta este ADR
- [ADR-013](ADR-013-viewer-ortogonal.md) — viewer ortogonal
- [ADR-019](ADR-019-performance-budget.md) — hard contract
- [ADR-007](ADR-007-modelos-y-renderizadores-de-diagramas.md) — proyecciones
