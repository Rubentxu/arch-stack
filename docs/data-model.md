# Data Model — `archctl`

> Documento fundacional del modelo de grafo sobre el que se asientan
> `archctl scan`, `archctl extract`, `archctl model build`, `archctl
> render` y `archctl explain`. Antes de implementar nada, este es el
> contrato.
>
> Anclajes: `docs/adr/README.md` ADR-0001 (C4+UML), ADR-0002
> (evidencia antes que diagramas), ADR-0004 (subagentes), ADR-0005
> (Structurizr canónico, otros son proyecciones), ADR-0007 (persistencia
> XDG, renderers locales). Diagrama textual de la idea global en
> `Skills-para-agentes-IA.md` líneas 1011-1051.

## 1. Principio rector

**Un único Property Graph para C4 y UML. Los diagramas son
proyecciones, nunca primeros ciudadanos.**

Por qué un grafo y no un árbol:

- C4 es jerárquico pero no estricto: un `container` puede aparecer
  dentro de un `softwareSystem` Y dentro de un `deploymentNode` Y ser
  referenciado por un `useCase`. Forzar árbol convierte el modelo en
  un mapa de jerarquías incompatibles.
- UML corta transversalmente sobre C4: una `class` vive dentro de un
  `component`, pero un `useCase` atraviesa varios `container`, y un
  `sequence` puede enlazar lifelines que viven en containers
  distintos.
- Las extensiones futuras (nuevos kinds, nuevas relaciones, nuevas
  vistas) entran sin migrar el esquema: discriminated unions +
  `properties` libre + `tags`.

Por qué persistencia en texto y no un servidor de grafo
(líneas 3675-3685 del doc inicial):

> La mayor aportación propia no sería analizar código, sino fusionar
> evidencias heterogéneas, conservar su procedencia y convertirlas en
> un grafo arquitectónico recuperable, completamente desacoplado del
> repositorio fuente.

Texto + índice derivable cubre eso con menos operación que Neo4j o
Memgraph. Si en el futuro hace falta un grafo distribuido, se
reproyecta desde el `graph.json` canónico.

## 2. Forma del modelo

Tres tipos de "objetos" solamente, todo lo demás es tipo + propiedades
+ evidencia.

### Node

```ts
interface Node {
  id: NodeId;                    // "<kind>:<kebab-name>" — p.ej. "container:orders-api"
  kind: NodeKind;
  name: string;                  // nombre humano
  namespace?: string;            // bounded context o package path
  tags: string[];                // clasificación arbitraria (no canónica)
  properties: Record<string, unknown>;  // ext-bag; nada crítico va aquí
  evidenceRefs: EvidenceRef[];   // nunca vacío si classification ∈ {fact, inference}
  classification: Classification;
  confidence: number;            // 0..1
  method: EvidenceMethod;
  sourceIdentityRef: string;     // apunta al SourceIdentity que lo produjo
  capturedAt: string;            // ISO-8601
  capturedBy: string;            // "<extractor>@<version>" | "human-overridden"
}
```

### Edge

```ts
interface Edge {
  id: EdgeId;                    // "rel:<sha256[:16]>"
  kind: EdgeKind;
  source: NodeId;
  target: NodeId;
  description?: string;
  technology?: string;
  via?: string;                  // "imports" | "calls" | "publishes" | "tls" | ...
  order?: number;                // secuencia / actividad: orden topológico
  tags: string[];
  properties: Record<string, unknown>;
  evidenceRefs: EvidenceRef[];
  classification: Classification;
  confidence: number;
  method: EvidenceMethod;
  sourceIdentityRef: string;
  capturedAt: string;
  capturedBy: string;
}
```

### View (proyección, NO parte del grafo)

```ts
interface View {
  id: string;                    // "view:<slug>" — humano, lo elige el usuario
  viewType: ViewType;
  title: string;
  description?: string;
  scope: ViewScope;
  layoutHints?: { rankDir?: "TB" | "LR"; autoLayout?: boolean };
  derivedFromViewId?: string;
}

interface ViewScope {
  includeNodeKinds?: NodeKind[];
  excludeNodeIds?: NodeId[];
  includeEdgeKinds?: EdgeKind[];
  subgraphRoots?: NodeId[];      // bounded contexts / containers a expandir
  maxDepth?: number;             // 1=context, 2=container, 3=component, 4=codeElement
}
```

## 3. Taxonomía

### NodeKind

```text
# C4
person
softwareSystem
container
component
codeElement
boundedContext

# Deployment
deploymentNode
infrastructureNode
containerInstance
softwareSystemInstance

# UML - casos de uso
actor
useCase
subject

# UML - clases
class
interface
enumeration
attribute
method

# UML - secuencia
lifeline
combinedFragment
fragmentOperand

# UML - estados
state
initial
final

# UML - actividad
activity
activityNode
decision
merge
fork
join

# Capa general
package
```

La lista está **abierta**: cualquier string es válido. Los de arriba son
los esperados. Añadir uno nuevo no rompe nada: una view que lo pida lo
recibe cuando empiece a existir en el grafo.

### EdgeKind

```text
# C4 estructural
containedIn             # jerarquía débil: un container puede tener varios padres

# C4 estática
uses
readsFrom
writesTo
publishes
subscribes

# Deployment
deployedOn
runsIn
replicatesTo

# UML - clases
generalizes
realises                # class → interface
hasAttribute
hasMethod
hasAssociation
hasAggregation
hasComposition
hasDependency
dependsOn               # package → package

# UML - casos de uso
participates            # actor ↔ useCase
includesUC              # useCase → useCase
extendsUC               # useCase → useCase

# UML - secuencia
sendsMessage            # lifeline → lifeline, con order
returnsMessage          # lifeline → lifeline, con order negativo
covers                  # combinedFragment → lifeline
guards                  # combinedFragment → fragmentOperand

# UML - estados / actividad
transitionsTo
flowsTo
forksInto
joinsFrom
```

### ViewType

```text
c4-landscape
c4-context
c4-container
c4-component
c4-deployment
c4-dynamic

uml-usecase
uml-class
uml-sequence
uml-state
uml-activity
```

## 4. Diagrama → query

Cada diagrama es una view con `viewType` + `ViewScope`. El `ViewScope`
es literalmente una consulta sobre el grafo. Tabla de mapeo:

| View type        | Scope típico                                                        | Backend canónico |
|------------------|---------------------------------------------------------------------|------------------|
| `c4-landscape`   | todos los `softwareSystem`, edges `uses`/`dependsOn` cross-system   | Structurizr DSL  |
| `c4-context`     | un `softwareSystem` y sus vecinos `uses`/`readsFrom`/`writesTo`     | Structurizr DSL  |
| `c4-container`   | un `softwareSystem`, expande `container` interno                    | Structurizr DSL  |
| `c4-component`   | un `container`, expande `component`                                 | Structurizr DSL  |
| `c4-deployment`  | `deploymentNode` + `containerInstance` + `softwareSystemInstance`   | Structurizr DSL / drawio |
| `c4-dynamic`     | escenario numerado sobre edges existentes                           | Structurizr DSL  |
| `uml-usecase`    | un `subject`, todos sus `actor` + `useCase`                         | PlantUML         |
| `uml-class`      | un `package`/`component` con sus `class`/`interface`/`enumeration`  | PlantUML         |
| `uml-sequence`   | lifelines seleccionadas + edges `sendsMessage` ordenados           | PlantUML         |
| `uml-state`      | un `state` raíz con sus `transitionsTo`                             | PlantUML         |
| `uml-activity`   | un `activity` con sus `flowsTo`/`forksInto`/`joinsFrom`             | PlantUML         |

Mermaid y draw.io entran como proyecciones alternativas del mismo set
de views; nunca como fuente.

## 5. Ejemplo concreto (mini)

```json
{
  "schemaVersion": 2,
  "sourceIdentities": [
    {
      "ref": "git:github.com/example/checkout@8db31d1",
      "type": "git",
      "repositoryId": "blake3:...",
      "worktreeId": "blake3:...",
      "rootCommit": "8db31d1"
    }
  ],
  "graph": {
    "nodes": [
      {
        "id": "container:orders-api",
        "kind": "container",
        "name": "Orders API",
        "namespace": "bc:checkout",
        "properties": { "port": 8080, "runtime": "axum-0.7" },
        "evidenceRefs": ["ev:9f2a"],
        "classification": "fact",
        "confidence": 0.93,
        "method": "heuristic-v1",
        "sourceIdentityRef": "git:github.com/example/checkout@8db31d1",
        "capturedAt": "2026-07-29T12:00:00Z",
        "capturedBy": "ast-grep-axum@0.4"
      },
      {
        "id": "class:orderaggregate",
        "kind": "class",
        "name": "OrderAggregate",
        "namespace": "container:orders-api",
        "properties": { "stereotype": "aggregate-root" },
        "evidenceRefs": ["ev:c4b1"],
        "classification": "fact",
        "confidence": 0.9,
        "method": "heuristic-v1",
        "sourceIdentityRef": "git:github.com/example/checkout@8db31d1",
        "capturedAt": "2026-07-29T12:00:00Z",
        "capturedBy": "go-shape-v1@0.1"
      },
      {
        "id": "lifeline:cashier",
        "kind": "lifeline",
        "name": "Cashier",
        "properties": { "refersTo": "person:cashier" },
        "evidenceRefs": [],
        "classification": "inference",
        "confidence": 0.7,
        "method": "human-overridden",
        "sourceIdentityRef": "git:github.com/example/checkout@8db31d1",
        "capturedAt": "2026-07-29T12:00:00Z",
        "capturedBy": "human-overridden"
      },
      {
        "id": "usecase:place-order",
        "kind": "useCase",
        "name": "Place Order",
        "namespace": "subject:checkout-sys",
        "evidenceRefs": [],
        "classification": "inference",
        "confidence": 0.65,
        "method": "human-overridden",
        "sourceIdentityRef": "git:github.com/example/checkout@8db31d1",
        "capturedAt": "2026-07-29T12:00:00Z",
        "capturedBy": "human-overridden"
      }
    ],
    "edges": [
      {
        "id": "rel:edge1",
        "kind": "uses",
        "source": "container:orders-api",
        "target": "container:payments",
        "via": "http",
        "description": "charges payment",
        "evidenceRefs": ["ev:1"],
        "classification": "fact",
        "confidence": 0.9,
        "method": "heuristic-v1",
        "sourceIdentityRef": "git:github.com/example/checkout@8db31d1",
        "capturedAt": "2026-07-29T12:00:00Z",
        "capturedBy": "ast-grep-axum@0.4"
      },
      {
        "id": "rel:edge2",
        "kind": "sendsMessage",
        "source": "lifeline:cashier",
        "target": "lifeline:orderaggregate",
        "description": "placeOrder(items)",
        "order": 1,
        "evidenceRefs": [],
        "classification": "inference",
        "confidence": 0.7,
        "method": "human-overridden",
        "sourceIdentityRef": "git:github.com/example/checkout@8db31d1",
        "capturedAt": "2026-07-29T12:00:00Z",
        "capturedBy": "human-overridden"
      },
      {
        "id": "rel:edge3",
        "kind": "participates",
        "source": "person:customer",
        "target": "usecase:place-order",
        "evidenceRefs": [],
        "classification": "inference",
        "confidence": 0.8,
        "method": "heuristic-v1",
        "sourceIdentityRef": "git:github.com/example/checkout@8db31d1",
        "capturedAt": "2026-07-29T12:00:00Z",
        "capturedBy": "human-overridden"
      }
    ]
  },
  "views": [
    {
      "id": "view:checkout-context",
      "viewType": "c4-context",
      "title": "Checkout — System Context",
      "scope": {
        "subgraphRoots": ["container:orders-api"],
        "maxDepth": 1
      },
      "derivedFromViewId": "view:checkout-landscape"
    },
    {
      "id": "view:place-order-uc",
      "viewType": "uml-usecase",
      "title": "Place Order",
      "scope": {
        "includeNodeKinds": ["actor", "useCase", "subject"],
        "subgraphRoots": ["subject:checkout-sys"]
      }
    },
    {
      "id": "view:place-order-seq",
      "viewType": "uml-sequence",
      "title": "Place Order (sequence)",
      "scope": {
        "includeEdgeKinds": ["sendsMessage", "returnsMessage"],
        "subgraphRoots": ["usecase:place-order"]
      }
    }
  ]
}
```

## 6. Persistencia en disco

```text
~/.local/share/archctl/projects/<projectId>/
├── snapshot/
│   └── <commit-sha>/
│       ├── graph.json             # canónico (texto), autoritativo
│       ├── ir.json                # derivado legacy, no fuente de verdad
│       ├── evidence.jsonl         # ledger (sin cambios estructurales)
│       └── views.yaml             # definiciones curadas de views
├── runs/
│   └── <run-id>/
│       ├── graph.json             # overlay provisional del run
│       ├── evidence.jsonl
│       └── ir.json
├── index.sqlite                   # opcional, derivado de graph.json
└── source-identities.json         # mapa de SourceIdentity (deflación)
```

Reglas:

1. **`graph.json` es la fuente de verdad.** Borrable y reconstruible
   desde el repo + el ledger si se necesita.
2. **`ir.json` se mantiene como derivado** del grafo para no romper el
   renderer Structurizr/PlantUML que ya tenemos; se regenera cuando el
   grafo cambia.
3. **`evidence.jsonl` queda como ledger append-only.** La evidencia
   pertenece a cada nodo/arista; `evidenceRefs` son punteros.
4. **Snapshot por commit, overlay por worktree** (líneas 3086-3125 del
   doc inicial). Hoy no lo implementamos; entra cuando aparezca el
   primer caso real de drift entre commits.

## 7. Índice SQLite (derivado)

```sql
CREATE TABLE node (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  namespace TEXT,
  name TEXT NOT NULL,
  captured_by TEXT NOT NULL,
  captured_at TEXT NOT NULL
);
CREATE INDEX node_kind ON node(kind);
CREATE INDEX node_ns ON node(namespace);
CREATE VIRTUAL TABLE node_fts USING fts5(id, name, namespace);

CREATE TABLE edge (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  source TEXT NOT NULL,
  target TEXT NOT NULL,
  ord INTEGER,
  captured_by TEXT NOT NULL,
  FOREIGN KEY (source) REFERENCES node(id),
  FOREIGN KEY (target) REFERENCES node(id)
);
CREATE INDEX edge_kind ON edge(kind);
CREATE INDEX edge_source ON edge(source);
CREATE INDEX edge_target ON edge(target);

CREATE TABLE evidence_ref (
  ev_id TEXT NOT NULL,
  ref_kind TEXT NOT NULL CHECK (ref_kind IN ('node', 'edge')),
  ref_id TEXT NOT NULL,
  PRIMARY KEY (ev_id, ref_kind, ref_id)
);

CREATE TABLE view (
  id TEXT PRIMARY KEY,
  view_type TEXT NOT NULL,
  derived_from TEXT
);
```

Borrar `index.sqlite` no pierde conocimiento; se regenera con
`archctl index rebuild`.

## 8. Auditoría local (no global)

Cada nodo y cada arista se audita por sí solo:

```ts
function auditNode(n: Node): AuditFinding {
  if (n.evidenceRefs.length === 0 &&
      (n.confidence >= 0.9 || n.classification === "fact" || n.classification === "inference")) {
    return { kind: "unsupported", id: n.id };
  }
  // ...
}
```

El `auditIR` actual se reescribe trivialmente como `auditGraph` que
aplica esa función a cada nodo y arista. Si una view pide un nodo sin
evidencia, la view se renderiza igualmente pero el nodo sale **marcado**
en el DSL (análogo al "línea roja = contradicción" del doc inicial, §19).
El `auditIR` global deja de ser el gate único; el auditar por elemento
siempre está disponible, sea la view renderizada o no.

## 9. Extensibilidad

| Cambio | Acción | Migración necesaria |
|--------|--------|---------------------|
| Nuevo `NodeKind` | Añadir string a la unión. | No. |
| Nuevo `EdgeKind` | Añadir string a la unión. | No. |
| Nueva `ViewType` | Añadir string a la unión + 3 líneas en `packages/core/src/project/` que mapeen a backend. | No. |
| Nueva propiedad obligatoria en `Node` | Bump `schemaVersion`, escribir migración en `packages/core/src/graph/migrations.ts`, ejecutar con `migrateToCurrent`. | Sí, una vez. |
| Nuevo adaptador (CLI externo) | Drop un JSON descriptor en `packages/core/src/adapters/`. | No. |
| Nuevo backend de render | Implementar `project<ViewType>(graph, view): string` en `packages/core/src/project/`. | No. |

Las cuatro primeras filas no tocan datos existentes. La quinta y sexta
nunca tocan el modelo.

## 10. Compatibilidad con el modelo actual

El `ArchitectureIR` de `packages/core/src/ir/ir.ts` (schemaVersion 1)
cabe enteramente como vista degenerada del grafo:

- `IRElement[]` → los nodos del grafo (subset de C4).
- `IRRelationship[]` → los edges del grafo (subset).
- `evidenceRefs` por elemento → idéntica semántica.

Una migración v1 → v2 pura:

1. Cada `IRElement` se convierte en `Node` con `kind` mapeado 1:1 y
   `properties = {}`, `tags = []`.
2. Cada `IRRelationship` se convierte en `Edge`.
3. `views[]` se inicializa como array vacío.
4. Las `evidence` records del ledger siguen siendo válidas; sólo cambia
   el nombre del campo que las ata: de `ir.json` a `graph.json`.

La migración vive en `packages/core/src/ir/migrations.ts` como
`Migration { from: 1, to: 2, apply }`. El `migrateToCurrent` existente
la ejecuta sin cambios externos.

## 11. Fuera de alcance (consistente con el reset)

Esto no entra, lo dice el doc inicial y lo confirmó el feedback del
usuario:

- **Sin** `validFrom`/`validTo` por arista. La historia vive en los
  snapshots por commit.
- **Sin** subagente falsificador separado: `auditGraph` por elemento
  cumple ese papel.
- **Sin** servidor de grafo (Neo4j o similar). Texto canónico + índice
  SQLite derivable.
- **Sin** plataforma de plugins dentro del repo.
- **Sin** multi-tenant ni cifrado: la persistencia es XDG local y la
  auditoría se hace en proceso.

## 12. Próximos pasos (orden)

1. **Revisar este doc.** Cualquier ajuste de taxonomía, semántica de
   jerarquía o reglas de auditoría se hace acá, no en el código.
2. Crear `packages/core/src/graph/` con:
   - `graph.ts` — tipos `Node`, `Edge`, `View`, `Graph`.
   - `migrations.ts` — extiende el array existente con
     `{ from: 1, to: 2, apply }`.
   - `audit-graph.ts` — `auditGraph(g): AuditResult` por nodo/arista.
   - `project-graph.ts` — `graphToStructurizr(g, view)` y
     `graphToPlantUML(g, view)`. Reemplaza los proyectores actuales
     como consumidores del grafo.
   - `index-builder.ts` — genera el `index.sqlite` a partir de
     `graph.json`.
3. Reemplazar los proyectores `packages/core/src/project/{structurizr,plantuml}.ts`
   por `graphToStructurizr`/`graphToPlantUML`. Los tests existentes se
   actualizan para usar fixtures de grafo en vez de `ArchitectureIR`
   literal.
4. Implementar `archctl scan`, `archctl extract`, `archctl model
   build`, `archctl explain` sobre el grafo, no sobre `ArchitectureIR`.
5. Backfill: convertir `m0-gate-zero/run.ts` para producir un grafo
   v2 y un `index.sqlite`. Verifica que el pipeline entero cuelga del
   nuevo modelo.

Las fases 1 y 2 son chicas (~300 líneas total en `packages/core/src/graph/`).
La 3 y la 4 son las que dan el clic de cara al usuario.
