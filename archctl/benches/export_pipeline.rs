//! Bench harness for the export pipeline.
//!
//! Each bench function builds a fresh `LbugStore` from a canonical dataset
//! fixture (see `benches/common/mod.rs` + `benchmarks/datasets/`), then
//! measures the full bundle export path under criterion's harness.
//!
//! Run with: `cargo bench --bench export_pipeline`
//! Quick smoke: `cargo bench --bench export_pipeline -- --quick`

use criterion::{criterion_group, criterion_main, Criterion};

mod common;
use common::{seed_medium, seed_small};

use archctl::clock::FixedClock;
use archctl::diagram::export::run_export;
use archctl::filesystem::MemoryFilesystem;

fn bench_export_small(c: &mut Criterion) {
    c.bench_function("export_small", |b| {
        b.iter(|| {
            let (store, _tmp) = seed_small();
            let clock = FixedClock::new("2026-08-01T00:00:00Z");
            let fs = MemoryFilesystem::new();
            let out = std::path::PathBuf::from("/out");
            let report = run_export(&store, "container:*", &out, &clock, &fs).unwrap();
            criterion::black_box(report);
        });
    });
}

fn bench_export_medium(c: &mut Criterion) {
    c.bench_function("export_medium", |b| {
        b.iter(|| {
            let (store, _tmp) = seed_medium();
            let clock = FixedClock::new("2026-08-01T00:00:00Z");
            let fs = MemoryFilesystem::new();
            let out = std::path::PathBuf::from("/out");
            let report = run_export(&store, "container:*", &out, &clock, &fs).unwrap();
            criterion::black_box(report);
        });
    });
}

criterion_group!(benches, bench_export_small, bench_export_medium);
criterion_main!(benches);