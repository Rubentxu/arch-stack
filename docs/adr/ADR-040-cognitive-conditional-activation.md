# ADR-040 — Cognitive layer: conditional activation (estado real de ADR-021/022/023)

> **Ciclo:** `m69-arch-stack-product-roadmap-convergence`
> **Estado:** Aceptado
> **Fecha:** 2026-08-09
> **Actualiza:** ADR-021 (header), ADR-022 (header), ADR-023 (header)
> **No Reescribe:** los cuerpos de ADR-021, ADR-022, ADR-023 permanecen idénticos

## Contexto

ADR-021, ADR-022 y ADR-023 fueron aceptados el 31 de julio de 2026 con estado
"Aceptado". Cada ADR define un alcance completo para la Cognitive Layer:

- **ADR-021**: Cognitive Layer — 7-layer architecture, contract uniforme,
  escalation ladder, MCP boundary.
- **ADR-022**: Agent catalog — 9 agentes especializados.
- **ADR-023**: Action Proposal & Policy Engine — ActionProposal estructurado,
  Policy Engine TOML, MCP gateway, audit log.

El estado shipped a 2026-08-09 refleja solo una parte del alcance:

| Capacidad | ADR define | Shipped | Notes |
|---|---|---|---|
| Agent contract (`ReactiveObserver` + `AgentContext` + `AgentOutput`) | ✓ | ✓ | Foundation contract shipped v0.15.0 |
| Architecture Agent (heuristic) | ✓ | ✓ | M22 shipped |
| Projection Agent (heuristic) | ✓ | ✓ | M22 shipped |
| Semantic Curator / Investigation / Impact / Planning / Documentation / Presenter / Review | 7 agents | 0 | Deferred |
| MCP gateway (read-only: `graph_query`, `schema_validate`, `run_tests_local`) | ✓ | ✓ | Shipped v0.15.0 |
| ActionProposal + Policy Engine | ✓ | 0 | Phase 1 PR #32 closed stale; phases 2-6 not started |
| HITL UI para proposals | ✓ | 0 | Not started |
| Audit log append-only | ✓ | 0 | Not started |

La situación real: **foundation shipped, 2/9 agentes implementados, 7 deferred,
action proposal pipeline no started**.

## Decisión

ADR-021, ADR-022 y ADR-023 se marcan como **Aceptado (conditional)**,
**Aceptado (parcial)** y **Aceptado (diferido)** respectivamente.

El cuerpo de cada ADR **no se modifica**. Las decisiones técnicas registradas
en esos documentos son válidas para cuando se reactiven. La única变化 es el
status header y este ADR de tracking.

### Estados actualizados

| ADR | Header status previo | Header status nuevo | Razón |
|---|---|---|---|
| ADR-021 | Aceptado | **Aceptado (conditional)** | Foundation shipped; full scope depends on real HITL workflow |
| ADR-022 | Aceptado | **Aceptado (parcial)** | 2/9 agentes shipped; 7 deferred indefinitely |
| ADR-023 | Aceptado | **Aceptado (diferido)** | Phase 1 PR #32 closed stale; phases 2-6 never started |

## Trigger de reactivacion

La Cognitive Layer (completa) se reactivará cuando:

> **Un workflow HITL real requiera agent-driven actions más allá de heurísticas.**
>
> No es una fecha. No es una versión. Es un workflow concreto con un usuario
> real que necesita que los agentes ejecuten acciones (no solo lean el grafo).

Ejemplos de triggers válidos:

- Un equipo usa arch-stack en producción y necesita que el Planning Agent
  genere ActionProposals para cada change que afecta >5 componentes.
- Un usuario pide explícitamente que el Documentation Agent proponga parches
  de docs como resultado de un `archctl code c4 discover`.
- El Audit log es un requisito de compliance para un equipo enterprise.

Ejemplos de triggers NO válidos:

- "la versión 2.0 está lista"
- "ya pasó suficiente tiempo"
- "sería bueno tener más features"

## Plan de reactivacion

Cuando el trigger se active:

1. **Nuevo ADR** (no reescribir 021/022/023) — ADR-0XX que defina el
   scope de la reactivación con el workflow HITL como referencia.
2. **Prioridad**: 7 agentes deferred se priorizan según el workflow que
   activate.
3. **MCP gateway expansion**: las tools writable (no solo read-only) se
   añaden según el capability gateway de ADR-023.
4. **Policy Engine**: las reglas TOML se escriben para el environment del
   usuario que activate.

## Nota sobre PR #32

PR #32 (phase 1 del Action Proposal pipeline) se cerró como stale en el
ciclo M59 (2026-08-07). Las phases 2-6 de ADR-023 no se han-started.

Cuando el trigger de reactivación se active, el equipo revisará si PR #32
tiene salvageable work o si se empieza de nuevo con el workflow HITL como
base.

## Referencias

- [ADR-021](ADR-021-cognitive-layer.md) — cognitive layer foundation
- [ADR-022](ADR-022-agent-catalog.md) — agent catalog
- [ADR-023](ADR-023-action-proposal-and-policy.md) — action proposal + policy
- `archctl/src/agent/` — implementación shipped de los 2 agentes
- `archctl/src/mcp/` — implementación shipped del MCP gateway
