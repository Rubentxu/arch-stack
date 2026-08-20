# Target Architecture

```text
                    INBOUND
 CLI | view HTTP | MCP | IDE | plugin | agent
                    │
                    ▼
              Application Use Cases
                    │
    ┌───────────────┼────────────────┐
    ▼               ▼                ▼
 IndexProject   AnalyzeImpact   ReconcileArchitecture
 Explain        BuildContext    SubmitFeedback
 Compare        BuildStory      SemanticReview
                    │
                    ▼
                  DOMAIN
 Element | Relation | Evidence | Observation | Claim
 Intent | Feedback | Finding | Revision | GraphEvent
 Lens | VisualRequest | VisualArtifact | UATScenario
                    │
                    ▼
               NARROW PORTS
 GraphRead / GraphWriteSession / SearchIndex
 EventJournal / Filesystem / SourceIndex / RuntimeEvidence
                    │
                    ▼
                 ADAPTERS
 Ladybug | Tantivy | JSONL | Git | Tree-sitter
 ast-grep | SCIP | OpenTelemetry | XDG
```

## Crate evolution

Objetivo lógico, no big-bang físico:
- `arch-domain`: modelo/invariantes/algoritmos puros.
- `arch-application`: commands/queries/use cases.
- `arch-adapter-ladybug`: mappings/migrations/Cypher.
- `archctl`: CLI/composition/view HTTP.
- `archview`: UI.

Primero aplicar dependency fitness y seams. Extraer crates sólo cuando reduzca acoplamiento medido.

## Ports
Evitar un `arch-ports` bag genérico. Ports pequeños cerca del use case propietario: `DependencyEvidenceReader`, `FeedbackWriter`, `RevisionReader`, `SearchProjection`.

## Transaction boundary
Extracción fuera del lock. Aplicación de `ObservationBatch` en transacciones cortas. Revisar/aislar cualquier `unsafe` innecesario en wrappers de transaction.
