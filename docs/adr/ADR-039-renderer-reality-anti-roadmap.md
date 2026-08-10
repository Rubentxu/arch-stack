# ADR-039 — Renderer reality + anti-roadmap (decisionesDeferred con reopen triggers)

> **Ciclo:** `m69-arch-stack-product-roadmap-convergence`
> **Estado:** Aceptado
> **Fecha:** 2026-08-09
> **Supersede:** ADR-020 (sección de renderer stack aspiracional)
> **Histórico:** ADR-020 se preserva como registro de la decisión original

## Contexto

ADR-020 (31 de julio de 2026) especifica un stack completo:

```
G6 5.x con WebGPU backend (primary)
cosmos.gl (>100k nodos)
SolidJS
Rust → WebAssembly (petgraph, roaring)
Apache Arrow + TypedArrays
Web Workers + SharedArrayBuffer
RoaringBitmap (Rust → WASM)
```

Este stack fue diseñado cuando el proyecto asumía que archview era un proyecto
separado con un equipo dedicado y presupuesto para investigar WebGPU. La realidad
del producto shipped en ADR-033 (2026-08-06) y el estado actual del código
revelan un stack diferente.

## Renderer realidad — lo que realmente está implementado

El renderer de archview actual es:

```
G6 5.x canvas (drag-canvas / zoom-canvas / drag-element)
├── Sin WebGPU (fallback canvas estándar)
├── Sin WASM (todo TypeScript)
├── Sin Apache Arrow (JSON + TypedArrays vanilla)
├── Sin SharedArrayBuffer (single-threaded)
└── Sin cosmos.gl (G6 built-in + dagre/d3-force para layouts)
```

La realidad técnica:

| Pieza | ADR-020 (aspiracional) | Implementado |
|---|---|---|
| Renderer | G6 5.x WebGPU | G6 5.x canvas |
| Compute | Rust → WASM | TypeScript puro |
| Data | Apache Arrow | JSON + TypedArrays |
| Layout | cosmos.gl + ELK.js | G6 built-in (dagre, d3-force) |
| Threading | Web Workers + SharedArrayBuffer | Single-threaded |
| Selection | RoaringBitmap (Rust→WASM) | JavaScript Set / Array |

### Por qué se eligió el stackcanvas

El stack canvas se eligió por:

1. **Simplicidad de distribución**: un solo binary con assets embebidos, sin WASM.
2. **Compatibilidad**: canvas funciona en todos los navegadores sin COOP/COEP headers.
3. **Suficiencia**: G6 5.x canvas maneja la mayoría de los grafos típicos
   (5k-50k nodos) sin problemas de performance.
4. **Tiempo deTTFP**: el bundle JavaScript es más pequeño sin el overhead de WASM.

## Anti-roadmap — decisiones Deferred con reopen triggers concretos

Las siguientes decisiones fueron evaluadas y postergadas. Se reopeningán
únicamente cuando se cumpla la condición de trigger asociada, medida de forma
objetiva.

### Tabla de anti-roadmap

| Decisión | Estado | Reopen trigger |
|---|---|---|
| **WGPU renderer** | Deferred | Benchmark p99 > ADR-019 budget AND JS/Worker insufficient para el target de 100k nodos |
| **Rust/WASM compute layer** | Deferred | ≥2 consumidores de terceros necesitan shared compute; medido por telemetry anónimo |
| **Apache Arrow** | Deferred | bundle size >X MB medido Y AND JSON parsing bottleneck demostrado por profiling |
| **cosmos.gl** | Deferred | node count >100k AND G6 canvas FPS <30 durante >500ms sostenido |
| **SceneGraph abstraction** | Deferred | ≥3 tipos de vista necesitan shared scene model; demostrado por repetición de lógica de traducción |
| **WIT Plugin SDK** | Deferred | ≥1 consumidor de terceros registrado; medido por API key count |
| **Event sourcing / replay** | Deferred | Temporal diff es un requisito explicitly shipped; medido por feature flag activo en producción |
| **Architecture Lab (forks hypothétiques)** | Deferred | ≥3 requests de usuarios únicos; medido por issue count |
| **Full 9-agent catalog** | Deferred | 2/2 agentes desplegados tienen >X% adopción medida por usage telemetry |
| **Desktop shell (Tauri)** | Deferred | browser-only es el blocker para ≥1 user segment; medido por surveys o GitHub issues |

### Condiciones de reopen — detalle técnico

**WGPU → reopen cuando:**
```
benchmark p99 render time > 16ms (60 FPS budget)
AND
JS Worker profiling muestra que el hot path (layout + render) no cabe en el
budget con las optimizacionesJS disponibles
AND
el equipo tiene bandwidth para mantener un segundo renderer (canvas + WGPU)
```

**Rust/WASM → reopen cuando:**
```
≥2 consumidores de terceros incluyen archview en su stack
AND
el shared compute scenario (e.g., corpus de grafos compartido entre consumers)
es un requisito explicitly solicitado
```

**Apache Arrow → reopen cuando:**
```
bundle size > 10MB ( gzip) medido en el percentil 95
AND
profiling del hot path muestra que JSON serialization/deserialization es el
bottleneck top-1 (>30% del tiempo total)
```

**cosmos.gl → reopen cuando:**
```
node count > 100k nodos en un solo grafo
AND
FPS < 30 durante más de 500ms consecutivos en el canvas de G6
AND
el bottleneck es el render (no el layout computation)
```

**SceneGraph abstraction → reopen cuando:**
```
≥3 view types distinta requieren el mismo scene model abstraction
AND
la lógica de traducción (view model → G6 data) se repite en ≥3 archivos
AND
un ADR propone formalmente la abstracción con un contract de consumers
```

**WIT Plugin SDK → reopen cuando:**
```
≥1 third-party consumer tiene un use case que requiere plugin extensibility
AND
el consumer proporciona un ADR-style justification con sus requirements
```

**Event sourcing → reopen cuando:**
```
un milestone explicitly incluya "temporal diff" o "replay" como requisito
AND
el product owner firma off en el additional complexity cost
```

**Architecture Lab → reopen cuando:**
```
≥3 usuarios unique submit un feature request o issue que requiere forks/hypotheticals
AND
la feature no puede resolverse con el workflow actual de archctl/archview
```

**Full 9-agent catalog → reopen cuando:**
```
los 2 agentes desplegados (Architecture + Projection) tienen >50% adoption
medido por weekly active users
AND
≥1 de los 7 agentes deferred tiene un customer-requested use case
```

**Tauri desktop → reopen cuando:**
```
≥1 user segment (documented via survey o GitHub issue) dice que browser-only
es un blocker para su workflow
AND
el team tiene bandwidth para mantener un segundo distribution channel
```

## §Historical Rationale

> Esta sección preserva verbatim el rationale de ADR-020 para el stack
> aspiracional original.

ADR-020 eligió G6 WebGPU + Rust/WASM + Apache Arrow por tres razones:

1. **Performance a escala**: ADR-019 establece un budget de TTFP <1s para 10k
   nodos. En 2026, G6 WebGPU + WASM era la combinación más rápida disponible.

2. **Equipo pequeño pero ambicioso**: el equipo de archview iba a construir el
   renderer desde cero para un target de 100k nodos. Rust/WASM parecía la
   inversión correcta.

3. **Lessons learned de BI**: el documento `Librerías-visualización-grafos-BI.md`
   recomendaba evitar React/Svelte/Sigma y elegir G6 + cosmos.gl para grafos
   grandes.

La realidad del shipped product (2026-08-06) usó el stackcanvas por velocidad
de distribución y simplicidad. El anti-roadmap asegura que las decisiones
aspiracionales no se olvidan — se reopeningán cuando las condiciones lo justifiquen.

## Consecuencias

### Positivas

- El producto shipped usa un stack que funciona y es fácil de mantener.
- El anti-roadmap documenta qué se evaluó y por qué se deferró.
- Los reopen triggers son medibles — no hay decisiones arbitrarias.

### Negativas

- El stackcanvas tiene limits de performance para grafos muy grandes (>100k nodos).
- Los users que necesitan WASM compute no lo tienen hoy.

## Cómo revertir este ADR

Este ADR se revierte cuando se implements el stack completo de ADR-020
y se mide que el budget de ADR-019 se cumple de forma sostenida.

## Referencias

- [ADR-020](ADR-020-renderer-stack.md) — renderer stack aspiracional original
- [ADR-019](ADR-019-performance-budget.md) — hard contract
- [ADR-033](ADR-033-archctl-view-embedded-workbench.md) — archctl view embedded
- `docs/Librerías-visualización-grafos-BI.md` — investigación de librerías
