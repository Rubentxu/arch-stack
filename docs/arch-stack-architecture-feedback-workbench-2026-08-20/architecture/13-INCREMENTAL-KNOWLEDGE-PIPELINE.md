# Incremental Knowledge Pipeline

## Cold scan
```text
ignore walk → BLAKE3 ledger → changed/new artifacts → parallel extract
  source: Tree-sitter/ast-grep
  docs: pulldown-cmark
  semantic: optional SCIP
  manifests: deterministic adapters
→ ObservationBatch[] → canonical sort/stable IDs → short Ladybug transaction
→ GraphRevision → Tantivy update + journal + GraphDelta
```

## Warm edit
```text
notify → debounce → hash
                ├─ same → drop
                └─ changed → reparse file → changed observations → revision
```

V1 file-granular. Changed-ranges/symbol-granular después.

## TreeCache
Durante `archctl view --watch`: `path -> content_hash + Tree + extractor_version + symbol_ranges`.

## Determinism invariant
`GraphDigest(full_index) == GraphDigest(incremental_history)` para mismo filesystem/configuración, independiente del thread count y orden de inputs.

## Invalidations
Cada observation referencia source artifact/hash/extractor version/revision. Cambio de hash o extractor invalida exactamente sus derivaciones.
