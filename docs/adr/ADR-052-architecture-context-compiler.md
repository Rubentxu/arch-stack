# ADR-052 — Task Context Compiler para agentes

> **Estado:** Aceptado — 2026-08-13 (shipped as P2-08 Task Context Compiler, v1.57.0; `archctl architecture context --task <text> [--budget-tokens N] [--top N] [--json]`)
> **Baseline de auditoría:** `main@518bb79d4c87a491fc901d54441de15e72c40bc2`
> **Ámbito:** consolidación arquitectónica posterior a v1.41
> **Propietario de decisión:** maintainers de Arch Stack

## Contexto

La capa cognitive ya contiene context/MCP/agentes. El siguiente salto debe compilar
contexto arquitectónico pequeño y verificable, no crear RAG genérico ni releer el
repo entero por tarea.

## Fuerzas de diseño

- Preservar los invariantes local-first, evidence-first y source-read-only.
- Mantener el grafo canónico como única fuente semántica de verdad.
- Favorecer determinismo, testabilidad y reversibilidad.
- Evitar una migración *big bang* que paralice la entrega.
- Hacer que los límites arquitectónicos sean verificables por tooling y CI, no solo por convención.

## Decisión

Evolucionar `cognitive/context.rs`: normalize task → deterministic seeds → graph
expansion → impact/policy/evidence enrichment → ranking → budget packing → trace.
LLM query expansion opcional nunca inventa entidades.

## Superficie propuesta

```bash
archctl context compile --task "..." --budget-tokens 12000 --json
```

## Rationale y beneficios

Reduce tokens/latencia, aumenta grounding y convierte el grafo en memoria
arquitectónica reusable.

## Costes y consecuencias negativas

Ranking puede omitir contexto; debe reportar truncation/unknowns y explicar selección.





## Estrategia de migración

CLI JSON deterministic primero; MCP después; preview en ImpactView; LLM expansion
solo posterior.

## Verificación y criterios de aceptación

- misma entrada y budget → mismo output en deterministic;
- IDs/provenance;
- excluded/truncated;
- no source completo default;
- golden budget tests.

## Alternativas consideradas

A) vector RAG: nueva index/source.
B) repomix completo: caro.
C) LLM elige ficheros: opaco.

## Referencias internas

cognitive/context.rs, cognitive/mcp, ImpactView, ADR-021.

## Changelog

- 2026-08-13 | proposed | ADR-052 creado a partir de la auditoría de consolidación.
