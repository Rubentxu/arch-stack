# Spec — GraphRevision & GraphDelta

## Purpose
Exponer updates semánticos incrementales sin reload completo.

## GraphRevision
`revision, created_at, source_digest, extractor_digest, parent_revision?, cause_event_id?`.

## GraphDelta
`from_revision, to_revision, nodes_added/changed/removed, edges_added/changed/removed, findings_changed, reconciliation_changed, affected_entity_ids`.

## Rules
Orden determinista, canonical IDs, delta replay A→B produce B, style-only distinguible de topology.

## HTTP
`GET /api/revision`, `GET /api/delta?after=N`.
