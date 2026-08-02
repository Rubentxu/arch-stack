//! Bench harness for the export pipeline.
//!
//! Each bench function builds a fresh `LbugStore` from a canonical dataset
//! fixture, then measures the read-path components of `archctl diagram
//! export`:
//! - `query_elements` — filtered Element + ElementVersion read
//! - `query_semantic_edges` — relation traversal
//! - `base_revision` — blake3 hash of the projection (cheap, isolates
//!   Cypher from serialization cost)
//!
//! The full `run_export` requires ElementVersion + SUPPORTED_BY edges +
//! Evidence nodes, which would inflate the seed cost beyond what
//! criterion's quick mode allows. The components above are the bulk of
//! the budget-relevant cost.
//!
//! All seeded benches use `iter_batched(NumIterations(10))` so the
//! `seed_*` cost is amortized 10× (setup runs once per batch of 10
//! measured iters instead of once per iter). This closes audit M5:
//! ADR-019 `export p99 <2s for <10k` was inflated by seed cost
//! dominating the measurement loop.
//!
//! Run with: `cargo bench --bench export_pipeline`
//! Quick smoke: `cargo bench --bench export_pipeline -- --quick`

use criterion::{Criterion, criterion_group, criterion_main};

mod common;
use common::{seed_medium, seed_small};

use archctl::diagram::export_types::Projection;
use archctl::diagram::hash::base_revision;
use archctl::diagram::queries::{query_elements, query_semantic_edges};

fn bench_query_elements_small(c: &mut Criterion) {
    c.bench_function("export_query_elements_small", |b| {
        b.iter_batched(
            seed_small,
            |(store, _tmp)| {
                let elements = query_elements(&store, "container", None).expect("query_elements");
                criterion::black_box(elements);
            },
            criterion::BatchSize::NumIterations(10),
        );
    });
}

fn bench_query_semantic_edges_medium(c: &mut Criterion) {
    c.bench_function("export_query_semantic_edges_medium", |b| {
        b.iter_batched(
            seed_medium,
            |(store, _tmp)| {
                let edges =
                    query_semantic_edges(&store, "container").expect("query_semantic_edges");
                criterion::black_box(edges);
            },
            criterion::BatchSize::NumIterations(10),
        );
    });
}

fn bench_base_revision(c: &mut Criterion) {
    // base_revision is a pure-function blake3 hash over the Projection.
    // Bench it in isolation to measure the serialization+hash cost
    // independent of Cypher.
    c.bench_function("export_base_revision_hash", |b| {
        b.iter(|| {
            // Synthetic projection with 100 nodes.
            let nodes: Vec<archctl::diagram::export_types::Node> = (0..100)
                .map(|i| archctl::diagram::export_types::Node {
                    id: format!("el:{i}"),
                    element_type: "container".into(),
                    name: format!("Service {i}"),
                    description: None,
                    canonical_key: Some(format!("service-{i}")),
                    status: Some("accepted".into()),
                    confidence: Some(0.9),
                    evidence_refs: None,
                })
                .collect();
            let projection = Projection {
                nodes,
                edges: vec![],
            };
            let rev = base_revision(&projection);
            criterion::black_box(rev);
        });
    });
}

criterion_group!(
    benches,
    bench_query_elements_small,
    bench_query_semantic_edges_medium,
    bench_base_revision
);
criterion_main!(benches);
