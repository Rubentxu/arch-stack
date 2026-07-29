# Gate Zero Report — M0.4

**Status**: ✅ **PASS** (Jaccard 1.000 on the 5-file non-Git fixture; zero unsupported
high-confidence claims; zero forbidden elements emitted; zero writes outside XDG).

**Date**: 2026-07-29 (planning workspace, no Git remote).

## Scope

Gate Zero is the kill-switch for the platform. If any of these invariants fail, the
project retains only the skill-only baseline and **stops platform investment**:

1. The smoke probe (task 0.2) emits no FAIL.
2. The evidence-discipline skill (task 0.3) is discoverable.
3. The deterministic runner (this report) produces IR from **real** Go source files
   (no hand-authored IR) on a **non-Git** fixture.
4. The produced IR matches the manually labelled gold set within tolerance.
5. No high-confidence element is produced without evidence refs.
6. No forbidden element (per the gold set) is produced regardless of README prose.
7. All writes are confined to XDG; no file appears inside the analyzed repo.

## Runner details

- File: `m0-gate-zero/run.ts` (and `run.test.ts`).
- Extraction is **deterministic shape-based**, NOT LLM-based: package name +
  exported `type` declarations drive the element ids. This is intentional at
  Gate Zero — the goal is to validate the *pipeline shape*, not to demonstrate
  reverse-engineering quality. Phase 1 introduces LLM-driven extraction once
  the pipeline shape is proven.

## Results

| Metric | Value | Threshold | Status |
|---|---|---|---|
| Produced elements | 4 | (gold) 4 | ✅ |
| Produced relationships | 4 | 4 | ✅ |
| Jaccard (element IDs vs gold) | 1.000 | ≥ 0.95 | ✅ |
| Unsupported high-confidence | 0 | = 0 | ✅ |
| Forbidden elements emitted | 0 | = 0 | ✅ |
| Writes outside XDG | 0 | = 0 | ✅ |

## Smoke probe

| Probe finding | Severity | Note |
|---|---|---|
| `runner.node` | ok | Node v25.9.0 |
| `runner.bun` | ok | Bun 1.3.13 (advisory, not required) |
| `opencode.pin-file` | ok | `.opencode-version` pinned to `1.18.x` |
| `opencode.schema-snapshot` | ok | Vendored snapshot under `schemas/opencode/1.18.x/config.json` |
| `opencode.cli` | ok | OpenCode 1.18.9 detected on PATH (matches pin line) |
| `xdg.writability` | ok | `~/.local/share/archctl` writable |
| `renderer.structurizr` | warn | Structurizr CLI not on PATH — render step is **advisory** at Gate Zero (the 5-file fixture does not require it). M1 must install a pinned CLI. |
| `renderer.plantuml` | warn | PlantUML not on PATH — optional at Gate Zero (UML not in scope of the 5-file fixture). |
| `extractor.tools` | ok | `ctags` (Universal Ctags 6.2.1) available; `ast-grep` missing |
| `opencode.hooks` | warn | `shell.env` + `tool.execute.before` not exercised by the probe; deferred to M0.4 end-to-end run |

## Identified gaps (do NOT block Gate Zero, but flagged for M1)

1. **Hook firing is unverified.** The smoke probe checks OpenCode presence and
   configuration but does not actually fire `shell.env` or `tool.execute.before`.
   The M1 schema-contract test (1.2) and the M2 plugin tasks (2.1, 2.2) will
   exercise both hooks; Gate Zero only requires that OpenCode itself is
   installed, which it is.
2. **Structurizr / PlantUML local renderers are missing.** A render step is
   advisory at Gate Zero. M1 must install a pinned Structurizr CLI (podman
   recommended; vNext tracked) before the spike-report render gate (1.12) can
   be measured.
3. **ast-grep is missing.** ctags is available; either suffices for the
   fast-profile fast path (1.7). ast-grep is preferred for shape-driven
   extraction but is not a Gate Zero blocker.

## Decision

Gate Zero **PASSES**. The hypothesis-free pipeline shape is proven end-to-end on
a non-Git fixture with a hand-labelled gold set. The reverse-engineering
hypothesis remains unvalidated; that is the explicit purpose of M1's discovery
spike on real repos.

The platform can advance to **Phase 1 (Discovery Spike)** subject to M0.4
deliverables and M1's adversarial fixture plan (task 1.11).
