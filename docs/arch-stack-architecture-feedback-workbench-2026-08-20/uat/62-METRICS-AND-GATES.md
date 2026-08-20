# Metrics & Gates

## Objective
Task correctness, time, errors, TTFAI, Evidence Distance, Context Recovery Time, Context Recall@K, incremental lag, graph digest equality, cross-view identity y false canonical promotions.

## Subjective
SEQ por task, NASA-TLX por bloque y SUS por release/horizon.

## Initial design targets
- false agent canonical promotion: **0**;
- full/incremental equality: **100% fixture states**;
- cross-view identity: **100%**;
- critical task correctness: **>=90%**;
- median Evidence Distance: **<=2** interacciones;
- warm no-op 10k files: target p95 <=500 ms;
- edit→visible normal file: target p95 <=1 s;
- median SEQ: target >=5/7.

Los performance values son hipótesis hasta medir sobre hardware documentado.
