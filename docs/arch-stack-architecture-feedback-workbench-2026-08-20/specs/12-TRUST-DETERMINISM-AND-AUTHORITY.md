# Spec — Trust, Determinism & Authority

## Version 1.0 (2026-08-20)

## Purpose

Codificar el contrato de promoción canónica del grafo de verdad:
qué productores pueden escribir `CanonicalObservedFact` y bajo
qué autoridad. ADR-063. ADR-P02. ADR-P03.

## Glossary

| Term | Definition |
|---|---|
| `CanonicalObservedFact` | Evidence row with `status == Accepted` reaching `archctl/src/diagram/export.rs:109` filter. NOT a new Rust type. Maps to ADR-021's "observed fact" in prose. |
| `ExecutionClass` | How a producer computed an answer (4 variants). |
| `AuthorityClass` | The epistemic weight an answer carries (5 variants). |
| `Adjudication` | An explicit human verdict that elevates a row from `Suggested` or `Normative` to `Adjudicated`. Distinct from ADR-023's `Approval` (different object — facts vs side effects). |
| `Canonical-write gate` | The function `trust::canonical_write_allowed(exec, authority) -> Result<(), TrustViolation>`. The single 2-input predicate every transition to `status == Accepted` must pass. |

## Producers (transcribed from `architecture/12-…:16-24`)

| Producer | SourceOrigin | tool_name | Execution | Authority |
|---|---|---|---|---|
| Tree-sitter | UserWorkspace | "tree-sitter" | PureDeterministic | Observed |
| SCIP | ToolOutput | "scip" | PureDeterministic | Observed |
| SCC | ToolOutput | "scc" | PureDeterministic | Derived |
| naming heuristic | ToolOutput | "naming_heuristic" | DeterministicHeuristic | Suggested |
| LLM analyst | ModelInference | "llm_analyst" | ModelInference | Suggested |
| ADR accepted | UserInput | "adr_accepted" | HumanDecision | Normative |
| reject/correction | UserInput | "human_adjudication" | HumanDecision | Adjudicated |

## Invariant

`ModelInference` no escribe directamente `CanonicalObservedFact`.
La matriz 4×5 codifica la excepción: `ModelInference × Suggested`
es la única celda verde para `ModelInference` (visibilidad de
candidatos, ADR-P02).

## Cross-references

- ADR-063: this spec's authority
- `archctl/src/trust.rs`: implementation
- `archctl/tests/uat_06_false_agent_claim.rs`: UAT-06 critical gate
- `sddk/m25-authority-execution-classes/spec.md` §3: Given-When-Then
- `sddk/m25-…/spec.md` §4: UAT-06 step-by-step

## See also

- `specs/30-GRAPH-REVISION-AND-DELTA.md` Version 1.1 — what changes in the graph revision/delta model when this spec lands.
