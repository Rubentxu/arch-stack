# bench-harness Specification

> **Change**: `m27-sandbox-benchmarks`
> **Status**: Active
> **Domain**: bench (sandboxed benchmarking harness)

## Purpose

Defines the reproducible, sandboxed benchmarking harness for `archctl` pre-v1.0
validation. The harness runs the C4 vertical plus complementary extractors across
10+ multi-language datasets, capturing exit code, wall time, RSS, JSON validity,
and determinism, and emitting a dated report.

## Requirements

### Requirement: Datasets

The harness MUST source datasets from `bench/datasets.toml`. Each dataset entry
MUST declare a pinned git SHA, language tag, extractor name, and timeout in seconds.

#### Scenario: Valid dataset declaration

- **GIVEN** `datasets.toml` contains `{name = "rust-axum", sha = "abc123", language = "rust", extractor = "c4-discover", timeout = 60}`
- **WHEN** the harness parses `datasets.toml`
- **THEN** dataset `rust-axum` is loaded with sha `abc123`, language `rust`, extractor `c4-discover`, timeout `60s`

#### Scenario: Missing required field

- **GIVEN** `datasets.toml` contains an entry missing the `sha` field
- **WHEN** the harness parses `datasets.toml`
- **THEN** the harness exits code 2 and reports "missing required field: sha"

#### Scenario: Empty datasets file

- **GIVEN** `datasets.toml` contains zero dataset entries
- **WHEN** the harness runs
- **THEN** the harness exits code 0 and emits an empty report

### Requirement: Container

The harness container MUST use `ubuntu:24.04` as its base image. The Rust toolchain
MUST be pinned to `1.97.1` via `rustup default 1.97.1` inside the Containerfile.
The `archctl` binary SHALL be mounted pre-built from the host at
`target/release/archctl`.

#### Scenario: Container builds successfully

- **GIVEN** `bench/Containerfile` with `FROM ubuntu:24.04` and `rustup default 1.97.1`
- **WHEN** `podman build -t archctl-bench .` is executed
- **THEN** the image builds without error and `rustc --version` reports `1.97.1`

#### Scenario: Container rejects floating tag

- **GIVEN** `bench/Containerfile` uses `FROM ubuntu:latest`
- **WHEN** `manifests/bench.toml` is checked with `archctl doctor --scopes bench`
- **THEN** the gate fails: "base image must be pinned to ubuntu:24.04"

### Requirement: Quadlet

The Quadlet unit `archctl-bench.container` MUST be `Type=oneshot`, run rootless
(no `--bind`, no `--reuse`), and declare explicit `--uidmap` for XDG path translation.

#### Scenario: Valid Quadlet unit

- **GIVEN** `archctl-bench.container` with `Type=oneshot`, no `--bind`, and `--uidmap 1000:0:1`
- **WHEN** the container is started via `systemctl --user start archctl-bench`
- **THEN** the service transitions to `inactive (dead)` with exit code 0 after completion

#### Scenario: Invalid Quadlet — daemon flag present

- **GIVEN** `archctl-bench.container` contains `--daemon` in `Exec=`
- **WHEN** the container is started
- **THEN** the Quadlet is rejected because Type=oneshot forbids daemonized containers

### Requirement: Orchestrator

The orchestrator `bench/run-bench.sh` MUST, for each dataset: clone (cached at
`~/.cache/archctl-smoke/`), run the configured extractor, capture exit code, wall
time (median of 3 runs), peak RSS via `/usr/bin/time -v`, JSON validity, and
determinism (2 runs comparing `baseRevision`).

#### Scenario: Happy path — rust-axum dataset

- **GIVEN** dataset `rust-axum` with `extractor = "c4-discover"` and `timeout = 60`
- **WHEN** `bench/run-bench.sh` executes
- **THEN** `archctl` exits 0 within 60s, wall time median and RSS are captured
- **AND** `diagram validate` exits 0 on the exported bundle
- **AND** determinism check shows identical `baseRevision` across 2 runs

#### Scenario: Timeout exceeded

- **GIVEN** dataset `clap-rs/clap` with timeout `30s` but `c4-discover` takes 45s
- **WHEN** the 30s timeout elapses
- **THEN** the orchestrator kills the process, marks row FAIL with note "timeout 30s"
- **AND** continues to the next dataset without aborting

#### Scenario: Non-deterministic output

- **GIVEN** dataset produces different `baseRevision` values across two runs
- **WHEN** the determinism check runs
- **THEN** the row is marked FAIL with both `baseRevision` values captured in the report

#### Scenario: JSON parse failure

- **GIVEN** `archctl diagram export` produces invalid JSON
- **WHEN** the orchestrator validates the output via `diagram validate`
- **THEN** the row is marked FAIL, and raw bytes are preserved in the report appendix

### Requirement: Report

The harness MUST emit `bench/reports/<YYYY-MM-DD>.md` with a markdown table per
dataset. Each row SHALL contain: name, language, exit code, wall time median (ms),
peak RSS (MB), valid (yes/no), deterministic (yes/no), and notes.

#### Scenario: Report generated for all datasets

- **GIVEN** 10 datasets are listed in `datasets.toml`
- **WHEN** `bench/run-bench.sh` completes
- **THEN** `bench/reports/2026-08-05.md` contains a markdown table with exactly 10 rows

#### Scenario: Report for mixed results

- **GIVEN** 5 of 10 datasets pass, 3 timeout, 2 produce invalid JSON
- **WHEN** the harness completes
- **THEN** the report contains 10 rows: 5 PASS, 3 FAIL (timeout), 2 FAIL (invalid JSON)

### Requirement: Threshold Gate

The harness MUST exit non-zero (code 1) if any dataset fails the success criteria
defined in `bench-methodology`. The harness SHALL exit zero only when all thresholds pass.

#### Scenario: All datasets pass thresholds

- **GIVEN** 10 datasets, all meet methodology thresholds
- **WHEN** `bench/run-bench.sh` runs
- **THEN** the harness exits 0

#### Scenario: One dataset fails threshold

- **GIVEN** dataset `clap-rs/clap` exceeds 30s wall time threshold
- **WHEN** `bench/run-bench.sh` runs
- **THEN** the harness exits 1
