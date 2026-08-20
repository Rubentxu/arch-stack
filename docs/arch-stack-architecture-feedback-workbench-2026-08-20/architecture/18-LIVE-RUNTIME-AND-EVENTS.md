# Live Runtime & Events

## Journal first, event sourcing later
Journal para causalidad, audit, invalidation y subscriptions. Ladybug continúa como semántica canónica.

Reconsiderar event sourcing sólo con una prueba estable `replay(events) == canonical graph`.

## EventEnvelope vNext
`event_id, seq, timestamp, event_type, schema_version, source, producer, correlation_id, causation_id, graph_revision?, payload`.

## P0
El journal existente debe abrirse sin truncar ficheros. Separar event sequence de per-consumer checkpoint.

## Core event taxonomy
`source.file.changed.v1`, `source.document.changed.v1`, `graph.revision.created.v1`, `analysis.finding.proposed.v1`, `agent.turn.completed.v1`, `human.feedback.submitted.v1`.

## Runtime
OpenTelemetry entra como fuente de observations; una contradicción produce reconciliation conflict, no overwrite.
