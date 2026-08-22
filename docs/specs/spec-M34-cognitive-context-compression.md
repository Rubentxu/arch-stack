# Spec: M34 — Cognitive Context Compression

> **Cycle:** `p-38e02210a9f14317/m34-cognitive-context-compression`
> **Status:** accepted (M34 W6)

`AgentContext::compress_for_budget` limits the token footprint of a
re-invoked agent by truncating oldest evidence first, surfacing only the N
most-recent ledger events, and walking the causation chain within a hop window.

## Public surface

- `AgentContext::compress_for_budget(&mut self, &CompressionPolicy, &EventLog) -> Result<CompressionReport, CompressionError>`
- `CompressionPolicy { budget_chars, preserve_causation_window, decision_priority }`
- `CompressionReport { truncated_fields, dropped_evidence_count, recent_events_used, preserved_causation_links }`
- `EventLog::recent(n, TailFilter)`, `EventLog::find_by_event_id(id)`

## Requirements

### R-M34-001 — recent_events cap

`recent_n = max(10, budget_chars / 500)`. After compression `context.recent_events.len() <= recent_n`.

### R-M34-002 — exempt invariants

`feedback_history` and `pending_adjudications` are **never** modified.

### R-M34-003 — evidence truncation

Evidence is removed oldest-first until estimated size <= budget_chars.

### R-M34-004 — causation window

BFS walks up to `preserve_causation_window` hops per recent event.

### R-M34-005 — serde back-compat

Pre-M34 `AgentContext` JSON (no `recent_events` field) deserializes with `recent_events: vec![]`.

## Schema version

No migration required. `recent_events` is additive with `#[serde(default)]`.
