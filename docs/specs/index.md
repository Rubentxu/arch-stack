# Specs Index

> Index of all current specs in `docs/specs/`. Each spec captures the
> contract for one bounded context, view, or pipeline stage: element
> kinds, edge predicates, projection shape, and the public surface
> that other modules rely on.
>
> Refreshed at cycle close (M58). The canonical scope and audience
> live here; the Given-When-Then scenario detail lives in each
> spec file.

## Diagram views (5)

| Spec | Audience | One-line summary |
|---|---|---|
| [`diagram-projection-bundle.md`](diagram-projection-bundle.md) | `diagram` module, external tooling | Bundle contract: manifest + projection + evidence + styles envelope (`schemaVersion: 1.0`). |
| [`use-case-view.md`](use-case-view.md) | `diagram` module, Mermaid renderer | `usecase:*` view: `uml.actor`, `uml.use_case`, `participates_in` edges. |
| [`state-and-c4-views.md`](state-and-c4-views.md) | `diagram` module, Mermaid renderer | `state:*` and `c4-*:*` views with Mermaid shape mapping (M41). |
| [`sequence-view-labels.md`](sequence-view-labels.md) | `diagram` module, Mermaid renderer | `sequence:*` view with edge label support from `edge.props["label"]` (M45). |
| [`code-class-diagram/`](code-class-diagram/spec.md) | `code` module, `class_diagram` projector | `code class-diagram` pipeline: tree-sitter CST walk → `ClassNode`/`ClassEdge` carriers. |

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
