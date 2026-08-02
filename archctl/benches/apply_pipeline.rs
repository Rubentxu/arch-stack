//! Bench harness for the apply pipeline.
//!
//! Each bench function builds a fresh `LbugStore` from a canonical dataset
//! fixture, then measures apply operations under criterion's harness:
//! - single SetLabel command (validates atomic label update path)
//! - single MoveMember command (validates DiagramOps dispatch)
//! - 100-command batch on large dataset (validates sustained throughput)
//!
//! The two enabled benches use `iter_with_setup` to exclude seed cost
//! from timing. The disabled `bench_apply_chained_commands_large`
//! uses `iter_batched(NumIterations(5))` so the bulk 10k-node seed
//! runs once per batch instead of once per iter (closes audit M5
//! seed-cost decomposition).
//!
//! Run with: `cargo bench --bench apply_pipeline`
//! Quick smoke: `cargo bench --bench apply_pipeline -- --quick`

use criterion::{criterion_group, criterion_main, Criterion};

mod common;
use common::{seed_large, seed_medium, seed_small};

use archctl::diagram::changeset_types::Command;
use archctl::diagram::view_types::Diagram;
use archctl::store::DiagramOps;

fn bench_apply_set_label_small(c: &mut Criterion) {
    c.bench_function("apply_set_label_small", |b| {
        // `iter_with_setup` runs the closure ONCE per batch, then
        // measures the apply path many times. Without this, the
        // 1k-node seed (~2.8s) dominates the apply cost (~370ms).
        b.iter_with_setup(
            || seed_small(),
            |(mut store, _tmp)| {
                store
                    .put_diagram(&Diagram {
                        id: "container:test".into(),
                        revision:
                            "blake3:0000000000000000000000000000000000000000000000000000000000000000"
                                .into(),
                        selector: "container:test".into(),
                        props: serde_json::json!({}),
                        created_at: None,
                        updated_at: None,
                    })
                    .unwrap();
                let cmd = Command::SetLabel {
                    member_id: "vm:container:test:el:1".into(),
                    label: "Bench Label".into(),
                };
                let _ = cmd.apply(&mut store, "container:test");
            },
        );
    });
}

fn bench_apply_move_member_medium(c: &mut Criterion) {
    c.bench_function("apply_move_member_medium", |b| {
        b.iter_with_setup(
            || seed_medium(),
            |(mut store, _tmp)| {
                store
                    .put_diagram(&Diagram {
                        id: "container:test".into(),
                        revision:
                            "blake3:0000000000000000000000000000000000000000000000000000000000000000"
                                .into(),
                        selector: "container:test".into(),
                        props: serde_json::json!({}),
                        created_at: None,
                        updated_at: None,
                    })
                    .unwrap();
                let cmd = Command::MoveMember {
                    member_id: "vm:container:test:el:1".into(),
                    element_id: "el:1".into(),
                    x: 240,
                    y: 160,
                };
                let _ = cmd.apply(&mut store, "container:test");
            },
        );
    });
}

#[allow(dead_code)]
fn bench_apply_chained_commands_large(c: &mut Criterion) {
    // 100-command batch on 10k-node store: the bulk Cypher seed takes
    // ~30s, so amortize via iter_batched(NumIterations(5)). Setup runs
    // once per batch of 5 measured iters; routine applies the same 10
    // SetLabel commands against the same store per batch. SetLabel is
    // idempotent on `label` and `updated_at` writes were dropped in
    // v0.9.2 (CP-W2), so re-applying is safe and measures the same
    // hot-path cost.
    let mut group = c.benchmark_group("apply_chained_commands_large");
    group.sample_size(10);
    group.measurement_time(std::time::Duration::from_secs(120));
    group.bench_function("apply_chained_commands_large", |b| {
        b.iter_batched(
            || {
                let (mut store, _tmp) = seed_large();
                store
                    .put_diagram(&Diagram {
                        id: "container:test".into(),
                        revision:
                            "blake3:0000000000000000000000000000000000000000000000000000000000000000"
                                .into(),
                        selector: "container:test".into(),
                        props: serde_json::json!({}),
                        created_at: None,
                        updated_at: None,
                    })
                    .unwrap();
                let commands: Vec<Command> = (1..=10)
                    .map(|i| Command::SetLabel {
                        member_id: format!("vm:container:test:el:{i}"),
                        label: format!("Bench Label {i}"),
                    })
                    .collect();
                (store, commands)
            },
            |(mut store, commands)| {
                for cmd in &commands {
                    let _ = cmd.apply(&mut store, "container:test");
                }
            },
            criterion::BatchSize::NumIterations(5),
        );
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_apply_set_label_small,
    bench_apply_move_member_medium,
);
criterion_main!(benches);
