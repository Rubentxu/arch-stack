# DESIGN — Architecture Feedback Workbench

## Objetivo
Transformar Arch Stack en un entorno de razonamiento visual sobre software donde código, documentación, intención, runtime, tests, agentes y feedback humano comparten identidades y evidencia.

## Arquitectura funcional

```text
Sources
  ↓
Incremental ingestion
  ↓
Observations
  ↓
Canonical graph
  ↓
Derived analysis / reconciliation
  ↓
Visual compiler
  ↓
Coordinated workbench
  ↓
Human feedback
  ↓
Graph + future agent context
```

## Tres lanes

- **Truth Lane**: parsing, extraction, policies, graph algorithms, diff, reconciliation.
- **Reasoning Lane**: heurísticas y LLMs; sólo candidates/proposals.
- **Feedback Lane**: accept/reject/correct/supersede.

## Data ownership
LadybugDB = semántica canónica; event journal = causalidad/audit; Tantivy = índice reconstruible; TreeCache = estado incremental efímero; workspace JSON = estado visual; vector index opcional = reconstruible.

## Visual architecture
G6 topology, ELK layered/hierarchical layout, Canvas2D dense matrix, D3 utilities hierarchy/scales/timeline, SolidJS state/composition/accessibility.

## Live mode
`archctl view --watch` → notify → index worker → GraphRevision → GraphDelta → polling desde Archview. No daemon/WebSocket hasta necesidad medida.

## Evolución segura
Cada fase debe ser vertical, medible y reversible. No externalizar LensSpec hasta cumplir el gate ya existente en el repo.
