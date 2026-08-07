# Spec — PlantUML local rendering (M40)

> **Change**: M40
> **Cycle**: CYC-2026-08-07-m40-plantuml-render-local
> **Branch**: `feat/m40-plantuml-render-local` @ `<tip>`
> **Status**: in_progress

This delta spec documents how archctl renders PlantUML source to SVG after
resolving the long-deferred "graphviz vendor strategy" question from M38.

---

## TL;DR

`archctl` does NOT render PlantUML itself. It delegates to a user-installed
PlantUML backend in this order of preference:

1. **`plantuml` in PATH** — Java PlantUML CLI. Canonical, byte-faithful. Install via `brew install plantuml` or download from <https://plantuml.com/download>.
2. **`docker` with `plantuml/plantuml` image** — `docker run --rm -i plantuml/plantuml -pipe -tsvg`.
3. **`archctl-puml-backend` in PATH** — user-supplied binary; reads puml on stdin, writes svg on stdout.

If none of these is available, `render` returns a clear error listing all three
install options. The error message includes the canonical install commands.

## Why delegate, not embed

ADR-011 (local-only): archctl must NOT open network connections. The previous
remote-renderer POST path was removed in 2026-08.

ADR-006 (envuelve, no reimplementa): archctl orchestrates adapters; it does not
compete with canonical PlantUML. Re-implementing PlantUML in Rust (byte-faithful
SVG output across 29+ diagram types) is out of scope.

ADR-019 (performance budget): the bundle export budget is < 2s for graphs < 10K
nodes. Spinning up Java PlantUML per render adds ~1s startup but is acceptable
for the `archctl render file.puml` use case (one-shot, not in a hot loop).

## Why not `plantuml-little`

Explored in the M40 cycle. The crate hard-links against `graphviz-anywhere` at
compile time (graphviz native library required even for use case / state /
class diagram layouts). This conflicts with ADR-011 because:

- The build script refuses to compile without a prebuilt `libgraphviz_api`
  static library (or graphviz installed system-wide, or a `GRAPHVIZ_ANYWHERE_ALLOW_DOWNLOAD=1`
  network fallback).
- At runtime, plantuml-little calls into the linked libgraphviz for layout of
  Class / State / Component / Use Case / Object / ERD / DOT / Archimate diagrams.

`default-features = false` only disables the `remote` feature (ureq); it does
NOT disable graphviz linking. So plantuml-little is not viable without either
vendoring a graphviz-anywhere prebuilt or requiring system graphviz at build
time. Both fail ADR-011.

A future cycle could revisit this if a pure-Rust graphviz layout engine ships,
but as of v1.12.0 the delegation strategy is the correct call.

## Code surface

### `archctl/src/render/plantuml.rs` (NEW, M40)

Exports:

- `pub fn render(source: &str) -> Result<String>` — runs the first available
  backend, returns SVG or an error with installation instructions.
- `pub fn looks_like_plantuml(path: &Path) -> bool` — extension-based dispatch
  helper for future path-based routing.

Internal:

- `enum Backend { PlantumlCli, DockerImage, CustomUserBinary }`
- `fn detect_backend() -> Option<Backend>` — probes PATH for each in order.

### `archctl/src/render.rs`

`mod plantuml;` added (alongside `mod mermaid` and `mod structurizr`).

The `RenderKind::Plantuml` arm of the dispatch table now calls
`plantuml::render(&body)` instead of bailing with "deferred to M40".

### Tests

- `archctl/src/render/plantuml.rs::tests`:
  - `detect_backend_returns_none_or_some` — runs without panicking regardless
    of host environment.
  - `render_without_backend_returns_clear_error` — when no backend is present,
    the error message names the three install options.
  - `looks_like_plantuml_recognises_puml_extension` — extension dispatch helper.

- `archctl/tests/plantuml_render_e2e.rs` (NEW):
  - `plantuml_render_real_world_use_case_to_svg` — feeds a real PlantUML
    use case diagram (matching the syntax emitted by `archctl diagram project
    --view usecase:* --format plantuml` per M39) and verifies the SVG contains
    actor + use case names. **SKIPS if no backend is installed.**
  - `plantuml_render_minimal_c4_container_to_svg` — minimal C4 container
    diagram. **SKIPS if no backend is installed.**

The skip-on-missing-backend design keeps the test suite usable on machines
without PlantUML installed (the typical CI / dev environment for this repo).
Machines with PlantUML get a real e2e test of the wire-up.

### Manifest

`manifests/render.toml` updated:

- `archctl/src/render/plantuml.rs` added to `editable`.
- PlantUML-specific items added to `must_hold` (`plantuml::render`, backend
  enumeration labels).

## Out of scope (deferred)

- **Pure-Rust PlantUML rendering** — needs a graphviz-free layout engine for
  Class / State / Use Case / etc. Tracked as future work if/when one ships.
- **Bundling a vendored graphviz-anywhere prebuilt** — violates ADR-011 (binary
  blob in repo) and ADR-019 (build size budget).
- **Auto-installing the backend** — violates ADR-011 (archctl must be a passive
  tool, not a package manager). User explicitly installs.

## Failure modes

| Condition | Behavior |
|---|---|
| No backend installed | Error with all 3 install options |
| `plantuml` not executable | `detect_backend` returns `None` (treated as missing) |
| `docker run` fails | Error includes docker stderr |
| Custom binary exits non-zero | Error includes backend stderr + exit code |
| Backend stdout is not UTF-8 | Error: "backend stdout is not valid UTF-8" |