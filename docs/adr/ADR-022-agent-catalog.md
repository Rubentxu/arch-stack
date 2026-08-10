# ADR-022 — Agent Catalog (9 agentes especializados)

**Aceptado (parcial)** — 2/9 agentes enviados; ver [ADR-040](ADR-040-cognitive-conditional-activation.md)
**Estado:** Aceptado
**Fecha:** 31 de julio de 2026
**Aplica a:** `archview` (TypeScript) + `archctl` (Rust) — implementación conforme a [ADR-021](ADR-021-cognitive-layer.md)
**Refuerza:** ADR-021 (cognitive layer), ADR-013 (workbench), ADR-019 (performance budget)
**Relacionado:** ADR-023 (action proposal + policy engine), ADR-020 (renderer stack)

## Contexto

[ADR-021](ADR-021-cognitive-layer.md) define el contrato uniforme de agente (`ReactiveObserver` + `AgentContext` + `AgentOutput`). Este ADR específica el **catálogo inicial de 9 agentes especializados** que la plataforma soporta.

Los agentes **no conversan entre sí**. Se coordinan publicando eventos en el grafo. Cada uno es especialista en un aspecto:

| # | Agente | Función | Output primario |
|---|---|---|---|
| 1 | Semantic Curator | Mantiene la calidad semántica del grafo | `MergeCandidate`, `AliasCandidate`, `StaleKnowledgeFinding` |
| 2 | Architecture | Interpreta código en términos de C4, capas, bounded contexts | `ArchitectureAssessment` (declared/discovered/observed drift) |
| 3 | Projection | Decide cómo representar una pregunta (qué vista elegir) | `ProjectionSpec` (sequence, C4, call graph, etc.) |
| 4 | Investigation | Investiga hipótesis sobre el grafo con cadena argumental | `InvestigationReport` (conclusión + evidencias + confianza + limitaciones) |
| 5 | Impact | Interpreta el blast radius devuelto por algoritmos | `ImpactAssessment` (clasificado: compilation / runtime / contract / data / deployment / org / docs) |
| 6 | Planning | Convierte findings en planes ejecutables | `ActionPlan` (grafo de pasos con precondiciones, capabilities, approval, evidence) |
| 7 | Documentation | Mantiene coherencia entre conocimiento y docs | `DocumentationPatchProposal` (cambios incrementales, no reescritura) |
| 8 | Presenter | Crea narración adaptada al destinatario | `Presentation` (mismo grafo, distinta proyección + lenguaje) |
| 9 | Review / Critic | Revisa interpretaciones o planes antes de aceptar | `ReviewReport` (challenges, refutaciones, confianza ajustada) |

## Decisión

Los 9 agentes comparten el mismo contrato `ReactiveObserver` (ver [ADR-021](ADR-021-cognitive-layer.md)) pero cada uno tiene:

1. **Suscripciones**: patrones de eventos que le interesan.
2. **Context view requerido**: la proyección del grafo que necesita.
3. **Output schema**: la variante de `AgentOutput` que produce.
4. **Activation rules**: cuándo se dispara.
5. **Budget**: tokens / time / $ permitidos.
6. **Determinism level**: heurística pura, modelo local pequeño, o LLM potente.

### 1. Semantic Curator Agent

**Función:** mantener la calidad semántica del grafo.

```rust
pub struct SemanticCuratorAgent {
    pub id: AgentId::semantic_curator,
    pub subscriptions: vec![
        EventPattern::SymbolAdded,
        EventPattern::SymbolUpdated,
        EventPattern::EvidenceAdded,
        EventPattern::SchemaChanged,
    ],
    pub context_view: ViewDefinition::Recent + Aliases,
    pub output: MergeCandidate | AliasCandidate | ComponentAssignmentCandidate | StaleKnowledgeFinding,
    pub model_policy: ModelPolicy::LocalSmall (Phi-3 / Llama-3-8B),
    pub budget: AgentBudget { max_tokens: 4096, timeout_ms: 2000, cost_usd: 0.0 },
    pub deterministic: false,  // LLM-influenced
    pub idempotent: true,
}
```

Tareas:
- Proponer nombres conceptuales para nodos anónimos.
- Deduplicar entidades (mismo concepto con dos IDs).
- Relacionar símbolos con componentes/bounded contexts.
- Detectar aliases (`OrderCreated` ≡ `OrderCreatedEvent` ≡ `order.created`).
- Asociar código, docs y ADR.
- Detectar conocimiento obsoleto (ADR contradice código, diagrama C4 stale, etc.).

### 2. Architecture Agent

**Función:** interpretar código en términos de arquitectura (C4, capas, bounded contexts, ownership).

```rust
pub struct ArchitectureAgent {
    pub id: AgentId::architecture,
    pub subscriptions: vec![
        EventPattern::ComponentChanged,
        EventPattern::PublicApiChanged,
        EventPattern::DependencyAdded,
        EventPattern::RuntimeCallObserved,
        EventPattern::AdrContradicted,
    ],
    pub context_view: ViewDefinition::Components + RuntimeTraces + AdrReferences,
    pub output: ArchitectureAssessment {
        confirmed_relations: Vec<Relation>,
        suspected_components: Vec<Component>,
        declared_observed_drift: Vec<Drift>,
        violations: Vec<RuleViolation>,
        missing_evidence: Vec<MissingEvidence>,
        recommended_views: Vec<ProjectionSuggestion>,
    },
    pub model_policy: ModelPolicy::LocalSmall + RemotePowerful (on conflict),
    pub budget: max_tokens: 8192, timeout_ms: 5000, cost_usd: 0.02,
}
```

Compara tres arquitecturas:

- **Declarada** (Structurizr, ADR, reglas declarativas).
- **Descubierta** (AST, símbolos, manifests, dependencias estáticas).
- **Observada** (OpenTelemetry, trazas runtime, tráfico, despliegues).

Output: `ArchitectureAssessment` con confidence por hallazgo, evidencia citada, y `recommended_views` que el Projection Agent puede usar.

### 3. Projection Agent

**Función:** decidir **cómo representar** una pregunta, no solo qué datos recuperar.

```rust
pub struct ProjectionAgent {
    pub id: AgentId::projection,
    pub subscriptions: vec![
        EventPattern::UserQuery,        // "muéstrame la secuencia de CreateOrder"
        EventPattern::ContextRequest,   // el Investigation Agent pide vista
    ],
    pub context_view: ViewDefinition::AllProjections + Taxonomy,
    pub output: ProjectionSpec {
        projection: ProjectionKind,   // Sequence | C4Context | C4Container | CallGraph | ClassDiagram | ...
        root: NodeId,
        participants: GroupingStrategy,
        include: Vec<ElementKind>,
        collapse: Vec<CollapseRule>,
        evidence_mode: EvidenceMode,    // StaticOnly | StaticAndRuntime | CompareStaticRuntime
        layout_hints: LayoutHints,
    },
    pub model_policy: ModelPolicy::LocalSmall,
    pub budget: max_tokens: 2048, timeout_ms: 1000, cost_usd: 0.0,
}
```

Mapea pregunta → proyección:

| Pregunta | Proyección |
|---|---|
| ¿Qué sistemas participan? | C4 Context |
| ¿Dónde está desplegado? | C4 Deployment |
| ¿Cómo llega la petición a la BD? | Sequence |
| ¿Quién llama a esta función? | Call graph |
| ¿Cómo fluye este dato? | Data flow |
| ¿Qué implementa esta interfaz? | UML clases |
| ¿Qué puede romperse? | Impact map |
| ¿Dónde están los ciclos? | Dependency graph / matriz |
| ¿Cómo cambia en el tiempo? | Diff temporal |
| ¿Qué ocurrió realmente? | Runtime sequence |

El Projection Agent no renderiza; emite un `ProjectionSpec` que el motor de proyección (Rust) y ELK (cuando jerárquico) ejecutan de forma determinista.

### 4. Investigation Agent

**Función:** investigar hipótesis sobre el grafo con cadena argumental explícita.

```rust
pub struct InvestigationAgent {
    pub id: AgentId::investigation,
    pub subscriptions: vec![EventPattern::UserQuery, EventPattern::FailedTest, EventPattern::RuntimeAnomaly],
    pub context_view: ViewDefinition::SubgraphAround(root, depth=2) + RelevantSources,
    pub output: InvestigationReport {
        conclusion: String,
        evidence: Vec<Evidence>,         // numeradas, citables
        confidence: f32,
        limitations: Vec<String>,
        next_steps: Vec<QueryRequest>,
    },
    pub model_policy: ModelPolicy::RemotePowerful (Claude/GPT),
    pub budget: max_tokens: 16384, timeout_ms: 8000, cost_usd: 0.10,
}
```

Output incluye cadena argumental explícita:

```text
Conclusión: PaymentController depende directamente de PostgresOrderRepository.

Evidencias:
1. Constructor injection detectada en PaymentController:42.
2. Tipo concreto importado desde infrastructure/.
3. Llamada save() observada estáticamente.
4. Span SQL observado en runtime (traces/last-24h).
5. No existe relación con OrderRepositoryPort.

Confianza: 0.91 (alta).
Limitaciones: no se analizó reflexión durante startup.
```

### 5. Impact Agent

**Función:** complementar al algoritmo de propagación de impacto con interpretación semántica.

```rust
pub struct ImpactAgent {
    pub id: AgentId::impact,
    pub subscriptions: vec![EventPattern::PublicApiChanged, EventPattern::HighImpactDetected],
    pub context_view: ViewDefinition::BlastRadius + Consumers + Tests,
    pub output: ImpactAssessment {
        algorithm_summary: ImpactSummary,    // del cálculo determinista
        interpretation: ImpactInterpretation, // del LLM
        classification: Vec<ImpactClass>,    // compilation/runtime/contract/data/deployment/org/docs
        risk_level: RiskLevel,               // low/medium/high/critical
        consumers: Vec<ConsumerClassification>,  // interno / test-only / external-unversioned
    },
    pub model_policy: ModelPolicy::LocalSmall,
    pub budget: max_tokens: 4096, timeout_ms: 3000, cost_usd: 0.0,
}
```

Ejemplo de output:

```text
Algorithm: 14 símbolos alcanzables, 3 servicios, 2 APIs, 8 tests, 2 equipos.
Interpretation:
- 4 consumidores solo se usan en tests (no son impacto de runtime).
- 2 llamadas son rutas de error (impacto de contrato).
- 1 API es pública y requiere compatibilidad (impacto contractual).
- 1 consumidor externo no está versionado (riesgo).
Risk: HIGH (contrato, no implementación).
```

### 6. Planning Agent

**Función:** convertir findings en planes ejecutables.

```rust
pub struct PlanningAgent {
    pub id: AgentId::planning,
    pub subscriptions: vec![EventPattern::HighImpactDetected, EventPattern::UserQuery],
    pub context_view: ViewDefinition::Findings + Consumers + Capabilities,
    pub output: ActionPlan {
        goal: Goal,
        cause: EventId,
        assumptions: Vec<Assumption>,
        steps: Vec<PlannedStep>,  // grafo, no lista
        required_capabilities: Vec<Capability>,
        approval_level: ApprovalLevel,
        expected_evidence: Vec<EvidencePredicate>,
        rollback: RollbackStrategy,
    },
    pub model_policy: ModelPolicy::RemotePowerful (cuando plan es sensible),
    pub budget: max_tokens: 8192, timeout_ms: 5000, cost_usd: 0.05,
}
```

Output ejemplo:

```text
ValidationPlan
├── localizar consumidores externos
├── ejecutar contract tests
├── generar diff OpenAPI
├── comprobar compatibilidad
├── actualizar secuencia CreateOrder
├── proponer versión major
└── solicitar revisión del equipo Payments
```

Cada step lleva: precondiciones, command, capabilities, approval, evidence esperada, success condition.

### 7. Documentation Agent

**Función:** mantener coherencia entre conocimiento y documentación.

```rust
pub struct DocumentationAgent {
    pub id: AgentId::documentation,
    pub subscriptions: vec![EventPattern::ComponentChanged, EventPattern::StaleDetected],
    pub context_view: ViewDefinition::Component + RecentChanges + Docs,
    pub output: DocumentationPatchProposal {
        affected_fragments: Vec<DocFragmentId>,
        reason: String,
        new_content: String,
        diff: String,
        evidence: Vec<Evidence>,
    },
    pub model_policy: ModelPolicy::LocalSmall,
    pub budget: max_tokens: 6144, timeout_ms: 4000, cost_usd: 0.01,
}
```

Genera `DocumentationPatchProposal` (no reescribe todo, solo patches incrementales). La página de docs mantiene referencias vivas:

```text
ArchitecturePage
├── GENERATED_FROM → ProjectionSnapshot
├── SUPPORTED_BY → EvidenceSet
├── VALID_AT → Commit
├── DESCRIBES → Component
└── INVALIDATED_BY → GraphDelta
```

### 8. Presenter Agent

**Función:** crear narración adaptada al destinatario (developer / architect / tech lead / security / ops).

```rust
pub struct PresenterAgent {
    pub id: AgentId::presenter,
    pub subscriptions: vec![EventPattern::UserQuery, EventPattern::PresentationRequest],
    pub context_view: ViewDefinition::Subgraph + Audience,
    pub output: Presentation {
        audience: Audience,            // Developer | Architect | TechLead | Security | Ops
        projection: ProjectionSpec,
        narrative: String,             // explicación humana
        references: Vec<EvidenceRef>,
        interactive_hints: Vec<Hint>,
    },
    pub model_policy: ModelPolicy::RemotePowerful (narrative es sensible),
    pub budget: max_tokens: 4096, timeout_ms: 3000, cost_usd: 0.03,
}
```

La presentación no cambia el conocimiento; cambia la **proyección y el lenguaje**. Mismo grafo, distinta audience.

### 9. Review / Critic Agent

**Función:** revisar interpretaciones o planes antes de aceptar.

```rust
pub struct ReviewCriticAgent {
    pub id: AgentId::review_critic,
    pub subscriptions: vec![
        EventPattern::ActionProposalCreated,
        EventPattern::HypothesisCreated,
    ],
    pub context_view: ViewDefinition::Proposal + Evidence + CounterArgument,
    pub output: ReviewReport {
        challenges: Vec<Challenge>,     // "esta conclusión podría ser incorrecta porque..."
        refutations: Vec<Refutation>,
        confidence_adjustment: f32,     // delta sobre la confianza original
        recommendation: ReviewAction,   // Accept | Reject | RequestMoreEvidence
    },
    pub model_policy: ModelPolicy::RemotePowerful (siempre),
    pub budget: max_tokens: 4096, timeout_ms: 3000, cost_usd: 0.03,
}
```

No usa otro LLM necesariamente. Puede ser la misma ejecución con:
- Contexto diferente (incluye el proposal original + sus evidencias).
- Prompt de crítica ("¿esta conclusión está respaldada?").
- Acceso solo a evidencias.
- Obligación de refutar (no confirmar).
- Salida estructurada.

El review agent **rebaja la confianza** si encuentra problemas. No sube confianza sin evidencia adicional.

## Capas de implementación por ciclo

| Cycle | v1.0 (M16) | 1.x (M18 + M22) | 2.0 |
|---|---|---|---|
| Architecture Agent | ✓ heurística pura (sin LLM) | + LLM local (Phi-3) | + LLM potente |
| Projection Agent | ✓ heurística pura (taxonomy rules) | + refinement LLM | full |
| Semantic Curator | ✗ (defer) | ✓ LLM local | full |
| Investigation | ✗ (defer) | ✓ LLM potente | full |
| Impact | ✗ (defer) | ✓ heurística + LLM local | full |
| Planning | ✗ (defer) | ✓ básico | full |
| Documentation | ✗ (defer) | ✓ básico | full |
| Presenter | ✗ (defer) | ✗ | full |
| Review / Critic | ✗ (defer) | ✗ | full |

Para v1.0, **2 agentes** funcionan end-to-end con heurística pura. Sirven para validar el contrato y la infraestructura.

## Consecuencias

### Positivas

- Cobertura amplia: cada aspecto del workbench (semántica, arquitectura, proyección, investigación, impacto, planning, docs, presentación, review) tiene un agente dedicado.
- Output estructurado y verificable (cada agente emite una variante de `AgentOutput`).
- Ladder de resolución minimiza costo: heurística → local → potente.
- Coordinación vía estado, no conversación (más testeable, replay-friendly).
- Agents pueden ser reemplazados/mejorados independientemente (mismo contrato).

### Negativas

- 9 agentes es mucho para mantener. Pero cada uno es chico (~100-300 LOC Rust + 50-100 LOC TS).
- LLMs potentes cuestan: cada invocation a Claude/GPT es ~$0.01-0.10. Para 1000 invocaciones/día = $10-100/día. Hay que monitorar.
- Tests son difíciles: comportamiento no determinista. Mitigación: evaluation datasets, golden queries, canary.
- Algunos agentes requieren telemetría / modelos externos. Dependencia operacional.
- "9 agentes" puede sonar a over-engineering. Pero cada uno tiene un scope claro y verificable.

### Métricas de éxito

- 9 agentes implementados al fin de M22 (1.x): 1.x target.
- 2 agentes (Architecture, Projection) funcionando end-to-end al fin de v1.0: M16 target.
- Latencia media de respuesta (todos los agentes): <3s para LLM potente, <500ms para local.
- 0 invocaciones a LLM que excedan budget.
- % de outputs de agentes aceptados sin review humano: >70% (heurística + local).
- % de ActionProposals ejecutados sin override: >80% (Policy Engine calibrado).

## Cómo revertir

| Decisión | Reversión |
|---|---|
| 9 agentes | Reducir a 2-3 core. Mantener los que aporten valor medido. |
| LLM local | Solo LLM potente. Más simple, más caro. |
| LLM potente | Solo heurística + LLM local. Para casos triviales. |
| Review / Critic | Skip. Confiar en el primer agente. |
| MCP gateway | Sustituir por wrapper custom. |

## Referencias

- `docs/Librerías-visualización-grafos-BI.md` — sección "Code Knowledge Graph Workbench" (catálogo fuente)
- [ADR-021](ADR-021-cognitive-layer.md) — contrato uniforme de agente
- [ADR-023](ADR-023-action-proposal-and-policy.md) — ActionProposal + Policy Engine
- [ADR-013](ADR-013-viewer-ortogonal.md) — workbench (donde viven los agentes)
- [ADR-019](ADR-019-performance-budget.md) — hard contract (los agentes no rompen el budget)
