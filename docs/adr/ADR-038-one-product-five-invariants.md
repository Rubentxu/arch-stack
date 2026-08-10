# ADR-038 — Un producto, cinco invariantes (arch-stack identity)

> **Ciclo:** `m69-arch-stack-product-roadmap-convergence`
> **Estado:** Aceptado
> **Fecha:** 2026-08-09
> **Supersede:** ADR-013 (sección "repositorio separado" contradicha por ADR-033 + código)
> **Complementa:** ADR-033 (`archctl view` embedded workbench)

## Contexto

ADR-013, en su revisión del 31 de julio de 2026, afirma en L6:

> **Proyecto:** `archview` (repositorio separado, NO parte de `archctl`)

Esta afirmación fue correcta en el momento de escribirla. Sin embargo, la decisión
implementada en ADR-033 (`archctl view` embedded workbench, 2026-08-06) y
confirmada por el código en `archctl/src/commands/view.rs` y `rust-embed` establece
una realidad diferente: **arch-stack es un solo producto** con `archctl` (CLI Rust)
y `archview` (workbench SolidJS + G6) distribuidos juntos.

La separación de repositorio no existe en el producto shipped. archctl y archview
se distribuyen como un único binario `archctl` que embebe el workbench via
`rust-embed` en el mismo binary artifact.

## Decisión

`arch-stack` es **un producto** compuesto por dos componentes mecánicamente
acoplados:

```
arch-stack (un producto, un binary)
├── archctl (CLI sidecar, Rust)
│   ├── LadybugDB (persistencia)
│   ├── extractores de código (AST, call graph, etc.)
│   └── exportador de bundles (diagram export)
└── archview (workbench embebido via rust-embed)
    ├── Renderer G6 5.x (canvas)
    └── UI SolidJS
```

**Comando de entrada:** `archctl view` (no `archctl` + `archview` por separado)

## Las cinco invariantes de arch-stack

Todo en el producto arch-stack respeta estas cinco invariantes:

### Invariante 1 — Grafo canónico (LadybugDB)

El estado canónico del sistema es el grafo en LadybugDB. No existe un "segundo
grafo" en memoria del workbench. archview consume proyecciones inmutables del
grafo (DiagramProjection bundles) y no escribe de vuelta al grafo canónico.

### Invariante 2 — Evidencia por nodo y arista

Cada elemento del grafo canónico tiene evidencia verificable. La evidencia es
primera-class: `evidence_refs` en cada nodo/arista, persistida en LadybugDB,
consumida por el workbench. Sin evidencia no hay confianza.

### Invariante 3 — Persistencia solo XDG

El estado persistente vive exclusivamente en XDG (`~/.local/share/archctl/`).
No se usa `localStorage`, `IndexedDB`, ni cookies. Los archivos de workspace
(bundle, viewport, filtros) se restauran entre ejecuciones de `archctl view`
porque el puerto efímero de cada invocación es diferente — XDG es la fuente
de verdad para el estado de la sesión de trabajo.

### Invariante 4 — Apply cosmético (nunca muta el grafo semántico)

Los cambios que el usuario hace en el workbench (mover nodos, colapsar grupos,
cambiar etiquetas) se expresan como ChangeSet y se aplican vía
`archctl diagram apply --changes`. Estos cambios son **cosméticos**: alteran
la proyección visual pero nunca mutan el grafo canónico. El grafo semántico
(estructura del código, relaciones, evidencias) solo cambia via los comandos
de extracción (`archctl code c4 discover`, `archctl code call-graph`, etc.).

### Invariante 5 — Renderers locales (sin red)

Los renderers (G6 canvas, Mermaid, PlantUML) se ejecutan localmente en el
navegador del usuario. No hay servidor de rendering, no hay CDN para los
assets del workbench, no hay network calls en el hot path. Esto cumple
ADR-010 (sin daemon) y ADR-011 (renderers locales y bloqueo de servicios
públicos).

## Relación con ADR-013 y ADR-033

ADR-013 se **supersede parcialmente**: la sección "repositorio separado" y la
afirmación de que archview es un proyecto independiente ya no reflejan el
producto shipped. El cuerpo del ADR-013 (arquitectura del bundle, ChangeSet,
contrato de proyección) sigue válido.

ADR-033 **complementa** este ADR: ADR-033 documenta la decisión de embeber
el workbench como servicio local one-shot. Este ADR documenta la consecuencia
de esa decisión: un solo producto, no dos.

## Consecuencias

### Positivas

- El usuario final tiene un solo comando (`archctl view`) para ejecutar todo
  el producto.
- La distribución es simple: un binary artifact por OS.
- La consistencia de versión entre archctl y archview está garantizada por
  construcción (mismo binary).
- Las 5 invariantes definen un contrato claro para cualquier código futuro.

### Negativas

- Los contributors que querían desarrollar archview independientemente tienen
  que adaptarse al binary embebido (pero pueden usar `cargo run -- view`
  para desarrollo local).

## Métricas de éxito

- `archctl view` abre el workbench embebido en <1s en el puerto local.
- El bundle que archview consume es generado por el mismo binary.
- Zero network calls en el hot path del renderer.
- XDG workspace state se restaura correctamente entre ejecuciones de `archctl view`.

## Cómo revertir

Si en el futuro un segundo consumidor necesita archview como library/repository
independiente:

1. Extraer `archview` a un segundo crate/paquete npm.
2. Publicar como `archview` independiente con su propio ciclo de release.
3. Modificar `archctl view` para descargar/fetchear el archview bundle desde
   una URL configurable.
4. Crear un ADR que superseda este, documentando la decisión de extraer.

---

## Referencias

- [ADR-013](ADR-013-viewer-ortogonal.md) — viewer ortogonal (se supersede parcialmente)
- [ADR-033](ADR-033-archctl-view-embedded-workbench.md) — archctl view embedded
- [ADR-019](ADR-019-performance-budget.md) — performance budget
- [ADR-010](ADR-010-concurrencia-ladybugdb.md) — sin daemon
- [ADR-011](ADR-011-renderers-locales-y-bloqueo-de-publicos.md) — renderers locales
