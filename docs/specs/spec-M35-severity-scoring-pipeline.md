---
title: M35 — Cognitive Finding Severity Scoring Pipeline
type: spec
capability: cognitive-severity-scoring
schemaVersion: "1.0"
cycle: p-38e02210a9f14317/m35-severity-scoring-pipeline
date: 2026-08-24
author: sddk-apply
source: sddk/m35-severity-scoring-pipeline/{spec,design,tasks}.md
status: accepted
naming-note: >
  **Naming collision alert**: the numeric tag `M35` collides with the vault
  milestone `M35-java-call-graph` (RELEASED 2026-08-07, tag v1.7.0, PR #92).
  This spec concerns exclusively the cognitive-layer severity scoring function.
  The historical `M35-java-call-graph` milestone is NOT renamed.
---

## Capability: `cognitive-severity-scoring`

`pub fn severity_for(&FindingCandidate, &SeverityContext) -> Severity`

Maps a `confidence` value (continuous, in `[0.0, 1.0]`) to a discrete
`Severity` using fixed bins. The function is **pure** — same `(finding, ctx)`
produces the same `Severity`.  No I/O, no clock, no `f64` leaked to consumers.

**Order of evaluation**: validate → overrides → bin → floor

- **Validate**: NaN or out-of-range confidence → `warn!` + `Info`
- **Overrides** (in order):
  1. `evidence_count == 0` → `Info`
  2. `severity_hint == EscalateToCritical` → `Critical`
  3. `rule_kind == Destructive` → `Critical`
  4. `severity_hint == FloorAtInfo` → `Info` (applied after bin)
- **Bin lookup**: `>=0.9 Critical`, `>=0.7 Error`, `>=0.4 Warning`, `<0.4 Info`
- **Safety floor**: `max(finding.severity, computed)` — scoring cannot lower
  a severity the agent already inflated by domain knowledge.

---

## Scenarios

### SCN-M35-A — High confidence maps to Critical

- **GIVEN** `confidence = 0.95`, `evidence_count = 2`, `rule_kind = Naming`, `severity_hint = None`
- **WHEN** `severity_for(&finding, &ctx)` is called
- **THEN** the returned `Severity` is `Critical`

### SCN-M35-B — Zero evidence overrides high confidence to Info

- **GIVEN** `confidence = 0.95`, `evidence_count = 0`, `rule_kind = Naming`, `severity_hint = None`
- **WHEN** `severity_for(&finding, &ctx)` is called
- **THEN** the returned `Severity` is `Info` (zero-evidence override beats bin)

### SCN-M35-C — NaN confidence emits warn and returns Info

- **GIVEN** `confidence = f64::NAN`, `evidence_count = 1`, `rule_kind = Naming`
- **WHEN** `severity_for(&finding, &ctx)` is called under a tracing subscriber
- **THEN** exactly one `WARN` event is captured AND the returned `Severity` is `Info`

### SCN-M35-D — Destructive rule kind forces Critical regardless of bin

- **GIVEN** `confidence = 0.5` (would map to `Warning`), `evidence_count = 5`, `rule_kind = Destructive`
- **WHEN** `severity_for(&finding, &ctx)` is called
- **THEN** the returned `Severity` is `Critical` (Destructive override forces Critical)

---

## Out-of-Scope Lock

| Rule | Statement |
|------|-----------|
| NEG-001 | No unification of the 5 disjoint `Severity` enums in the crate |
| NEG-002 | `DecisionPriority` remains `RecencyOnly` (no new variants) |
| NEG-003 | No CLI surface (`archctl cognitive score-finding` not added) |
| NEG-004 | No `pub fn severity_score(...) -> f64` exposed in the API |
| NEG-005 | No new Cargo dependency introduced |
