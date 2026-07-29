# ADR-006 — Política de integración de herramientas: preferir librerías, descartar CLIs

**Estado:** **DEPRECADO** — sustituido íntegramente por [ADR-012](ADR-012-adopcion-incremental-crates-analisis.md) (política de adopción de librerías de análisis) y [ADR-013](ADR-013-viewer-ortogonal.md) (separación viewer).
**Fecha original:** 29 de julio de 2026
**Fecha de deprecación:** 29 de julio de 2026

## Contexto histórico (preservado)

> Este ADR es **histórico**. La política vigente está consolidada en
> ADR-012. No tomar decisiones basándose en este documento. Conservar
> el texto original solo para trazabilidad de la evolución del proyecto.

---

## Decisión original (29 de julio de 2026, DEPRECADA)

`archctl` implementará adaptadores y normalizadores, no analizadores.

### Núcleo (original)

- Git.
- ripgrep.
- `ast-grep`.
- herramientas nativas del build.
- Structurizr CLI.
- PlantUML.
- Mermaid CLI cuando corresponda.

### Opcionales (original)

- LSP, SCIP, Universal Ctags, dependency-cruiser, `jdeps`, Semgrep, Joern, Terraform, Helm, kubectl, Syft.

### Capabilities (originales)

Los agentes solicitan:

```text
inventory.repository
syntax.patterns
symbols.list
references.find
dependencies.module
calls.path
infrastructure.topology
diagram.render
diagram.validate
```

### Salida normalizada (original)

Todo adaptador produce elementos candidatos, relaciones, evidencias,
herramienta+versión, confianza, snapshot y diagnósticos.

---

## Por qué se deprecó

El contenido de este ADR fue absorbido por dos ADRs posteriores que lo
cierran con criterios más precisos:

| Sección original | ADR que la reemplaza |
|---|---|
| "Adaptadores CLI externos" | [ADR-012](ADR-012-adopcion-incremental-crates-analisis.md) § Política "descartar CLIs" |
| "Renderers (PlantUML, Structurizr, Mermaid)" | [ADR-013](ADR-013-viewer-ortogonal.md) — el viewer es proyecto separado |
| "LSP, SCIP, herramientas semánticas" | [ADR-012](ADR-012-adopcion-incremental-crates-analisis.md) § Diferidos (Fase 2) |
| "Perfiles fast/semantic/deep" | Mismo concepto en ADR-013 § Stack de archview |

Cualquier referencia a este ADR debe leerse como histórica.
Para la política vigente, consultar ADR-012 y ADR-013.
