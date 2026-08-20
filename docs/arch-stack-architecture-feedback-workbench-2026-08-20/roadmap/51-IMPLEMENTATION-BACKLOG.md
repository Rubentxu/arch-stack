# Implementation Backlog — PR-sized

## T0 Trust
- TRUST-001 EventLog open without truncation + reopen regression.
- TRUST-002 event IDs + correlation/causation + per-consumer checkpoint.
- TRUST-003 AuthorityClass/ExecutionClass mapping.
- TRUST-004 no canonical write from model-backed output.
- TRUST-005 real confidence/status into Observation/Fusion.
- TRUST-006 FreshnessPolicy by source.

## T1 Index
- IDX-001 ArtifactLedger BLAKE3.
- IDX-002 notify watcher abstraction.
- IDX-003 debounce/coalesce.
- IDX-004 Rayon extraction pipeline.
- IDX-005 ObservationBatch canonicalization.
- IDX-006 changed-file apply.
- IDX-007 removed/renamed invalidation.
- IDX-008 differential harness.
- IDX-009 Criterion cold/warm benchmarks.

## T2 Docs/Search
- DOC-001 Document/Section extraction.
- DOC-002 ADR recognizer.
- DOC-003 deterministic reference linker.
- SRCH-001 Tantivy schema/index adapter.
- SRCH-002 revision-aware commit/rebuild.
- SRCH-003 hybrid seed resolver.
- SRCH-004 ContextBundle `included_because`.

## T3 Live
- LIVE-001 GraphRevision.
- LIVE-002 GraphDelta.
- LIVE-003 revision/delta HTTP.
- LIVE-004 index worker in `view --watch`.
- LIVE-005 Archview polling store.
- LIVE-006 style vs topology update.
- LIVE-007 selection/viewport preservation.

## T4 Visual
- VIS-001 SelectionBus.
- VIS-002 adjacency/read-model index.
- VIS-003 InspectorRegistry.
- VIS-004 Smart System Overview.
- VIS-005 internal LensDefinition.
- VIS-006 migrate C4 consumer.
- VIS-007 migrate Impact consumer.
- VIS-008 DSM sparse model.
- VIS-009 Canvas2D matrix renderer.
- VIS-010 Graph↔DSM↔Source linking.
- VIS-011 System Map d3-hierarchy.
- VIS-012 metric overlay contract.

## T5 Intent
- INT-001 IntentCandidate/AcceptedIntent.
- INT-002 deterministic Reconciliation.
- INT-003 Reconciliation projection/API.
- INT-004 Reconciliation Matrix.
- INT-005 Intent Map.
- INT-006 Intent Coverage.

## T6 Agent/Feedback
- AGV-001 ProjectionSpec→VisualRequest compatibility.
- AGV-002 Visual Compiler.
- AGV-003 VisualArtifact.
- AGV-004 selection→AgentContext.
- AGV-005 Feedback model/write.
- AGV-006 feedback retrieval.
- AGV-007 proposed/ghost visual state.

## T7 Change
- CHG-001 Expected Change Surface.
- CHG-002 IntentDiff.
- CHG-003 SemanticReview model.
- CHG-004 synchronized before/after.
- CHG-005 test impact.
- CHG-006 UAT impact.

## T8+
STORY-*, CAUSAL-*, OTEL-* y WHATIF-* sólo tras gates previos.
