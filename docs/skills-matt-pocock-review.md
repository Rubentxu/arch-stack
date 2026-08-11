# Matt Pocock Skills Evaluation — arch-stack

> Evaluación de los 9 skills + 5 agents + 1 plugin de arch-stack contra los
> criterios de Matt Pocock (Skills For Real Engineers, 2026).
> Baseline: [mattpocock/skills](https://github.com/mattpocock/skills)

## Criterios de evaluación

| # | Criterio | Descripción |
|---|---|---|
| C1 | **Trigger claro** | El skill/agent tiene triggers explícitos en description (when to invoke) |
| C2 | **Composabilidad** | Hace UNA cosa bien; reutilizable en diferentes flujos |
| C3 | **Output determinista** | Schema de output versionado; salida predecible |
| C4 | ** контракт ejecutable** | Comandos concretos, con ejemplos de CLI; ningún paso ambiguo |
| C5 | **Distinción user/model** | Claramente model-invoked o user-invoked |
| C6 | **Frontal de calidad** | Tiene gate de calidad propio (validate, review, evidence check) |
| C7 | **CI eval** | Routing testeable en CI (evita que silenciosamente deje de funcionar) |
| C8 | **Direcciona failure modes de IA** | Mitiga un failure mode conocido (fabricación, desalineo, verbosidad) |

**Escala**: ✅ Fuertemente, ⚠️ Parcialmente, ❌ Ausente, N/A No aplica

---

## Skills

### 1. `architecture-discovery` — ✅ FUERTE

| Criterio | Score | Notas |
|---|---|---|
| Trigger claro | ✅ | "Use when starting work on a repo, after a refactor, or when asked to discover/scan/map the architecture" |
| Composabilidad | ✅ | Extrae e inyecta en el grafo; composable con cualquier skill queLea el grafo |
| Output determinista | ✅ | `output-schema: c4-discover-report-v1`, JSON estruturado |
| Контракт ejecutable | ✅ | 5 pasos concretos con CLI ejemplo por paso |
| Distinción user/model | ✅ | Model-invoked (el agente lo dispara tras `architecture-discovery`) |
| Frontal de calidad | ✅ | `evidence accept` como gate post-discovery |
| CI eval | ⚠️ | Routing testeable por frontmatter, pero no hay CI eval en el repo |
| Direcciona failure modes | ✅ | Extrae con provenance → mitiga fabricación de relaciones |

**Veredicto**: Es el skill más sólido de arch-stack. Es el equivalente funcional del `/research` de Pocock (investigar facts con provenance).

---

### 2. `c4-from-graph` — ✅ FUERTE

| Criterio | Score | Notas |
|---|---|---|
| Trigger claro | ✅ | "Use when the user asks for any C4 diagram at a known level and root" |
| Composabilidad | ✅ | Proyector puro; encadenable con architecture-discovery |
| Output determinista | ✅ | `output-schema: c4-view-spec-v1` |
| Контракт ejecutable | ✅ | Selector grammar explícito, 2 tipos de output (bundle + DSL) |
| Distinción user/model | ✅ | Model-invoked |
| Frontal de calidad | ⚠️ | La validación va por `diagram-review` (separado), no inline |
| CI eval | ⚠️ | Mismo gap que C7 general |
| Direcciona failure modes | ✅ | Proyección determinista sobre IDs canónicos → mitiga fabricación |

**Veredicto**: Sólido. Equivalente funcional del `/codebase-design` de Pocock (proyectar una vista desde datos estructurados).

---

### 3. `class-view-from-graph` — ✅ FUERTE

| Criterio | Score | Notas |
|---|---|---|
| Trigger claro | ✅ | "Use when the user wants class structure, interface contracts, aggregates, or module boundaries" |
| Composabilidad | ✅ | Scope control (module: vs file:) permite composición |
| Output determinista | ✅ | `output-schema: uml-class-spec-v1` |
| Контракт ejecutable | ✅ | CLI completa con selector y dry-run |
| Distinción user/model | ✅ | Model-invoked |
| Frontal de calidad | ⚠️ | diagram-review es gate separado |
| CI eval | ⚠️ | Mismo gap |
| Direcciona failure modes | ✅ | AST-puro mitiga fabricación de estructura de clases |

**Veredicto**: Comparable a `c4-from-graph` en calidad. Equivalente al `/codebase-design` de Pocock para UML.

---

### 4. `diagram-review` — ✅ DESTACADO

| Criterio | Score | Notas |
|---|---|---|
| Trigger claro | ✅ | "Use as the final gate of any diagram invocation" |
| Composabilidad | ✅ | Gate genérico reutilizable para cualquier tipo de diagrama |
| Output determinista | ✅ | JSON verdict (PASS/FAIL_WITH_REASONS) |
| Контракт ejecutable | ✅ | 4 pasos con validación cruzada (schema + graph + evidence) |
| Distinción user/model | ✅ | Model-invoked; gate automático |
| Frontal de calidad | ✅ | Tiene su propio gate (valida antes de aceptar) |
| CI eval | ⚠️ | Mismo gap |
| Direcciona failure modes | ✅ | Addressa el failure mode #1 de Pocock: "The Agent Is Way Too Verbose" — el reviewer corta la verbosidad del agente sobre el grafo |

**Veredicto**: El skill más alineado con la filosofía de Pocock. Es un gate automático que fuerza al agente a confrontar evidencia real. Equivale al `/code-review` de Pocock ( Standards + Spec review en paralelo).

---

### 5. `evidence-lifecycle` — ✅ FUERTE

| Criterio | Score | Notas |
|---|---|---|
| Trigger claro | ✅ | "Use when accepting discoveries, reviewing drafted evidence, replacing stale facts, or auditing what backs a diagram" |
| Composabilidad | ✅ | Estado finito drafted→accepted→superseded; reutilizable en cualquier flujo |
| Output determinista | ✅ | `output-schema: evidence-lifecycle-v1` |
| Контракт ejecutable | ✅ | Comandos concretos por estado |
| Distinción user/model | ✅ | Model-invoked |
| Frontal de calidad | ✅ | Es intrinscamente un mecanismo de calidad (el lifecycle ES la calidad) |
| CI eval | ⚠️ | Mismo gap |
| Direcciona failure modes | ✅ | addressa "fabricated relationships" — cada hecho requiere evidence backing |

**Veredicto**: Skill de infraestructura crítica. Equivale al layer de evidencia en un sistema de type checking — sin él, todo se rompe.

---

### 6. `sequence-from-scenario` — ✅ FUERTE

| Criterio | Score | Notas |
|---|---|---|
| Trigger claro | ✅ | "Use when the user asks for runtime trace, call path, or inter-service choreography" |
| Composabilidad | ✅ | Cap depth + max-interactions; composable con call-graph |
| Output determinista | ✅ | `output-schema: uml-sequence-spec-v1` |
| Контракт ejecutable | ✅ | CLI completa; dry-run; control de volumen |
| Distinción user/model | ✅ | Model-invoked |
| Frontal de calidad | ⚠️ | diagram-review como gate separado |
| CI eval | ⚠️ | Mismo gap |
| Direcciona failure modes | ✅ | Scope discipline (depth cap) mitiga unbounded output |

**Veredicto**: Bien diseñado. El truncamiento explícito (`--depth`, `--max-interactions`) es buena práctica que falta en muchos skills de Pocock.

---

### 7. `stack-management` — ⚠️ MIXED

| Criterio | Score | Notas |
|---|---|---|
| Trigger claro | ✅ | "Use when setting up a new machine, upgrading versions, checking component alignment, or onboarding" |
| Composabilidad | ✅ | Operaciones discretas (install, update, status, doctor) |
| Output determinista | ✅ | Output estructurado (doctor JSON) |
| Контракт ejecutable | ✅ | CLI concreta por operación |
| Distinción user/model | ⚠️ | Ambiguo: es user-invoked (setup) pero también se invoca automáticamente en doctor |
| Frontal de calidad | ✅ | `archctl doctor` es un self-check robusto |
| CI eval | ⚠️ | Mismo gap |
| Direcciona failure modes | ✅ | Addressa desalineo de versiones entre componentes |

**Veredicto**: Funciona bien. La distinción user/model podría clarificarse. No es un skill de "codificación" sino de "infraestructura" — eso está bien, pero está en un limbo entre skill y comando. El `/setup` de Pocock es similar (setup por repo).

---

### 8. `use-cases-from-graph` — ⚠️ PARCIAL

| Criterio | Score | Notas |
|---|---|---|
| Trigger claro | ✅ | "Use when the user asks for use cases, actor mapping, or system landscape" |
| Composabilidad | ⚠️ | Depende de state-machine + evidence put; acoplamiento alto |
| Output determinista | ✅ | `output-schema: uml-usecase-spec-v1` |
| Контракт ejecutable | ✅ | Pasos concretos |
| Distinción user/model | ✅ | Model-invoked |
| Frontal de calidad | ⚠️ | diagram-review como gate separado; evidencia requerida |
| CI eval | ⚠️ | Mismo gap |
| Direcciona failure modes | ✅ | Distingue candidatos inferidos vs confirmados mitiga fabricación |

**Veredicto**: `maturity: experimental` en frontmatter — la composición es el punto débil. Requiere state-machine + evidence put en cadena; si falta uno, el skill no funciona. Pocock diría que intenta demasiado.

---

### 9. `workbench-view` — ✅ FUERTE

| Criterio | Score | Notas |
|---|---|---|
| Trigger claro | ✅ | "Use when the user wants to explore diagrams interactively, pan/zoom graphs, or inspect a bundle in the browser" |
| Composabilidad | ✅ | Stateless; recibe bundle y sirve; idempotente |
| Output determinista | ✅ | URL determinista + HTTP health check |
| Контракт ejecutable | ✅ | 4 pasos triviales |
| Distinción user/model | ✅ | User-invoked (launch del workbench) |
| Frontal de calidad | ⚠️ | El health check es del server, no del skill |
| CI eval | ⚠️ | Mismo gap |
| Direcciona failure modes | ✅ | Abstrae la complejidad de archview para el usuario |

**Veredicto**: Skill más simple pero necesario. Equivale a "/open-browser" en Pocock — existe porque el modelo no puede abrir browsers (aún).

---

## Agents

### 10. `diagram-architect` — ✅ DESTACADO

| Criterio | Score | Notas |
|---|---|---|
| Descripción clara | ✅ | "Orchestrates evidence-driven C4 + UML diagram generation" |
| Responsabilidades delimitadas | ✅ | 5 responsabilidades explícitas |
| Delega en vez de implementar | ✅ | Delegation map + Nunca hace extracción directamente |
| Contrato ejecutable | ✅ | Comandos concretos por responsabilidad |
| Modo claro | ✅ | `mode: primary` + `model: default` |
| Direcciona failure modes | ✅ | "Never invent relationships" explicit; surface evidence + uncertainties |

**Veredicto**: El mejor agent de arch-stack. Equivale al `ask-matt` router de Pocock (decide qué hacer) + `implement` (orquesta). El Never block es excelente.

---

### 11. `architecture-evidence` — ✅ FUERTE

| Criterio | Score | Notas |
|---|---|---|
| Scope limitado | ✅ | "Only agent that talks to source tree" |
| Контракт ejecutable | ✅ | Comandos concretos |
| No cruza responsabilidades | ✅ | Solo extrae y query; nunca diagrama |
| Equivalente a skill | ✅ | Modela un skill como agent |
| Direcciona failure modes | ✅ | "Inspect through archctl, never directly" — enforced |

**Veredicto**: Bien scopeado. Equivale al `evidence-lifecycle` skill elevado a agent. Fuerte.

---

### 12. `c4-modeler` — ✅ FUERTE

| Criterio | Score | Notas |
|---|---|---|
| Scope limitado | ✅ | Solo C4; no cruza a UML |
| Контракт ejecutable | ✅ | Selector grammar + validación antes de handoff |
| Nivel de C4 correcto | ✅ | Entiende que Context excluye Components, etc. |
| Evidence check | ✅ | `evidence list` antes de aceptar |
| Direcciona failure modes | ✅ | "Never synthesize edges" — mitigación explícita |

**Veredicto**: Equivale al `c4-from-graph` skill elevado a agent con validación inline. Muy alineado con Pocock.

---

### 13. `diagram-reviewer` — ✅ DESTACADO

| Criterio | Score | Notas |
|---|---|---|
| Gate explícito | ✅ | "You are the gate before a diagram is accepted" |
| Fail criteria claras | ✅ | PASS / PASS_WITH_WARNINGS / FAIL con ids |
| Контракт ejecutable | ✅ | Comandos concretos por tipo de verificación |
| Equivalente a skill | ✅ | Mapea 1:1 con `diagram-review` skill |
| Direcciona failure modes | ✅ | "Reject renderable-but-unsupported diagrams" |

**Veredicto**: El gate que todo agent necesita. Equivale al `/code-review` de Pocock. Muy alineado.

---

### 14. `uml-modeler` — ⚠️ PARCIAL

| Criterio | Score | Notas |
|---|---|---|
| Scope amplio | ⚠️ | Class + Sequence + Use case + State + Activity — 5 tipos |
| Composabilidad interna | ⚠️ | 5 ramas de implementación distintas |
| Контракт ejecutable | ✅ | CLI concreta por tipo |
| Scope discipline | ✅ | "Prefer module/aggregate over class:*" — disciplina correcta |
| Direcciona failure modes | ✅ | Cap depth/interactions explícitos |

**Veredicto**: Pocock diría que intenta demasiado en un solo agent. Equivale a 5 skills distintas en un solo archivo. Recomendación: splittear en `class-diagram-modeler`, `sequence-modeler`, `usecase-modeler`, `state-machine-modeler`.

---

## Plugin

### 15. `archctl-env.ts` — ✅ MINIMALISTA

| Criterio | Score | Notas |
|---|---|---|
| Scope mínimo | ✅ | Solo inyecta env vars (projectId, projectDir, sourceIdentity) |
| Sin efectos secundarios | ✅ | Read-only (spawnSync con --json) |
| Error handling | ✅ | null check + status validation |
| Alineado con arch-stack | ✅ | Integra con `archctl project resolve` |
| Composabilidad | ✅ | Plugin de contexto; no interfiere con otros plugins |

**Veredicto**: Sólido para lo que hace. Equivale a un `env` resolver. Muy Pocock — una cosa, bien hecha.

---

## Gap transversal: CI Evaluation Framework

**Ausente en todos los skills y agents.**

Matt Pocock tiene un eval framework en su repo que corre en CI y verifica:
1. Skills que hacen routing correcto (`/ask-matt` routing check)
2. Descriptions que usan el vocabulario que los usuarios realmente dicen
3. Ningún par de skills colisiona en routing

**arch-stack no tiene esto.** Los triggers son textuales y podrían colisionar:
- `architecture-discovery` (discover/scan/map) y `stack-management` (setup/onboarding) comparten vocabulario
- `c4-from-graph` y `class-view-from-graph` podrían competir si el usuario dice "diagram"

**Recomendación**: Añadir `profile/skills/eval/` con routing tests. Ejemplo:
```bash
# Test que architecture-discovery se dispara con "map the architecture"
# Test que c4-from-graph NO se dispara con "map" (es discovery)
# Test que diagram-review NO se dispara solo (necesita un diagrama primero)
```

---

## Scorecard Summary

| # | Skill/Agent | C1 | C2 | C3 | C4 | C5 | C6 | C7 | C8 | Veredicto |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | architecture-discovery | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ FUERTE |
| 2 | c4-from-graph | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ⚠️ | ✅ | ✅ FUERTE |
| 3 | class-view-from-graph | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ⚠️ | ✅ | ✅ FUERTE |
| 4 | diagram-review | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ DESTACADO |
| 5 | evidence-lifecycle | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ FUERTE |
| 6 | sequence-from-scenario | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ⚠️ | ✅ | ✅ FUERTE |
| 7 | stack-management | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ | ⚠️ | ✅ | ⚠️ MIXED |
| 8 | use-cases-from-graph | ✅ | ⚠️ | ✅ | ✅ | ✅ | ⚠️ | ⚠️ | ✅ | ⚠️ PARCIAL |
| 9 | workbench-view | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ⚠️ | ✅ | ✅ FUERTE |
| 10 | diagram-architect | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ DESTACADO |
| 11 | architecture-evidence | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ FUERTE |
| 12 | c4-modeler | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ FUERTE |
| 13 | diagram-reviewer | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ DESTACADO |
| 14 | uml-modeler | ⚠️ | ⚠️ | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | ⚠️ PARCIAL |
| 15 | archctl-env.ts | ✅ | ✅ | ✅ | ✅ | N/A | N/A | ❌ | ✅ | ✅ MINIMALISTA |

**Conteo**: 9 FUERTE/DESTACADO, 3 MIXED/PARCIAL, 1 MINIMALISTA, 1 GAP (CI eval)

---

## Findings Priorizados

### CRÍTICO (resolver en siguiente ciclo)

1. **`uml-modeler` hace demasiado** — 5 tipos de UML en un solo agent.
   Splittear en 4 agents: `class-diagram-modeler`, `sequence-diagram-modeler`,
   `usecase-modeler`, `state-machine-modeler`. Cada uno con 1 tipo de output.
   **Estimación**: 1 ciclo SDD pequeño.

2. **`use-cases-from-graph` tiene madurez experimental + acoplamiento alto** —
   Depende de state-machine + evidence put en cadena. Si falta evidencia,
   el skill no funciona. Considerar: (a) marcar como deprecated y splittear
   en `actor-extraction` + `usecase-discovery`, o (b) bajar a internal-only.
   **Estimación**: 1 ciclo SDD.

### ALTO (resolver en el roadmap)

3. **Ausencia de CI eval framework** — Ningún skill tiene routing tests.
   Pocock dice: "If a skill stops triggering, CI catches it rather than you
   discovering the problem silently later." Añadir `profile/skills/eval/`
   con routing tests para los 9 skills.
   **Estimación**: 0.5 ciclo SDD.

### MEDIO (mejorar cuando haya demanda)

4. **`stack-management` distinción user/model ambigua** — Funciona como
   user-invoked (setup) pero también como health-check automático. Clarificar
   en description si es user-invoked o model-invoked. Si puede ser ambos,
   documentar cuándo cuál.

5. **`diagram-review` / `diagram-reviewer` duplicación** — El skill y el agent
   hacen cosas similares pero no son equivalentes. El skill es para el flujo
   CLI; el agent es para el flujo orquestado. No es un bug, pero debería
   estar documentado en AGENTS.md para evitar confusión.

---

## Conclusión

**arch-stack tiene skills de alta calidad por encima de la media de la industria.** La mayoría:
- Tienen triggers claros ✅
- Son composables ✅
- Tienen output determinista con schema versionado ✅
- Addressan failure modes de IA específicos ✅

**Los gaps respecto a Pocock:**
1. CI eval framework (ausente en todo el stack)
2. `uml-modeler` sobredimensionado (hace 5 cosas)
3. `use-cases-from-graph` experimental + acoplado
4. Duplicación skill/agent no documentada

**Lo que arch-stack hace mejor que Pocock:**
- Frontmatter con `license`, `maturity`, `output-schema` (Pocock no tiene esto)
- Contrato de evidence explícito (Pocock no tiene equivalente directo)
- Separation extraction/projection/juggling (Pocock mezcla estos en skills)

**Recomendación final**: Priorizar split de `uml-modeler` + CI eval framework en el próximo ciclo de mantenimiento.
