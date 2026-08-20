# Migration & Integration Plan

## ADR history
No reescribir historia. `ADR-Pxx` son proposals de este paquete. Al incorporar:
1. buscar si ya existe decisión equivalente;
2. amend/supersede cuando corresponda;
3. asignar número real del repo;
4. preservar ADR histórico.

## Anchors existentes que deben respetarse
- ADR-010: no daemon hasta necesidad medida.
- ADR-013/033: workbench embedded.
- ADR-019: performance budgets.
- ADR-038/039: one product + anti-roadmap.
- ADR-041: XDG workspace.
- evidence/claim infrastructure existente.
- ADR-056/062: moldable workbench / navigation.
- LensSpec entry criteria existentes siguen binding.

## Types a evolucionar
- `AgentOutput` añade candidate/visual variants.
- `ProjectionSpec` migra compatiblemente a VisualRequest.
- `AgentContext` añade revision/selection/feedback.
- `FusedClaim` usa confidence/freshness reales.
- `EventEnvelope` añade IDs/schema/causality.
- view API añade revision/delta.

## Avoid duplication
Antes de crear module/type nuevo: buscar carrier equivalente, definir migration adapter y deprecar sólo después de parity.
