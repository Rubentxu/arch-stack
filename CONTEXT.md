# CONTEXT — Architecture Intelligence Platform (archctl)

> Concise domain glossary for the `architecture-intelligence-platform` change.
> Authoritative definitions live here; ADRs in `docs/adr/` hold the *decisions* behind them.
> Source design compass: `Skills-para-agentes-IA.md` (traceability, not authority).

## Core terms

| Term | Definition | Status |
|---|---|---|
| **Architecture IR** | Neutral, renderer-independent model: elements + relationships + `confidence` + `classification` + evidence refs. The single source of truth. | ✅ Resolved (ADR-0002) |
| **Evidence** | A source-grounded observation (path/lines/commit/hash) with `extractor` + `confidence` + `classification`. Stored append-only in the ledger. | ✅ Resolved (ADR-0004) |
| **Evidence ledger** | Append-only JSONL store of evidence records under XDG. | ✅ Resolved |
| **Projection** | A pure, deterministic function `IR → renderer DSL` (Structurizr/PlantUML/Mermaid). Diagrams are projections, never the source. | ✅ Resolved (ADR-0005) |
| **Renderer** | A deterministic local CLI that turns a DSL into an image (Structurizr `local`, PlantUML jar). Not an agent. | ✅ Resolved |
| **Capability router** | Maps an abstract capability (`extract.dependencies`) to a concrete tool adapter via a registry. OCP-compliant. | ✅ Resolved (ADR-0006) |
| **Adapter** | Uniform contract: `run(ctx) → RawEvidence[]`. Default impl is a declarative `ShellAdapter` driven by YAML. | ✅ Resolved (ADR-0006) |
| **Classification** | One of `fact | inference | hypothesis | unknown | conflict` — the epistemic class of an element/evidence. | ✅ Resolved (ADR-0004) |
| **Confidence** | Numeric 0..1 self-reported certainty with provenance. Calibration method is open. | ⚠️ Method unresolved |
| **SourceIdentity** | Discriminated project identity resolved by the plugin at session start: `git` mode (repository/worktree ids) or `directory` mode (directory id). | ✅ Resolved (ADR-0003) |
| **Repository id** | Git mode: `BLAKE3(normalized_remote + root_commit)` — stable and **sharable** across machines. Branch is NOT part of identity. | ✅ Resolved (ADR-0003) |
| **Worktree id** | Git mode: `BLAKE3(repository_id + realpath(show_toplevel))`. | ✅ Resolved |
| **Directory id** | Directory mode (no Git): `BLAKE3(canonical_realpath)` — **local-only** stability; not portable across hosts without explicit rebind. | ✅ Resolved (ADR-0003) |
| **Portable project id** | Stable UUID carried by export bundles; re-bound to a local SourceIdentity on import (the machine-specific anchor differs). | ✅ Resolved (ADR-0003) |
| **XDG store** | `~/.local/share/archctl/projects/<id>/` — trusted runtime state; never touches the analyzed repo. | ✅ Resolved (ADR-0003) |
| **Export bundle** | Explicit `archctl project export` archive (model + evidences, no sensitive code + skillset.lock) for sharing. | ✅ Resolved |
| **Write-guard** | Plugin `tool.execute.before` hook + config permission confining writes to `$ARCHCTL_PROJECT_DIR`. | ✅ Resolved (ADR-0008) |
| **Unsupported claim** | An element/relation with `confidence ≥ 0.9` and zero evidence refs. **HARD FAIL** invariant. | ✅ Resolved (ADR-0004) |
| **Profiles** | `fast` (git + ast-grep + ctags + build tools), `semantic` (SCIP/LSP), `deep` (Joern/CodeQL, on-demand). | ✅ Resolved |

## Graph axes (declared / static / observed)

| Graph | Meaning | MVP? |
|---|---|---|
| **Declared** | What docs/ADR/IaC say exists | ✅ Phase 1 |
| **Static** | What code/imports/contracts imply | ✅ Phase 1 |
| **Observed** | What runtime traces show | ❌ Phase 5 (needs telemetry) |

## Resolved vs unresolved

| Item | Status |
|---|---|
| "Architecture intelligence platform" scope = skill-only baseline + 4-role plugin-first MVP (no 9-agent fantasy) | ✅ Resolved (ADR-0007) |
| Canonical C4 store = IR is truth; Structurizr DSL is the canonical projection | ✅ Resolved (ADR-0002) |
| Storage = XDG runtime + explicit export (no in-repo `.architecture/`) | ✅ Resolved (ADR-0003) |
| Project identity = discriminated SourceIdentity (git | directory); Git is an *optional* capability adapter, not a universal prerequisite | ✅ Resolved (ADR-0003) |
| `mcp` is the OpenCode config key (NOT `mcpServers`) | ✅ Resolved (ADR-0007) |
| Structurizr Lite is EOL — `local` is a self-hosted workspace viewer; headless validation/export via pinned Structurizr CLI; track vNext | ✅ Resolved (ADR-0005) |
| Mermaid C4 is experimental — non-canonical preview only | ✅ Resolved (ADR-0005) |
| `experimental.session.compacting` is a plugin hook, NOT a config key; compaction config is top-level `compaction` | ✅ Resolved |
| Rust timing = deferred, conditional on validation | ✅ Resolved (ADR-0001) |
| Confidence calibration method | ⚠️ **Unresolved** — Phase-1 experiment |
| Exact OpenCode hook signatures across versions | ⚠️ **Unresolved** — needs version-pin + schema-test |

## Flagged ambiguities

- **"architecture-intelligence-platform"** — resolved to mean the *recovery + projection system* (this repo), not a generic term. Scope is deliberately narrow: recover → model → project, with falsifiable validation gates.
