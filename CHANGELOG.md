# Changelog

## [unreleased] — second reset: aligned to the source document

Discarded the first reset's framing and rewrote from
`Skills-para-agentes-IA.md` literally:

- **CONTEXT.md** — the project is a CLI (`archctl`) that drives
  OpenCode + skills to produce C4 and UML diagrams from a repo.
- **ADRs** (7) — only the operational decisions the source document
  asks for: output scope, evidence-first, tool wrapping (no parsers),
  OpenCode orchestrator + 8 subagents, Structurizr canonical,
  external skills in 3 modes (direct/wrapped/patched), XDG
  persistence + local renderers.
- **ROADMAP** — 3 short phases (C4+UML, OpenCode, skill registry +
  drift) plus a "fuera del roadmap" section that names the pieces the
  first reset over-invented: temporal twin, falsifier agent, IR
  migrations, plugin platform, MCP profiles, Rust core.

## [0.1.0] — first reset (commit `f5e7f83` / `b7b57a6`)

Removed inflated planning artifacts and rewrote CONTEXT, ROADMAP,
ADRs and CHANGELOG from `Skills-para-agentes-IA.md`. Replaced an
8-document roadmap with a flat user-facing list. Kept the existing
MVP: TypeScript CLI with ast-grep + ctags extractors, Structurizr /
PlantUML projections, XDG persistence, local podman renderers, three
fixtures with SPDX-labelled `gold.json`.
