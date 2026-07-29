# M1 Spike Report (synthetic — pre-real-fixtures)

**Date**: 2026-07-29
**Status**: ⚠️ **PARTIAL** — synthetic IR against the canonical 5-file fixture and the
adversarial suites. **Real** Rust/TypeScript fixtures are placeholders only and
land when M1 starts.

## What was measured

| Surface | Result | Threshold | Status |
|---|---|---|---|
| IR produced from canonical fixture | 4 containers, 0 forbidden elements, 0 unsupported high-confidence | — | ✅ |
| Jaccard vs gold set | 1.000 | ≥ 0.95 | ✅ |
| Write confinement (XDG only) | 0 writes outside `$ARCHCTL_PROJECT_DIR` | = 0 | ✅ |
| Symlink-escape containment | Realpath containment test asserts escape detected | Must detect | ✅ |
| Prompt-injection defence | Auditor HARD-FAILS phantom element with high confidence + zero refs | Must HARD FAIL | ✅ |
| Projection purity (`IR → workspace.dsl`) | Same IR → byte-identical DSL | PURE | ✅ |
| Projection purity (`IR → .puml`) | Same IR → byte-identical PUML | PURE | ✅ |

## What was **not** measured (deferred to M1)

- **Real** Rust tiny (≤5 kLoC) and TypeScript medium (10–30 kLoC) repos.
  Placeholder directories exist under `m0-gate-zero/fixtures/spike/`; the
  real source material is the M1 discovery spike's first task.
- **LLM-driven extraction**. The Gate Zero runner is deterministic-shape
  based; Phase 1 introduces LLM-driven extraction once the runner's
  shape-based output is proven sufficient.
- **Local renderer execution** (Structurizr `local` + PlantUML jar / internal
  Kroki). Projections are written and pure; renderers are deferred until
  the operator installs them locally. M1's spike report (1.12) measures the
  full render gate once they are present.

## Adversarial fixtures

- `symlink-escape/`: a manifest declares a symlink whose target is `/etc/passwd`
  (inert — the manifest is metadata). The runner's `realpath`-based
  containment correctly detects the escape.
- `prompt-injection/`: a README contains an instruction-shaped payload. The
  auditor HARD-FAILS any IR that promotes the README content into a
  high-confidence element with zero evidence refs (defence in depth).

## Gate Zero (carried forward)

Gate Zero remains PASS (Jaccard 1.000 on the 5-file non-Git fixture). The
M1 spike report's only addition is the **adversarial** axis and the
**projection** axis; both pass.

## What blocks M1 from being declared fully PASS

1. Real repos + LLM-driven extraction.
2. Local renderer execution against the projections.
3. End-to-end hook firing via OpenCode runtime.

Until those land, the spike report's verdict is **PARTIAL** and the platform
must not advance to Phase 2.
