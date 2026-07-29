# ADR-0005: Renderer Routing / Local-First Policy

- **Status**: Proposed
- **Date**: 2026-07-29
- **Decides**: Which renderers are used, their canonicality, and the offline-first posture.

## Context

Rendering must turn the IR into diagrams without inventing layout or leaking data. Two
concrete problems force a decision:

1. **Structurizr Lite is end-of-life/deprecated.** The Docker image `structurizr/lite` is
   marked deprecated. Its successor, `local`, is a **self-hosted workspace tool/viewer** —
   not a generic "distribution" to depend on blindly. For headless DSL validation/export we
   pin the currently supported **Structurizr CLI** and track migration to Structurizr vNext.
2. **Public rendering services exfiltrate data.** Public Kroki and public PlantUML servers
   receive the full diagram (and, for PlantUML, potentially embedded content). For an
   enterprise-architecture tool, that is an unacceptable default.

Mermaid C4 syntax remains experimental with incomplete layout, so it cannot be the canonical
representation — but it is invaluable for inline documentation previews.

## Decision

**Rendering is `IR → projection (pure function) → local renderer CLI`.** No LLM participates
in placement.

| Target | Renderer (local) | Role | Canonical? |
|---|---|---|---|
| C4 | **Structurizr `local`** (self-hosted workspace viewer; Structurizr Lite is EOL) | `workspace.dsl` → SVG/PNG | **Yes** |
| UML + complex C4 | PlantUML local jar (or internal Kroki container) | `.puml` → image | No |
| Preview | Mermaid | `.mmd` → inline | **No** (experimental) |
| Editable | draw.io export | `.drawio` | No |

**Local-first policy:** public Kroki / public PlantUML servers are **forbidden by default**.
Only local jars or an internal (self-hosted) Kroki container. Source snippets are never sent
to any renderer (`store-source-snippets: false`).

**Structurizr tooling:** `local` is a self-hosted workspace tool/viewer, not a generic
distribution. Headless DSL validation/export uses the **Structurizr CLI**, pinned to a
specific supported version (recorded in the supply-chain inventory — ADR-0008); we track
migration to Structurizr vNext and re-pin when the CLI command surface changes.

## Consequences

- **Positive**: No data exfiltration by default; deterministic, reproducible renders;
  Structurizr `local` stays on a maintained path.
- **Negative**: Local renderers are an operational dependency (a Java runtime for PlantUML,
  the Structurizr CLI). Mitigated by `archctl doctor` probing and progressive profiles.
- **Neutral**: Mermaid remains available for previews/PRs — its non-canonical status is
  documented, not removed.

## Alternatives considered

- **`structurizr/lite` (EOL)** — rejected: deprecated, unmaintained cliff.
- **Public Kroki/PlantUML default** — rejected: data exfiltration risk for an enterprise tool.
- **Mermaid as canonical C4** — rejected: experimental, incomplete, non-deterministic layout.
- **LLM-driven layout** — rejected: layout decisions by an LLM are precisely the
  invent-architecture failure mode this project exists to prevent.
