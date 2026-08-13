# Risk Register

| Riesgo | Prob. | Impacto | Mitigación |
|---|---|---|---|
| Ladybug crate/native ABI drift | Alta | Crítico | ADR-048 + doctor |
| Plugin path traversal/supply chain | Media | Crítico | ADR-046 |
| Refactor big-bang | Media | Alto | strangler + golden |
| Cypher leak perpetuo | Alta | Alto | ADR-044 + gate |
| Capability docs drift | Alta | Medio | ADR-045 |
| Evidence ambiguo | Media | Alto | ADR-049 |
| Snapshot storage sin límite | Media | Medio | retention/GC |
| Context ranking incompleto | Media | Alto | trace + unknowns |
| Policy DSL gigante | Media | Medio | rule set cerrado |
| Workbench acoplado a UI semantics | Media | Alto | contract IDs |
| ADR renumber rompe enlaces | Alta | Medio | mapping/tombstones |
| ArchBundle filtra secretos | Baja/Media | Crítico | allowlist + scanner |
| Demasiados crates | Media | Medio | boundary-before-extraction |
| Más diagramas diluye roadmap | Alta | Medio | outcome gate |
