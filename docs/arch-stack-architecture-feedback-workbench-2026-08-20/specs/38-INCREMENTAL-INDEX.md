# Spec — Incremental Index

## ArtifactLedger
`path, kind, content_hash, extractor_version, last_revision`.

## Flow
watch → debounce → hash → extract changed → ObservationBatch → apply.

## Required tests
No-op no crea revision; changed file reparse; removed invalidates; rename definido; full==incremental digest; thread/order invariance.
