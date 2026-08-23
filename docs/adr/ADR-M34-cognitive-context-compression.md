# ADR-M34 — Cognitive Context Compression

**Status:** proposed (becomes accepted at release)
**Date:** 2026-08-22
**Cycle:** `p-38e02210a9f14317/m34-cognitive-context-compression`

## Context

`AgentContext` grows unboundedly: `evidence` accumulates, `feedback_history`
grows, and the ledger stores unbounded history. No compression hook existed.
Three gaps: (1) no `recent_events` field, (2) no causal tail read, (3) budget
field was declarative (no enforcement).

## Decision

Approach 2 (full implementation): add `EventLog::recent(n, TailFilter)`,
`EventLog::find_by_event_id(id)`, `recent_events` field, and
`compress_for_budget` with:
- `recent_n = max(10, budget_chars / 500)`
- Exempt invariants preserve `feedback_history` and `pending_adjudications`
- Evidence truncation oldest-first until size <= budget_chars
- Causation BFS up to `preserve_causation_window` hops
- Fail-open on errors

## Consequences

### Positive

- Token budget enforced at context-build time
- Agents see the causal tail of events
- Additive `#[serde(default)]`: no schema migration
- No new external dependencies

### Negative

- `find_by_event_id` is O(n) linear scan
- Two new manifest gates (`cognitive.toml` updates)

## Alternatives Rejected

### Approach 1 — Declarative budget only

Broke the contract: budget declared but never enforced.

### Approach 3 — Full event sourcing

Would require dispatcher replay on every call. Out of scope for v1.

### Skip causation BFS

Loses causal lineage. BFS ensures agents see *why*, not just *what*.
