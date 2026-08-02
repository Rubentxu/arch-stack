//! Bench harness for the read/query pipeline.
//!
//! Each bench function builds a fresh `LbugStore` from a canonical dataset
//! fixture, then measures Cypher query throughput under criterion's harness:
//! - count() over the Elements node label (small)
//! - semantic-edge traversal (medium)
//! - evidence path filter (large)
//!
//! The two enabled benches use `iter_batched(NumIterations(10))` so the
//! `seed_*` cost is amortized 10× per batch. Closes audit M5
//! follow-up (seed-cost decomposition).
//!
//! Run with: `cargo bench --bench query_pipeline`
//! Quick smoke: `cargo bench --bench query_pipeline -- --quick`

use criterion::{Criterion, criterion_group, criterion_main};

mod common;
use common::{seed_medium, seed_small};

use archctl::store::GraphStore;

fn bench_query_count_small(c: &mut Criterion) {
    c.bench_function("query_count_elements_small", |b| {
        b.iter_batched(
            seed_small,
            |(store, _tmp)| {
                let rows = store
                    .query("MATCH (e:Element) RETURN count(e) AS n;")
                    .expect("count query");
                criterion::black_box(rows);
            },
            criterion::BatchSize::NumIterations(10),
        );
    });
}

fn bench_query_semantic_edges_medium(c: &mut Criterion) {
    c.bench_function("query_semantic_edges_medium", |b| {
        b.iter_batched(
            seed_medium,
            |(store, _tmp)| {
                let rows = store
                    .query(
                        "MATCH (a:Element)-[r:SEMANTIC_EDGE]->(b:Element) \
                         RETURN a.id AS src_id, r.relation_id AS rel_id, b.id AS tgt_id LIMIT 1000;",
                    )
                    .expect("semantic edges query");
                criterion::black_box(rows);
            },
            criterion::BatchSize::NumIterations(10),
        );
    });
}

#[allow(dead_code)]
fn bench_query_evidence_filter_large(c: &mut Criterion) {
    // Evidence filter on a 10k-node store takes ~30s per iter due to
    // the bulk seed cost — criterion's default 100-sample budget blows
    // past the 60s smoke-test ceiling. Disable by default; enable
    // explicitly with `cargo bench --bench query_pipeline -- query_evidence_filter_large`.
    let mut group = c.benchmark_group("query_evidence_filter_large");
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(60));
    group.bench_function("query_evidence_filter_large", |b| {
        b.iter(|| {
            let (store, _tmp) = common::seed_large();
            let rows = store
                .query("MATCH (e:Element) WHERE e.category = 'container' RETURN count(e);")
                .expect("evidence filter query");
            criterion::black_box(rows);
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_query_count_small,
    bench_query_semantic_edges_medium,
);
criterion_main!(benches);
