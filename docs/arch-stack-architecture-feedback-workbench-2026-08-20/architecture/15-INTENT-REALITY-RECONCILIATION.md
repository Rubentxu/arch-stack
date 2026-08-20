# Intent / Reality Reconciliation

## Planos
Intent, Static, Runtime, Tests/UAT y Human feedback.

## Estados
CONFIRMED, INTENDED_ONLY, OBSERVED_ONLY, RUNTIME_ONLY, CONFLICTING, UNVERIFIED, INFERRED, STALE, PROPOSED.

## Reconciliation
Función determinista sobre assertions/evidence, no tarea de LLM.

## Freshness por fuente
- code: content hash;
- runtime: temporal window;
- ADR: supersession;
- tests: mapping/revision;
- human decision: until superseded.

## Superficies visuales
Reconciliation Matrix, conflict overlay, Intent Map, Why/Evidence, synchronized Intent/Architecture/Code diff.
