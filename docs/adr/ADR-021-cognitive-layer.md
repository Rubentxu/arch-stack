# ADR-021 — Cognitive Layer (Agentic Intelligence)

**Estado:** Aceptado
**Fecha:** 31 de julio de 2026
**Aplica a:** `archctl` (Rust) + `archview` (TypeScript) — capa transversal sobre el grafo de conocimiento
**Refuerza:** ADR-007 (proyecciones), ADR-013 (workbench), ADR-019 (performance budget), ADR-020 (renderer stack)
**Relacionado:** ADR-022 (agent catalog), ADR-023 (action proposal + policy engine)

## Contexto

`archview` proyecta el grafo de conocimiento de código en 5 vistas coordinadas (ADR-013, ADR-020). El workbench es **zero-jank** en cualquier nivel de complejidad, pero las preguntas del developer son semánticas, no estructurales:

- *"¿Por qué este cambio importa?"*
- *"¿Este módulo representa un bounded context?"*
- *"¿Qué proyección muestra el blast radius?"*
- *"¿Qué tests están acoplados a este cambio?"*
- *"¿Conviene refactorizar este ciclo?"*

Los algoritmos deterministas del plano de datos (Rust → WASM) saben **qué existe y qué cambió**, pero no **qué significa, por qué importa ni qué conviene hacer**. Un LLM puro aplicado sobre el código completo del repo no escala, no es verificable y no respeta el grafo.

La separación fundamental es:

```text
Algoritmos deterministas     Agentes de IA
  Saben qué existe            Interpretan qué significa
  Saben qué cambió            Proponen qué conviene hacer
  Calculan caminos, CCs       Generan explicaciones con evidencia
  Producen findings            Formulan planes ejecutables
```

El documento `docs/Librerías-visualización-grafos-BI.md` (sección "Code Knowledge Graph Workbench", julio 2026) formaliza este patrón y se inspira en ActiveGraph para el modelo de coordinación.

## Decisión

`archview` + `archctl` incorporan una **Cognitive Layer** — agentes especializados que observan el grafo, lo interpretan, formulan planes y proponen acciones. Los agentes **nunca**:

- Sustituyen algoritmos deterministas.
- Mantienen el estado canónico (vive en el grafo).
- Ejecutan acciones fuera de un capability gateway.
- Reciben el repositorio completo (reciben `AgentContext` acotado).

### Arquitectura en 7 planos

```text
┌─────────────────────────────────────────────────────┐
│ 1. Developer Experience                              │
│    Viewer · IDE · CLI · Chat · Reports                │
├─────────────────────────────────────────────────────┤
│ 2. Agentic Cognitive Layer          ◄── ESTE ADR    │
│    Interpret · Query · Explain · Plan · Review        │
│    Agentes especializados, contrato uniforme           │
├─────────────────────────────────────────────────────┤
│ 3. Projection and Analytics Layer                    │
│    C4 · UML · Call graph · Sequence · Impact · Data   │
├─────────────────────────────────────────────────────┤
│ 4. Reactive Runtime                                  │
│    Events · Observers · Policies · Replay · Forks     │
├─────────────────────────────────────────────────────┤
│ 5. Code Knowledge Graph                              │
│    Facts · Evidence · Derived knowledge · Time        │
├─────────────────────────────────────────────────────┤
│ 6. Deterministic Intelligence                        │
│    AST · LSP · SCIP · Algorithms · Rules · OTel       │
├─────────────────────────────────────────────────────┤
│ 7. Sensors and Actuators                             │
│    Git · CI · IDE · MCP · Kubernetes · Files          │
└─────────────────────────────────────────────────────┘
```

### Contrato uniforme de agente

Todo agente cumple el mismo contrato `ReactiveObserver` (extendido):

```rust
pub trait AgentObserver: Send + Sync {
    fn descriptor(&self) -> AgentDescriptor;
    fn matches(&self, event: &EventEnvelope, delta: &GraphDelta) -> bool;
    async fn observe(&self, context: AgentContext) -> Result<AgentOutput>;
}

pub struct AgentDescriptor {
    pub id: AgentId,
    pub version: Version,
    pub subscriptions: Vec<EventPattern>,
    pub required_views: Vec<GraphViewDefinition>,
    pub output_schema: Schema,
    pub model_policy: ModelPolicy,    // qué modelos puede invocar
    pub budget: AgentBudget,           // tokens, time, $
    pub capabilities: Vec<Capability>,  // qué herramientas puede invocar
    pub deterministic: bool,           // ¿es siempre reproducible?
    pub idempotent: bool,
}

pub struct AgentContext {
    pub goal: Goal,
    pub triggering_event: EventId,
    pub graph_view: GraphView,          // proyección acotada, no el grafo entero
    pub source_fragments: Vec<SourceFragment>,
    pub evidence: Vec<Evidence>,
    pub applicable_rules: Vec<Rule>,
    pub available_tools: Vec<ToolDescriptor>,
    pub budget: AgentBudget,
    // El agente puede pedir MÁS contexto explícitamente:
    // request_context("callers_of(symbol, depth=2)")
    // request_context("traces_involving(component, last=24h)")
    // request_context("adr_related_to(component)")
}

pub enum AgentOutput {
    Hypothesis(Hypothesis),
    FindingCandidate(FindingCandidate),
    QueryPlan(QueryPlan),
    ProjectionSpec(ProjectionSpec),
    ActionPlan(ActionPlan),
    ActionProposal(ActionProposal),
    DocumentationPatch(DocumentationPatch),
    ContextRequest(ContextRequest),
    NoAction(NoActionReason),
}
```

El agente **siempre** produce un output estructurado. La explicación visible se genera a partir de la estructura, no al revés.

### Reglas de activación (cuándo corre un agente)

Los agentes no se ejecutan ante cada actualización trivial. El runtime decide según:

```text
Ejecutar agente cuando:
  - confianza determinista insuficiente (algoritmo no resolvió);
  - existe una contradicción entre fuentes;
  - impacto supera un umbral configurado;
  - el cambio afecta a una API pública;
  - se detectan varias señales correlacionadas;
  - el usuario solicita una explicación;
  - hay que formular un plan o proposal.
```

### Escalera de resolución (de barato a caro)

```text
1. Regla determinista
        ↓ si no resuelve
2. Algoritmo o heurística (Rust, <50ms)
        ↓ si sigue ambiguo
3. Modelo local pequeño (Phi-3, Llama-3-8B, <500ms)
        ↓ si el impacto lo justifica
4. Modelo razonador potente (Claude, GPT, <3s)
        ↓ si la acción es sensible
5. Revisión humana (HITL, async)
```

Esto reduce latencia, consumo de tokens, alucinaciones y dependencia de modelos externos.

### Coordinación vía estado, no conversación

```text
Agent A ─X─► Agent B
Agent A ─► Graph Event ─► Agent B
```

Los agentes no se llaman entre sí. Se coordinan publicando eventos en el grafo (reactive runtime, ADR-013 §"Eventos y reacciones"). El grafo es el bus de mensajes. Esto evita cadenas de dependencias y permite forks, replay y tests aislados.

### MCP como capability boundary

MCP (Model Context Protocol) define tres categorías:

```text
MCP resources   → contexto: grafo, vistas, documentos, evidencia
MCP tools       → operaciones: GitHub, CI, tests, editor, Kubernetes
MCP prompts     → procedimientos: investigaciones, planificaciones
```

**Los agentes invocan tools solo a través del MCP gateway.** El gateway valida capabilities, rate limits, approval y emite eventos de evidence. Esto es la frontera de seguridad entre el LLM y el mundo.

### Implementación por ciclo

- **v1.0 (M16)**: contrato `AgentObserver`, `AgentContext`, `AgentOutput` + 1 agente proof-of-concept (Architecture Agent). Cero invocación real a LLM; el agente puede ser una heurística determinista. Sirve para validar el contrato end-to-end.
- **1.x (M18 + M22)**: 3-4 agentes especializados (Semantic Curator, Projection, Investigation, Impact) + integration con MCP + primer LLM local pequeño (Phi-3 / Llama-3-8B).
- **2.0 (M_future)**: catálogo completo de 9 agentes + modelo razonador potente (Claude/GPT) + Action Proposal con Policy Engine maduro.

## Consecuencias

### Positivas

- El workbench responde preguntas semánticas, no solo estructurales.
- Los agentes son **verificables**: cada output es estructurado y se persiste como `Hypothesis`, `FindingCandidate`, `ActionProposal` (con confianza, evidencias, autor).
- La escalera de resolución minimiza costo y latencia.
- MCP da una frontera de seguridad clara.
- Coordinación vía estado permite forks, replay y tests aislados.
- Los algoritmos ganan interpretación; los agentes ganan estructura.

### Negativas

- Complejidad operacional: el Cognitive Layer añade runtime, costs, infra.
- Riesgo de alucinación: los agentes pueden inventar conclusiones. Mitigación: outputs estructurados + confidence + evidence + review agent.
- Tests difíciles: el comportamiento de un agente no es determinista por diseño. Mitigación: golden queries, evaluation datasets, idempotency.
- Costos de inferencia: cada llamada a un LLM potente cuesta. Mitigación: escalera + caché de contextos.
- Local model performance: los modelos pequeños (Phi-3) no son tan buenos como los grandes. Mitigación: híbrida (local para trivial, remoto para complejo).

### Métricas de éxito

- % de queries respondidas con evidencia verificable: >90%.
- Latencia media de respuesta con LLM local: <500ms.
- Latencia media con LLM potente: <3s.
- % de ActionProposals rechazados por Policy Engine: <20% (calibración).
- % de agentes que ejecutan con `DeterminismLevel::Pure` y no requieren review humano: >70% (cheap path).
- 0 invocaciones a tools fuera del MCP gateway.

## Cómo revertir

| Decisión | Reversión |
|---|---|
| Cognitive Layer | Reducir a v0: solo `HeuristicAgent` sin LLM. Workbench responde solo queries estructurales. |
| 7 planos | Fusionar Cognitive con Projection Layer. |
| MCP gateway | Sustituir por un wrapper custom más simple. |
| Escalera de resolución | Plano único: solo LLM potente (más simple, más caro). |

## Referencias

- `docs/Librerías-visualización-grafos-BI.md` — sección "Code Knowledge Graph Workbench" (la fuente de este ADR)
- [ADR-013](ADR-013-viewer-ortogonal.md) — viewer ortogonal (donde corre el viewer de los agentes)
- [ADR-019](ADR-019-performance-budget.md) — hard contract (los agentes no rompen el budget)
- [ADR-020](ADR-020-renderer-stack.md) — stack (SolidJS UI para el chat/presencia de agentes en el viewer)
- [ADR-022](ADR-022-agent-catalog.md) — los 9 agentes especializados
- [ADR-023](ADR-023-action-proposal-and-policy.md) — ActionProposal + Policy Engine
- ActiveGraph — inspiración arquitectónica (no implementación completa)
