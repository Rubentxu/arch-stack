# Roadmap de consolidación — H5 a H8

Este roadmap es un **delta propuesto** sobre el roadmap existente. Mantiene los
horizontes históricos y añade outcomes centrados en fiabilidad, arquitectura y utilidad.

## H5 — Stabilization & Trust Boundary

**Objetivo:** producto compilable, distribuible y seguro antes de crecer.

### P0.1 LadybugDB compatibility boundary
- encapsular crate/native/ABI/toolchain;
- `doctor --scope storage`;
- pin de artefacto nativo coherente;
- build de cada plataforma en runner compatible;
- matriz de compatibilidad explícita.

### P0.2 Plugin hardening
- corregir namespace XDG;
- crear install root antes de staging;
- value objects `PluginAuthor/PluginName/PluginVersion`;
- rechazo de path traversal;
- checksum obligatorio para remoto;
- unpack seguro;
- trust metadata.

### P0.3 Documentation integrity
- resolver IDs ADR duplicados sin romper enlaces;
- `check-adr-integrity`;
- licencia raíz coherente con Cargo/README;
- referencias ADR verificables.

### P0.4 Pre-merge CI
PR gate rápido y determinista; post-merge conserva benchmarks costosos.

**Exit H5:** release reproducible + security/integrity gates verdes.

---

## H6 — Enforced Hexagonal Architecture

**Objetivo:** transformar la hexagonal conceptual en propiedad del build.

- composition root explícito;
- módulos por bounded capability;
- store → repositories/ports;
- extractors por lenguaje;
- Capability Registry;
- contract tests;
- Architecture Fitness Gate.

Boundaries objetivo:

```text
arch-model
arch-analysis
arch-knowledge
arch-projection
arch-workbench
arch-distribution
archctl
archview-contract
```

**Exit H6:** imports prohibidos fallan antes de merge y handlers CLI no construyen
infraestructura fuera del composition root.

---

## H7 — Explainable Architecture Intelligence

**Objetivo:** que el grafo sea herramienta de razonamiento verificable.

- Architecture Diff reutilizando `cognitive/delta.rs` y Drift/Impact;
- Explain/provenance;
- Confidence & Coverage;
- Intent vs Reality / Fitness;
- Task Context Compiler extendiendo `cognitive/context.rs`;
- Evidence Fusion.

**Exit H7:** un PR obtiene diff + policy report + context package con provenance sin
servicio externo.

---

## H8 — Moldable Architecture Workbench

**Objetivo:** representación adaptable a pregunta, selección, escala y tarea.

- Git-linked snapshots;
- sanitized `.archbundle`;
- semantic zoom;
- action palette;
- moldable lenses;
- plugin trust/capabilities.

**Exit H8:** desde un nodo o pregunta se navega y explica arquitectura sin saber de
antemano qué tipo de diagrama hace falta.

## Regla de priorización

Una feature futura puntúa por:
1. trust,
2. reducción de coste cognitivo,
3. change safety,
4. agent context,
5. reutilización del grafo/evidence.

Si solo añade una nueva forma de dibujar lo ya conocido, queda detrás de H5–H8.
