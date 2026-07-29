# ADR-0006: Reuse-over-Rebuild and Capability Adapter Contract

- **Status**: Accepted
- **Date**: 2026-07-29
- **Decides**: Whether to build custom analyzers, and the shape of the tool-integration seam.
- **Accepted by**: orchestrator, per user directive on 2026-07-29.

## Context

The source document lists 10+ capable CLIs (ast-grep, ctags, SCIP, cargo/go/jdeps,
dependency-cruiser, syft, terraform graph, etc.). Building custom multi-language
parsers/indexers would be a multi-quarter effort to reproduce tools that already exist and
are maintained by larger communities. archctl's differential value is **fusing heterogeneous
evidence with provenance into a trustworthy model**, not re-analyzing code.

The integration question is the *shape of the seam* between "I need a capability" and "the
concrete tool." Three radically different shapes were compared (see design §5):

- **A — Fat typed adapter** (one interface per capability): type-safe but N interfaces; adding
  a capability forces caller changes ⇒ breaks OCP, high type connascence.
- **B — Thin uniform adapter** (`run(ctx) → RawEvidence[]`): one contract; OCP-perfect; the
  adapter owns normalization (deep module).
- **C — Pure declarative** (YAML command descriptor): zero-code adapters for shell+JSON tools,
  but needs a parser-module and can't express semantic adapters.

## Decision

**Reuse over rebuild: archctl is a router + normalizer, never an analyzer.** No custom
parsers or indexers. Every CLI tool is *adapted*, not reimplemented.

**The contract is Shape B (uniform `Adapter`), with Shape C (declarative `ShellAdapter`) as
the default implementation.**

```ts
interface Adapter {
  capability: string;                  // "extract.dependencies"
  requires: string[];                   // ["cargo","git"] — probed by `doctor`
  run(ctx: RunContext): Promise<RawEvidence[]>;
}
```

The capability router maps a capability name to an adapter by registry entry. ~90% of
fast-profile adapters are zero-code YAML descriptors (ast-grep, ctags, cargo metadata,
dependency-cruiser); complex adapters (semantic SCIP/LSP) implement the `Adapter` interface
in code. The seam validates `RawEvidence` once, then the ledger is append-only.

## Consequences

- **Positive**: Genuinely OCP-compliant (add a tool = add a registry entry, zero caller
  change — the design's strongest entropy property, explore-report §9); uniform testing;
  minimal caller leakage (callers know only a `capability` string).
- **Negative**: Weaker per-call type safety than Shape A (mitigated by validating
  `RawEvidence` at the seam).
- **Neutral**: This seam is also what keeps a future Rust core (ADR-0001) open — the same
  adapters can be hosted by either runtime.

## Alternatives considered

- **Shape A (fat typed adapter)** — rejected: OCP violation, high coupling, more code.
- **Shape C alone (pure declarative)** — rejected as insufficient: can't express non-JSON or
  semantic adapters; kept as the *default implementation* under Shape B instead.
- **Custom parsers/indexers** — rejected: multi-quarter rebuild of existing maintained tools;
  diverges from the actual differential value (evidence fusion).
