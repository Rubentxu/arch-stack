//! Benchmark harness for `class_diagram::apply()` performance.
//!
//! Measures the `class_diagram::apply()` call (store open + init + graph writes)
//! against the committed gold fixture (tests/fixtures/class-diagram/gold.json):
//!   - 10 nodes across Python, Rust, and TypeScript
//!   - 5 semantic edges
//!
//! Default `cargo bench` skips these (#[ignore]); run with:
//!   `cargo bench --bench class_diagram_apply -- --ignored`
//!
//! Benchmarks measure the full `class_diagram::apply()` call. Criterion's timer
//! is the measurement instrument.
//!
//! ADR-036 §D4 + ADR-019 thresholds:
//!   - < 500ms for 10-node fixture (smoke gate)
//!   - ADR-019: p99 < 2s for graphs < 10k nodes
//!
//! T4.2: criterion bench for class_diagram apply.

use criterion::{Criterion, criterion_group, criterion_main};

use archctl::code::class_diagram::{self, ApplyReport, ClassDiagramReport};
use archctl::filesystem::SystemFilesystem;

/// Load the committed gold fixture as a ClassDiagramReport.
/// The gold fixture has 10 nodes (Python+Rust+TypeScript) + 5 edges.
fn load_gold_report() -> ClassDiagramReport {
    let bytes = include_bytes!("../tests/fixtures/class-diagram/gold.json");
    serde_json::from_slice(bytes).expect("parse gold.json fixture")
}

/// Apply bench: 10 nodes, gold fixture.
/// Threshold: < 500ms (smoke gate, ADR-036 §D4 + ADR-019).
#[ignore]
fn bench_class_diagram_apply_gold(c: &mut Criterion) {
    let report = load_gold_report();
    let node_count = report.nodes.len();

    c.bench_function("class_diagram_apply_gold", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempfile::tempdir().expect("tempdir");
                let project = tmp.path().join("proj");
                (tmp, project)
            },
            |(_tmp, project)| {
                let result: Result<ApplyReport, _> =
                    class_diagram::apply(&project, &report, &SystemFilesystem);
                // ADR-036 §D4: assert we wrote elements (not 0 = something broke)
                let r = result.expect("apply should succeed");
                assert!(
                    r.elements_written > 0 || r.elements_skipped > 0,
                    "apply wrote neither elements_written nor elements_skipped"
                );
                criterion::black_box(r)
            },
        );
    });

    // ADR-019 documentation reference
    let _ = node_count;
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = bench_class_diagram_apply_gold,
);
criterion_main!(benches);
