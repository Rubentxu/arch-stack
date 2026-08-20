# ADR-P05 — Incremental Knowledge Index
**Status:** Proposed

## Decision
`notify + BLAKE3 + Tree-sitter + ast-grep + Rayon`; sólo changed observations se aplican en batches deterministas y transacciones cortas.

## Invariant
Full index y incremental index del mismo estado final tienen el mismo graph digest.
