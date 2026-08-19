# `archview` — Audit + Plan de Rediseño (Fase 5 / M17.1+)

> Auditoría brutal del workbench, escrita tras feedback del usuario
> ("interfaz poco intuitiva, no se ven grafos, horriblemente feo") con
> el vocabulario de la skill `impeccable` y un mapa de sprints
> priorizados. El objetivo: **convertir archview en una herramienta
> útil para visualizar arquitectura, no un placeholder visual.**

---

## 1. Resumen ejecutivo (lo que falla)

| Categoría | Falla | Impacto | Evidencia |
|---|---|---|---|
| **Visualización** | Las vistas específicas no usan `@antv/g6` (instalado, integrado, sin uso en las vistas). Todo es `<ul>`/`<table>`/`<div>` con texto. | **Crítico** — para arquitectura un humano necesita ver relaciones, no listas. | `C4View.tsx:60-120` (renders `<ul>`), `CallGraphView.tsx:14` ("No G6 canvas here — text-based list view (M17.2 MVP). M17.2.1 can upgrade to G6 with hierarchical layout.") |
| **Onboarding** | Empty state solo dice "Load a bundle from the top bar to start exploring." sin indicar qué es un bundle, de dónde sale, ni qué pasa tras cargarlo. | **Alto** — usuario nuevo no sabe qué hacer. | `App.tsx: empty-canvas`, screenshot `01_empty_state.png` |
| **Mental model** | El C4 se modela como niveles (Context/Container/Component/Code) que se descubren por drill-in. Pero la vista renderiza todos los niveles a la vez en columnas paralelas, sin indicar visualmente que drill-in es **moverse de nivel**. | **Alto** — la metáfora "zoom" se pierde. | `C4View.tsx:60-150`, screenshot `02_bundle_loaded.png` (3 columnas simultáneas sin jerarquía visual) |
| **Design system** | `styles.css` línea 1: "minimal MVP styles. Will be replaced with a design system in M17.1+". 4 neutros + 1 acento, sin escalas, sin tipografía, sin spacing scale, sin modo light. | **Medio** — la estética es pobre pero funcional. | `archview/src/styles.css:1-15` |
| **Sidebar** | 404 líneas, layout vertical sin jerarquía visual, secciones (Bundle/Selection/Actions/Evidence/Relations) sin separación clara. | **Medio** — funciona pero abruma. | `archview/src/components/Sidebar.tsx` |
| **Tipografía** | `font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif` (system stack). Sin escalas, sin jerarquía. | **Bajo** — funciona, sin personalidad. | `styles.css:14` |
| **Color** | Dark mode único, un solo acento (azul `--accent: #5b8def`). No hay semántica de color (C4 levels, drift, error, warning). | **Medio** — falta lenguaje visual. | `styles.css:6-13` |
| **Responsive** | Sin media queries en 1594 líneas de CSS. Probablemente roto en mobile/tablet. | **Medio** — depende del device. | `styles.css` (grep sin `@media` más allá de `prefers-reduced-motion`) |
| **Motion** | Prácticamente cero. Un `transition: background 0.15s` en botones, nada más. | **Bajo** — funciona, falta deleite. | `styles.css` (grep `transition|animation`) |
| **Componentes reutilizables** | 0 primitives; cada vista tiene su propio JSX inline. Sin `<Button>`, `<Card>`, `<EmptyState>`, etc. | **Medio** — DRY violado. | `archview/src/views/*.tsx` |

**Tasa global de cumplimiento de bans de impeccable**: 8/10.
- ❌ "Side-stripe borders": aparece en `.breadcrumbs li button` (1px) y `.sidebar-selection` (1px) — borderline, no es un fail claro.
- ❌ "Identical card grids" en PackageView (`PackageView.tsx:75+`): card grid repetido con misma forma. Candidato a `distill`.
- ⚠️ "Tiny uppercase tracked eyebrow": el `Sample C4 container (archctl)` no es eyebrow pero los `<h3>c4-level-title</h3>` en C4View son pequeñas mayúsculas implícitas en `Context (1)`, `Container (3)` — efecto visual de eyebrow sin serlo. Borderline.

---

## 2. Por qué el workbench no se entiende (mecánica)

El usuario reportó: **"no acabo de entender nada la mecánica de uso"**. Mapeo del journey humano actual:

1. **Aterrizaje** — abres `http://127.0.0.1:18777`. Ves 7 botones "Sample…" + un input vacío + un toggle "drift mode". El texto central: "Load a bundle from the top bar to start exploring." No explica qué es un bundle, qué hace cada sample, ni qué pasa al cargar uno.
2. **Selección** — click en un sample. Tras 1-2s aparece: breadcrumb "All systems", 3 columnas simultáneas ("Context (1)", "Container (3)", …) y una sidebar con metadatos del bundle. **No hay señal visual de qué es cada columna** (¿es un nivel? ¿es una vista?).
3. **Drill-in** — click en un container → la sidebar se llena. Pero "drill in" es un hint label, no un botón obvio. El usuario probablemente no sabe que puede hacer zoom.
4. **Selección de nodo** — la sidebar dice "Select a node to inspect its evidence" — pero el usuario ya clickeó un container. **El feedback de selección es silencioso**: el `c4-element` seleccionado no cambia de color, no hay ring, no hay zoom-to-fit.

**Problemas concretos** (cada uno se resuelve con un sprint):

- **Empty state genérico** (no explica) → Sprint C
- **Sin jerarquía visual de niveles** (3 columnas paralelas, no una pirámide drill-down) → Sprint A (grafo G6 con layout dagre vertical)
- **Selección sin feedback** (no ring, no zoom-to-fit) → Sprint A
- **Sidebar abrumador** (404 líneas, sin secciones visibles) → Sprint B
- **"Drill in" como hint** (no como affordance obvia) → Sprint A (botón de zoom explícito o click-to-zoom)

---

## 3. Plan de rediseño (3 sprints)

### Sprint A — Activar grafos G6 con layout jerárquico (alto impacto, ~1 sesión)

**Objetivo**: hacer que las vistas específicas (C4, CallGraph, ClassDiagram, Package, Impact, Drift, Sequence) rendericen con `@antv/g6` ya integrado, con layout jerárquico para C4.

**Cambios**:
- `C4View.tsx`: sustituir el render de `<ul>` por un canvas G6 con `layout: { type: 'dagre', rankdir: 'TB' }`. Drill-in = foco en nodo + fit-view con animación. C4 levels como colores semánticos (Context=primario, Container=secundario, Component=terciario, Code=cuaternario).
- `CallGraphView.tsx`: cambiar comentario M17.2 MVP → activar `GraphRenderer` con `layout: { type: 'force' }` o `d3-force`. El render actual con `<ul>` se descarta.
- `ClassDiagramView.tsx`: G6 con `layout: { type: 'dagre' }` y nodos UML compartments.
- `ImpactView.tsx` y `PackageView.tsx`: G6 con `layout: { type: 'concentric' }` (radial blast-radius) y `force` respectivamente.
- `App.tsx`: cuando `kind === 'c4'` y `App.tsx` tiene el render de `GraphView` fallback, **decidir**: ¿usar siempre G6 vía `GraphRenderer`? Sí — un solo renderer para todos los tipos.

**Compatibilidad**:
- `bundle/loader.ts`: no cambia.
- `renderer/g6.ts`: extender para aceptar `layout` y `node-color-by-kind` (sin tocar el wrapper base, solo opciones).
- `__tests__`: los 4 tests baseline no se rompen porque mockean `renderer/g6` (ver `__tests__/App.navigation.test.tsx:157`).

**Criterios de done**:
- C4 sample (`c4-container.json`) renderiza como grafo jerárquico, NO como 3 columnas `<ul>`.
- Click en un container = selección visible (ring, fit-view, sidebar fill).
- Drill-in = zoom animado al nivel hijo.
- Suite `pnpm test` + `pnpm build` + `pnpm lint` verdes.

**Riesgo**: el rendimiento con bundles >10k nodos. Mitigación: layout en Web Worker con `comlink` o `worker-loader` (TODO oficial en AGENTS.md). Posponerlo a un Sprint A.1 si bloquea.

---

### Sprint B — Design system + estética (medio impacto, ~1 sesión)

**Objetivo**: pasar de "minimal MVP" a tokens cohesivos con paleta semántica.

**Cambios**:
- Crear `archview/src/styles/tokens.css` (o `theme.ts` para Solid):
  - **Color** (OKLCH):
    - Neutrales: `--bg-0` (#0e1116) → `--bg-5` (más claro) con 5 stops, cada uno con chroma hacia el brand.
    - Acento (azul): `--accent-1` → `--accent-5` para estados.
    - Semánticos C4: `--c4-context`, `--c4-container`, `--c4-component`, `--c4-code` (cada uno con -bg y -fg).
    - Estado: `--ok`, `--warn`, `--err`, `--info`.
  - **Tipografía**: 1 familia sans (Inter o system stack) + 1 mono (JetBrains Mono o system mono). Escala: `--fs-xs` 12px → `--fs-2xl` 32px (con `clamp()` para responsive).
  - **Spacing**: escala 4-8-12-16-24-32-48-64.
  - **Radius**: 4-8-12 (sm, md, lg).
  - **Shadow**: 4 niveles (xs, sm, md, lg) con alpha modesto.
- Crear primitives Solid: `<Button>`, `<Card>`, `<Tag>`, `<EmptyState>`, `<Tooltip>` en `archview/src/components/primitives/`.
- Modo light: switch `color-scheme: light dark` con `@media (prefers-color-scheme: light)` ajustando los tokens neutros.

**Criterios de done**:
- `pnpm test` verde (los primitives tienen tests).
- Contraste body text ≥4.5:1 (medido con `axe` o DevTools).
- Un cambio de `--accent` propaga a todos los usos.
- Modo light funcional.

---

### Sprint C — Onboarding y discoverability (alto impacto, ~media sesión)

**Objetivo**: que un humano nuevo entienda qué hace el workbench en 30s.

**Cambios**:
- **Empty state**: en lugar de "Load a bundle from the top bar…", una sección con 3 cards (Sample / URL / About) y un comando `archctl view --cwd <repo>` destacado.
- **Tooltips**: todos los botones "Sample…" con tooltip explicando qué bundle cargan.
- **Breadcrumb de navegación**: el "All systems" debe verse como un link/button que regresa, no como un texto disabled.
- **Tour opcional**: botón "Take a 30s tour" que abre un overlay explicando la mecánica.
- **README del workbench**: nuevo `archview/README.md` con 1 screenshot + 1 párrafo + 3 comandos.

**Criterios de done**:
- Usuario nuevo ve empty state y entiende qué hacer sin leer docs.
- Tooltips accesibles (no `title` HTML, sino componente con role/aria).
- Tour cubre: cargar sample → ver grafo → seleccionar nodo → ver evidencia → drill-in.

---

## 4. Lo mínimo más impactante (si solo haces UN sprint)

**Sprint A** (activar G6) es el cambio más impactante porque:

- Resuelve la queja principal: "no se ven grafos".
- Es el gap técnico más obvio: el motor está instalado, el renderer está escrito, falta conectarlo a las vistas.
- El AGENTS.md oficial lo lista como TODO.
- 1 sesión de trabajo.
- Compatible con la arquitectura existente (SolidJS, `bundle/loader.ts` no cambia).

Sprint C (onboarding) tiene ROI similar pero depende de que A esté hecho: si los grafos no funcionan, el onboarding miente.

Sprint B (design system) tiene ROI alto a largo plazo pero no desbloquea ningún insight arquitectónico inmediato.

**Mi recomendación**: Sprint A → Sprint C → Sprint B (en ese orden).

---

## 5. Riesgos y dependencias

- **G6 + Solid reactivity**: hay un patrón conocido de memory leak si `GraphRenderer` no se destruye al desmontar. Verificar `renderer/g6.ts` con `dispose()`.
- **Web Worker para layout**: el TODO oficial habla de ELK.js en worker. Si se pospone, paquetes >1k nodos pueden colgar la UI thread. Mitigación: documentar el límite actual (~500 nodos en MVP).
- **Cross-view consistency**: si Sprint A se aplica a algunas vistas y no otras, el workbench se siente inconsistente. Mejor aplicar a todas o un subset explícito.
- **Bundle format**: `viewer-bundle` schema 1.1.1 está estable; no requiere cambios.

---

## 6. Próximo paso

**Decisión del usuario**: ¿qué sprint priorizamos?

| Opción | Esfuerzo | Impacto | Bloquea |
|---|---|---|---|
| A — Activar G6 | 1 sesión | Crítico (resuelve "no grafos") | B y C se benefician |
| B — Design system | 1 sesión | Medio (estética) | — |
| C — Onboarding | 0.5 sesión | Alto (UX new user) | Depende de A |
| A + C en una | 1.5 sesiones | Crítico + Alto | B puede esperar |
| Todo | 2-3 sesiones | Completo | — |

Si confirmas **A** (o **A+C**), abro un cambio `m17-graph-views` y lo implemento en esta misma sesión, commiteado en PR atómico. Si quieres que primero validemos algo (ej. prototipo de C4 con G6 antes de tocar las otras vistas), lo hago en un sub-PR de "demo graph".
