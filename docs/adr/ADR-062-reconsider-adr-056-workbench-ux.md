# ADR-062 — Reconsiderar ADR-056: Workbench UX parcial (items 31–33)

> **Estado:** Aceptado — 2026-08-19
> **Supersedes**: ADR-056 (reconsideración de alcance — no de triggers)
> **Baseline**: `main@d209219` (v1.67.0)
> **Ámbito**: Desbloqueo de items 31–33 Wave 3 (workbench UX sin LensSpec)
> **Propietario de decisión**: maintainers de Arch Stack

## Resumen ejecutivo

ADR-056 (Moldable Architecture Workbench) fue diferido el 2026-08-18 con
un criterio de entrada ("≥2 consumers with LensSpec-translatable
duplication OR measured need") estructuralmente insatisfacible para un
producto de un mantenedor sin consumers externos. Su propia estrategia de
migración ("Cross-view identity → action palette → semantic zoom → lens
recommendation") ordena un camino incremental cuyo alcance mínimo **no
requiere la maquinaria LensSpec (P3-05)**. Este ADR reabre ADR-056 en
alcance parcial (pasos 1–3) y mantiene deferida la lens recommendation.

## Contexto

El explore `wave-3-workbench-ux` (2026-08-19) verificó con evidencia:

- El bundle ya materializa la jerarquía C4 (`level`, `parentId` en
  `archview/src/types.ts:25-31`) y preserva la evidencia (`R1`).
- `GET /api/export?selector=<c4-kind>:<id>` ya acepta selectores por nivel
  (`archctl/src/view.rs:112-168`, `selector.rs` grammar).
- El patrón HTTP cliente↔server existe (`archview/src/lib/workspace.ts`)
  con seam de test.
- `archview/AGENTS.md` documenta "M17.1" como próximo ciclo con
  exactamente este trabajo (semantic zoom C4 + sidebar tabs) — necesidad
  medida interna del producto.
- El item 29 (read-only) se entregó en v1.61.0; sin navegación cross-view
  ni acciones de nodo, el workbench no cierra su loop de exploración.

## Decisión

1. ADR-056 se reabre en **alcance parcial**, siguiendo su propia estrategia
   de migración:
   1. **Cross-view identity** — `NavigationTarget` sobre IDs canónicos ya
      estables + pila de navegación con breadcrumbs y back/forward.
   2. **Action palette** — acciones fijas por nodo: copy id, zoom in/out
      C4, explain (vía endpoint nuevo `GET /api/explain` en
      `archctl view`), relations (aristas del nodo desde el bundle).
   3. **Semantic zoom C4** — Context↔Container↔Component por re-export
      con selector existente. Sin cambios al grafo canónico ni al schema.
2. **P3-05 lens recommendation** (query→projection composition, XL)
   permanece **deferida** con el trigger original de ADR-056.
3. El nivel "Code" (C4→class-diagram) queda **fuera de alcance**: exige un
   endpoint nuevo de proyección code (`file:` selector existe en
   `archctl code class-diagram` pero el mapping componente→archivos es
   heurístico). Reopen: ≥1 consumidor con necesidad real de drill-down a
   código.
4. Los criterios de aceptación de ADR-056 §Verificación aplican en alcance
   parcial: entidad cruza vistas (niveles C4), breadcrumbs, back/forward
   estable, budget 10k nodos.

## Consecuencias

- **Positivas**: el workbench se convierte en entorno de comprensión
  navegable sin big bang; cada paso es reversible e independiente.
- **Negativas**: estado de navegación cliente añade complejidad (stack +
  breadcrumbs); explain en strict bundles debe degradarse (el receptor no
  tiene store local) — mismo patrón que source preview/editor handoff.

## Alternativas consideradas

- **A) Standalone sin ADR**: rechazada — ADR-056 §Decisión lista
  literalmente "Semantic zoom bidireccional, action palette".
- **B) Reopen completo incluyendo P3-05**: rechazada — big bang que
  ADR-056 §Fuerzas rechaza explícitamente.

## Referencias internas

- [ADR-056](ADR-056-moldable-architecture-workbench.md) — ADR reconsiderado
- [ADR-061](ADR-061-reconsider-adr-055-sanitized-bundle.md) — precedente de
  reconsideración
- `sddk/wave-3-workbench-ux/explore-report.md`

## Changelog

- 2026-08-19 | proposed | ADR-062 creado a partir del explore
  `wave-3-workbench-ux`.
