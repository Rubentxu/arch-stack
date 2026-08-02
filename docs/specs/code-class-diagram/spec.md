# `code-class-diagram` Specification

## Purpose

`archctl code class-diagram` MUST produce a local, deterministic class projection for supported source files.

## Requirements

### Requirement: Class-diagram extraction

The command MUST extract declared types, methods, and explicit relationships.

#### Scenario: Rust struct
- GIVEN `a.rs` defines `struct A` and `impl A { fn f() {} }`
- WHEN executed
- THEN node `A` lists method `f`

#### Scenario: TypeScript inheritance
- GIVEN one file defines `B` and `class A extends B`
- WHEN executed
- THEN two nodes and edge `A extends B` are emitted

#### Scenario: Python multiple inheritance
- GIVEN one file defines `A`, `B`, and `class C(A, B)`
- WHEN executed
- THEN three nodes and two `extends` edges are emitted

### Requirement: Selector resolution

The command MUST accept `file:<path>`, `module:<id>`, or no selector.

#### Scenario: File selector
- GIVEN `file:src/a.rs`
- WHEN executed
- THEN only `src/a.rs` contributes output

#### Scenario: Module selector
- GIVEN `module:billing` resolves to several files
- WHEN executed
- THEN only billing files contribute output

#### Scenario: Default selector
- GIVEN no selector
- WHEN executed
- THEN all supported repository files are processed

#### Scenario: Unknown selector
- GIVEN malformed selector `unknown:value`
- WHEN executed
- THEN it exits 64 with a message containing `unknown selector`

#### Scenario: Empty selection
- GIVEN an empty repo or valid selector with no supported matches
- WHEN executed
- THEN it exits 0 with empty nodes and edges

### Requirement: Projection bundle emission

The command MUST emit a versioned, bounded projection.

#### Scenario: Versioned artifact
- GIVEN successful extraction
- WHEN a bundle is written
- THEN `class-diagram.v1.json` has `schemaVersion: "1.0"`

#### Scenario: Stable node IDs
- GIVEN two nodes share the same `file:line:kind:name` tuple
- WHEN they are projected in separate runs
- THEN their IDs are identical
- AND changing any tuple component changes the ID

#### Scenario: Edge vocabulary
- GIVEN explicit inheritance, implementation, or same-file typed ownership
- WHEN projected
- THEN edge kinds are only `extends`, `implements`, or `composes`

#### Scenario: Bundle bound
- GIVEN a projection below 10,000 nodes
- WHEN its bundle is written
- THEN total bundle size is below 1 MB

#### Scenario: JSON stdout
- GIVEN `--json`
- WHEN executed
- THEN projection is written to stdout and no bundle is created

### Requirement: Intra-file scope

The command MUST resolve relationships only between types declared in one file.

#### Scenario: Cross-file inheritance
- GIVEN child and parent are declared in different files
- WHEN projected
- THEN no inheritance edge joins them

#### Scenario: Same-file composition
- GIVEN `A` has a field typed `B`
- WHEN projected
- THEN `A composes B` is emitted only if `B` shares the file

#### Scenario: Cyclic references
- GIVEN same-file types `A` and `B` reference each other
- WHEN projected
- THEN it terminates with exactly two `composes` edges

### Requirement: Error handling

The command MUST reject invalid selectors and SHOULD isolate file failures.

#### Scenario: Missing selected file
- GIVEN `file:missing.rs` does not exist
- WHEN executed
- THEN it exits 64 with a message containing `file not found`

#### Scenario: Parse failure
- GIVEN one malformed and one valid supported file
- WHEN executed
- THEN the malformed file is warned and skipped
- AND the valid file is projected

#### Scenario: Unsupported extension
- GIVEN a selected `.go` file
- WHEN executed
- THEN it is warned and skipped without failing the run

### Requirement: Graph application

With `--apply`, the command MUST persist evidenced results idempotently.

#### Scenario: Idempotent application
- GIVEN a fresh graph and valid projection
- WHEN executed with `--apply` twice
- THEN corresponding `uml.*` elements and relations have resolvable evidence
- AND element and relation counts remain unchanged

### Requirement: Determinism

Equivalent runs MUST produce byte-identical projections.

#### Scenario: Repeated projection
- GIVEN unchanged input and options
- WHEN projected twice
- THEN `class-diagram.v1.json` bytes are identical

### Requirement: Performance budget

The command SHALL satisfy ADR-019 for graphs below 10,000 nodes.

#### Scenario: Export latency
- GIVEN the representative benchmark fixture
- WHEN export p99 is measured
- THEN latency is below 2 seconds
