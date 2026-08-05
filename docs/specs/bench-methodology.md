# bench-methodology Specification

> **Change**: `m27-sandbox-benchmarks`
> **Status**: Active
> **Domain**: bench-methodology (pre-v1.0 release gate thresholds)

## Purpose

Defines the explicit pre-v1.0 release gate for `archctl`: success thresholds,
per-dataset overrides, FP/FN rubric, fallback path, and the gate condition that
MUST be satisfied before declaring v1.0.

## Requirements

### Requirement: Success Thresholds

The system MUST declare the following release-gate thresholds. A dataset is
gate-blocking if it violates any threshold for its configured extractor.

| Threshold | Criterion |
|-----------|-----------|
| `exit_zero_rate` | ≥90% of datasets with exit 0 on at least 1 extractor |
| `c4_discover_time` | <30s wall time median for `c4-discover --apply` on <30MB projects |
| `export_time` | <5s wall time median for `diagram export container:*` on <100 nodes |
| `peak_rss` | <500MB peak RSS |
| `bundle_validity` | 100% `diagram validate` exit 0 |
| `determinism` | 100% determinism (baseRevision identical across 2 runs) |
| `fp_ratio` | False positive ratio <20% (manual rubric) |
| `fn_ratio` | False negative ratio <30% (manual rubric) |

#### Scenario: All thresholds pass at pre-v1.0 gate

- **GIVEN** 12 datasets run through the harness
- **WHEN** the report shows 11 exit 0 (91.7%), all <30s, all <500MB RSS, 100% valid and deterministic
- **THEN** the release gate is satisfied

#### Scenario: c4-discover time exceeds threshold

- **GIVEN** dataset `clap-rs/clap` (21MB) takes 45s median for `c4-discover`
- **WHEN** the threshold comparison runs
- **THEN** the gate reports FAIL: "c4_discover_time: 45s > 30s threshold for clap-rs/clap"

#### Scenario: Bundle validity below 100%

- **GIVEN** dataset `vueuse/vueuse` produces a bundle where `diagram validate` exits 1
- **WHEN** the threshold comparison runs
- **THEN** the gate reports FAIL: "bundle_validity: 91.7% < 100% threshold"

### Requirement: Per-Dataset Override

The methodology MUST allow per-dataset timeout override in `datasets.toml`. If
no override is declared, the global default of 60s SHALL apply.

#### Scenario: Custom timeout declared

- **GIVEN** `datasets.toml` has `{name = "clap-rs/clap", timeout = 90}`
- **WHEN** the harness runs `c4-discover` on clap-rs/clap
- **THEN** the timeout is 90s, not the global default of 60s

#### Scenario: No override — global default

- **GIVEN** `datasets.toml` has `{name = "tokio-rs/axum"}` with no `timeout` field
- **WHEN** the harness runs the extractor
- **THEN** the timeout is 60s (global default)

### Requirement: FP/FN Rubric

The methodology MUST document the FP/FN rubric per repository in the report.
FP and FN ratios are subjective and manual; the rubric SHALL be recorded as a
markdown section in the dated report.

#### Scenario: Rubric recorded per repo

- **GIVEN** `run-bench.sh` completes for dataset `square/okhttp`
- **WHEN** the human reviewer manually counts false positives and false negatives
- **THEN** the report includes a section: "FP/FN Rubric — square/okhttp"
- **AND** the section states FP count, FN count, and a 1-line justification per classification

#### Scenario: FP threshold exceeded — gate blocks

- **GIVEN** dataset `JetBrains/kotlin` has 8 FP out of 20 containers = 40% FP
- **WHEN** the FP/FN threshold comparison runs against the rubric
- **THEN** the gate reports FAIL: "fp_ratio: 40% > 20% threshold for JetBrains/kotlin"

### Requirement: Fallback

If A1 (Quadlet) is unworkable, `run-bench.sh` MUST be invokable directly on
the host without the Quadlet container. This path SHALL be documented as the
regression fallback.

#### Scenario: Direct invocation outside Quadlet

- **GIVEN** podman rootless Quadlet is not available
- **WHEN** the user runs `bench/run-bench.sh` directly on the host
- **THEN** the harness executes archctl from `target/release/archctl` using host paths
- **AND** produces the same `bench/reports/<date>.md` format

#### Scenario: Quadlet unavailable — no error

- **GIVEN** `bench/quadlets/archctl-bench.container` does not exist
- **WHEN** `bench/run-bench.sh` is invoked
- **THEN** the harness logs a warning: "Quadlet not found, running natively"
- **AND** proceeds with native execution

### Requirement: Pre-v1.0 Gate

The architecture MUST NOT ship v1.0 unless all thresholds pass. The most recent
`bench/reports/<date>.md` SHALL be the gate artifact. A report older than 30 days
MUST be considered stale and require a fresh run.

#### Scenario: Gate blocked — v1.0 cannot ship

- **GIVEN** `bench/reports/2026-08-05.md` shows 2 threshold failures
- **WHEN** a release decision is considered
- **THEN** the gate is BLOCKED; v1.0 MUST NOT ship until all thresholds pass

#### Scenario: Gate passed — v1.0 can ship

- **GIVEN** the most recent report shows all thresholds pass
- **WHEN** a release decision is considered
- **THEN** the gate is OPEN; v1.0 may ship

#### Scenario: Stale report

- **GIVEN** the most recent report is older than 30 days
- **WHEN** a release decision is considered
- **THEN** the report is considered stale and a fresh run MUST be executed
