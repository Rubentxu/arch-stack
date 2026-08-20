# Spec — Agent/Event Journal

## Envelope
`event_id, seq, ts, type, schema_version, source, producer, correlation_id, causation_id, graph_revision, payload`.

## Storage
Append-only JSONL XDG.

## Critical invariant
Abrir journal existente nunca trunca.

## Checkpoints
Por consumidor; no mutan eventos.

## Recovery
Final line malformada tras crash se reporta y eventos anteriores siguen legibles.
