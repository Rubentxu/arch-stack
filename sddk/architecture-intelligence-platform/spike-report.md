# M1 Spike Report (now with local renderers)

**Date**: 2026-07-29
**Status**: ✅ **PASS** — pipeline projections, adversarial axes, and **local
renderers** all green. End-to-end: synthetic IR → workspace.dsl + diagram.puml
→ PNG via local Kroki.

## What was measured

| Surface | Result | Threshold | Status |
|---|---|---|---|
| IR produced from canonical fixture | 4 containers, 0 forbidden, 0 unsupported high-confidence | — | ✅ |
| Jaccard vs gold set | 1.000 | ≥ 0.95 | ✅ |
| Write confinement (XDG only) | 0 writes outside `$ARCHCTL_PROJECT_DIR` | = 0 | ✅ |
| Symlink-escape containment | Realpath containment test asserts escape detected | Must detect | ✅ |
| Prompt-injection defence | Auditor HARD-FAILS phantom element with high confidence + zero refs | Must HARD FAIL | ✅ |
| Projection purity (`IR → workspace.dsl`) | Same IR → byte-identical DSL | PURE | ✅ |
| Projection purity (`IR → diagram.puml`) | Same IR → byte-identical PUML | PURE | ✅ |
| Local PlantUML render | Kroki `/plantuml/png` returns 200, PNG written to disk | render-success | ✅ |
| Smoke probe (renderers) | Structurizr local HTTP on :18080 + Kroki on :18000 reachable | reachable | ✅ |

## Renderers (locally running)

- `structurizr/structurizr:latest` (podman) on `localhost:18080` — the
  `local` subcommand's self-hosted workspace viewer. Used as a human
  inspection surface; headless DSL → image rendering goes through Kroki.
- `yuzutech/kroki:latest` (podman) on `localhost:18000` — `/plantuml/png`
  endpoint renders the projection to PNG. The bundled PlantUML in
  Kroki is configured with `SECURE=false` + `ALLOW_NETWORK=false`
  (default), so no outbound traffic is generated.

## Adversarial fixtures

- `symlink-escape/`: a manifest declares a symlink whose target is `/etc/passwd`
  (inert — the manifest is metadata). The runner's `realpath`-based
  containment correctly detects the escape.
- `prompt-injection/`: a README contains an instruction-shaped payload. The
  auditor HARD-FAILS any IR that promotes the README content into a
  high-confidence element with zero evidence refs.

## Gate Zero (carried forward)

Gate Zero remains PASS (Jaccard 1.000 on the 5-file non-Git fixture).

## What was **not** measured (deferred)

- **Real** Rust tiny (≤5 kLoC) and TypeScript medium (10–30 kLoC) repos.
  Placeholder directories exist under `m0-gate-zero/fixtures/spike/`; the
  real source material is M1's first task.
- **LLM-driven extraction**. The Gate Zero runner is deterministic-shape
  based; Phase 1 introduces LLM-driven extraction once the runner's
  shape-based output is proven sufficient.
- **End-to-end OpenCode hook firing**. Schema contract is validated; the
  runtime firing of `shell.env` + `tool.execute.before` is still unverified.

## M1 verdict

✅ **PASS** for everything that can be measured without an external LLM
and without real repositories. The remaining work is to instantiate the
real fixtures (Rust tiny + TS medium) and connect LLM-driven extraction,
both of which are M1's own scope.

The platform may advance to Phase 2 (M2: MVP plugin-first) subject to
the M2 entry criteria (the `archctl doctor` + the 4-role topology + the
plugin shell.env / write-guard).
