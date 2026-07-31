# ADR-019 — Performance budget (hard contract)

**Estado:** Aceptado
**Fecha:** 31 de julio de 2026
**Aplica a:** `archview` (Code Knowledge Graph Workbench), con implicaciones para `archctl` (generador de bundles)
**Refuerza:** ADR-013 (viewer ortogonal), ADR-020 (renderer stack)
**Relacionado:** ADR-007 (diagramas como proyecciones), ADR-011 (renderers locales)

## Contexto

`archview` consume bundles JSON que pueden contener grafos de cientos a millones de elementos. Sin un contrato de performance explícito, el workbench corre el riesgo de:

- Tornarse inutilizable con datasets medianos (~5k nodos).
- Tener latencia visible en interacción (jank, scroll stuttering).
- Consumir memoria excesiva en laptops de gama media.
- Bloquear el hilo principal con cómputo pesado.

La re-evaluación del roadmap contra `docs/Librerías-visualización-grafos-BI.md` (julio 2026) identificó que el target real es un workbench para developers/arquitectos, no un dashboard BI. Este público es **extremadamente sensible a la latencia** — un dev que interactúa con el grafo 30 veces por minuto no tolera un frame de 200ms.

## Decisión

`archview` opera bajo un **hard contract de performance**. Cada métrica es un test que corre en CI; una regresión fuera de presupuesto bloquea el merge.

### Performance budget (target contract)

| Métrica | Target | Hardware objetivo | Anti-pattern que rompe |
|---|---|---|---|
| **TTFP** (Time To First Paint, bundle 10k nodos) | **<1s** | Mid-range laptop (M1/M2 base, Intel i5-1135G7) | per-node JS objects, JSON parse, sync initial layout |
| **TTFI** (Time To First Interaction) | **<1.5s** | mismo | initial layout bloqueante en main thread |
| **TTFP massive** (bundle 100k nodos) | **<3s** | mismo | loading full dataset; sin streaming/proyección |
| **Pan/zoom latency** | **<16ms** (60 FPS) | mismo | layout recalc on viewport change, sync queries |
| **Filter response** (categórico, rango) | **<50ms** | mismo | full dataset iteration; JSON serialization |
| **Selection latency** (click → highlight) | **<16ms** (1 frame) | mismo | O(n) CPU picking; per-node iteration |
| **Hover latency** | **<16ms** (1 frame) | mismo | layout recalc on hover; event handling en main thread |
| **Layout convergence** (10k nodos) | **<2s** | mismo | CPU-only force-directed; sync en main thread |
| **Layout convergence** (100k nodos) | **<10s** (WebGPU compute) | dedicated GPU | CPU-only; sin compute shaders |
| **Memory** (10k nodos) | **<200MB** | mismo | per-node JS objects; no buffer reuse |
| **Memory** (100k nodos) | **<500MB** | mismo | JSON; per-property strings; no CSR/Arrow |
| **CPU during interaction** (laptop gama media) | **<30%** | mismo | sync layout; JS-based algorithms |
| **Data transfer** (10k nodos, cold load) | **<1MB compressed** | n/a | JSON; per-property strings |
| **No long tasks** (>50ms) durante interacción | **0** | n/a | synchronous heavy work en main thread |
| **Bundle size** (JS + WASM, gzipped) | **<2MB** | n/a | monolithic bundle; no code splitting |

### Anti-patterns (reglas explícitas)

Cualquier PR que introduzca uno de los siguientes patrones requiere justificación explícita + plan de mitigación:

1. ❌ **Per-node JavaScript objects** — usar Struct-of-Arrays (typed arrays) o `slotmap` para node IDs.
2. ❌ **JSON serialization** para datasets >1000 nodos — usar Apache Arrow o MessagePack.
3. ❌ **Single-threaded layout** para >10k nodos — Web Worker o GPU compute.
4. ❌ **React** para UI shell — usar SolidJS (fine-grained reactivity) o vanilla TS.
5. ❌ **Sprotty** — descartado por el doc `Librerías-visualización-grafos-BI.md` ("no escala para grafos grandes").
6. ❌ **Per-frame re-renders** — usar signals (Solid) o subscripciones granulares.
7. ❌ **Synchronous heavy computation** en main thread — Web Worker o WASM.
8. ❌ **Loading full dataset** para queries — usar proyecciones y paginación.
9. ❌ **Canvas2D** para grafos >5k nodos — WebGL2/WebGPU con instanced rendering.
10. ❌ **Text labels rendered per-frame** — SDF o atlas de glyphs cacheados.
11. ❌ **CPU picking** (iterar todos los nodos) — GPU picking (framebuffer invisible).
12. ❌ **Re-render all on state change** — selectores granulares + memoization.

### Performance budget enforcement

1. **CI gate**: una suite de benchmarks corre en cada PR. Si una métrica se degrada >10% vs `main`, el PR no se mergea.
2. **Bundle size limit**: el bundle JS+WASM gzipped no puede exceder 2MB. CI bloquea el merge.
3. **Lighthouse score**: el workbench debe mantener Lighthouse Performance ≥90 en el bundle de 10k nodos.
4. **Profiling on regression**: si una métrica falla, el CI adjunta un flamegraph al PR.

### Benchmarking dataset (canonical)

Para tests reproducibles, `archview` mantiene un dataset canónico:

```text
benchmarks/datasets/
├── small-10k.json    (~10k nodos, 30k relaciones)
├── medium-100k.json  (~100k nodos, 300k relaciones)
└── large-1m.json     (~1M nodos, 3M relaciones)
```

Cada dataset es un grafo sintético generado proceduralmente con propiedades realistas. Los benchmarks corren contra estos datasets y reportan todas las métricas del budget.

### Casos de uso objetivo (validation)

| Caso | Métrica crítica | Target |
|---|---|---|
| Abrir un bundle de 10k nodos | TTFP | <1s |
| Hacer zoom en una región con 1k nodos | FPS | 60 |
| Filtrar por equipo + entorno | Filter latency | <50ms |
| Marcar 500 nodos (Ctrl-click + drag) | Selection latency | <100ms end-to-end |
| Generar call graph de un módulo | Layout time | <2s para 10k |
| Drift detection C4 vs actual | Computation | <3s para 100k |
| Carga 100k + interacción continua | Sustained FPS | ≥30 |

## Consecuencias

### Positivas

- El workbench es **predecible** y **zero-jank** a cualquier nivel de complejidad dentro del budget.
- Las decisiones arquitectónicas se justifican con métricas concretas, no con preferencias.
- El CI bloquea regresiones antes de que lleguen a usuarios.
- Los devs pueden usar el workbench con confianza — saben que no se va a atascar con sus repos reales.

### Negativas

- Limita opciones de implementación: si una librería de UI no cumple el budget, no se puede usar.
- Requiere inversión inicial: implementar WebGPU compute, GPU picking, Web Workers con SharedArrayBuffer lleva tiempo.
- Costo de CI: el benchmark suite corre en cada PR (puede tomar minutos).
- Tamaño del bundle: 2MB es un techo apretado si se incluyen todas las dependencias.

### Métricas de éxito

- El workbench abre un bundle de 10k nodos en <1s en hardware objetivo.
- 0 frames por debajo de 60 FPS durante interacción continua con 10k nodos.
- Bundle JS+WASM gzipped <2MB en la v1.0.
- 0 regresiones de performance detectadas por usuarios en los primeros 3 meses post-launch.

## Cómo revertir

| Decisión | Reversión |
|---|---|
| Performance budget | Incrementar targets (con justificación). No aceptable degradar el budget sin trade-off documentado. |
| SolidJS como UI framework | Migrar a Svelte (más liviano) o vanilla TS. Más trabajo. |
| G6+WebGPU como renderer | Migrar a cosmos.gl (massive) o Sigma (análisis). Ambos cumplen performance, son intercambiables. |
| WASM compute layer | Degradar a JS algorithms — implica perder el budget de latency. |
| Hard contract CI gate | Soft warning en lugar de bloqueante. Aceptable solo si los budgets son inalcanzables. |

## Referencias

- `docs/Librerías-visualización-grafos-BI.md` — investigación que sustenta este ADR
- [ADR-013](ADR-013-viewer-ortogonal.md) — viewer ortogonal
- [ADR-020](ADR-020-renderer-stack.md) — stack específico
- Sección 6 del doc — "Objetivos de rendimiento" (mismas métricas)
