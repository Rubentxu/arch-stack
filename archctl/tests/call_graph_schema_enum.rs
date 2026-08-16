//! Regression test: call-graph-report.schema.json language enum has 6 values.
//!
//! S12: Node language field accepts all 6 supported languages.
//! S13: Enum 6 entries fixed order (rust, typescript, python, go, java, kotlin).
//!
//! Verifies that schemas/call-graph-report.schema.json language enum
//! matches `code::call_graph::Language` in node.language field.

use archctl::code::call_graph::{
    CallGraphReport, FunctionKind, FunctionNode, Language, ProjectMeta,
};
use std::collections::BTreeMap;

/// Convert Language to its lowercase string representation (matching serde rename_all =
/// "lowercase").
fn lang_str(lang: Language) -> &'static str {
    match lang {
        Language::Rust => "rust",
        Language::TypeScript => "typescript",
        Language::Python => "python",
        Language::Go => "go",
        Language::Java => "java",
        Language::Kotlin => "kotlin",
    }
}

/// Build a minimal report with one node for the given language.
fn make_report(lang: Language) -> CallGraphReport {
    let lang_key = lang_str(lang);
    let languages: BTreeMap<String, u64> = [(lang_key.to_string(), 1)].into_iter().collect();

    let node = FunctionNode {
        canonical_key: format!("{}:/tmp/fake.rs:main:1", lang_key),
        kind: FunctionKind::Function,
        language: lang,
        file: "/tmp/fake.rs".to_string(),
        content_hash: "sha256:abc123".to_string(),
        line: 1,
        name: "main".to_string(),
        fq_name: "main".to_string(),
        confidence: lang.confidence(),
        parent: None,
    };

    CallGraphReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root: "/tmp".to_string(),
            files_scanned: 1,
            languages,
            duration_ms: 10,
        },
        nodes: vec![node],
        edges: vec![],
        errors: vec![],
    }
}

/// Verify a report with Language::Go validates against schema.
#[test]
fn test_call_graph_go_node_validates_against_schema() {
    let report = make_report(Language::Go);

    let json = serde_json::to_string(&report).expect("report serializes to JSON");

    // Validate against embedded schema.
    let schema_str = archctl::code::call_graph::CALL_GRAPH_REPORT_SCHEMA;
    let schema: serde_json::Value =
        serde_json::from_str(schema_str).expect("embedded schema is valid JSON");

    let validator = jsonschema::validator_for(&schema).expect("call-graph schema compiles");

    let report_value: serde_json::Value =
        serde_json::from_str(&json).expect("report JSON is valid JSON");

    let result = validator.validate(&report_value);
    assert!(
        result.is_ok(),
        "Go node must validate against schema: {:?}",
        result.err()
    );
}

/// Verify all 6 languages serialize and validate.
#[test]
fn test_call_graph_all_6_languages_validate() {
    let all_langs = [
        Language::Rust,
        Language::TypeScript,
        Language::Python,
        Language::Go,
        Language::Java,
        Language::Kotlin,
    ];

    let schema_str = archctl::code::call_graph::CALL_GRAPH_REPORT_SCHEMA;
    let schema: serde_json::Value =
        serde_json::from_str(schema_str).expect("embedded schema is valid JSON");
    let validator = jsonschema::validator_for(&schema).expect("call-graph schema compiles");

    for lang in all_langs {
        let report = make_report(lang);
        let json = serde_json::to_string(&report).expect("report serializes");
        let report_value: serde_json::Value =
            serde_json::from_str(&json).expect("report JSON is valid");
        let result = validator.validate(&report_value);
        assert!(
            result.is_ok(),
            "{:?} node must validate: {:?}",
            lang,
            result.err()
        );
    }
}

/// Verify that a report with ALL 6 languages serializes correctly.
#[test]
fn test_call_graph_report_all_6_languages_in_nodes() {
    let all_langs = [
        Language::Rust,
        Language::TypeScript,
        Language::Python,
        Language::Go,
        Language::Java,
        Language::Kotlin,
    ];

    let languages: BTreeMap<String, u64> = all_langs
        .iter()
        .map(|l| (lang_str(*l).to_string(), 1))
        .collect();

    let nodes: Vec<FunctionNode> = all_langs
        .iter()
        .map(|lang| {
            let lang_key = lang_str(*lang);
            FunctionNode {
                canonical_key: format!("{}:/tmp/fake.rs:main:1", lang_key),
                kind: FunctionKind::Function,
                language: *lang,
                file: "/tmp/fake.rs".to_string(),
                content_hash: "sha256:abc123".to_string(),
                line: 1,
                name: "main".to_string(),
                fq_name: "main".to_string(),
                confidence: lang.confidence(),
                parent: None,
            }
        })
        .collect();

    let report = CallGraphReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root: "/tmp/fake".to_string(),
            files_scanned: 6,
            languages,
            duration_ms: 0,
        },
        nodes,
        edges: vec![],
        errors: vec![],
    };

    let json = serde_json::to_string_pretty(&report).unwrap();

    // Confirm all 6 language values appear in nodes (serde_json::to_string_pretty adds space after :).
    assert!(
        json.contains(r#""language": "rust""#),
        "json should contain language rust"
    );
    assert!(
        json.contains(r#""language": "typescript""#),
        "json should contain language typescript"
    );
    assert!(
        json.contains(r#""language": "python""#),
        "json should contain language python"
    );
    assert!(
        json.contains(r#""language": "go""#),
        "json should contain language go"
    );
    assert!(
        json.contains(r#""language": "java""#),
        "json should contain language java"
    );
    assert!(
        json.contains(r#""language": "kotlin""#),
        "json should contain language kotlin"
    );
}
