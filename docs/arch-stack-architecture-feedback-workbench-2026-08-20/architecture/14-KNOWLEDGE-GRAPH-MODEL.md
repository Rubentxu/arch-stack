# Knowledge Graph Model

## Conceptual overlays sobre una sola DB

- **L0 Evidence**: SourceArtifact, Evidence, ToolRun, Evaluation.
- **L1 Code**: Repository, File, Package, Module, Class, Function, Symbol, Import, Call.
- **L2 Architecture**: Person, System, Container, Component, Interface, Dependency.
- **L3 Intent & Knowledge**: Intent, Requirement, Constraint, Decision, Rationale, Assumption, Alternative, ADR.
- **L4 Quality**: Policy, Finding, Test, UATScenario, Coverage.
- **L5 Runtime**: RuntimeService, Endpoint, Trace, Span, Queue, DB, ObservedCall.
- **L6 Change & Time**: Commit, Snapshot, GraphRevision, GraphEvent, ChangeSet.
- **L7 Human/Agent**: Task, AgentSession, AgentTurnAnchor, Candidate, Feedback.

## Assertions
No usar edges desnudos cuando hagan falta identidad, evidence, version, confidence o feedback. Reificar la relación semántica y mantener derived traversal edges cuando mejore rendimiento.

## Intent
`HumanUtterance → IntentCandidate → AcceptedIntent`. Estados: proposed, accepted, rejected, superseded, experimental, uncertain. No convertir prompts automáticamente en requirements.
