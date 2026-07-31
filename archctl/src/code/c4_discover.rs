//! C4 Container boundary inference engine + report types.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::code::strategies::Strategy;
use crate::filesystem::Filesystem;

/// JSON Schema for DiscoverReport (JSON Schema 2020-12).
pub const DISCOVER_REPORT_SCHEMA: &str =
    include_str!("../../../schemas/discover-report.schema.json");

/// One Container detected by one strategy. Multiple ContainerCandidate
/// rows with the same canonical_key are merged into one Container.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContainerCandidate {
    pub canonical_key: String,
    pub name: String,
    pub strategy: String,
    pub confidence: f64,
    pub evidences: Vec<Evidence>,
}

/// One piece of evidence supporting a Container detection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Evidence {
    /// Relative path from project root
    pub file: String,
    /// 1-based line number
    pub line: u32,
    pub kind: EvidenceKind,
    pub text: String,
}

/// Mirrors evidence::EvidenceKind (kept separate to avoid coupling
/// `code` to `evidence` module — discover is a producer, not a
/// consumer of the B1-lifecycle).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum EvidenceKind {
    Structural,
    Config,
    Annotation,
    Lexical,
    Other,
}

impl From<EvidenceKind> for crate::evidence::EvidenceKind {
    fn from(kind: EvidenceKind) -> Self {
        match kind {
            EvidenceKind::Structural => crate::evidence::EvidenceKind::Structural,
            EvidenceKind::Config => crate::evidence::EvidenceKind::Config,
            EvidenceKind::Annotation => crate::evidence::EvidenceKind::Annotation,
            EvidenceKind::Lexical => crate::evidence::EvidenceKind::Lexical,
            EvidenceKind::Other => crate::evidence::EvidenceKind::Other,
        }
    }
}

/// Final Container after cross-strategy merge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Container {
    pub canonical_key: String,
    pub name: String,
    pub strategy: String,
    pub confidence: f64,
    #[serde(rename = "mergedFrom")]
    pub merged_from: Vec<String>,
    pub evidences: Vec<Evidence>,
}

/// Per-project metadata about the discover run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectMeta {
    pub root: String,
    #[serde(rename = "filesScanned")]
    pub files_scanned: u64,
    pub languages: BTreeMap<String, u64>,
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
}

/// One error captured during discovery (graceful degradation).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoverError {
    pub strategy: String,
    pub path: String,
    pub message: String,
}

/// Final report emitted by `archctl code c4 discover`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiscoverReport {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    pub project: ProjectMeta,
    pub discovered: Vec<Container>,
    pub errors: Vec<DiscoverError>,
}

/// Run all strategies, merge cross-strategy candidates, return the
/// report. Pure: no I/O beyond filesystem walk + language detection
/// via inventory::tree().
pub fn discover(
    project_root: &Path,
    strategies: &[Box<dyn Strategy>],
    fs: &dyn Filesystem,
    _clock: &dyn crate::clock::Clock,
) -> Result<DiscoverReport> {
    let start = Instant::now();

    // Walk the project tree once for inventory metadata.
    let tree = crate::inventory::tree(project_root, Some(8), 50_000)
        .context("walk project tree")?;
    let files_scanned = tree.len() as u64;
    let mut languages: BTreeMap<String, u64> = BTreeMap::new();
    for entry in &tree {
        if let Some(lang) = &entry.language {
            *languages.entry(lang.clone()).or_insert(0) += 1;
        }
    }

    // Run each strategy. Capture errors but continue (SCN-103).
    let mut all_candidates: Vec<ContainerCandidate> = Vec::new();
    let mut errors: Vec<DiscoverError> = Vec::new();
    for strategy in strategies {
        match strategy.detect(project_root, fs) {
            Ok(mut candidates) => all_candidates.append(&mut candidates),
            Err(e) => errors.push(DiscoverError {
                strategy: strategy.id().to_string(),
                path: project_root.display().to_string(),
                message: e.to_string(),
            }),
        }
    }

    // Cross-strategy merge: group by canonical_key, take highest-confidence
    // strategy, union evidences. Deterministic ordering: sort by canonical_key
    // (BTreeMap guarantees).
    let mut by_key: BTreeMap<String, Container> = BTreeMap::new();
    for candidate in all_candidates {
        let canonical_key = candidate.canonical_key.clone();
        let strategy_id = candidate.strategy.clone();
        let confidence = candidate.confidence;
        let name = candidate.name.clone();
        let candidate_evidences = candidate.evidences;

        if let Some(existing) = by_key.get_mut(&canonical_key) {
            // Merge into existing container
            if confidence > existing.confidence {
                existing.strategy = strategy_id.clone();
                existing.confidence = confidence;
                existing.name = name.clone();
            }
            if !existing.merged_from.contains(&strategy_id) {
                existing.merged_from.push(strategy_id.clone());
            }
            let existing_files: Vec<String> = existing.evidences
                .iter()
                .map(|e| e.file.clone())
                .collect();
            for ev in candidate_evidences {
                if !existing_files.contains(&ev.file) {
                    existing.evidences.push(ev);
                }
            }
        } else {
            // Insert new container
            by_key.insert(canonical_key.clone(), Container {
                canonical_key,
                name,
                strategy: strategy_id.clone(),
                confidence,
                merged_from: vec![strategy_id],
                evidences: candidate_evidences,
            });
        }
    }

    // Deterministic ordering by canonical_key.
    let discovered: Vec<Container> = by_key.into_values().collect();

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(DiscoverReport {
        schema_version: "1.0".to_string(),
        project: ProjectMeta {
            root: project_root.display().to_string(),
            files_scanned,
            languages,
            duration_ms,
        },
        discovered,
        errors,
    })
}

/// Persist a DiscoverReport to the graph. One Element per Container
/// (with ElementVersion.props.inferred=true), one Evidence per evidence
/// (status="Drafted"), one SourceArtifact per unique file.
/// Idempotent: skips canonical_keys that already exist.
pub fn apply(
    project_dir: &Path,
    report: &DiscoverReport,
    _fs: &dyn Filesystem,
) -> Result<ApplyReport> {
    use crate::store::open_default;

    let mut store = open_default(project_dir)
        .map_err(|e| anyhow::anyhow!("failed to acquire DB lock: {e}"))?;
    store.init().context("graph init (c4 discover apply)")?;

    // Seed mt.container MetaType if it doesn't exist
    let seed_metatype = r#"
        MERGE (mt:MetaType {id: 'mt.container'})
        SET mt.namespace = 'c4', mt.name = 'container', mt.category = 'structure'
        RETURN mt.id;
    "#;
    store.query(seed_metatype).ok();

    let mut elements_written = 0usize;
    let mut elements_skipped = 0usize;
    let mut evidences_written = 0usize;
    let mut source_artifacts_written = 0usize;

    // Build a quick lookup: existing canonical_keys → skip
    let existing_keys: std::collections::HashSet<String> = store
        .query("MATCH (e:Element) WHERE e.canonical_key IS NOT NULL RETURN e.canonical_key;")?
        .into_iter()
        .filter_map(|row| {
            row.get("e.canonical_key")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect();

    for container in &report.discovered {
        if existing_keys.contains(&container.canonical_key) {
            elements_skipped += 1;
            continue;
        }

        let element_id = format!("c4:container:{}", container.canonical_key);
        let version_props = serde_json::json!({
            "inferred": true,
            "strategy": container.strategy,
            "confidence": container.confidence,
            "merged_from": container.merged_from,
            "discovery_schema_version": "1.0",
        });
        let version_props_str = serde_json::to_string(&version_props).unwrap_or_default();
        let version_props_escaped = version_props_str.replace('\'', "\\'");
        let version_id =
            format!("blake3:{}", blake3::hash(version_props_str.as_bytes()).to_hex());
        let name_escaped = container.name.replace('\'', "\\'");

        // Element: no 'props' column (that's on ElementVersion)
        let element_cypher = format!(
            "MERGE (e:Element {{id: '{element_id}'}}) SET \
             e.kind_id = 'mt.container', \
             e.category = 'c4', \
             e.canonical_key = '{canonical_key}', \
             e.current_name = '{name_escaped}', \
             e.current_status = 'active', \
             e.current_confidence = {confidence}, \
             e.current_version_id = '{version_id}';",
            element_id = element_id,
            canonical_key = container.canonical_key.replace('\'', "\\'"),
            name_escaped = name_escaped,
            confidence = container.confidence,
            version_id = version_id,
        );
        store.query(&element_cypher).context("put_element")?;
        elements_written += 1;

        // ElementVersion: holds the props with inferred data
        let version_cypher = format!(
            "MERGE (v:ElementVersion {{id: '{version_id}'}}) SET \
             v.element_id = '{element_id}', \
             v.name = '{name_escaped}', \
             v.status = 'drafted', \
             v.origin = 'c4-discover', \
             v.confidence = {confidence}, \
             v.props = '{version_props_escaped}';",
            version_id = version_id,
            element_id = element_id,
            name_escaped = name_escaped,
            confidence = container.confidence,
            version_props_escaped = version_props_escaped,
        );
        store.query(&version_cypher).ok();

        // CURRENT_VERSION edge: Element → ElementVersion
        let current_version_cypher = format!(
            "MATCH (e:Element {{id: '{element_id}'}}), (v:ElementVersion {{id: '{version_id}'}}) \
             MERGE (e)-[:CURRENT_VERSION]->(v);"
        );
        store.query(&current_version_cypher).ok();

        // VERSION_OF edge: ElementVersion → Element
        let version_of_cypher = format!(
            "MATCH (v:ElementVersion {{id: '{version_id}'}}), (e:Element {{id: '{element_id}'}}) \
             MERGE (v)-[:VERSION_OF]->(e);"
        );
        store.query(&version_of_cypher).ok();

        // OF_TYPE edge: Element → MetaType
        let of_type_cypher = format!(
            "MATCH (e:Element {{id: '{element_id}'}}), (mt:MetaType {{id: 'mt.container'}}) \
             MERGE (e)-[:OF_TYPE]->(mt);"
        );
        store.query(&of_type_cypher).ok();

        // Build SourceArtifact + Evidence rows
        let mut source_artifact_ids: BTreeMap<String, String> = BTreeMap::new();
        for evidence in &container.evidences {
            // SourceArtifact (dedup by file path)
            let sa_id = if let Some(id) = source_artifact_ids.get(&evidence.file) {
                id.clone()
            } else {
                let id = format!("src:{}", blake3::hash(evidence.file.as_bytes()).to_hex());
                let kind_escaped = "manifest".replace('\'', "\\'");
                let path_escaped = evidence.file.replace('\'', "\\'");
                let sa_cypher = format!(
                    "MERGE (s:SourceArtifact {{id: '{id}'}}) SET \
                     s.kind = '{kind_escaped}', \
                     s.relative_path = '{path_escaped}', \
                     s.language = '', \
                     s.content_hash = '', \
                     s.generated = false, \
                     s.props = '{{}}';"
                );
                store.query(&sa_cypher).ok();
                source_artifact_ids.insert(evidence.file.clone(), id.clone());
                source_artifacts_written += 1;
                id
            };

            // Evidence (status Drafted per B1-lifecycle D3)
            let evidence_id = format!(
                "ev:{}",
                blake3::hash(
                    format!("{}:{}:{}", element_id, evidence.file, evidence.line).as_bytes()
                )
                .to_hex()
            );
            let evidence_props = serde_json::json!({
                "file_refs": [format!("{}:{}", evidence.file, evidence.line)],
                "text": evidence.text,
                "status": "Drafted",
            });
            let ev_props_json = serde_json::to_string(&evidence_props).unwrap_or_default();
            let ev_props_escaped = ev_props_json.replace('\'', "\\'");
            let evidence_text_escaped = evidence.text.replace('\'', "\\'");
            let evidence_cypher = format!(
                "MERGE (ev:Evidence {{id: '{evidence_id}'}}) SET \
                 ev.kind = '{kind}', \
                 ev.claim = '{text}', \
                 ev.path = '{file}', \
                 ev.start_line = {line}, \
                 ev.end_line = {line}, \
                 ev.tool_name = 'archctl', \
                 ev.tool_version = env!(\"CARGO_PKG_VERSION\"), \
                 ev.rule_id = 'c4-discover:{strategy}', \
                 ev.language = '', \
                 ev.observed_at = '', \
                 ev.props = '{ev_props_escaped}', \
                 ev.status = 'Drafted';",
                evidence_id = evidence_id,
                kind = match evidence.kind {
                    EvidenceKind::Structural => "structural",
                    EvidenceKind::Config => "config",
                    EvidenceKind::Annotation => "annotation",
                    EvidenceKind::Lexical => "lexical",
                    EvidenceKind::Other => "other",
                },
                text = evidence_text_escaped,
                file = evidence.file.replace('\'', "\\'"),
                line = evidence.line,
                strategy = container.strategy.replace('\'', "\\'"),
                ev_props_escaped = ev_props_escaped,
            );
            store.query(&evidence_cypher).ok();
            evidences_written += 1;

            // SUPPORTED_BY edge: ElementVersion → Evidence
            let link_ev_cypher = format!(
                "MATCH (v:ElementVersion {{id: '{version_id}'}}), (ev:Evidence {{id: '{evidence_id}'}}) \
                 MERGE (v)-[:SUPPORTED_BY]->(ev);"
            );
            store.query(&link_ev_cypher).ok();

            // EXTRACTED_FROM edge: Evidence → SourceArtifact
            let link_cypher = format!(
                "MATCH (ev:Evidence {{id: '{evidence_id}'}}), (s:SourceArtifact {{id: '{sa_id}'}}) \
                 MERGE (ev)-[:EXTRACTED_FROM]->(s);"
            );
            store.query(&link_cypher).ok();
        }
    }

    Ok(ApplyReport {
        elements_written,
        elements_skipped,
        evidences_written,
        source_artifacts_written,
    })
}

/// Report from a successful `--apply` run.
#[derive(Debug, Clone, Serialize)]
pub struct ApplyReport {
    pub elements_written: usize,
    pub elements_skipped: usize,
    pub evidences_written: usize,
    pub source_artifacts_written: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::MemoryFilesystem;

    fn dummy_strategy(id: &'static str, confidence: f64) -> Box<dyn Strategy> {
        Box::new(MockStrategy { id, confidence })
    }

    struct MockStrategy {
        id: &'static str,
        confidence: f64,
    }

    impl Strategy for MockStrategy {
        fn id(&self) -> &'static str { self.id }
        fn confidence(&self) -> f64 { self.confidence }
        fn detect(&self, _project_root: &Path, _fs: &dyn Filesystem) -> Result<Vec<ContainerCandidate>> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn merge_two_strategies_same_canonical_key() {
        // SCN-140: two strategies infer the same canonical_key
        let fs = MemoryFilesystem::new();
        let clock: &dyn crate::clock::Clock = &crate::clock::FixedClock::new("2025-01-01T00:00:00Z");
        let tmp = tempfile::tempdir().unwrap();

        let candidates: Vec<ContainerCandidate> = vec![
            ContainerCandidate {
                canonical_key: "auth-svc".to_string(),
                name: "auth-svc".to_string(),
                strategy: "cargo-workspace".to_string(),
                confidence: 0.85,
                evidences: vec![Evidence {
                    file: "Cargo.toml".to_string(),
                    line: 8,
                    kind: EvidenceKind::Structural,
                    text: "Cargo workspace member: auth-svc".to_string(),
                }],
            },
            ContainerCandidate {
                canonical_key: "auth-svc".to_string(),
                name: "auth-svc".to_string(),
                strategy: "dockerfile".to_string(),
                confidence: 0.60,
                evidences: vec![Evidence {
                    file: "auth-svc/Dockerfile".to_string(),
                    line: 1,
                    kind: EvidenceKind::Structural,
                    text: "Dockerfile for service: auth-svc".to_string(),
                }],
            },
        ];

        #[derive(Clone)]
        struct InjectStrategy { candidates: Vec<ContainerCandidate> }
        impl Strategy for InjectStrategy {
            fn id(&self) -> &'static str { "inject" }
            fn confidence(&self) -> f64 { 1.0 }
            fn detect(&self, _: &Path, _: &dyn Filesystem) -> Result<Vec<ContainerCandidate>> {
                Ok(self.candidates.clone())
            }
        }

        let strategies: Vec<Box<dyn Strategy>> = vec![
            Box::new(InjectStrategy { candidates }),
        ];

        let report = discover(tmp.path(), &strategies, &fs, clock).unwrap();
        assert_eq!(report.discovered.len(), 1);
        let c = &report.discovered[0];
        assert_eq!(c.canonical_key, "auth-svc");
        // Highest confidence wins
        assert_eq!(c.strategy, "cargo-workspace");
        assert_eq!(c.confidence, 0.85);
        // Both strategies recorded
        assert!(c.merged_from.contains(&"cargo-workspace".to_string()));
        assert!(c.merged_from.contains(&"dockerfile".to_string()));
        // Union of evidences (2 different files)
        assert_eq!(c.evidences.len(), 2);
    }

    #[test]
    fn merge_preserves_evidences_from_both() {
        let fs = MemoryFilesystem::new();
        let clock: &dyn crate::clock::Clock = &crate::clock::FixedClock::new("2025-01-01T00:00:00Z");
        let tmp = tempfile::tempdir().unwrap();

        let candidates = vec![
            ContainerCandidate {
                canonical_key: "svc".to_string(),
                name: "svc".to_string(),
                strategy: "s1".to_string(),
                confidence: 0.90,
                evidences: vec![Evidence {
                    file: "file1.txt".to_string(),
                    line: 1,
                    kind: EvidenceKind::Structural,
                    text: "evidence 1".to_string(),
                }],
            },
            ContainerCandidate {
                canonical_key: "svc".to_string(),
                name: "svc".to_string(),
                strategy: "s2".to_string(),
                confidence: 0.80,
                evidences: vec![Evidence {
                    file: "file2.txt".to_string(),
                    line: 2,
                    kind: EvidenceKind::Config,
                    text: "evidence 2".to_string(),
                }],
            },
        ];

        // Inject candidates via a custom strategy
        #[derive(Clone)]
        struct InjectStrategy { candidates: Vec<ContainerCandidate> }
        impl Strategy for InjectStrategy {
            fn id(&self) -> &'static str { "inject" }
            fn confidence(&self) -> f64 { 1.0 }
            fn detect(&self, _: &Path, _: &dyn Filesystem) -> Result<Vec<ContainerCandidate>> {
                Ok(self.candidates.clone())
            }
        }

        let strategies: Vec<Box<dyn Strategy>> = vec![
            Box::new(InjectStrategy { candidates }),
            dummy_strategy("s2", 0.80),
        ];

        let report = discover(tmp.path(), &strategies, &fs, clock).unwrap();
        let c = &report.discovered[0];
        assert_eq!(c.evidences.len(), 2);
        let files: Vec<_> = c.evidences.iter().map(|e| e.file.clone()).collect();
        assert!(files.contains(&"file1.txt".to_string()));
        assert!(files.contains(&"file2.txt".to_string()));
    }

    #[test]
    fn merge_orders_by_canonical_key_deterministically() {
        let fs = MemoryFilesystem::new();
        let clock: &dyn crate::clock::Clock = &crate::clock::FixedClock::new("2025-01-01T00:00:00Z");
        let tmp = tempfile::tempdir().unwrap();

        #[derive(Clone)]
        struct InjectStrategy { keys: Vec<String> }
        impl Strategy for InjectStrategy {
            fn id(&self) -> &'static str { "inject" }
            fn confidence(&self) -> f64 { 1.0 }
            fn detect(&self, _: &Path, _: &dyn Filesystem) -> Result<Vec<ContainerCandidate>> {
                Ok(self.keys.iter().map(|k| ContainerCandidate {
                    canonical_key: k.clone(),
                    name: k.clone(),
                    strategy: "test".to_string(),
                    confidence: 1.0,
                    evidences: vec![],
                }).collect())
            }
        }

        let strategies: Vec<Box<dyn Strategy>> = vec![
            Box::new(InjectStrategy { keys: vec!["zebra".to_string(), "apple".to_string(), "mango".to_string()] }),
        ];

        let report = discover(tmp.path(), &strategies, &fs, clock).unwrap();
        let keys: Vec<_> = report.discovered.iter().map(|c| c.canonical_key.clone()).collect();
        assert_eq!(keys, vec!["apple", "mango", "zebra"]);
    }

    #[test]
    fn no_strategies_returns_empty_report() {
        let fs = MemoryFilesystem::new();
        let clock: &dyn crate::clock::Clock = &crate::clock::FixedClock::new("2025-01-01T00:00:00Z");
        let tmp = tempfile::tempdir().unwrap();
        let strategies: Vec<Box<dyn Strategy>> = vec![];
        let report = discover(tmp.path(), &strategies, &fs, clock).unwrap();
        assert!(report.discovered.is_empty());
        assert!(report.errors.is_empty());
    }

    #[test]
    fn schema_is_valid_json() {
        let parsed: serde_json::Value =
            serde_json::from_str(DISCOVER_REPORT_SCHEMA).expect("DISCOVER_REPORT_SCHEMA must be valid JSON");
        assert_eq!(parsed["$schema"].as_str().unwrap(), "https://json-schema.org/draft/2020-12/schema");
        assert_eq!(parsed["type"].as_str().unwrap(), "object");
        assert_eq!(parsed["properties"]["schemaVersion"]["const"].as_str().unwrap(), "1.0");
    }

    #[test]
    fn schema_validates_valid_report() {
        use serde_json::json;
        let jsonschema = json!(
            serde_json::from_str::<serde_json::Value>(DISCOVER_REPORT_SCHEMA)
                .expect("schema is valid JSON")
        );
        let report = json!({
            "schemaVersion": "1.0",
            "project": {
                "root": "/tmp/test",
                "filesScanned": 42,
                "languages": {"rust": 30, "typescript": 12},
                "durationMs": 150
            },
            "discovered": [{
                "canonical_key": "auth-svc",
                "name": "auth-svc",
                "strategy": "cargo-workspace",
                "confidence": 0.85,
                "mergedFrom": ["cargo-workspace"],
                "evidences": [{
                    "file": "Cargo.toml",
                    "line": 8,
                    "kind": "structural",
                    "text": "Cargo workspace member: auth-svc"
                }]
            }],
            "errors": []
        });
        let validator = jsonschema::validator_for(&jsonschema)
            .expect("schema must be valid JSON Schema");
        let result = validator.validate(&report);
        assert!(result.is_ok(), "valid report must pass schema: {:?}", result.err());
    }

    #[test]
    fn apply_writes_element_with_inferred_props() {
        use crate::store::LbugStore;

        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path();

        let report = DiscoverReport {
            schema_version: "1.0".to_string(),
            project: ProjectMeta {
                root: project.display().to_string(),
                files_scanned: 10,
                languages: BTreeMap::new(),
                duration_ms: 50,
            },
            discovered: vec![Container {
                canonical_key: "test-svc".to_string(),
                name: "test-svc".to_string(),
                strategy: "cargo-workspace".to_string(),
                confidence: 0.85,
                merged_from: vec!["cargo-workspace".to_string()],
                evidences: vec![Evidence {
                    file: "Cargo.toml".to_string(),
                    line: 5,
                    kind: EvidenceKind::Structural,
                    text: "Cargo workspace member".to_string(),
                }],
            }],
            errors: vec![],
        };

        let fs = MemoryFilesystem::new();
        let result = apply(project, &report, &fs);
        assert!(result.is_ok(), "apply must succeed: {:?}", result.err());
        let r = result.unwrap();
        assert_eq!(r.elements_written, 1, "should write exactly one element");
        assert_eq!(r.elements_skipped, 0);
        assert!(r.evidences_written >= 1);
        assert!(r.source_artifacts_written >= 1);
    }

    #[test]
    fn apply_is_idempotent_skips_existing_canonical_key() {
        use crate::store::LbugStore;

        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path();

        let report = DiscoverReport {
            schema_version: "1.0".to_string(),
            project: ProjectMeta {
                root: project.display().to_string(),
                files_scanned: 10,
                languages: BTreeMap::new(),
                duration_ms: 50,
            },
            discovered: vec![Container {
                canonical_key: "dup-svc".to_string(),
                name: "dup-svc".to_string(),
                strategy: "cargo-workspace".to_string(),
                confidence: 0.85,
                merged_from: vec!["cargo-workspace".to_string()],
                evidences: vec![Evidence {
                    file: "Cargo.toml".to_string(),
                    line: 5,
                    kind: EvidenceKind::Structural,
                    text: "Cargo workspace member".to_string(),
                }],
            }],
            errors: vec![],
        };

        let fs = MemoryFilesystem::new();

        // First apply — writes the element
        let r1 = apply(project, &report, &fs).unwrap();
        assert_eq!(r1.elements_written, 1);

        // Second apply — skips the existing canonical_key
        let r2 = apply(project, &report, &fs).unwrap();
        assert_eq!(r2.elements_skipped, 1, "second apply must skip existing canonical_key");
        assert_eq!(r2.elements_written, 0, "second apply must not write duplicates");
    }
}
