# ADR-026 — State Machine Metamodel + AST Extraction Strategy

> **Ciclo:** `b1-source-evaluation-types`
> **Estado:** Aceptado (propuesto)
> **Fecha:** 2026-08-04
> **Complementa:** [ADR-009](ADR-009-relaciones-semanticas-reificadas.md) + [ADR-016](ADR-016-activegraph-packs-investigacion.md) §B1

## Problema

El sistema actual soporta 5 tipos de proyecciones de diagrama: C4 (context, container, component), UML class, sequence, y use case. La extracción de **máquinas de estados** no existe: no hay metatypes para representar estados, transiciones, guards ni eventos, y no hay extractor AST que detecte patrones de state machine en código.

El gap impide que los agentes completen el ciclo de creación para diagramas de comportamiento dinámico (e.g., order lifecycle, payment state machine, session state).

## Decisiones

### D1 — 5 metatypes nuevos para máquinas de estados

**Elección:** Añadir al metamodelo declarativo (`docs/schema/metamodel-core.json`) los siguientes metatypes:

| Metatype | Namespace | Categoría | Descripción |
|----------|-----------|-----------|-------------|
| `uml.state_machine` | uml | behavior | Contenedor de una máquina de estados con nombre |
| `uml.state` | uml | behavior | Un estado named (regular, no pseudostate) |
| `uml.pseudostate` | uml | behavior | Estado especial: initial, final, choice, junction |
| `uml.transition` | uml | behavior | Arista dirigida entre estados |
| `uml.guard` | uml | behavior | Condición booleana en una transición |
| `uml.event` | uml | behavior | Disparador de una transición |

**Rationale:** `uml.state_machine` como contenedor sigue el patrón de `behavior.scenario` (contenedor de interacciones). `uml.pseudostate` modela los estados especiales de UML que no son estados regulares. `uml.transition` como metatype (no relación reificada) es consistente con el modelo directo de archctl (ADR-009).

**Alternativas rechazadas:**
- Reificar `Transition` como relación con nodos propios — rechazado: el modelo de archctl usa `SemanticEdge` directo con `predicate_id` para representar relaciones entre Elementos. Usar un nodo `Transition` separaría la semántica del modelo.
- Añadir solo `uml.state` y delegar transiciones a `code.transitions` — rechazado: transiciones tienen semántica propia (trigger, guard) que no cabe en `code`.

### D2 — 5 predicates nuevos para transiciones

**Elección:** Añadir al metamodelo declarativo los siguientes predicates:

| Predicate | Namespace | Directed | Descripción |
|-----------|-----------|----------|-------------|
| `behavior.source_state` | behavior | true | transition → state (estado origen) |
| `behavior.target_state` | behavior | true | transition → state (estado destino) |
| `behavior.has_transition` | behavior | true | state_machine → transition |
| `behavior.trigger` | behavior | true | transition → event (disparador) |
| `behavior.has_guard` | behavior | true | transition → guard |

**Rationale:** `behavior.source_state`/`behavior.target_state` son análogos a `behavior.sender`/`behavior.receiver` para interacciones. `behavior.has_transition` conecta el contenedor con sus transiciones. `behavior.trigger` y `behavior.has_guard` representan la semántica condicional de las transiciones.

### D3 — Extractor AST-puro; semántica ambigua vía `evidence put`

**Elección:** El extractor `code state-machine` sigue el patrón `extract()`/`apply()` de `call_graph.rs` y `class_diagram.rs`: tree-sitter CST walk → carrier types deterministas → MERGE apply-time. La semántica que NO puede inferirse del AST (guards con condiciones complejas, eventos de negocio, transiciones condicionales) se deja al **agente** que usa `evidence put` para completar el grafo.

**Lo que el extractor detecta de forma determinista:**
- Rust: `enum Foo { A, B, C }` con `match x { Foo::A => ..., Foo::B => ... }` → estados (variants) + transiciones (arms)
- TypeScript: `type Foo = "A" | "B" | "C"` con `switch (x) { case "A": ... }` → estados + transiciones
- Python: clases con decorador `@transition("A", "B")` (librería `transitions`) → estados + transiciones

**Lo que NO detecta (responsabilidad del agente vía `evidence put`):**
- Guard conditions complejas: `if x > 5 && y < 3`
- Eventos de negocio: `OrderConfirmed`, `PaymentReceived` como strings
- Transiciones implícitas: métodos que devuelven `Self` sin `match`
- History states, fork/join, deferred events

**Rationale:** Intentar inferir guards/events sin convenciones de naming produciría **falsos positivos** que degradan la confianza del grafo. El patrón híbrido (extractor determinista + agente completa semántica) es el mismo que usa C4 discovery.

### D4 — MERGE apply-time, no migración

**Elección:** El seeding de los 6 metatypes y 5 predicates se hace vía `MERGE` en `apply()` del extractor, **no** mediante una nueva migración de schema.

**Rationale:** Los metatypes son declaraciones de dominio, no cambios de estructura del grafo. Las migraciones (`001_initial_schema`, `002_source_evaluation`, `003_view_nodes`) crean la estructura (tablas `Element`, `ElementVersion`, `Evidence`, `MetaType`, `Predicate`, columnas). El contenido de `MetaType`/`Predicate` es información — se puebla via MERGE idempotente. Esto es consistente con cómo `call_graph.rs` y `class_diagram.rs` siembran sus propios metatypes (ADR-009: el grafo se auto-documenta).

**Alternativas rechazadas:**
- Nueva migración `004_state_machine_schema.cypher` — rechazado: no hay nuevas tablas ni columnas, solo entradas en MetaType/Predicate. Overhead de migración innecesario.
- Poblar desde `metamodel-core.json` en una migración — rechazado: el JSON es declarativo, no es la fuente de verdad runtime. El extractor es la fuente de verdad.

### D5 — Confidence < 1.0 para carrier types heuristic

**Elección:** Los `StateMachine` carrier types produced by `extract()` carry a `confidence: f64` field. When heuristics were applied (e.g., inferring transitions from `match` arms without exhaustive checking), confidence is set to 0.7. When the pattern is unambiguous (e.g., `#[state(exclusive)]` attribute in Rust), confidence is 1.0.

**Rationale:** Confidence < 1.0 marca incertidumbre para que el agente sepa qué hechos aceptar/rechazar vía el lifecycle `drafted → accepted`. Esto es consistente con `class_diagram.rs` que juga `confidence` en version props.

---

## Metamodelo — entradas exactas para `metamodel-core.json`

```json
// En "metatypes":
{
  "id": "uml.state_machine",
  "namespace": "uml",
  "name": "State Machine",
  "category": "behavior",
  "schema_version": 1,
  "property_schema": {},
  "validation_rules": {},
  "renderer_hints": {},
  "description": "Contenedor de estados y transiciones que modela el comportamiento de un objeto con lifecycle"
},
{
  "id": "uml.state",
  "namespace": "uml",
  "name": "State",
  "category": "behavior",
  "schema_version": 1,
  "property_schema": {},
  "validation_rules": {},
  "renderer_hints": {},
  "description": "Un estado nomméico en una máquina de estados"
},
{
  "id": "uml.pseudostate",
  "namespace": "uml",
  "name": "Pseudostate",
  "category": "behavior",
  "schema_version": 1,
  "property_schema": {},
  "validation_rules": {},
  "renderer_hints": {},
  "description": "Estado especial UML: initial, final, choice, junction, shallow history, deep history"
},
{
  "id": "uml.transition",
  "namespace": "uml",
  "name": "Transition",
  "category": "behavior",
  "schema_version": 1,
  "property_schema": {},
  "validation_rules": {},
  "renderer_hints": {},
  "description": "Arista dirigida entre dos estados en una máquina de estados"
},
{
  "id": "uml.guard",
  "namespace": "uml",
  "name": "Guard",
  "category": "behavior",
  "schema_version": 1,
  "property_schema": {},
  "validation_rules": {},
  "renderer_hints": {},
  "description": "Condición booleana que debe cumplirse para que una transición ocurra"
},
{
  "id": "uml.event",
  "namespace": "uml",
  "name": "Event",
  "category": "behavior",
  "schema_version": 1,
  "property_schema": {},
  "validation_rules": {},
  "renderer_hints": {},
  "description": "Disparador de una transición en una máquina de estados"
}

// En "predicates":
{
  "id": "behavior.source_state",
  "namespace": "behavior",
  "name": "Source State",
  "directed": true,
  "transitive": false,
  "symmetric": false,
  "schema_version": 1,
  "allowed_pairs": [],
  "property_schema": {},
  "validation_rules": {},
  "renderer_hints": {},
  "description": "La transición tiene origen en este estado"
},
{
  "id": "behavior.target_state",
  "namespace": "behavior",
  "name": "Target State",
  "directed": true,
  "transitive": false,
  "symmetric": false,
  "schema_version": 1,
  "allowed_pairs": [],
  "property_schema": {},
  "validation_rules": {},
  "renderer_hints": {},
  "description": "La transición tiene destino en este estado"
},
{
  "id": "behavior.has_transition",
  "namespace": "behavior",
  "name": "Has Transition",
  "directed": true,
  "transitive": false,
  "symmetric": false,
  "schema_version": 1,
  "allowed_pairs": [],
  "property_schema": {},
  "validation_rules": {},
  "renderer_hints": {},
  "description": "La máquina de estados contiene esta transición"
},
{
  "id": "behavior.trigger",
  "namespace": "behavior",
  "name": "Trigger",
  "directed": true,
  "transitive": false,
  "symmetric": false,
  "schema_version": 1,
  "allowed_pairs": [],
  "property_schema": {},
  "validation_rules": {},
  "renderer_hints": {},
  "description": "La transición es disparada por este evento"
},
{
  "id": "behavior.has_guard",
  "namespace": "behavior",
  "name": "Has Guard",
  "directed": true,
  "transitive": false,
  "symmetric": false,
  "schema_version": 1,
  "allowed_pairs": [],
  "property_schema": {},
  "validation_rules": {},
  "renderer_hints": {},
  "description": "La transición tiene esta condición de guarda"
}
```

---

## Lo que NO captura este diseño

1. ** guards/events complejos** — condiciones que requieren análisis de valor (e.g., `x > 5`) o eventos con parámetros (`OrderConfirmed { order_id: 42 }`). El agente los define via `evidence put`.
2. **State chart UML completo** — fork, join, entry/exit actions, deferred events. MVP solo modela estados, transiciones, guards y triggers simples.
3. **Comunicación entre state machines** — señales, eventos de dominio cruzados. Modelados como edges `code.calls` existentes o como `evidence put` del agente.
4. **Transiciones implícitas** — métodos que devuelven `Self` sin match exhaustivo. El extractor solo sigue `match` arms.

---

## Consequences

### Positivos
- Metamodelo declarativo actualizado con 6 metatypes y 5 predicates para máquinas de estados
- Extractor reutilizable en Rust (enum+match), TypeScript (state libs), Python (transitions)
- Confidence < 1.0 marca heurística para el agente
- Separation of concerns: extractor = estructural, agente = semántico

### Negativos
- El agente necesita entender cuándo usar `evidence put` para completar transiciones ambiguas
- `confidence < 1.0` requiere que las queries filtren por confidence si quieren solo hechos ciertos

### Riesgos residuales
- State machines sin match exhaustivo generan transiciones faltantes (mitigado: confidence 0.7)
- Patrones de naming no convencionales producen falsos negativos (mitigado: documentación de heurísticas)

## Prohibiciones

- El extractor NO debe inferir guards/events sin convenciones de naming explícitas (e.g., `if_guard_`, `on_` prefixes)
- El extractor NO debe modificar `status` de Evidence a `accepted` — eso es responsabilidad del agente
- El seeding de metatypes NO debe vivir en una migración — debe vivir en `apply()` del extractor
