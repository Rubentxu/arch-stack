# ADR-0001: Plugin-First / No-Rust-First with Conditional Rust Extraction Gate

- **Status**: Accepted
- **Date**: 2026-07-29
- **Decides**: Technology stack and investment timing for the architecture-intelligence-platform.
- **Accepted by**: orchestrator, per user directive on 2026-07-29 ("a tu criterio, busca máximo valor para el usuario, utilidad y facilidad").

## Context

The source design document (`Skills-para-agentes-IA.md`) commits to a Rust core
(capability-router + normalizer + IR + state-machine) behind a TypeScript OpenCode plugin
shim. The exploration report (`explore-report.md`) found three structural defects in the
source doc, the most severe being **scope expansion by 10×** and a **front-loaded
irreversible technology choice** before the load-bearing hypothesis — *reliable reverse-
engineering of architecture from real codebases* — has been validated.

Every prior tool surveyed in the source document fails at exactly that reverse-engineering
step. The cited research (arXiv:2510.22787, arXiv:2605.24453) validates multi-agent C4
generation from a **text brief** and **visualization** respectively — neither proves
reliable recovery from real, large, multi-language repositories.

A Rust core front-loads: build infrastructure, a Rust↔TS IPC boundary, slower IR-schema
iteration, and product-grade commitment before any validation signal.

## Decision

**Build plugin-first / no-Rust-first.** The platform is implemented as OpenCode-native
agents + skills + a thin TypeScript plugin + JSON schemas. **No Rust.**

Rust is **conditional and deferred**: it is activated only if **both** of the following hold:

1. The Phase 2 validation gate passes (the core hypothesis survives), AND
2. The measured TypeScript normalization overhead justifies the cost of maintaining a
   Rust binary + IPC contract.

If the hypothesis is falsified, the skill-only baseline (Phase 0) remains a useful product.
If the hypothesis holds but TS overhead is acceptable, the system lives indefinitely at the
TypeScript level — that is a valid outcome, not a failure.

**Decision rule for activating Rust (operationalised):** Rust is activated **only if both**
conditions hold AND a measurable, repeatable threshold is met. Concretely:

1. **Validation gate** — the Phase 2 kill-criteria all pass (`precision ≥ 0.85`,
   `recall ≥ 0.80`, `cost < 50k tokens`, `lead-time < 10m`), AND
2. **Performance gate** — a profiling artifact (`docs/perf/rust-decision.md`) shows **at least
   one** of:
   - worst-case fast-profile adapter overhead in TypeScript > 2× the same adapter hosted in a
     pinned Rust binary; **OR**
   - end-to-end normalisation time > 30s for the medium TS fixture; **OR**
   - the per-evidence-record memory footprint > 2× the equivalent Rust implementation.

If none of those thresholds is exceeded, **stay on TypeScript permanently**. The cost of a
Rust binary, an IPC contract, and an extra build target is not repaid without a measurable
benefit.

## Consequences

- **Positive**: Highest reversibility — skills + TS plugin + JSON schemas delete without
  compile cycles; IR schema evolves at JSON speed; skills load natively; failure leaves
  residual value (disciplined diagramming skills + reusable schemas).
- **Negative**: TypeScript is slower for heavy normalization; weaker type safety than Rust;
  the result is not a "product-grade binary" on day one.
- **Neutral**: The uniform capability-adapter contract (ADR-0006) is the seam that keeps the
  Rust door open without committing — a future Rust core can host the same adapters.

## Alternatives considered

- **Rust core + TS plugin (source doc's design)** — rejected: high upfront cost, IPC
  boundary, slower schema iteration, product commitment before validation. Irreversibility:
  Medium-High.
- **Deliberately simple: one agent + skill stack (explore Option C)** — kept as the Phase 0
  fallback baseline, not the MVP: ships immediate value but no persistent evidence ledger
  or temporal model.
- **Skill-as-Code SDK headless (source §23)** — rejected for MVP: most complex, depends on
  unverified SDK/SSE stability; revisit only at Phase 5.
