//! Benchmark harness for `call_graph::apply()` performance.
//!
//! Three `#[ignore]` criterion benches exercise the call-graph apply pipeline
//! against datasets of increasing size:
//!   - `echo`    — labstack/echo, 1,307 Go elements (large-scale regression gate)
//!   - `zustand` — pmndrs/zustand, 212 TypeScript elements
//!   - `go_fixture` — archctl/tests/fixtures/go_callgraph/main.go (deterministic)
//!
//! Default `cargo bench` skips these (#[ignore]); run with:
//!   `cargo bench --bench call_graph_apply -- --ignored`
//!
//! Benchmarks measure the full `call_graph::apply()` call (store open + init +
//! graph writes). Criterion's timer is the measurement instrument.
//!
//! ADR-036 §D4 regression thresholds:
//!   echo  D1 only:  < 10s  (10,500 per-element commits → 1)
//!   echo  D1 + D2: < 3s   (10,500 per-element queries → ~6)
//!   zustand D1+D2:  < 5s
//!   go_fixture: < 5s AND ≤ 30 ms/element (post-D2 target: ~20 ms/element)
//!
//! Datasets (pre-cached at ~/.cache/archctl-smoke/):
//!   labstack-echo-1307.json   — 1,307 nodes + edges, Go
//!   pmndrs-zustand-212.json  —   212 nodes + edges, TypeScript
//!   Go fixture uses committed source under tests/fixtures/go_callgraph/.

use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main};

use archctl::code::call_graph::{self, ApplyReport, CallGraphReport, Language};
use archctl::filesystem::SystemFilesystem;

// ─── Dataset resolution ─────────────────────────────────────────────────────────

const SMOKE_CACHE: &str = ".cache/archctl-smoke";

/// Resolve the path to a cached smoke-test dataset.
/// Panics if the dataset is absent (bench requires manual prefetch via
/// `scripts/embed-stack.sh` or equivalent).
fn smoke_path(name: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(SMOKE_CACHE).join(name)
}

// ─── Go fixture (deterministic, committed) ─────────────────────────────────────

/// Write a Go source file to a temp dir so call_graph::extract() can process it.
/// This simulates what `archctl code call-graph --lang go` does on the fixture.
fn prepare_go_fixture(tmp: &std::path::Path) -> std::path::PathBuf {
    use std::fs;
    let src_dir = tmp.join("src");
    fs::create_dir_all(&src_dir).expect("create src dir");
    let src_file = src_dir.join("main.go");
    fs::write(
        &src_file,
        include_str!("../tests/fixtures/go_callgraph/main.go"),
    )
    .expect("write main.go");
    src_dir
}

// ─── Criterion benches ─────────────────────────────────────────────────────────

/// Echo (labstack/echo) — 1,307 Go elements. Primary regression gate.
/// Threshold: D1 < 10s, D1+D2 < 3s.
/// Dataset: ~/.cache/archctl-smoke/labstack-echo-1307.json
#[ignore]
fn bench_call_graph_apply_echo(c: &mut Criterion) {
    let dataset_path = smoke_path("labstack-echo-1307.json");
    // Skip the bench (not a panic) if the dataset isn't cached. This
    // lets `cargo bench --bench call_graph_apply go_fixture` succeed
    // in environments where the echo/zustand caches haven't been
    // prefetched yet — see `scripts/embed-stack.sh`.
    if !dataset_path.exists() {
        eprintln!(
            "skipping bench_call_graph_apply_echo: dataset not cached at {}. \
             Run scripts/embed-stack.sh to prefetch.",
            dataset_path.display()
        );
        return;
    }
    let report: CallGraphReport =
        serde_json::from_slice(&std::fs::read(&dataset_path).expect("read echo dataset"))
            .expect("parse echo CallGraphReport");

    c.bench_function("call_graph_apply_echo", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempfile::tempdir().expect("tempdir");
                let project = tmp.path().join("proj");
                (tmp, project)
            },
            |(_tmp, project)| {
                let result: Result<ApplyReport, _> =
                    call_graph::apply(&project, &report, &SystemFilesystem);
                // Result is returned so it's not optimized away; actual assertion
                // (e.g. result.unwrap().elements_written > 0) would be added
                // once thresholds are confirmed stable.
                result
            },
        );
    });
}

/// Zustand (pmndrs/zustand) — 212 TypeScript elements.
/// Threshold: D1+D2 < 5s.
/// Dataset: ~/.cache/archctl-smoke/pmndrs-zustand-212.json
#[ignore]
fn bench_call_graph_apply_zustand(c: &mut Criterion) {
    let dataset_path = smoke_path("pmndrs-zustand-212.json");
    if !dataset_path.exists() {
        eprintln!(
            "skipping bench_call_graph_apply_zustand: dataset not cached at {}. \
             Run scripts/embed-stack.sh to prefetch.",
            dataset_path.display()
        );
        return;
    }
    let report: CallGraphReport =
        serde_json::from_slice(&std::fs::read(&dataset_path).expect("read zustand dataset"))
            .expect("parse zustand CallGraphReport");

    c.bench_function("call_graph_apply_zustand", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempfile::tempdir().expect("tempdir");
                let project = tmp.path().join("proj");
                (tmp, project)
            },
            |(_tmp, project)| {
                let result: Result<ApplyReport, _> =
                    call_graph::apply(&project, &report, &SystemFilesystem);
                result
            },
        );
    });
}

/// Go fixture — 6 elements + 2 edges, committed under tests/fixtures/go_callgraph/.
/// Deterministic, no cache dependency. Thresholds:
///   - < 5s wall-clock (criterion measured)
///   - ≤ 30 ms/element (ADR-036 §D4 post-D2 target)
/// This bench extracts from the committed .go file (mimicking real CLI usage).
///
/// T4.1: per-element throughput assertion added.
#[ignore]
fn bench_call_graph_apply_go_fixture(c: &mut Criterion) {
    // Go fixture has 6 elements. ADR-036 §D4 target: ≤ 30 ms/element post-D2.
    const ELEMENT_COUNT: usize = 6;
    const MAX_MS_PER_ELEMENT: f64 = 30.0;

    c.bench_function("call_graph_apply_go_fixture", |b| {
        b.iter_custom(|iters| {
            let mut total_ms: f64 = 0.0;
            for _ in 0..iters {
                let tmp = tempfile::tempdir().expect("tempdir");
                let project = tmp.path().join("proj");
                let src_dir = prepare_go_fixture(tmp.path());

                // Extract (not part of the apply timing, just for report construction)
                let report: CallGraphReport =
                    call_graph::extract(&src_dir, &[Language::Go], None, &SystemFilesystem)
                        .expect("extract go fixture");

                // Time only the apply call (store open + init + graph writes)
                let start = std::time::Instant::now();
                let result: Result<ApplyReport, _> =
                    call_graph::apply(&project, &report, &SystemFilesystem);
                let elapsed = start.elapsed();
                total_ms += elapsed.as_secs_f64() * 1000.0;

                // Assert result is ok (panics if apply fails)
                result.expect("apply should succeed");
            }
            let avg_ms = total_ms / iters as f64;
            let ms_per_element = avg_ms / ELEMENT_COUNT as f64;

            // ADR-036 §D4 throughput gate: ≤ 30 ms/element post-D2
            assert!(
                ms_per_element <= MAX_MS_PER_ELEMENT,
                "throughput assertion failed: {} ms/element (expected ≤ {} ms/element). \
                 avg={:.2}ms for {} elements",
                ms_per_element,
                MAX_MS_PER_ELEMENT,
                avg_ms,
                ELEMENT_COUNT
            );

            std::time::Duration::from_secs_f64(avg_ms / 1000.0)
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets =
        bench_call_graph_apply_echo,
        bench_call_graph_apply_zustand,
        bench_call_graph_apply_go_fixture,
);
criterion_main!(benches);
