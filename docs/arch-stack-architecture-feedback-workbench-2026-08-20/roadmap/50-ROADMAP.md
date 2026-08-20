# Roadmap — Architecture Feedback Workbench

## Regla
No avanzar porque una fase “esté codificada”; avanzar cuando cumple exit gates.

## T0 — Epistemic Trust
EventLog safety, authority/execution separation, real observation confidence, source-specific freshness y prohibition de model-output canonical write.
**Exit:** false promotion impossible; journal reopen safe.

## T1 — Incremental Knowledge Engine
notify/BLAKE3 ledger, Rayon extraction, changed-file reparse, deterministic batches, full-vs-incremental differential tests.
**Exit:** UAT-04 y equality gate.

## T2 — Structured Docs & Search
pulldown-cmark, ADR/spec recognizers, Tantivy y explainable retrieval.
**Exit:** UAT-08 recall target.

## T3 — Live Revision Loop
GraphRevision, GraphDelta, revision/delta endpoints, `view --watch`, polling y stable updates.
**Exit:** edit→visible budget y context preserved.

## T4 — Visual Reasoning Foundation
SelectionBus, InspectorRegistry, internal LensDefinition, Smart Overview, DSM, System Map y graph↔matrix↔source.
**Exit:** UAT 01/02/03/10.

## T5 — Intent & Reconciliation
Intent graph refinements, Reconciliation engine/Matrix, Intent Map y Intent Coverage.
**Exit:** UAT-05.

## T6 — Agent ↔ Visual ↔ Feedback
VisualRequest, VisualArtifact compiler, selection en AgentContext, Feedback persistence/retrieval y false-claim guard.
**Exit:** UAT-06/14.

## T7 — Change Intelligence
Expected Change Surface, Semantic Review, policy/test/UAT delta.
**Exit:** UAT-09.

## T8 — Stories & Causality
ArchitectureStory, agent causal graph y Time Machine.
**Exit:** UAT-07/13.

## T9 — Runtime Reality
OTel importer, mapping static/runtime y runtime drift.
**Exit:** known runtime drift explained.

## T10 — What-if
Forked/proposed graph, policy/impact recomputation y Thinking Canvas spike sólo tras probar valor core.

## T11 — Advanced Intelligence
Condicional: SCIP expansion, communities, co-change, centrality, semantic retrieval y benchmark de alternate massive renderer.
