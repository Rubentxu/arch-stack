# Trust, Determinism & Authority

## ExecutionClass
- `PureDeterministic`
- `DeterministicHeuristic`
- `ModelInference`
- `HumanDecision`

## AuthorityClass
- `Observed`
- `Derived`
- `Suggested`
- `Normative`
- `Adjudicated`

| Productor | Execution | Authority |
|---|---|---|
| Tree-sitter | PureDeterministic | Observed |
| SCIP | PureDeterministic | Observed |
| SCC | PureDeterministic | Derived |
| naming heuristic | DeterministicHeuristic | Suggested |
| LLM analyst | ModelInference | Suggested |
| ADR accepted | HumanDecision | Normative |
| reject/correction | HumanDecision | Adjudicated |

## Invariant
`ModelInference` no escribe directamente `CanonicalObservedFact`.

## Pipeline
```text
LLM → Candidate → schema validation → evidence resolution → deterministic policy
                                  ↓
                         PROPOSED / UNVERIFIED
                                  ↓
                           human/verifier
                                  ↓
                          accept / reject
```

## Reproducibility metadata
Persistir agent/model/runtime, prompt/template version, input context digest, evidence IDs, tool versions, output digest y correlation/causation IDs.

Una visualización histórica usa el candidate persistido; no vuelve a ejecutar el modelo.
