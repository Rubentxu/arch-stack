# Changelog

## [unreleased] — adopt `docs/` v2 spec as authoritative

The previous single-document proposals in this repo (a flat list of 7
ADRs, a JSON+SQLite property graph, and a 3-phase roadmap) contradicted
the parallel `docs/` specification. Resolution:

- **`docs/data-model.md`** removed. Superseded by the v2
  [`docs/DATA-MODEL-LADYBUGDB.md`](docs/DATA-MODEL-LADYBUGDB.md)
  (LadybugDB, reified relations, versioned elements).
- **`docs/adr/README.md`** replaced with an index over the 11
  individual ADRs in `docs/adr/`. ADR-000 documents the scope
  restart; ADR-011 (new) closes the public-renderer-policy gap that
  was missing in the v2 spec.
- **`ROADMAP.md`** (root) replaced with a redirect to
  [`docs/ROADMAP.md`](docs/ROADMAP.md) (M0–M11).
- **`CONTEXT.md`** rewritten to match the v2 vocabulary (OpenCode
  first, `archctl` sidecar, LadybugDB) and cross-link the new docs.
- **`docs/manifest.json`** updated to register ADR-011 and to list
  what the v2 supersedes.
- The full v2 document tree under `docs/` is now tracked.

## [unreleased] — second reset: aligned to the source document

Discarded the first reset's framing and rewrote from
`Skills-para-agentes-IA-v2.md` literally:

- A CLI (`archctl`) that drives OpenCode + skills for C4/UML
  architecture diagrams of a repo.
- 7 ADRs (was 8) — only operational decisions reflected in code.
- No "Architecture IR" (the user wanted diagrams, not a graph store).
- No "Architecture Auditor / Falsifier" (the user wanted tooling, not
  an autonomous agent platform).
- No "Temporal Architecture" / "Drift Detector" / "Coherence Gate" /
  "Spike Report" — those came from the planning agent's drift.

## [0.1.0] — first reset (commit `f5e7f83` / `b7b57a6`)

Removed inflated planning artifacts and rewrote CONTEXT, ROADMAP,
ADRs and CHANGELOG from `Skills-para-agentes-IA.md`. Replaced an
8-document roadmap with a flat user-facing list. Kept the existing
MVP: TypeScript CLI with ast-grep + ctags extractors, Structurizr /
PlantUML projections, XDG persistence, local podman renderers, three
fixtures with SPDX-labelled `gold.json`.
