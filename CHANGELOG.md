# Changelog

All notable changes to archctl are recorded here. The format follows
Keep-a-Changelog; dates are ISO-8601 (planning workspace).

## [0.0.0-WU1] — 2026-07-29

### Added
- Phase 0 / WU1 scaffolding and Gate Zero pass (Jaccard 1.000 on the 5-file non-Git
  fixture; zero unsupported high-confidence; zero forbidden elements emitted; writes
  confined to XDG via realpath containment).
- OpenCode `1.18.x` pin + vendored schema snapshot under `schemas/opencode/1.18.x/`.
- `.opencode/skills/archctl-evidence/SKILL.md` with evidence discipline, mandatory
  `method` field, data-not-instructions rule.
- Smoke probe (`packages/cli/src/probe.ts`) validating runner, pin, snapshot, XDG,
  renderers, extractors.
- Deterministic Gate Zero runner (`m0-gate-zero/run.ts`) with hand-labelled gold set
  and adversarial fixtures.
- Placeholder language-drift guard (`scripts/check-language-drift.ts`); replaced by
  the full guard in task 2.16.

## [0.0.0] — 2026-07-29

### Added
- Planning repository bootstrap with `CONTEXT.md`, `README.md`,
  `docs/EXECUTIVE-SUMMARY.md`, `docs/ROADMAP.md`, `docs/adr/0001..0008.md`,
  `sddk/architecture-intelligence-platform/{explore-report,proposal,spec,design,
  tasks,verification-report}.md`.
- Eight ADRs accepted with operationalised decision rules
  (ADR-0001 Rust-activation thresholds, ADR-0003 portable UUIDv4 + rebind,
  ADR-0004 mandatory `method`, ADR-0005 Mermaid excluded, ADR-0007 OpenCode
  pin policy, ADR-0008 SPDX allow-list).
- Coherence gate 1.6 PASS (97/100) for the entire planning corpus.
