# ADR-023 — Action Proposal & Policy Engine

**Aceptado (diferido)** — fase 1 PR #32 cerrada; ver [ADR-040](ADR-040-cognitive-conditional-activation.md)
**Estado:** Aceptado
**Fecha:** 31 de julio de 2026
**Aplica a:** `archctl` (Rust) + `archview` (TypeScript) — capa transversal
**Refuerza:** ADR-021 (cognitive layer), ADR-022 (agent catalog)
**Relacionado:** ADR-013 (workbench), ADR-020 (renderer stack), [Model Context Protocol](https://modelcontextprotocol.io/)

## Contexto

Los agentes de la [Cognitive Layer](ADR-021-cognitive-layer.md) pueden proponer acciones: ejecutar tests, modificar código, abrir un PR, desplegar, etc. **Sin un policy engine, un LLM con acceso a tools es un vector de ataque**: prompt injection, errores con consecuencias, scope creep.

El documento `docs/Librerías-visualización-grafos-BI.md` (sección "Reactive Runtime" + "Actuators", julio 2026) formaliza el patrón:

```text
Agente
   ↓
ActionProposal (estructurado, con capabilities + approval + evidence esperada)
   ↓
Policy Engine (reglas + presupuestos + approval levels)
   ↓
Approval, si procede (HITL async)
   ↓
MCP Tool Gateway (única frontera de ejecución)
   ↓
Actuator (GitHub, CI, Kubernetes, files, ...)
   ↓
Result
   ↓
Evidence Event (vuelve al grafo)
```

Sin este patrón, los agentes son peligrosos. Con este patrón, son **gobernados y auditables**.

## Decisión

`archview` y `archctl` implementan un **Action Proposal + Policy Engine** que media entre los agentes y el mundo exterior. Los agentes **nunca** ejecutan acciones directamente.

### ActionProposal (estructura)

```rust
pub struct ActionProposal {
    pub id: ProposalId,
    pub goal: Goal,
    pub cause: EventId,                       // qué disparó la propuesta
    pub triggering_agent: AgentId,
    pub command: Command,                     // qué se ejecutaría
    pub required_capabilities: Vec<Capability>, // qué tools necesita
    pub approval: ApprovalRequirement,         // quién debe aprobar
    pub expected_evidence: Vec<EvidencePredicate>, // qué evidencia esperamos
    pub success_condition: Predicate,         // cuándo consideramos éxito
    pub rollback: RollbackStrategy,           // cómo deshacer si falla
    pub cost_estimate: CostEstimate,           // tokens / time / $ / side effects
    pub confidence: f32,                       // 0.0-1.0
    pub ttl: Duration,                         // cuándo expira si no se aprueba
}

pub enum ApprovalRequirement {
    Auto,                          // ejecutable sin intervención
    Notify(subscribers: Vec<UserId>), // notifica, no bloquea
    Review(reviewer: ApprovalLevel), // bloquea hasta approval humano
    Forbidden,                     // bloqueado, no ejecutable
}

pub enum ApprovalLevel {
    SelfApproval,                  // el propio agente
    PeerApproval,                   // cualquier developer
    TechLeadApproval,               // tech lead del equipo afectado
    SecurityApproval,               // incluye review de seguridad
    MultiPartyApproval { required: u32, total: u32 },
}
```

### Policy Engine (reglas)

```rust
pub trait Policy {
    fn matches(&self, proposal: &ActionProposal, context: &PolicyContext) -> bool;
    fn evaluate(&self, proposal: &ActionProposal, context: &PolicyContext) -> PolicyDecision;
}

pub enum PolicyDecision {
    Allow,                              // ejecuta
    AllowWithNotify { to: Vec<UserId> },
    RequireApproval { by: ApprovalLevel, reason: String },
    Deny { reason: String },             // bloqueado, loguea evidencia
    Escalate { to: EscalationTarget },  // pasa a nivel superior
}

pub struct PolicyContext {
    pub user: UserId,                    // quién está interactuando
    pub environment: Environment,         // dev / staging / prod
    pub affected_components: Vec<Component>,
    pub security_impact: SecurityImpact,   // low / medium / high / critical
    pub cost_ceiling: CostCeiling,         // lo que el usuario autoriza gastar
    pub recent_audit: Vec<AuditEntry>,     // historial reciente
}
```

### Reglas por defecto (fáciles de extender)

```toml
# policies/default.toml
[[policy]]
match.command = "run_tests"
match.environment = "dev"
evaluate = "Allow"                                    # tests en dev: auto

[[policy]]
match.command = "run_tests"
match.environment = "production"
evaluate = "RequireApproval"
require.by = "PeerApproval"
require.reason = "tests on production need peer sign-off"

[[policy]]
match.command = "modify_source"
match.environment = "any"
evaluate = "RequireApproval"
require.by = "TechLeadApproval"
require.reason = "modifying source files requires tech lead"

[[policy]]
match.command = "deploy_production"
match.environment = "any"
evaluate = "Forbidden"                                # never auto-deploy to prod
deny.reason = "production deploys require human + CI"

[[policy]]
match.command = "execute_external_api"
match.security_impact = "high"
evaluate = "RequireApproval"
require.by = "SecurityApproval"
require.reason = "external API with security impact"

[[policy]]
match.cost.tokens > 50000
evaluate = "RequireApproval"
require.by = "SelfApproval"
require.reason = "high-token proposal"

[[policy]]
match.confidence < 0.6
evaluate = "RequireApproval"
require.by = "PeerApproval"
require.reason = "low-confidence proposal"
```

### MCP (Model Context Protocol) como capability boundary

[MCP](https://modelcontextprotocol.io/) define tres categorías que mapean limpio a nuestra arquitectura:

```text
MCP resources   → contexto de solo lectura: grafo, vistas, docs, evidencia
MCP tools       → operaciones con efectos: GitHub, CI, tests, editor, K8s
MCP prompts     → procedimientos: investigaciones, planificaciones
```

**Los agentes invocan tools SOLO a través del MCP gateway.** El gateway valida:

```text
1. ¿La capability está en `proposal.required_capabilities`?
2. ¿La regla del policy engine permite el command?
3. ¿El cost_estimate está dentro del cost_ceiling?
4. ¿Hay un approval pending para esta proposal?
5. ¿La confidence supera el threshold del command?
6. ¿El command está dentro de los commands permitidos del environment?
7. ¿El user tiene la capability específica?
```

Si todo OK: ejecuta y emite `EvidenceEvent` con el resultado. Si falla: emite `RejectionEvent`.

### Audit trail (inmutability)

```rust
pub struct AuditEntry {
    pub timestamp: DateTime,
    pub agent: AgentId,
    pub proposal: ProposalId,
    pub policy_decision: PolicyDecision,
    pub outcome: ActionOutcome,
    pub evidence_emitted: Vec<EvidenceId>,
    pub user_who_approved: Option<UserId>,
    pub rollback_executed: bool,
}
```

Cada `ActionProposal` deja un `AuditEntry` en el grafo (inmutable, append-only). El Presenter Agent puede usar este log para explicar "¿qué hizo el agente ayer?".

### Ejemplo end-to-end

```text
1. Semantic Curator Agent emite:
   MergeCandidate(symbol_id=A, symbol_id=B, reason="alias detected in 3 places")

2. ActionProposal:
   {
     id: "prop-2026-07-31-001",
     goal: "merge A and B (alias)",
     command: merge_symbols(A, B),
     required_capabilities: [ModifyGraph],
     approval: Auto,
     expected_evidence: [alias_count >= 3, no_public_api_calls],
     success_condition: !ErrorAfterMerge,
     rollback: split_back,
     cost_estimate: { tokens: 200, time: 50ms, $: 0.0, side_effects: merge },
     confidence: 0.85,
     ttl: 1h
   }

3. Policy Engine: match(ModifyGraph) → Allow (rule: "graph mutations in dev are auto-approvable if confidence > 0.7")

4. MCP Gateway: validates capability (ModifyGraph ✓) + policy (Allow) + cost (within ceiling) → executes

5. Result: 2 symbols merged into 1, evidence event emitted

6. AuditEntry: appended to log

7. Presenter Agent can now show the user: "I detected and merged 2 aliased symbols based on 3 cross-references"
```

### Politica por environment

| Command | dev | staging | production |
|---|---|---|---|
| run_tests | Auto | Notify | Review (Peer) |
| modify_source | Review (TechLead) | Review (TechLead) | MultiParty (2/3) |
| run_lint | Auto | Auto | Review (Peer) |
| execute_migration | Forbidden | Review (TechLead) | MultiParty (3/5) |
| deploy | Forbidden | Review (TechLead) | MultiParty (3/5) + CI green |
| modify_graph | Auto (conf > 0.7) | Review (TechLead) | MultiParty (2/3) |
| open_pull_request | Auto | Review (TechLead) | Review (TechLead) |
| send_notification | Auto | Auto | Review (TechLead) |

### Implementación por ciclo

- **v1.0 (M16)**: ActionProposal + Policy Engine básico (3-5 reglas hardcoded), MCP gateway para 2-3 tools (read-only: graph_query, schema_validate, run_tests_local). 0 ActionProposals con efectos irreversibles en v1.0.
- **1.x (M18)**: 8-10 reglas configurables, integration con `archctl`, primer ActionProposal ejecutado (e.g., `archctl code c4 discover --auto-apply` si confidence > 0.9).
- **2.0**: Policy engine completo con human-in-the-loop UI en `archview`, audit log persistente en grafo, rollback automático.

## Consecuencias

### Positivas

- Los agentes son **gobernados y auditables**: cada acción deja rastro.
- MCP da una **frontera de seguridad clara** entre LLM y mundo exterior.
- Las reglas son declarativas (TOML) y editables sin recompilar.
- El audit trail permite explicar "¿qué hizo el agente?".
- Rollback strategies previenen acciones irreversibles mal ejecutadas.
- Confidence + cost_estimate permiten decisiones racionales (no ejecutar proposals de baja confianza o alto costo).

### Negativas

- Complejidad: el Policy Engine añade runtime, configs, audit log.
- Fricción: el usuario puede sentirse restringido. Mitigación: defaults permisivos en dev, restrictivos en prod.
- Latencia añadida: cada ActionProposal pasa por el gateway. Mitigación: <10ms para Allow auto-approved, <100ms para Review (con timeout).
- Configuración: el TOML de policies puede ser complejo. Mitigación: defaults sensatos + UI para editar.
- Test surface: políticas + capabilities + audit log = mucho que testear.

### Métricas de éxito

- Latencia media del policy engine: <10ms (auto-approved), <100ms (review required).
- % de ActionProposals auto-approved: >80% (heurística + LLM local cubren la mayoría).
- % de ActionProposals rechazados o requieren override: <10%.
- 0 invocaciones a tools fuera del MCP gateway.
- 0 acciones irreversibles sin audit entry.
- Audit log indexable: el Presenter Agent puede explicar cualquier acción de los últimos 90 días.

## Cómo revertir

| Decisión | Reversión |
|---|---|
| MCP gateway | Wrapper custom. Más simple, menos interoperable. |
| Policy Engine | Hardcoded rules en código. Más simple, menos configurable. |
| Auto-approval en dev | Todo Requiere Review. Más seguro, más fricción. |
| Confidence threshold | Eliminarlo. Confiar en el policy engine. |
| Audit log | Eliminarlo. Sin trazabilidad. No recomendado. |

## Referencias

- [ADR-021](ADR-021-cognitive-layer.md) — Cognitive Layer (define la cadena Agente → ActionProposal)
- [ADR-022](ADR-022-agent-catalog.md) — los 9 agentes que emiten ActionProposals
- [ADR-013](ADR-013-viewer-ortogonal.md) — workbench (donde se muestran los ActionProposals al usuario)
- [ADR-011](ADR-011-renderers-locales-y-bloqueo-de-publicos.md) — la regla "todo es local" aplica también a MCP tools
- [Model Context Protocol](https://modelcontextprotocol.io/) — la frontera de capabilities
