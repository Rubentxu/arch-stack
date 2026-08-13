# ADR Integration Map

## Problema existente
El árbol auditado contiene IDs duplicados:

| ID | Documentos |
|---|---|
| ADR-040 | versioned distribution / cognitive conditional activation |
| ADR-041 | self-update / workspace state persistence |

No hacer mass-renumber de todos los ADR posteriores. Preservar historial y enlaces.

## Nuevas decisiones propuestas

| ADR | Decisión | Prioridad |
|---|---|---|
| 043 | Modular hexagonal boundaries | P1 |
| 044 | Persistence ports/raw query | P1 |
| 045 | Capability Registry | P1 |
| 046 | Plugin supply-chain security | P0/P3 |
| 047 | Pre-merge CI | P0 |
| 048 | Ladybug native compatibility | P0 |
| 049 | Observation/Evidence/Claim | P2 |
| 050 | Git-linked snapshots | P2/P3 |
| 051 | Workbench session security | P3 |
| 052 | Task Context Compiler | P2 |
| 053 | Architecture Diff | P2 |
| 054 | Architecture Policy | P2 |
| 055 | Sanitized ArchBundle | P3 |
| 056 | Moldable Architecture Workbench | P3 |

## Workflow
`Proposed → Accepted → Implementing → Accepted/Implemented`.

Una decisión invalidada pasa a `Rejected` o `Superseded by ADR-NNN`. No se reescribe
un ADR aceptado para fingir que la decisión histórica fue otra.
