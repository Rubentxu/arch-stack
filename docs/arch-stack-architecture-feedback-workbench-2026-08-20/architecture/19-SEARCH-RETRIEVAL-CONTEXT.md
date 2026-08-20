# Search, Retrieval & Agent Context

## Pipeline
```text
exact canonical/name/path + Tantivy BM25 + current selection
                    ↓
                  seeds
                    ↓
       deterministic graph expansion
                    ↓
 ranking(distance, relation weight, confidence, freshness, revision proximity)
                    ↓
              ContextBundle
```

## Tantivy indexa
Symbols, paths, comments, Markdown sections, ADR/specs, findings, feedback y raw text anchors de agent sessions.

## Embeddings
Diferidos. Sólo activar FastEmbed+USearch si UAT demuestra fallos repetidos de recall conceptual. Nunca crean facts.

## Explainable retrieval
Cada item tiene `included_because`: exact match, selected entity, graph distance, ADR reference, recent change, etc.
