# Spec — GraphRevision & GraphDelta

## Version 1.1 (2026-08-20)

Added canonical-write policy enforcement at `accept_evidence` (ADR-063). Evidence rows with `SourceOrigin::ModelInference` cannot transition to `Accepted` via the standard CLI path. The predicate is `trust::canonical_write_allowed(ExecutionClass, AuthorityClass)` and the matrix is in `specs/12-TRUST-DETERMINISM-AND-AUTHORITY.md`. Honest `Evaluation` attestation replaces the hardcoded `"user_accepted"` / `"archctl:lifecycle_v1"` pair with the actual caller and invocation path.

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
