# Technology Stack

| Capacidad | Tecnología | Decisión |
|---|---|---|
| Repo walk | `ignore` | mantener |
| Git | `gix` | mantener |
| Hash | `blake3` | mantener |
| FS watch | `notify` | añadir |
| CPU parallelism | `rayon` | añadir |
| Syntax | `tree-sitter` | mantener |
| AST patterns | `ast-grep-core` | mantener |
| Precise semantics | SCIP | opcional |
| Markdown | `pulldown-cmark` | añadir |
| Canonical graph | LadybugDB | mantener |
| Graph algorithms | petgraph + propios | mantener |
| Lexical search | Tantivy | añadir |
| Causal journal | JSONL XDG | endurecer |
| Semantic retrieval | FastEmbed + USearch | diferir |
| Runtime evidence | OpenTelemetry/OTLP | posterior |
| Local HTTP | tiny_http | mantener |
| Live delivery | revision/delta polling | primero |
| UI | SolidJS | mantener |
| Graph | G6 | mantener |
| Hierarchical layout | ELK.js Worker | mantener |
| Matrix | Canvas2D custom | añadir |
| Hierarchy/maps | d3-hierarchy | añadir |
| Timeline | d3-scale/d3-array | añadir |
| Browser UAT | Playwright | añadir dev |
| Thinking Canvas | tldraw spike | diferir |

## Technology budget
Camino crítico nuevo: `notify`, `rayon`, `pulldown-cmark`, `tantivy`; frontend `d3-hierarchy`, `d3-scale`, `d3-array`; dev `@playwright/test`.

## No introducir inicialmente
Kafka/NATS, Redis, Elasticsearch/OpenSearch, Qdrant/Milvus, GraphQL, React rewrite, ECharts/Vega, daemon permanente, vector DB dedicado o Axum/Tokio en producción sin trigger medido.

## Razón
Cada nueva pieza debe resolver un acceso específico. Ladybug no debe convertirse en search engine; Tantivy no debe convertirse en truth store; el journal no debe convertirse prematuramente en event source.
