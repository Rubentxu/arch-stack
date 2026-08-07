# ADR-037 — Call-graph language strategy consolidation

## Status

accepted (2026-08-07)

## Cycle

`m34-call-graph-strategy-consolidation`

## Context

M30 added Go extraction (function + method) to the call-graph engine, growing the
number of `extract_*_function` bodies from 6 to 8. The M30 debt-report flagged
D2: the 8 extractors for Rust (function + closure), TypeScript (function +
method + arrow), Python (function), and Go (function + method) had near-identical
bodies — differ only in child_kind string, FunctionKind variant, confidence
constant, and parent_key use. The debt-report estimated ~240 LOC reducible.

The same cycle also flagged D3 (inline `GO_SAMPLE` string), W3
(`#[allow(dead_code)]` on `InvalidLanguage`), W4 (vacuous confidence test), and
D4/D5/D6 (scattered metadata: 9+ confidence constants, 3 duplicated help strings,
duplicate `write_call_edge` comment).

## Decision

### D2 — Single helper function

Replace the 8 extractor bodies with a single `extract_function(node, source, lang,
file, child_kind, kind, confidence, parent_key)` helper. The 8 wrappers become
1–3-line forwards. The helper handles two sub-groups:

| Group | Extractors | Mechanism |
|---|---|---|
| Name-extract (6) | rust-fn, ts-fn, python-fn, go-fn, go-method, ts-method | Look up `child_kind` child → name |
| Synthetic-name (2) | rust-closure, ts-arrow | `child_kind = None` → generate `"closure@<line>"` / `"arrow@<line>"` |

`extract_python_function` has an additional `is_method: bool` parameter folded in
via a thin wrapper that calls the helper with the appropriate `FunctionKind`.

Reserve a `LanguageStrategy` trait or dispatch table for when a 5th language
lands. Adding the abstraction now would be premature: 4 languages × 8 extractors
is not enough for a strategy pattern.

### D3 — Fixture single source of truth

Delete `const GO_SAMPLE` (inline copy of the Go fixture). Unit and smoke tests
both read `tests/fixtures/go_callgraph/main.go` via a shared `read_fixture`
helper. Divergence risk eliminated.

### W3 — Dead-code annotation

Annotate `CallGraphError::InvalidLanguage` with `#[allow(dead_code)]` and a
comment explaining why: clap's `value_enum` guard rejects invalid values before
the variant is ever constructed. The variant is preserved for spec scenario 11
wording and future post-parse validation.

### W4 — Real regression gate

Replace the tautology test `let go_conf = 0.85; assert_eq!(go_conf, 0.85)` with
a test that creates a TempDir Go fixture, parses it with tree-sitter, calls
`extract_go_method`, and asserts `node.confidence == 0.85`. The test fails if the
confidence constant changes.

### D4 — Language::confidence() method

Add `Language::confidence(&self) -> f64` returning 0.90 / 0.85 / 0.80 / 0.85 for
Rust / TypeScript / Python / Go. Replace 9+ magic numbers in extractors and
`make_call_edge` with `lang.confidence()`.

### D5 — Deduplicated help strings

Add `pub const SUPPORTED_LANGUAGES: &str` at module level in `cli.rs`. All 3 CLI
help strings use the identical literal.

### D6 — Duplicate comment removed

Remove the duplicate `version_id` comment block from `write_call_edge`. Keep the
copy at lines 1193–1198.

## Consequences

- **~240 LOC reduction** in `call_graph.rs` (8 × ~30-line bodies → 1 helper +
  8 wrappers).
- **Behavioral equivalence**: 525+ tests pass unchanged (verified by
  characterization tests added before the refactor).
- **Adding a 5th language** is now mostly mechanical: 1 wrapper function + 1
  entry in `Language::confidence()`.
- **Confidence constants centralized** — one place to change, all sites update.
- **No public API changes** — the refactor touches only private helper functions.

## Alternatives considered

### LanguageStrategy trait

More idiomatic OOP. Rejected: 4 languages × 8 extractors is not enough for a
strategy pattern. The helper is simpler and sufficient.

### Static dispatch table

Declarative approach using closures in a map. Rejected: Rust's static dispatch
with closures is awkward; the helper function is clearer.

### Keep the duplication

Rejected: M30 debt-report D2 explicitly flagged the duplication as tech debt.
The ~240 LOC reduction and reduced per-language maintenance cost justify the
refactor.

## References

- M34 cycle: `sddk/m34-call-graph-strategy-consolidation/`
- M30 debt-report: D2, D3, W3, W4, D4/D5/D6
- ADR-035: Go Call-Graph Extraction (M30)
