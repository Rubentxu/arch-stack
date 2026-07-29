# Changelog

## [unreleased] — M0 scaffold: OpenCode profile + minimal `archctl`

Per [`docs/ROADMAP.md`](docs/ROADMAP.md), M0 is "Validación de
OpenCode". This commit ships the M0 deliverables:

- **`profile/`** — OpenCode profile source.
  - `opencode.jsonc`: sets `default_agent: diagram-architect`, lists
    the four subagents, registers the `/diagram` command, restricts
    `edit` to the XDG project dir, allow-lists `archctl *` and the
    read-only git commands, denies `webfetch`.
  - `agents/diagram-architect.md` (primary) and the four subagents
    (`architecture-evidence`, `c4-modeler`, `uml-modeler`,
    `diagram-reviewer`) lifted from the v2 §5.
  - `commands/diagram.md`: the `/diagram <kind> [args]` dispatcher.
  - `skills/c4-context.md` and `skills/plantuml-sequence.md`: M0
    skeleton wrappers (full wrappers in M1).
  - `plugins/archctl-env.ts`: `shell.env` injection of `ARCHCTL_*`.
- **`archctl/`** — minimal TypeScript CLI for M0.
  - `cli.ts`: dispatcher for `doctor`, `project resolve`, `render`.
  - `doctor.ts`: XDG writability, Structurizr / PlantUML / Kroki
    reachability, OpenCode / `archctl` on PATH.
  - `render.ts`: HTTP POST to local Kroki on `:18000` and write the
    SVG beside the source.
  - `resolve.ts`: stub `project resolve` returning a default
    `SourceIdentity`. M1+ replaces it with the XDG-aware resolver.
- **`scripts/install.sh`** — copies `profile/` to
  `$XDG_CONFIG_HOME/opencode-architecture/`, prints the launch
  command.
- **`README.md`** — entry point with the install + run flow.
- **`.gitignore`** — also ignores `.archctl-rendered/`, the
  `target/` (Rust M2), and `~/.local/share/archctl/`.

Tests are deferred to M1: M0's exit criterion is end-to-end via
`/diagram`, not a unit test. The `archctl` CLI is intentionally
TypeScript here; M2 replaces it with the Rust binary per
[`docs/ROADMAP.md`](docs/ROADMAP.md).

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

## [0.1.0] — first reset (commits `f5e7f83` / `b7b57a6`)

Removed inflated planning artifacts and rewrote CONTEXT, ROADMAP,
ADRs and CHANGELOG from `Skills-para-agentes-IA.md`. Replaced an
8-document roadmap with a flat user-facing list. Kept the existing
MVP: TypeScript CLI with ast-grep + ctags extractors, Structurizr /
PlantUML projections, XDG persistence, local podman renderers, three
fixtures with SPDX-labelled `gold.json`.
