# Risks & Open Questions

| Riesgo | Mitigación |
|---|---|
| feature zoo visual | task-fit grammar + UAT |
| graph crece con transcripts | detail en journal/search; graph anchors |
| LLM contamina truth | authority gate |
| watcher pierde eventos | hash reconcile / PollWatcher fallback |
| embedded DB concurrency | extract outside lock; short transactions |
| Tantivy stale | graph/search revision alignment |
| semantic precision insuficiente | optional SCIP |
| layout instability | stable positions/topology-aware update |
| embeddings añaden complejidad | deferred trigger |
| tldraw fragmenta frontend | isolated spike only |
| LensSpec overengineering | existing entry gate |
| docs vuelven a quedar stale | capability/traceability dogfood |
| UAT subjetivo | ground truth + task outcome + measures |
| multimodal judge unreliable | advisory only |
