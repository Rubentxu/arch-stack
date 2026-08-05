# ADR-032: Bench Methodology — Pre-v1.0 Release Gate

## Status

Accepted — 2026-08-05

## Context

v0.14.10 ships 6 bugs found by smoke-testing the C4 vertical against a single
real project (`tokio-rs/axum`, ADR-031). The 402-test unit suite passed while
those bugs existed; the false-confidence signal is exactly what ADR-031 warns
about. v1.0 cannot ship on "tests pass" alone.

The current benchmark surfaces cover **synthetic** criterion micro-benchmarks
(`archctl/benches/`, `scripts/bench-compare.sh`), but **not** real-project
behaviour across multiple languages. M27 (`docs/ROADMAP.md` M27) introduces
this third surface — a manually-invoked, on-demand harness that runs `archctl`
end-to-end against pinned commits of public repositories and emits a dated
report.

This ADR formalises the **methodology** behind the harness: which metrics
matter, what thresholds gate v1.0, and how subjective judgments (FP/FN) are
recorded. The harness itself is specified in `docs/specs/bench-harness.md`.

## Decision

### Three benchmark surfaces, distinct purposes

| Surface | Kind | Where | CI? | Purpose |
|---|---|---|---|---|
| Criterion micro-bench | synthetic, deterministic | `archctl/benches/` | `bench-smoke` | ADR-019 sub-op latency |
| `bench-compare.sh` | criterion regression gate | `scripts/` | `bench-compare` | block perf regressions >10% |
| **`bench/run-bench.sh`** | **real-project E2E, manual** | `bench/` (NEW) | **NOT a CI job** | **pre-v1.0 release gate** |

### Eight thresholds, equal weight

A dataset is gate-blocking if it violates any threshold for its configured
extractor. The release gate is **open** only when all eight pass. The full
threshold table is in `docs/specs/bench-methodology.md` Requirement: Success
Thresholds.

| Threshold | Criterion | Rationale |
|---|---|---|
| `exit_zero_rate` | ≥90% of datasets with exit 0 on ≥1 extractor | permits up to 10% of language×strategy combinations to fail without blocking v1.0 (e.g., a Kotlin repo where `class-diagram` extraction has known gaps) |
| `c4_discover_time` | <30s median for `c4-discover --apply` on <30MB projects | covers ~90% of datasets; clap-rs/clap (21MB) is the upper bound |
| `export_time` | <5s median for `diagram export container:*` on <100 nodes | ensures `archview` can load bundles without perceptible delay |
| `peak_rss` | <500MB | matches ADR-019 memory budget for 100k nodes scaled down |
| `bundle_validity` | 100% (`diagram validate` exit 0) | forbidden to emit an invalid bundle — this is a hard contract, not a regression margin |
| `determinism` | 100% (`baseRevision` identical across 2 runs) | foundation of auditability (ADR-008) |
| `fp_ratio` | <20% (manual rubric) | exact match vs reality; 20% tolerance for noisy strategies |
| `fn_ratio` | <30% (manual rubric) | under-counting is more tolerable than over-counting (FN doesn't generate wrong diagrams, FP does) |

### Determinism is binary, FP/FN is manual

`baseRevision` is a content-hash (`blake3`) over the bundle's canonical JSON
(ADR-013). Two runs producing different `baseRevision` values is a
determinism failure — even if the difference is timestamp-based. This is a
hard gate.

FP/FN is recorded as a per-repo markdown section in the dated report. The
human reviewer counts true positives (real containers correctly detected),
false positives (phantom containers), and false negatives (real containers
missed). The rubric is **not** automated because true positives require
reading the repo's README/structure to know what counts as "real."

### Per-dataset timeout override

`datasets.toml` declares a `timeout` field per dataset. The default is 60s.
Large repos (clap-rs/clap at 21MB) may need 90s. The timeout is the
deterministic budget for the extractor run; the orchestrator kills the
process and marks the row FAIL with the elapsed time.

### Container base image: `ubuntu:24.04` + `rustup` pin

We rejected `catthehacker/ubuntu:rust-latest` despite the roadmap's
suggestion. The reasoning:

1. **Supply-chain trust.** `catthehacker/ubuntu:rust-latest` is a floating
   community tag; there is no SHA-pinned readme, no build provenance, and
   no first-party maintainer. A binary embedded in that image becomes part
   of our reproducible build chain.
2. **Toolchain exactness.** The `rust-latest` tag has no guarantee of
   matching `rustc 1.97.1` (the project pin in `rust-toolchain.toml`).
3. **Substitutability.** `ubuntu:24.04` is a first-party LTS image with
   reproducible build infrastructure. `rustup default 1.97.1` inside the
   Containerfile gives us the same end-state with auditable inputs.

`target/release/archctl` is mounted pre-built from the host. The harness
container does **not** build `archctl` from source — building the full dep
tree (clap, tree-sitter, lbug, etc.) inside a clean container takes 10+
minutes on first run, which is incompatible with a release gate.

### XDG path translation

`archctl` resolves project data via `~/.local/share/archctl/projects/<uuid>/`
(ADR-004). Mounting this into a rootless container risks permission/path
bugs — the same bug class as ADR-031 B1 (where `apply()` wrote to `<cwd>`
but `graph_query` read from XDG). The harness runs `archctl` with a
container-local `XDG_DATA_HOME` and a fixed UUID, so the host XDG is not
contaminated by harness runs.

### Manual, on-demand release gate

The harness is **not** a CI job. The roadmap explicitly scopes CI
integration out (line 343: "CI integration → cuadrar con
`bench-compare.sh` existente"). Reasons:

- `git clone` requires network.
- Per-dataset run can take 1-3 minutes; 10+ datasets = 10-30 minutes total.
- FP/FN is a human judgment.

The harness is invoked manually before declaring v1.0. The most recent
`bench/reports/<date>.md` is the gate artifact. A report older than 30
days is considered stale and requires a fresh run.

### Fallback path

If the Quadlet container is unworkable on a host (missing subuid, no
systemd user session, etc.), `run-bench.sh` MUST be invokable directly on
the host. The script is the actual logic; the Quadlet is the systemd
wrapper. This is the documented regression path.

## Consequences

### Positive

- v1.0 ship decision is grounded in empirical data, not test counts.
- 6 of the 8 thresholds are automatable; the date-stamped report is a
  permanent audit record.
- `ubuntu:24.04` + `rustup` pin is auditable (root-signed LTS image +
  Pin via `rustup`).
- Fallback path removes the "Quadlet or nothing" risk.

### Negative

- FP/FN is subjective; reviewers may disagree. The rubric is published so
  disagreements are visible, not hidden.
- Dataset cache (`~/.cache/archctl-smoke/`) consumes ~100MB+ host disk.
- `git clone` of 10+ repos is a network dependency for the gate.
- The harness is **not** CI; it does not protect against regressions on
  feature branches. The existing `bench-compare.sh` (criterion) continues
  to cover that surface.

### Neutral

- 28 spec scenarios in `docs/specs/bench-{harness,methodology}.md` are the
  contract for `sddk-apply`. Tests are not automated at the script level;
  each scenario is testable by hand-running the harness with a fixture.

## Alternatives Considered

- **A2 (native bash, no container)** — fastest data, but no reproducibility
  and no Quadlet precedent. Deferred to a follow-up if A1 proves
  unworkable.
- **A3 (podman without Quadlet)** — compromise between A1 and A2. No
  precedent in repo, but simpler than Quadlet. Not chosen because the
  Quadlet follow-up is a known-good upgrade path.
- **Criterion-based real-project benches** — rejected. Criterion is for
  micro-benchmarks; `archctl` against a real project is a different kind
  of measurement (state on disk, I/O, binary spawn).

## References

- `docs/ROADMAP.md` M27 (lines 267-353)
- `docs/adr/ADR-031-c4-vertical-validation.md` (predecessor)
- `docs/specs/bench-harness.md` (spec contract)
- `docs/specs/bench-methodology.md` (spec contract)
- `sddk/m27-sandbox-benchmarks/explore-report.md`
- `sddk/m27-sandbox-benchmarks/proposal.md`
- `archctl/tests/smoke_real_projects.rs` (direct predecessor)
- `scripts/bench-compare.sh` (convention reference)
