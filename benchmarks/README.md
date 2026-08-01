# archctl benchmarks

This directory holds the **archctl-side performance harness** that
validates [ADR-019 (Performance budget)](../adr/ADR-019-performance-budget.md)
on the producer side.

The complementary consumer-side harness for `archview` (TTFP, pan/zoom,
layout convergence, memory at 100k nodes) lives in the `archview`
project, not here.

## Layout

```text
benchmarks/
├── README.md                         # this file
├── datasets/
│   ├── small-100.json                # 100 elements, 250 relations (65 KB)
│   ├── medium-1k.json                # 1,000 elements, 2,500 relations (660 KB)
│   └── large-10k.json                # 10,000 elements, 25,000 relations (6.6 MB)
└── scripts/  (lives at scripts/generate_bench_datasets.py)
```

The bench harness itself lives at `archctl/benches/`:

```text
archctl/benches/
├── common/mod.rs                     # seed_small / seed_medium / seed_large
├── export_pipeline.rs                # read-path benches
├── apply_pipeline.rs                 # write-path benches
└── query_pipeline.rs                 # raw Cypher benches
```

## Datasets

Three deterministic JSON fixtures matching the canonical schema shape
that `archctl graph query` returns for `MATCH (e:Element) ...`.

| Fixture | Nodes | Relations | Use |
|---|---:|---:|---|
| `small-100.json` | 100 | 250 | sanity baseline (sub-second export target) |
| `medium-1k.json` | 1,000 | 2,500 | typical repo C4 model |
| `large-10k.json` | 10,000 | 25,000 | ADR-019 export p99 <2s target |

Generation is deterministic via Python's `random.seed(0xC0DE0001)`.
Re-running `scripts/generate_bench_datasets.py` produces byte-identical
fixtures, so bench measurements are reproducible across machines and
PRs.

## Running

```bash
# Run all benches (full sample count — slow, ~minutes per large bench)
cargo bench

# Smoke mode — 10 samples per bench, 1s measurement window
cargo bench -- --quick

# Run a specific bench binary
cargo bench --bench export_pipeline
cargo bench --bench apply_pipeline
cargo bench --bench query_pipeline

# Run a specific bench function
cargo bench --bench query_pipeline -- query_count_elements_small

# The large benches (gated to a separate benchmark_group) need an
# explicit filter to avoid the quick-mode timeout:
cargo bench --bench apply_pipeline -- apply_chained_commands_large
cargo bench --bench query_pipeline -- query_evidence_filter_large
```

## Baseline measurements (mid-range dev machine, --quick mode)

| Bench | Time | Notes |
|---|---|---|
| `export_query_elements_small` | ~380 ms | 100 nodes, `MATCH (e:Element) WHERE category = ...` |
| `export_query_semantic_edges_medium` | ~2.8 s | 1k nodes, 2500 relations, semantic-edge traversal |
| `export_base_revision_hash` | ~570 µs | pure-function blake3 over 100-node Projection |
| `query_count_elements_small` | ~360 ms | 100 nodes, `MATCH (e:Element) RETURN count(*)` |
| `query_semantic_edges_medium` | ~2.8 s | 1k nodes, 2500 relations, raw MATCH traversal |
| `apply_set_label_small` | ~370 ms | 100 nodes, single SetLabel command via `update_view_member_label` |
| `apply_move_member_medium` | ~2.9 s | 1k nodes, single MoveMember command (requires Element seed for `link_renders`) |

The 1k-node medium benches clock ~3s because each iteration is
**dominated by the seed cost** (bulk Cypher inserts via `MATCH ... CREATE`
on the SEMANTIC_EDGE REL TABLE), not by the actual query/apply. The
medium bench is therefore a **seed-cost benchmark** more than a
query-cost benchmark. Future cycles should split seed cost out of
the measurement loop (e.g. seed once in `c.bench_function`'s setup
phase if criterion supports it, or use a one-time setup pattern).

## ADR-019 budget mapping

| ADR-019 metric | Archctl-side bench | Target |
|---|---|---|
| export p99 <2s for <10k nodes | `export_pipeline` (full bundle) | not yet — see "Follow-ups" |
| cold start <100ms | (not yet benched) | follow-up |
| RSS idle <50MB | (not yet benched) | follow-up |
| Bundle export <1MB for C4 standard | (not yet benched) | follow-up |

The current bench harness covers the **sub-operations** that the export
pipeline composes (query_elements, query_semantic_edges, base_revision).
The full `run_export` pipeline is not benched because it requires a
fully-seeded store (ElementVersion + SUPPORTED_BY + Evidence rows);
the seed cost dominates the measurement.

## Follow-ups (out of cycle scope)

- **Seed-cost decomposition**: split seed from measurement in each
  bench function so the medium/large results reflect operation cost,
  not fixture-construction cost.
- **Full `run_export` bench**: pre-seed a complete store (including
  ElementVersion, Evidence, SUPPORTED_BY edges) so the export
  pipeline can be measured end-to-end.
- **Cold-start bench**: measure time from `cargo run archctl --version`
  to first output byte.
- **RSS measurement**: capture peak memory via `/usr/bin/time -v` or
  `dhat` crate during a 10k-node `query_elements` run.
- **CI gate**: per ADR-019 §"Performance budget enforcement", a CI
  workflow should block PRs with >10% regression on any metric. This
  is a future cycle (CI workflows are out of repo scope per
  `AGENTS.md` rules around generated/external files).

## Regenerating fixtures

```bash
python3 scripts/generate_bench_datasets.py [--out benchmarks/datasets]
```

Output is byte-identical across runs (deterministic seed). To force a
different dataset shape, edit the `SIZES` constant in the script and
re-run; the bench harness auto-discovers the file by name.

## Adding a new bench

1. Add a `bench_*` function in the appropriate `archctl/benches/*.rs`.
2. Use `seed_small()`/`seed_medium()`/`seed_large()` for the fixture.
3. Wrap the operation under measurement in `criterion::black_box(...)`.
4. For large fixtures (≥10k nodes), wrap in a separate `benchmark_group`
   with `sample_size(10)` and `measurement_time(Duration::from_secs(120))`
   to avoid blowing the smoke-mode timeout.
5. Add a row to the "Baseline measurements" table above with the
   measured time on the reference machine.
6. Update `manifests/benchmark.toml` `public_symbols` with the new
   function name.