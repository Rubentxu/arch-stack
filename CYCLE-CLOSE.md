# Cycle 1 closure — archctl

**Closed**: 2026-07-29 (planning workspace, no remote, no tags).
**Branch**: `main`.
**Commits**: 13.
**Tests**: 60/60 passing.

## What is in the repository

| Area | Status | File |
|---|---|---|
| Planning corpus | complete | `sddk/architecture-intelligence-platform/{explore-report,proposal,spec,design,tasks,verification-report,gate-zero-report,spike-report}.md` |
| Decisions (8 ADRs) | all Accepted, all with operationalised rules | `docs/adr/0001..0008.md` |
| Roadmap | M0–M5 milestones; current state M2 closed | `docs/ROADMAP.md` |
| Executive summary | one-page equivalent | `docs/EXECUTIVE-SUMMARY.md` |
| M2 overview | acceptance gates + drift-guard policy | `docs/m2-overview.md` |
| Domain glossary | SourceIdentity, evidence, IR, IR-projection, drift guard, etc. | `CONTEXT.md` |
| README | concise landing | `README.md` |
| Source code | TypeScript M0–M2 (ADR-0001 enforced) | `packages/{core,cli,opencode-plugin}/src` |
| Agents | 4-role topology (orchestrator + extractor + synthesizer + auditor) | `.opencode/agents/archctl-{orchestrator,extractor,synthesizer,auditor}.md` |
| Slash command | `/archctl` | `.opencode/commands/archctl.md` |
| Evidence skill | evidence discipline + data-not-instructions | `.opencode/skills/archctl-evidence/SKILL.md` |
| OpenCode pin | `1.18.x` + vendored snapshot | `.opencode-version`, `schemas/opencode/1.18.x/` |
| Renderers | local Structurizr `:18080` + Kroki `:18000` (running via podman) | n/a (runtime) |
| Language-drift guard | ADR-0001 enforcement (single source of truth) | `scripts/check-language-drift.ts` |
| Gate Zero fixture | 5-file non-Git Go + adversarial suites | `m0-gate-zero/fixtures/{re,security,spike}/` |

## Verdicts

| Gate | Verdict |
|---|---|
| Coherence gate 1.6 (planning corpus) | **PASS** (97/100) |
| Gate Zero (5-file non-Git fixture) | **PASS** (Jaccard 1.000) |
| Spike report (projections + adversarial + renderers) | **PASS** |
| `archctl doctor` | **OK** |
| Full language-drift guard | **OK** |
| 60/60 tests | green |

## How to pick this back up

The next session should treat this repository as a **decision-grade
planning base** with a working TypeScript MVP underneath. To resume
work:

1. Read `docs/EXECUTIVE-SUMMARY.md` (one page) → `docs/ROADMAP.md`
   (next action) → `CHANGELOG.md` (what's been done).
2. Verify environment with `npx tsx packages/cli/src/probe.ts --human`
   and `npx tsx packages/cli/src/doctor.ts --human`.
3. Re-run the gate: `npx tsx m0-gate-zero/run.ts --human`.
4. Re-run the drift guard: `npx tsx scripts/check-language-drift.ts`.
5. Pick one of the deferred follow-ups below and start a new cycle.

## Deferred follow-ups (out of cycle 1 scope)

1. **Real Rust + TypeScript fixture repos** — materialise
   `m0-gate-zero/fixtures/spike/{rust-tiny,ts-medium}/` from real
   open-source projects with SPDX-licensed content.
2. **LLM-driven extraction** — wire a model behind `archctl-extractor`
   so the deterministic runner becomes the fallback for the LLM
   output. Requires a model choice and per-run token budgets.
3. **End-to-end OpenCode hook firing** — exercise `shell.env` and
   `tool.execute.before` inside the vendored `1.18.x` runtime;
   the contract test already asserts schema presence.
4. **`skills.lock.json` enforcement** — the doctor already reports
   the SPDX allow-list; the lock file content (M2.9) is the missing
   artefact.
5. **Bundle import/export CLI** (M2.12) and on-call doc (M2.14) —
   pure code tasks; safe to start anytime.
6. **M3 Rust extraction** — only if `archctl doctor` reports TS
   normalisation overhead above the ADR-0001 thresholds (>2× adapter,
   >30 s, >2× memory) AND M2 has been validated end-to-end.
7. **M4 temporal model + drift diff CI** — defer until value
   demonstrated by M2 on real repos.
8. **M5 observed graph + deep analyzers** — out of cycle 1 scope
   entirely.
