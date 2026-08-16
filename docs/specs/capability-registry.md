# Capability Registry Specification

> **Change**: `p1-08-capability-registry` · **Branch**: `feat/p1-08-capability-registry` @ `7ca1373` · **Status**: Active · **Domain**: `capability`

## Purpose

Replaces nine drift sources (`README.md`, `MANUAL.md`, `cli::SUPPORTED_LANGUAGES`, `schemas/call-graph-report.schema.json`, `STATE.md`, `ROADMAP.md`, `docs/specs/index.md`, ADRs) with one introspectable registry that emits a versioned JSON, a deterministic Markdown table, and an alignment gate.

## Requirements

### Requirement: Registry Model

The registry SHALL expose `archctl::capability::Capability` carriers. Each entry SHALL carry `id`, `maturity` (`stable`|`beta`|`experimental`|`proposed`), `deterministic` (bool), `requirements[]`, `availability` (`available`|`opt_in`|`experimental`), `providers[]`. Providers SHALL carry `language` (lowercase) and per-provider `maturity`. IDs SHALL match `<domain>.<kind>` (e.g. `code.call_graph`).

#### Scenario: Stable ID format

- **GIVEN** the registry enumerates `code::call_graph`
- **WHEN** `archctl capabilities --json` runs
- **THEN** the entry's `id` equals `"code.call_graph"`
- **AND** `providers[]` lists `rust, typescript, python, go, java, kotlin` (6)

#### Scenario: Provider maturity downgrade keeps the entry

- **GIVEN** `code::call_graph` ships `kotlin` at `maturity: beta`
- **WHEN** the source enumerates providers
- **THEN** the entry remains; only the provider `maturity` changes
- **AND** the alignment test passes

### Requirement: CLI Surface

`archctl capabilities` SHALL default to `--json`. JSON SHALL be a single object with top-level `schemaVersion: "1"`. The command SHALL accept `--format markdown` (deterministic, sorted) and `--check` (non-zero on stale `docs/CAPABILITIES.md`). Invalid `--format` SHALL be rejected with a clap usage error.

#### Scenario: Default JSON shape

- **GIVEN** a built `archctl` binary
- **WHEN** the user runs `archctl capabilities`
- **THEN** stdout is valid JSON with `schemaVersion` equal to `"1"`
- **AND** `capabilities[]` length matches the contract (8 categories, 79 entries)

#### Scenario: Markdown is deterministic

- **GIVEN** the registry enumerates N entries
- **WHEN** `archctl capabilities --format markdown` runs twice back-to-back
- **THEN** both invocations produce byte-identical stdout sorted by `id` then `provider.language`

#### Scenario: Invalid --format rejected

- **GIVEN** a user supplies `--format xml`
- **WHEN** the command is dispatched
- **THEN** clap exits non-zero with an invalid-value error for `--format <FORMAT>`

### Requirement: Alignment Invariants

Every strategy, renderer, view kind, doctor scope, IDE adapter, MCP tool, and CLI subcommand declared in code SHALL have a matching registry entry, and vice versa. Violations SHALL fail tests in `archctl/src/capability/alignment.rs`.

#### Scenario: New Language variant without entry fails

- **GIVEN** a developer adds `Language::Ruby` to `code::call_graph::Language`
- **WHEN** the alignment suite runs
- **THEN** it fails naming the missing `code.call_graph` provider `ruby`

#### Scenario: New entry without code support fails

- **GIVEN** `source_code.rs` adds a `code.call_graph` provider `ruby` with no matching `Language::Ruby` variant
- **WHEN** the alignment suite runs
- **THEN** it fails naming the orphan provider `ruby`

#### Scenario: Strategy drift is caught

- **GIVEN** `code::strategies` adds `CargoBinaryStrategy` not in `source_code.rs`
- **WHEN** the strategy-alignment test runs
- **THEN** it fails naming `CargoBinaryStrategy` as the missing entry

### Requirement: Generated Docs

`archctl capabilities --format markdown` SHALL produce output checked into the repo at `docs/CAPABILITIES.md`. `scripts/verify-local.sh` cheap mode SHALL diff on-disk vs fresh; `scripts/test-ci-gates.sh` SHALL mirror the check (per `dep-fitness-baseline` p1-09). Stale docs SHALL cause `--check` to exit 1.

#### Scenario: --check exits zero when fresh

- **GIVEN** `docs/CAPABILITIES.md` equals `archctl capabilities --format markdown` output
- **WHEN** `archctl capabilities --check` runs
- **THEN** exit code is 0 and stderr is empty

#### Scenario: --check exits non-zero on stale docs

- **GIVEN** `docs/CAPABILITIES.md` has a hand-added extra row
- **WHEN** `archctl capabilities --check` runs
- **THEN** exit code is 1 and stderr reports the diff line numbers

#### Scenario: verify-local.sh gate wired

- **GIVEN** `scripts/verify-local.sh` cheap mode runs
- **WHEN** it reaches the capabilities step
- **THEN** it executes `archctl capabilities --check` and propagates the exit code

### Requirement: Call-Graph Schema Fix

`schemas/call-graph-report.schema.json` `nodes.items.properties.language.enum` SHALL list exactly the 6 languages supported by `code::call_graph::Language`: `rust`, `typescript`, `python`, `go`, `java`, `kotlin` (lowercase).

#### Scenario: Go report validates against the schema

- **GIVEN** a Go call-graph report with a node `language: "go"`
- **WHEN** it is validated against `schemas/call-graph-report.schema.json`
- **THEN** validation succeeds; same holds for `java` and `kotlin`

#### Scenario: Enum has 6 entries in fixed order

- **GIVEN** the fixed schema file
- **WHEN** the `language` enum array is read
- **THEN** it contains `"rust", "typescript", "python", "go", "java", "kotlin"` in that order

### Requirement: SUPPORTED_LANGUAGES Removal

`archctl::cli::SUPPORTED_LANGUAGES` SHALL be removed. CLI help SHALL NOT advertise stale language lists. No consumer SHALL remain (verified by `rg "SUPPORTED_LANGUAGES" archctl/src` returning zero matches).

#### Scenario: Constant no longer in cli.rs

- **GIVEN** the change is applied
- **WHEN** `rg "SUPPORTED_LANGUAGES" archctl/src` runs
- **THEN** exit code is 1 (no matches)

#### Scenario: --help does not list languages

- **GIVEN** `archctl capabilities --help` runs
- **WHEN** clap renders help text
- **THEN** no line enumerates a hand-maintained language list

### Requirement: MCP Tools as Metadata

The registry SHALL list each `cognitive::mcp::gateway::ALLOWED_TOOLS` entry as `mcp.tool.<name>`. The runtime `ALLOWED_TOOLS` SHALL remain untouched and SHALL continue to gate executable MCP surface.

#### Scenario: MCP tools enumerated in registry

- **GIVEN** `ALLOWED_TOOLS = ["graph_query", "schema_validate", "run_tests_local"]`
- **WHEN** `archctl capabilities --json` runs
- **THEN** JSON contains exactly 3 entries: `mcp.tool.graph_query`, `mcp.tool.schema_validate`, `mcp.tool.run_tests_local`
- **AND** `cognitive/mcp/gateway.rs` is unchanged

### Requirement: Namespaced Capability

The new registry type SHALL live at `archctl::capability::Capability`. The pre-existing `archctl::cognitive::descriptor::Capability` SHALL keep its slot and SHALL NOT be renamed.

#### Scenario: Both Capability types coexist

- **GIVEN** both modules compile
- **WHEN** `cargo doc --no-deps` runs
- **THEN** doc index lists `capability::Capability` and `cognitive::descriptor::Capability` as distinct items
- **AND** neither is a re-export of the other

## Non-Goals

Runtime MCP gating from the registry (ADR-021 follow-up); plugin enumeration beyond `plugin.loadpoint` placeholder; regenerating `docs/STATE.md`, `docs/ROADMAP.md`, or `docs/specs/index.md` content (tracked separately).

## Cross-references

ADR-045 (promoted); ADR-004 (XDG persistence); ADR-011 (local-only renderers); ADR-021 (MCP boundary); ADR-022 (cognitive layer); ADR-042 (IDE adapter abstraction).
