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
//! Thresholds (documented, not enforced at runtime):
//!   echo  D1 only:  < 10s  (10,500 per-element commits → 1)
//!   echo  D1 + D2: < 3s   (10,500 per-element queries → ~6)
//!   zustand D1+D2:  < 5s
//!   go_fixture:      < 5s
//!
//! Datasets (pre-cached at ~/.cache/archctl-smoke/):
//!   labstack-echo-1307.json   — 1,307 nodes + edges, Go
//!   pmndrs-zustand-212.json  —   212 nodes + edges, TypeScript
//! The Go fixture uses committed source under tests/fixtures/go_callgraph/.

use std::collections::BTreeMap;
use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main};

use archctl::code::call_graph::{
    self, ApplyReport, CallEdge, CallGraphReport, CallKind, FunctionKind, FunctionNode, Language,
    MessageKind,
};
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

/// Minimal Go fixture report mirroring tests/fixtures/go_callgraph/main.go.
/// 6 functions + 2 call edges; committed, no cache needed.
/// (Currently unused — the bench extracts fresh from the committed
/// .go file via `prepare_go_fixture` + `call_graph::extract`. Kept
/// here for future inline benchmarks that skip the extract step.)
#[allow(dead_code)]
fn go_fixture_report() -> CallGraphReport {
    CallGraphReport {
        schema_version: "1.0".to_string(),
        project: call_graph::ProjectMeta {
            root: "/tmp/go_fixture".to_string(),
            files_scanned: 1,
            languages: BTreeMap::from([("go".to_string(), 1)]),
            duration_ms: 0,
        },
        nodes: vec![
            FunctionNode {
                canonical_key: "go:/tmp/go_fixture/main.go:greet:10".to_string(),
                kind: FunctionKind::Function,
                language: Language::Go,
                file: "main.go".to_string(),
                content_hash:
                    "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        .to_string(),
                line: 10,
                name: "greet".to_string(),
                fq_name: "main.greet".to_string(),
                confidence: 0.90,
                parent: None,
            },
            FunctionNode {
                canonical_key: "go:/tmp/go_fixture/main.go:Server.Save:17".to_string(),
                kind: FunctionKind::Method,
                language: Language::Go,
                file: "main.go".to_string(),
                content_hash:
                    "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        .to_string(),
                line: 17,
                name: "Save".to_string(),
                fq_name: "Server.Save".to_string(),
                confidence: 0.90,
                parent: None,
            },
            FunctionNode {
                canonical_key: "go:/tmp/go_fixture/main.go:Server.Name:21".to_string(),
                kind: FunctionKind::Method,
                language: Language::Go,
                file: "main.go".to_string(),
                content_hash:
                    "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        .to_string(),
                line: 21,
                name: "Name".to_string(),
                fq_name: "Server.Name".to_string(),
                confidence: 0.90,
                parent: None,
            },
            FunctionNode {
                canonical_key: "go:/tmp/go_fixture/main.go:handler:25".to_string(),
                kind: FunctionKind::Function,
                language: Language::Go,
                file: "main.go".to_string(),
                content_hash:
                    "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        .to_string(),
                line: 25,
                name: "handler".to_string(),
                fq_name: "main.handler".to_string(),
                confidence: 0.90,
                parent: None,
            },
            FunctionNode {
                canonical_key: "go:/tmp/go_fixture/main.go:main:32".to_string(),
                kind: FunctionKind::Function,
                language: Language::Go,
                file: "main.go".to_string(),
                content_hash:
                    "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        .to_string(),
                line: 32,
                name: "main".to_string(),
                fq_name: "main.main".to_string(),
                confidence: 0.90,
                parent: None,
            },
            FunctionNode {
                canonical_key: "go:/tmp/go_fixture/main.go:init:38".to_string(),
                kind: FunctionKind::Function,
                language: Language::Go,
                file: "main.go".to_string(),
                content_hash:
                    "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        .to_string(),
                line: 38,
                name: "init".to_string(),
                fq_name: "main.init".to_string(),
                confidence: 0.90,
                parent: None,
            },
        ],
        edges: vec![
            CallEdge {
                canonical_key: "go:/tmp/go_fixture/main.go:main→fmt.Println:34".to_string(),
                caller: "go:/tmp/go_fixture/main.go:main:32".to_string(),
                callee: "fmt.Println".to_string(),
                file: "main.go".to_string(),
                content_hash:
                    "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        .to_string(),
                line: 34,
                kind: CallKind::DirectCall,
                message_kind: MessageKind::SyncCall,
                confidence: 0.90,
            },
            CallEdge {
                canonical_key: "go:/tmp/go_fixture/main.go:greet→fmt.Println:11".to_string(),
                caller: "go:/tmp/go_fixture/main.go:greet:10".to_string(),
                callee: "fmt.Println".to_string(),
                file: "main.go".to_string(),
                content_hash:
                    "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                        .to_string(),
                line: 11,
                kind: CallKind::DirectCall,
                message_kind: MessageKind::SyncCall,
                confidence: 0.90,
            },
        ],
        errors: vec![],
    }
}

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
/// Deterministic, no cache dependency. Threshold: < 5s.
/// This bench extracts from the committed .go file (mimicking real CLI usage).
#[ignore]
fn bench_call_graph_apply_go_fixture(c: &mut Criterion) {
    c.bench_function("call_graph_apply_go_fixture", |b| {
        b.iter_with_setup(
            || {
                let tmp = tempfile::tempdir().expect("tempdir");
                let project = tmp.path().join("proj");
                let src_dir = prepare_go_fixture(tmp.path());
                (tmp, project, src_dir)
            },
            |(_tmp, project, src_dir)| {
                // Extract from the committed Go fixture (simulates archctl code call-graph --lang go)
                let report: CallGraphReport =
                    call_graph::extract(&src_dir, &[Language::Go], None, &SystemFilesystem)
                        .expect("extract go fixture");
                let result: Result<ApplyReport, _> =
                    call_graph::apply(&project, &report, &SystemFilesystem);
                result
            },
        );
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
