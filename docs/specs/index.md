# Specs Index

> Index of all current specs in `docs/specs/`. Each spec captures the
> contract for one bounded context, view, or pipeline stage: element
> kinds, edge predicates, projection shape, and the public surface
> that other modules rely on.
>
> Refreshed at cycle close (M69). The canonical scope and audience
> live here; the Given-When-Then scenario detail lives in each
> spec file.

## Diagram views (5)

| Spec | Audience | One-line summary |
|---|---|---|
| [`diagram-projection-bundle.md`](diagram-projection-bundle.md) | `diagram` module, external tooling | Bundle contract: manifest + projection + evidence + styles envelope (`schemaVersion: 1.1`; cosmetic fields added in M81). |
| [`use-case-view.md`](use-case-view.md) | `diagram` module, Mermaid renderer | `usecase:*` view: `uml.actor`, `uml.use_case`, `participates_in` edges. |
| [`state-and-c4-views.md`](state-and-c4-views.md) | `diagram` module, Mermaid renderer | `state:*` and `c4-*:*` views with Mermaid shape mapping (M41). |
| [`sequence-view-labels.md`](sequence-view-labels.md) | `diagram` module, Mermaid renderer | `sequence:*` view with edge label support from `edge.props["label"]` (M45). |
| [`code-class-diagram/`](code-class-diagram/spec.md) | `code` module, `class_diagram` projector | `code class-diagram` pipeline: tree-sitter CST walk → `ClassNode`/`ClassEdge` carriers. |

## Horizons H0–H3 (new in m69 convergence cycle)

| Spec | Horizon | One-line summary |
|---|---|---|
| [`executable-bundle-contract.md`](executable-bundle-contract.md) | H0 | viewer-bundle schema as cross-language executable truth; Rust DTO ↔ JSON schema ↔ TS types field alignment; configurable selector. |
| [`durable-workspace-state.md`](durable-workspace-state.md) | H1 | WorkspaceState persisted to XDG (not localStorage) for ephemeral-port restart recovery. |
| [`source-drawer-read-only.md`](source-drawer-read-only.md) | H1 | Read-only source drawer with path-traversal rejection and IDE handoff. |
| [`cosmetic-changeset-roundtrip.md`](cosmetic-changeset-roundtrip.md) | H2 | Cosmetic ChangeSet round-trip with baseRevision integrity; **closed by M81** (schema v1.1, D1 label preservation, D2 cosmetic fields). |
| [`arrows-compatibility-adapter.md`](arrows-compatibility-adapter.md) | H2 | `.arrows` (Arrows.app v0.8) export adapter; **export realised in v1.41.0 (M80b)** via `archctl diagram export --format arrows`; import deferred to phase 2 (not implemented until a real consumer triggers it). Canonical graph is the only source of truth. |
| [`lens-spec-entry-criteria.md`](lens-spec-entry-criteria.md) | H3 | LensSpec entry criteria gated by 2 consumers OR measured need; reversibility clause required. |

## Code extraction (3)

| Spec | Audience | One-line summary |
|---|---|---|
| [`source-evaluation-types.md`](source-evaluation-types.md) | `code` module, evidence subsystem | Source & evaluation graph types: how source files map to evaluation rules. |
| [`filesystem-port.md`](filesystem-port.md) | `code` module, integration tests | `Filesystem` port trait: `read`, `write`, `exists`, `walk` over an in-memory adapter for tests. |
| *(no spec yet)* — `code::call_graph` | code module | Suggested future scope (M66 will add a dedicated spec). |

## Rendering (1)

| Spec | Audience | One-line summary |
|---|---|---|
| [`plantuml-render.md`](plantuml-render.md) | `render` module, PlantUML backend | PlantUML render via user-installed backend (Java CLI / docker / custom). Local-only by default (ADR-011). |

## Benchmarking (2)

| Spec | Audience | One-line summary |
|---|---|---|
| [`bench-harness.md`](bench-harness.md) | `bench` module, release pipeline | Criterion harness with 3 datasets, deterministic seed, ADR-019 budget guard. |
| [`bench-methodology.md`](bench-methodology.md) | `bench` module, future contributors | When to add a benchmark vs a perf test; dataset selection; reporting. |

## E2E suites (3)

| Spec | Audience | One-line summary |
|---|---|---|
| [`e2e-installation.md`](e2e-installation.md) | release engineer, CI | Bootstraps a fresh install (`./install.sh`) in a temp dir; ADR-034 §1. |
| [`e2e-render.md`](e2e-render.md) | release engineer, CI | Renders all 5 view types via Mermaid + PlantUML; asserts exit 0 + non-empty output. ADR-034 §2. |
| [`e2e-sandbox.md`](e2e-sandbox.md) | release engineer, CI | Sandboxed benchmark suite (`bench/sandbox-e2e.sh`); ADR-034 §4. |

## How to read a spec

Each spec follows the same skeleton:

1. **Purpose** — what the bounded context exposes and why.
2. **Public surface** — types, functions, errors the caller can rely on.
3. **Element kinds & edge predicates** — the canonical `Element` /
   `Relation` vocabulary (when the spec owns a graph extension).
4. **Projection shape** — the JSON schema for the bundle/carrier
   (when the spec is a view).
5. **Scenarios** — Given-When-Then examples; some are executed as
   tests, others are documentation only.
6. **Cross-references** — ADRs, related specs, issue/PR links.

Specs that are **deltas** (M32-style change records) carry a header
like `> **Change**: <cycle-name>` and modify the canonical spec of
the bounded context.

## How to add a spec

1. Pick the right folder under `docs/specs/` (one folder per bounded
   context if the spec owns more than one file).
2. Use the skeleton above. Add the file to `docs/README.md` under
   "View specs" — the index here is the canonical table of contents.
3. Open a PR with the spec + the manifest update if the new spec
   changes `public_symbols` (see `CONTRIBUTING.md` § Manifest hygiene).

## What is NOT in this index

- **ADRs** — those live in `docs/adr/` and have their own index
  (`docs/adr/README.md`).
- **Cycle-specific working files** — those live in `sddk/<cycle>/`
  for the cycle's lifetime and are archived on close.
- **Generated docs** — anything in `docs/reports/` (releases) is
  gitignored and regenerated.
