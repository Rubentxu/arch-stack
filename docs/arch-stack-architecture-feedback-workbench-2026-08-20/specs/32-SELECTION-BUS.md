# Spec — SelectionBus

## SelectionState
`revision, entity_ids, relation_ids, evidence_ids, origin_lens, focus_mode`.

## Consumers
Graph, DSM, SystemMap, SourceDrawer, Inspector, Timeline, AgentContext.

## Invariants
No alternate identity; selection sobrevive vistas compatibles; stale selection degrada explícitamente; origin/version evita loops.
