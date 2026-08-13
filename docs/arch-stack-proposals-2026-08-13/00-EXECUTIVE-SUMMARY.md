# Executive Summary — de Diagrammer a Architecture Intelligence

**Baseline:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`  
**Fecha:** 2026-08-13

## Diagnóstico

La base del producto es sólida: persistencia local, grafo canónico, evidencia,
extractores deterministas, proyecciones, `archview`, adaptadores de IDE y capa
cognitiva. El principal riesgo ya no es la falta de features, sino la **erosión de
límites** provocada por el rápido crecimiento.

Indicadores observados:

- `archctl/src/cli.rs` ≈ 99 KB;
- `archctl/src/store.rs` ≈ 97 KB;
- `archctl/src/code/call_graph.rs` ≈ 85 KB;
- `archctl/src/code/class_diagram.rs` ≈ 60 KB;
- `archctl/src/code/state_machine.rs` ≈ 47 KB;
- existe `cognitive/` con context, delta, MCP, policy y agents;
- `archview` ya dispone de C4, Call Graph, Class, Sequence, Drift, Impact y Package;
- existen manifests y quality gates propios;
- los ADR 040 y 041 están duplicados;
- CI/release estaban bloqueados en la auditoría por el acoplamiento nativo de LadybugDB.

## Goal propuesto

> Arch Stack es un motor local-first de Architecture Intelligence que transforma
> evidencia verificable del software en un grafo arquitectónico canónico, y lo
> proyecta en representaciones adecuadas para comprender, validar, comparar y
> modificar software con ayuda de humanos y agentes.

## Cuatro outcomes

### Trust
Toda afirmación arquitectónica importante puede responder: qué la originó, qué
extractor/agente la produjo, qué fichero/línea/commit la soporta, qué confianza
tiene y si existen observaciones contradictorias.

### Change intelligence

```bash
archctl architecture diff main..HEAD
```

explica **qué arquitectura cambia**, no únicamente qué archivos cambian.

### Agent context

```bash
archctl context compile   --task "añadir cache distribuida a checkout"   --budget-tokens 12000
```

construye contexto arquitectónico relevante, trazable y acotado.

### Moldable exploration

`archview` evoluciona de viewer a **Architecture Workbench**: una pregunta o selección
determina el lens/proyección adecuada y permite navegar System → Container →
Component → Module/Class → Function → Source y volver.

## Prioridad

```text
P0 — Stabilize truth
  build/release, Ladybug boundary, plugins, ADR integrity, licenses, PR CI

P1 — Enforce architecture
  modular hexagonal boundaries, repositories/ports, capability registry,
  contract tests, fitness gates

P2 — Deliver intelligence
  diff, explain, confidence/coverage, policies, context compiler, evidence fusion

P3 — Compound utility
  snapshots, sanitized bundles, moldable workbench, plugin trust/capabilities
```

## Regla estratégica

No ampliar horizontalmente el roadmap hasta completar P0 y la mayor parte de P1.
Una nueva notación de diagrama solo entra si demuestra un resultado que no pueda
resolverse como proyección/adaptador de capacidades actuales.

## Definition of Done del programa

1. `main` y release matrix verdes en targets soportados.
2. Límites de módulos/crates verificados automáticamente.
3. Application/domain no dependen de LadybugDB, HTTP, filesystem real ni GitHub.
4. ADR con identidad única exigida por CI.
5. Capacidades/lenguajes en un único registro.
6. `architecture diff`, `explain` y `context compile` con salidas estables.
7. Políticas arquitectónicas bloquean regresiones mediante formatos estándar.
8. `archview` consume esos resultados sin duplicar semántica.
