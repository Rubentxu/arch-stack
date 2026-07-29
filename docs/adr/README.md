# Architecture Decision Records (ADR)

> Each ADR captures one architecturally significant decision: hard to reverse, surprising
> without context, and the result of a real trade-off.

## Format

Each ADR follows the Michael Nygard template: **Title · Status · Context · Decision · Consequences · Alternatives**.

## Status lifecycle

`Proposed` → `Accepted` → `Superseded` (by a later ADR) | `Deprecated`.

> **All ADRs below are `Proposed`.** They will not move to `Accepted` until the design is
> approved and (for irreversible ones) the relevant validation gate passes. The orchestrator
> promotes status explicitly; this file is never auto-accepted.

## Index

| # | Title | Status | Decides |
|---|---|---|---|
| [0001](0001-plugin-first-no-rust-first.md) | Plugin-First / No-Rust-First with Conditional Rust Extraction Gate | Proposed | Defer Rust until the core hypothesis is validated |
| [0002](0002-neutral-ir-truth-structurizr-projection.md) | Neutral Architecture IR as Truth; Structurizr as C4 Projection | Proposed | Resolve dual-truth ambiguity: IR is source, Structurizr is projection |
| [0003](0003-xdg-runtime-state-export-bundle.md) | XDG Runtime State + Explicit Export Bundle | Proposed | No in-repo `.architecture/`; XDG default + explicit export |
| [0004](0004-evidence-ontology-confidence-provenance.md) | Evidence Ontology and Confidence Provenance | Proposed | fact/inference/hypothesis/unknown/conflict + provenance + unsupported-claim gate |
| [0005](0005-renderer-routing-local-first.md) | Renderer Routing / Local-First Policy | Proposed | Structurizr `local` (not lite EOL); offline-first; Mermaid non-canonical |
| [0006](0006-reuse-over-rebuild-capability-adapters.md) | Reuse-over-Rebuild and Capability Adapter Contract | Proposed | No custom parsers; uniform Adapter seam (Shape B + declarative ShellAdapter) |
| [0007](0007-opencode-version-pin-schema-contract-minimal-topology.md) | OpenCode Version Pin / Schema-Contract and Minimal Agent Topology | Proposed | `mcp` (not mcpServers); version-pin + CI schema-test; 4 roles max |
| [0008](0008-supply-chain-pinning-sandbox.md) | Supply-Chain Pinning / Sandbox Policy | Proposed | skills.lock.json pinning + license + sandbox + write-guard |

## Relationship to the source design compass

These ADRs distill and *correct* decisions first sketched in `Skills-para-agentes-IA.md`
and refined through `sddk/architecture-intelligence-platform/explore-report.md` →
`proposal.md` → `design.md`. Where the source doc was ambiguous or wrong, the ADR records
the resolved position and the evidence.
