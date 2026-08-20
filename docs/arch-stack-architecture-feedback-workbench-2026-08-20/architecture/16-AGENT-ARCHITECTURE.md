# Agent Architecture

## Usar agentes para
Interpretar intent, proponer boundaries, sintetizar explanations, patrones blandos, sugerir queries/lenses, stories y refactors/what-if.

## No usar LLM para
Hash, parsing, imports/calls extraíbles, SCC/cycles, reachability, diff, policy enforcement, IDs, reconciliation o rendering/layout.

## AgentContext vNext
Goal, triggering event, graph revision, relevant subgraph, source fragments, evidence, intent, feedback, recent changes, current visual selection, rules, tools, budget y `included_because`.

## Outputs
Evolucionar el `AgentOutput` existente: IntentCandidate, LensSuggestion, VisualRequest, StoryProposal, BoundaryProposal.

## Promotion
Los outputs model-backed entran como candidates y sólo cambian autoridad mediante verificación/adjudicación.
