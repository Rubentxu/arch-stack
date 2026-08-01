//! Bench harness for the read/query pipeline.
//!
//! Each bench function builds a fresh `LbugStore` from a canonical dataset
//! fixture, then measures Cypher query throughput under criterion's harness:
//! - count() over the Elements node label (small)
//! - semantic-edge traversal (medium)
//! - evidence path filter (large)
//!
//! Run with: `cargo bench --bench query_pipeline`
//! Quick smoke: `cargo bench --bench query_pipeline -- --quick`

use criterion::{criterion_group, criterion_main, Criterion};

mod common;
use common::{seed_large, seed_medium, seed_small};

use archctl::store::GraphStore;

fn bench_query_count_small(c: &mut Criterion) {
    c.bench_function("query_count_elements_small", |b| {
        b.iter(|| {
            let (store, _tmp) = seed_small();
            let rows = store
                .query("MATCH (e:Element) RETURN count(e) AS n;")
                .expect("count query");
            criterion::black_box(rows);
        });
    });
}

fn bench_query_semantic_edges_medium(c: &mut Criterion) {
    c.bench_function("query_semantic_edges_medium", |b| {
        b.iter(|| {
            let (store, _tmp) = seed_medium();
            let rows = store
                .query(
                    "MATCH (a:Element)-[r:SemanticRelation]->(b:Element) \
                     RETURN a.id, r.id, b.id LIMIT 1000;",
                )
                .expect("semantic edges query");
            criterion::black_box(rows);
        });
    });
}

fn bench_query_evidence_filter_large(c: &mut Criterion) {
    c.bench_function("query_evidence_filter_large", |b| {
        b.iter(|| {
            let (store, _tmp) = seed_large();
            // Pre-seed a few Evidence rows so the filter has something to find.
            for i in 0..50 {
                let path = format!("src/lib{i}.rs");
                let path = archctl::graph::validate_identifier(&path).expect("path");
                let _ = store.query(&format!(
                    "CREATE (:Evidence {{id: 'ev:bench:{i}', \
                     kind: 'structural', classification: 'derived', \
                     claim: 'bench claim', confidence: 0.9, path: '{path}', \
                     start_line: 1, end_line: 10, commit_hash: '', \
                     content_hash: 'sha256:bench', \
                     tool_name: 'archctl', tool_version: '0.1.0', \
                     rule_id: 'bench:rule', \
                     props: '{{\"status\":\"accepted\"}}'}});"
                ));
            }
            let rows = store
                .query(
                    "MATCH (e:Evidence) WHERE e.path STARTS WITH 'src/lib' \
                     RETURN e.id, e.path LIMIT 100;",
                )
                .expect("evidence filter query");
            criterion::black_box(rows);
        });
    });
}

criterion_group!(
    benches,
    bench_query_count_small,
    bench_query_semantic_edges_medium,
    bench_query_evidence_filter_large
);
criterion_main!(benches);