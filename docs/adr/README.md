# Architecture Decision Records (ADR)

> Each ADR captures one architecturally significant decision: hard to reverse, surprising
> without context, and the result of a real trade-off.

## Format

Each ADR follows the Michael Nygard template: **Title · Status · Context · Decision · Consequences · Alternatives**.

## Status lifecycle

`Proposed` → `Accepted` → `Superseded` (by a later ADR) | `Deprecated`.

> **All eight ADRs are now `Accepted`.** They were promoted on 2026-07-29 by the orchestrator
> per the user's directive to proceed "a tu criterio, busca máximo valor para el usuario,
> utilidad y facilidad". Each acceptance added an operationalised decision rule that makes
> the original ADR actionable without further debate; see each file's "Decision" section.

## Index

| # | Title | Status | Decides |
|---|---|---|---|
| [0001](0001-plugin-first-no-rust-first.md) | Plugin-First / No-Rust-First with Conditional Rust Extraction Gate | Accepted | Defer Rust until the core hypothesis is validated; measurable thresholds for activation |
| [0002](0002-neutral-ir-truth-structurizr-projection.md) | Neutral Architecture IR as Truth; Structurizr as C4 Projection | Accepted | Resolve dual-truth ambiguity: IR is source, Structurizr is projection |
| [0003](0003-xdg-runtime-state-export-bundle.md) | XDG Runtime State + Explicit Export Bundle | Accepted | No in-repo `.architecture/`; XDG default + explicit export with portable `projectId` + explicit rebind |
| [0004](0004-evidence-ontology-confidence-provenance.md) | Evidence Ontology and Confidence Provenance | Accepted | fact/inference/hypothesis/unknown/conflict + mandatory `method` declaration + unsupported-claim gate |
| [0005](0005-renderer-routing-local-first.md) | Renderer Routing / Local-First Policy | Accepted | Structurizr `local` viewer + pinned headless CLI (NOT Lite EOL); offline-first; Mermaid excluded by default |
| [0006](0006-reuse-over-rebuild-capability-adapters.md) | Reuse-over-Rebuild and Capability Adapter Contract | Accepted | No custom parsers; uniform Adapter seam (Shape B + declarative ShellAdapter) |
| [0007](0007-opencode-version-pin-schema-contract-minimal-topology.md) | OpenCode Version Pin / Schema-Contract and Minimal Agent Topology | Accepted | `mcp` (not `mcpServers`); `OpenCode 1.18.x` initial pin; CI schema-test; 4 roles max |
| [0008](0008-supply-chain-pinning-sandbox.md) | Supply-Chain Pinning / Sandbox Policy | Accepted | `skills.lock.json` pinning + SPDX allow-list + sandbox + canonical-root write-guard + MCP/tool inventory |

## Acceptance deltas (operationalised)

Each ADR gained a small, high-value operationalisation that made it immediately executable:

- **ADR-0001:** Rust-activation rule with three measurable thresholds (`>2× adapter overhead`,
  `>30s normalisation`, `>2× memory per evidence`).
- **ADR-0003:** Portable `projectId` is **UUIDv4** derived from `SHA256(SOURCE_IDENTITY_CONTENT +
  firstExportTimestamp)`; rebind collision policy defaults to "reject + ask", with three
  explicit actions (replace, keep both, abort).
- **ADR-0004:** Mandatory `method` enum on every `confidence` value (`heuristic-v1`,
  `calibrated-v1`, `human-overridden`); absent `method` is rejected at write time.
- **ADR-0005:** Mermaid **excluded** from generated views by default; a single authored
  Mermaid in `README.md` is allowed inside a `<!-- archctl:preview -->` marker; CI rejects any
  auto-generated Mermaid in `sddk/` or auto-rendered diagrams.
- **ADR-0007:** Initial OpenCode pin is **`1.18.x`** (live schema snapshot 2026-07-27 matches);
  pin bumps only via Gate Zero + schema-contract test; live drift check is advisory only.
- **ADR-0008:** License allow-list is explicit (MIT, Apache-2.0, BSD-2/3, ISC, MPL-2.0,
  Unicode-DFS-2016); non-listed licenses require explicit operator opt-in via
  `archctl skills allow-license <SPDX>` and are logged.

## Relationship to the source design compass

These ADRs distill and *correct* decisions first sketched in `Skills-para-agentes-IA.md`
and refined through `sddk/architecture-intelligence-platform/explore-report.md` →
`proposal.md` → `design.md`. Where the source doc was ambiguous or wrong, the ADR records
the resolved position and the evidence.
