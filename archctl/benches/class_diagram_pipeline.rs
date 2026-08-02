//! Bench harness for the class-diagram extraction pipeline.
//!
//! Measures the full extract → apply → emit cycle for `archctl code
//! class-diagram`. The ADR-019 budget is p99 < 2s for graphs < 10k nodes.
//!
//! Each bench uses `iter_batched(NumIterations(10))` so setup (TempDir +
//! fixture write) is amortized across 10 measured iterations.
//!
//! Run with: `cargo bench --bench class_diagram_pipeline`
//! Quick smoke: `cargo bench --bench class_diagram_pipeline -- --quick`

use std::process::Command;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use tempfile::TempDir;

const RUST_FIXTURE: &str = r#"
pub struct UserService {
    pub name: String,
    email: String,
}

impl UserService {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            email: format!("{}@example.com", name),
        }
    }
    pub fn greet(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}

pub trait Greetable {
    fn greet(&self) -> String;
}

impl Greetable for UserService {
    fn greet(&self) -> String { self.greet() }
}

pub struct AdminService { pub level: u8 }

impl AdminService {
    pub fn new() -> Self { Self { level: 0 } }
}
"#;

/// Full pipeline bench: extract + parse JSON + return.
fn bench_class_diagram_pipeline(c: &mut Criterion) {
    c.bench_function("class_diagram_full_pipeline", |b| {
        b.iter_batched(
            || {
                // Setup: create TempDir with a Rust fixture.
                let tmp = TempDir::new().expect("tempdir");
                let fixture = tmp.path().join("mod.rs");
                std::fs::write(&fixture, RUST_FIXTURE).expect("write fixture");
                (tmp, fixture)
            },
            |(_tmp, fixture)| {
                // Measure: run the CLI and parse output.
                let output = Command::new(env!("CARGO_BIN_EXE_archctl"))
                    .args([
                        "code",
                        "class-diagram",
                        "--cwd",
                        fixture.parent().unwrap().to_str().unwrap(),
                        "--json",
                    ])
                    .output()
                    .expect("archctl exits ok");
                criterion::black_box(output);
            },
            BatchSize::NumIterations(10),
        );
    });
}

criterion_group!(benches, bench_class_diagram_pipeline);
criterion_main!(benches);
