//! C4 Container boundary inference engine + report types.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::code::strategies::Strategy;
use crate::filesystem::Filesystem;
use crate::store::{EvidenceRepository, GraphStore, LbugStore};

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
    /// `"sha256:<hex>"` of the file content (D2 identity input).
    /// Populated at apply time via the injected Filesystem.
    #[serde(default, rename = "contentHash")]
    pub content_hash: String,
    /// 1-based line number
    pub line: u32,
    pub kind: EvidenceKind,
    pub text: String,
}

/// Mirrors evidence::EvidenceKind (kept separate to avoid coupling
/// `code` to `evidence` module — discover is a producer, not a
/// consumer of the B1-lifecycle).
/// NOTE: uses snake_case to match the schema's lowercase enum values.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Structural,
    Config,
    Annotation,
    Lexical,
    Other,
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
    let tree =
        crate::inventory::tree(project_root, Some(8), 50_000).context("walk project tree")?;
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
            let existing_files: Vec<String> =
                existing.evidences.iter().map(|e| e.file.clone()).collect();
            for ev in candidate_evidences {
                if !existing_files.contains(&ev.file) {
                    existing.evidences.push(ev);
                }
            }
        } else {
            // Insert new container
            by_key.insert(
                canonical_key.clone(),
                Container {
                    canonical_key,
                    name,
                    strategy: strategy_id.clone(),
                    confidence,
                    merged_from: vec![strategy_id],
                    evidences: candidate_evidences,
                },
            );
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

/// Derive a SourceArtifact language label from a manifest file name.
/// Manifests are `Cargo.toml`, `package.json`, `Dockerfile`, `Chart.yaml`,
/// etc. — none of them is a programming language, so the label is the file
/// extension (or a stable slug). Must pass `validate_identifier` (alnum,
/// `. - _ : /`, non-empty).
fn c4_language_label(file: &str) -> &'static str {
    let lower = file.to_ascii_lowercase();
    if lower.ends_with("dockerfile") {
        return "dockerfile";
    }
    match lower.rsplit_once('.') {
        Some((_, ext)) => match ext {
            "toml" => "toml",
            "json" => "json",
            "yaml" | "yml" => "yaml",
            _ => "manifest",
        },
        None => "manifest",
    }
}

/// Write an Evidence node and its two edges (SUPPORTED_BY, EXTRACTED_FROM).
fn write_evidence(
    store: &mut LbugStore,
    element_id: &str,
    version_id: &str,
    sa_id: &str,
    evidence: &Evidence,
    strategy: &str,
) -> Result<()> {
    let evidence_id = format!(
        "ev:{}",
        blake3::hash(format!("{}:{}:{}", element_id, evidence.file, evidence.line).as_bytes())
            .to_hex()
    );

    let kind_str = match evidence.kind {
        EvidenceKind::Structural => "structural",
        EvidenceKind::Config => "config",
        EvidenceKind::Annotation => "annotation",
        EvidenceKind::Lexical => "lexical",
        EvidenceKind::Other => "other",
    };

    let mut props_map = serde_json::Map::new();
    props_map.insert(
        "file_refs".to_string(),
        serde_json::Value::Array(vec![serde_json::Value::String(format!(
            "{}:{}",
            evidence.file, evidence.line
        ))]),
    );
    props_map.insert(
        "text".to_string(),
        serde_json::Value::String(evidence.text.clone()),
    );
    props_map.insert(
        "status".to_string(),
        serde_json::Value::String("drafted".to_string()),
    );

    EvidenceRepository::put_structural_evidence(
        store,
        &crate::graph::StructuralEvidence {
            id: evidence_id.clone(),
            kind: kind_str.to_string(),
            claim: evidence.text.clone(),
            file: evidence.file.clone(),
            line: u64::from(evidence.line),
            confidence: 0.85,
            rule_id: format!("c4-discover:{}", strategy),
            props: props_map,
        },
    )
    .context("write_evidence: put_structural_evidence")?;

    let _ = EvidenceRepository::link_supported_by(store, version_id, &evidence_id).ok();
    // Fuse-on-write (Item 27 residual): recompute fused claims for the
    // affected version. Best-effort — never breaks the discovery run.
    let _ = crate::architecture::fusion::recompute_fused_for_versions(
        store,
        &[version_id.to_string()],
        &crate::architecture::fusion::MaxMemberEvaluator,
    );
    let _ = EvidenceRepository::link_extracted_from(store, &evidence_id, sa_id).ok();
    Ok(())
}

/// Persist a DiscoverReport to the graph. One Element per Container
/// (with ElementVersion.props.inferred=true), one Evidence per evidence
/// (status="Drafted"), one SourceArtifact per unique file.
/// Idempotent: skips canonical_keys that already exist.
pub fn apply(
    project_dir: &Path,
    report: &DiscoverReport,
    fs: &dyn Filesystem,
) -> Result<ApplyReport> {
    use crate::code::apply_common::write_source_artifact;
    use crate::store::{ElementRepository, LbugStore, UnitOfWork};

    let mut store =
        LbugStore::open(project_dir).map_err(|e| anyhow::anyhow!("failed to open store: {e}"))?;
    store.init().context("c4_discover apply: init")?;

    let mut elements_written = 0usize;
    let mut elements_skipped = 0usize;
    let mut evidences_written = 0usize;
    let mut source_artifacts_written = 0usize;

    // D5: wrap writes in single transaction via UnitOfWork.
    // On error: return propagates, Transaction drops → implicit rollback
    // (tracing::warn! on failure). On success: explicit commit.
    let mut tx = UnitOfWork::begin_transaction(&mut store)
        .context("c4_discover apply: begin_transaction")?;

    // Reborrow through Transaction to call LbugStore repository methods.
    let s: &mut LbugStore = tx.as_mut();

    // Seed required C4 MetaTypes before link_of_type runs (P1-04 regression fix).
    // The link_of_type call is best-effort; if the MetaType node doesn't exist,
    // OPTIONAL MATCH makes MERGE a silent no-op. These seeds ensure the nodes exist.
    // Moved inside tx so they are part of the atomic write boundary.
    ElementRepository::ensure_metatype(s, "mt.container", "c4", "container", "structure")?;
    ElementRepository::ensure_metatype(s, "mt.component", "c4", "component", "structure")?;

    // M32 D2: Hoist existing_canonical_keys OUT of per-container loop.
    // Skip-before-batch: only include containers NOT already in store in UNWIND batches.
    let existing_keys = ElementRepository::existing_canonical_keys(s)?;

    // ── Phase 1: Collect candidates across all containers ─────────────────────────
    // Pre-compute version_ids before batching so we can construct Element + ElementVersion
    // in a single pass.
    let mut container_elements: Vec<crate::graph::Element> = Vec::new();
    let mut container_versions: Vec<crate::graph::ElementVersion> = Vec::new();
    let mut container_metatypes: Vec<String> = Vec::new(); // parallel to elements/versions

    for container in &report.discovered {
        if existing_keys.contains(&container.canonical_key) {
            elements_skipped += 1;
            continue;
        }

        // Derive metatype and element prefix from strategy name (single unified path)
        let (metatype, element_prefix) = match container.strategy.as_str() {
            "components" => ("mt.component", "component"),
            _ => ("mt.container", "container"),
        };
        // Element ids must stay within the identifier charset. Canonical
        // keys from npm package names carry '@' and '/' (`@vueuse/core`),
        // which validate_identifier rejects (vueuse UAT smoke 2026-08-19).
        let element_id = format!(
            "c4:{}:{}",
            element_prefix,
            crate::graph::sanitize_identifier(&container.canonical_key)
        );

        // Pre-compute version_props (mirrors write_element_version logic)
        let version_props = serde_json::json!({
            "inferred": true,
            "strategy": container.strategy,
            "confidence": container.confidence,
            "merged_from": container.merged_from,
            "discovery_schema_version": "1.0",
        });
        let version_props_str = serde_json::to_string(&version_props).unwrap_or_default();
        let version_id = format!(
            "blake3:{}",
            blake3::hash(format!("{version_props_str}:{element_id}").as_bytes()).to_hex()
        );

        container_elements.push(crate::graph::Element {
            id: element_id.clone(),
            kind_id: metatype.to_string(),
            category: "c4".to_string(),
            canonical_key: container.canonical_key.clone(),
            current_name: container.name.clone(),
            current_status: "active".to_string(),
            current_confidence: container.confidence,
            current_version_id: version_id.clone(),
        });

        let mut props_map = serde_json::Map::new();
        if let Some(obj) = version_props.as_object() {
            for (k, v) in obj {
                props_map.insert(k.clone(), v.clone());
            }
        }
        container_versions.push(crate::graph::ElementVersion {
            id: version_id,
            element_id,
            name: container.name.clone(),
            status: "drafted".to_string(),
            origin: "c4-discover".to_string(),
            confidence: container.confidence,
            props: props_map,
        });
        container_metatypes.push(metatype.to_string());
        elements_written += 1;
    }

    // ── Phase 2: Batch-insert via ElementRepository trait methods ─────────────────
    // M32 D2: containers (Element + ElementVersion) — CURRENT_VERSION + VERSION_OF edges
    // created by batch_upsert_element_versions internally.
    if !container_elements.is_empty() {
        ElementRepository::batch_upsert_elements(s, &container_elements)?;
        ElementRepository::batch_upsert_element_versions(s, &container_versions)?;
    }

    // ── Phase 3: OF_TYPE edges batched via UNWIND ─────────────────────────────
    // CURRENT_VERSION and VERSION_OF edges are already created by batch_upsert_element_versions.
    // HIGH-5: replaced per-element loop with batch call.
    let of_type_pairs: Vec<(String, String)> = container_elements
        .iter()
        .zip(container_metatypes.iter())
        .map(|(element, metatype)| (element.id.clone(), metatype.clone()))
        .collect();
    if !of_type_pairs.is_empty() {
        ElementRepository::batch_link_of_type(s, &of_type_pairs)
            .context("c4_discover batch_link_of_type")?;
    }

    // ── Phase 4: Evidence writes (kept as-is, per-evidence loop inside tx) ────
    // Evidence writes need fs.read_to_string, so they can't be batched via UNWIND.
    // They are inside the UnitOfWork tx so no per-query commit cost.
    for container in &report.discovered {
        if existing_keys.contains(&container.canonical_key) {
            continue;
        }

        let (_metatype, element_prefix) = match container.strategy.as_str() {
            "components" => ("mt.component", "component"),
            _ => ("mt.container", "container"),
        };
        // Sanitized like Phase 1 (npm package names carry '@'/'/').
        let element_id = format!(
            "c4:{}:{}",
            element_prefix,
            crate::graph::sanitize_identifier(&container.canonical_key)
        );

        let version_props = serde_json::json!({
            "inferred": true,
            "strategy": container.strategy,
            "confidence": container.confidence,
            "merged_from": container.merged_from,
            "discovery_schema_version": "1.0",
        });
        let version_props_str = serde_json::to_string(&version_props).unwrap_or_default();
        let version_id = format!(
            "blake3:{}",
            blake3::hash(format!("{version_props_str}:{element_id}").as_bytes()).to_hex()
        );

        // SourceArtifact deduplication map (keyed by file path)
        let mut source_artifact_ids: BTreeMap<String, String> = BTreeMap::new();
        for evidence in &container.evidences {
            let sa_id = if let Some(id) = source_artifact_ids.get(&evidence.file) {
                id.clone()
            } else {
                let content_hash = fs
                    .read_to_string(&project_dir.join(&evidence.file))
                    .map(|s| crate::evidence::content_hash_of(&s))
                    .unwrap_or_default();
                let lang_label = c4_language_label(&evidence.file);
                let id = write_source_artifact(s, &evidence.file, &content_hash, lang_label)?;
                source_artifact_ids.insert(evidence.file.clone(), id.clone());
                source_artifacts_written += 1;
                id
            };

            write_evidence(
                s,
                &element_id,
                &version_id,
                &sa_id,
                evidence,
                &container.strategy,
            )?;
            evidences_written += 1;
        }
    }

    tx.commit().context("c4_discover apply: tx.commit")?;

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
        fn id(&self) -> &'static str {
            self.id
        }
        fn confidence(&self) -> f64 {
            self.confidence
        }
        fn metatype(&self) -> &'static str {
            "mt.container"
        }
        fn detect(
            &self,
            _project_root: &Path,
            _fs: &dyn Filesystem,
        ) -> Result<Vec<ContainerCandidate>> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn merge_two_strategies_same_canonical_key() {
        // SCN-140: two strategies infer the same canonical_key
        let fs = MemoryFilesystem::new();
        let clock: &dyn crate::clock::Clock =
            &crate::clock::FixedClock::new("2025-01-01T00:00:00Z");
        let tmp = tempfile::tempdir().unwrap();

        let candidates: Vec<ContainerCandidate> = vec![
            ContainerCandidate {
                canonical_key: "auth-svc".to_string(),
                name: "auth-svc".to_string(),
                strategy: "cargo-workspace".to_string(),
                confidence: 0.85,
                evidences: vec![Evidence {
                    content_hash: String::new(),
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
                    content_hash: String::new(),
                    file: "auth-svc/Dockerfile".to_string(),
                    line: 1,
                    kind: EvidenceKind::Structural,
                    text: "Dockerfile for service: auth-svc".to_string(),
                }],
            },
        ];

        #[derive(Clone)]
        struct InjectStrategy {
            candidates: Vec<ContainerCandidate>,
        }
        impl Strategy for InjectStrategy {
            fn id(&self) -> &'static str {
                "inject"
            }
            fn confidence(&self) -> f64 {
                1.0
            }
            fn metatype(&self) -> &'static str {
                "mt.container"
            }
            fn detect(&self, _: &Path, _: &dyn Filesystem) -> Result<Vec<ContainerCandidate>> {
                Ok(self.candidates.clone())
            }
        }

        let strategies: Vec<Box<dyn Strategy>> = vec![Box::new(InjectStrategy { candidates })];

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
        let clock: &dyn crate::clock::Clock =
            &crate::clock::FixedClock::new("2025-01-01T00:00:00Z");
        let tmp = tempfile::tempdir().unwrap();

        let candidates = vec![
            ContainerCandidate {
                canonical_key: "svc".to_string(),
                name: "svc".to_string(),
                strategy: "s1".to_string(),
                confidence: 0.90,
                evidences: vec![Evidence {
                    content_hash: String::new(),
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
                    content_hash: String::new(),
                    file: "file2.txt".to_string(),
                    line: 2,
                    kind: EvidenceKind::Config,
                    text: "evidence 2".to_string(),
                }],
            },
        ];

        // Inject candidates via a custom strategy
        #[derive(Clone)]
        struct InjectStrategy {
            candidates: Vec<ContainerCandidate>,
        }
        impl Strategy for InjectStrategy {
            fn id(&self) -> &'static str {
                "inject"
            }
            fn confidence(&self) -> f64 {
                1.0
            }
            fn metatype(&self) -> &'static str {
                "mt.container"
            }
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
        let clock: &dyn crate::clock::Clock =
            &crate::clock::FixedClock::new("2025-01-01T00:00:00Z");
        let tmp = tempfile::tempdir().unwrap();

        #[derive(Clone)]
        struct InjectStrategy {
            keys: Vec<String>,
        }
        impl Strategy for InjectStrategy {
            fn id(&self) -> &'static str {
                "inject"
            }
            fn confidence(&self) -> f64 {
                1.0
            }
            fn metatype(&self) -> &'static str {
                "mt.container"
            }
            fn detect(&self, _: &Path, _: &dyn Filesystem) -> Result<Vec<ContainerCandidate>> {
                Ok(self
                    .keys
                    .iter()
                    .map(|k| ContainerCandidate {
                        canonical_key: k.clone(),
                        name: k.clone(),
                        strategy: "test".to_string(),
                        confidence: 1.0,
                        evidences: vec![],
                    })
                    .collect())
            }
        }

        let strategies: Vec<Box<dyn Strategy>> = vec![Box::new(InjectStrategy {
            keys: vec![
                "zebra".to_string(),
                "apple".to_string(),
                "mango".to_string(),
            ],
        })];

        let report = discover(tmp.path(), &strategies, &fs, clock).unwrap();
        let keys: Vec<_> = report
            .discovered
            .iter()
            .map(|c| c.canonical_key.clone())
            .collect();
        assert_eq!(keys, vec!["apple", "mango", "zebra"]);
    }

    #[test]
    fn no_strategies_returns_empty_report() {
        let fs = MemoryFilesystem::new();
        let clock: &dyn crate::clock::Clock =
            &crate::clock::FixedClock::new("2025-01-01T00:00:00Z");
        let tmp = tempfile::tempdir().unwrap();
        let strategies: Vec<Box<dyn Strategy>> = vec![];
        let report = discover(tmp.path(), &strategies, &fs, clock).unwrap();
        assert!(report.discovered.is_empty());
        assert!(report.errors.is_empty());
    }

    #[test]
    fn schema_is_valid_json() {
        let parsed: serde_json::Value = serde_json::from_str(DISCOVER_REPORT_SCHEMA)
            .expect("DISCOVER_REPORT_SCHEMA must be valid JSON");
        assert_eq!(
            parsed["$schema"].as_str().unwrap(),
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(parsed["type"].as_str().unwrap(), "object");
        assert_eq!(
            parsed["properties"]["schemaVersion"]["const"]
                .as_str()
                .unwrap(),
            "1.0"
        );
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
        let validator =
            jsonschema::validator_for(&jsonschema).expect("schema must be valid JSON Schema");
        let result = validator.validate(&report);
        assert!(
            result.is_ok(),
            "valid report must pass schema: {:?}",
            result.err()
        );
    }

    #[test]
    fn apply_writes_element_with_inferred_props() {
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
                    content_hash: String::new(),
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

    /// Regression: npm package names carry '@' and '/' (`@vueuse/core`);
    /// the derived element id must be sanitized or OF_TYPE batch writes
    /// reject it (vueuse UAT smoke 2026-08-19).
    #[test]
    fn apply_sanitizes_element_ids_from_npm_package_names() {
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
                canonical_key: "@vueuse/core".to_string(),
                name: "@vueuse/core".to_string(),
                strategy: "npm-workspace".to_string(),
                confidence: 0.8,
                merged_from: vec!["npm-workspace".to_string()],
                evidences: vec![Evidence {
                    content_hash: String::new(),
                    file: "packages/core/package.json".to_string(),
                    line: 2,
                    kind: EvidenceKind::Structural,
                    text: "npm workspace package".to_string(),
                }],
            }],
            errors: vec![],
        };

        let fs = MemoryFilesystem::new();
        let result = apply(project, &report, &fs);
        assert!(result.is_ok(), "apply must succeed: {:?}", result.err());
        let r = result.unwrap();
        assert_eq!(r.elements_written, 1);
    }

    #[test]
    fn apply_is_idempotent_skips_existing_canonical_key() {
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
                    content_hash: String::new(),
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
        assert_eq!(
            r2.elements_skipped, 1,
            "second apply must skip existing canonical_key"
        );
        assert_eq!(
            r2.elements_written, 0,
            "second apply must not write duplicates"
        );
    }

    // ─── CRIT-1 regression: real Container round-trip against schema ───────────
    // This test would have caught the PascalCase/lowercase mismatch. It
    // serialises a real Container (not hand-crafted JSON) then validates
    // against the embedded schema.

    #[test]
    fn serialize_container_then_validate_against_schema() {
        // Build a real DiscoverReport with real Container + Evidence
        let report = DiscoverReport {
            schema_version: "1.0".to_string(),
            project: ProjectMeta {
                root: "/tmp/test".to_string(),
                files_scanned: 5,
                languages: BTreeMap::from([("rust".to_string(), 5)]),
                duration_ms: 42,
            },
            discovered: vec![
                Container {
                    canonical_key: "auth-svc".to_string(),
                    name: "auth-svc".to_string(),
                    strategy: "cargo-workspace".to_string(),
                    confidence: 0.85,
                    merged_from: vec!["cargo-workspace".to_string()],
                    evidences: vec![
                        Evidence {
                            content_hash: String::new(),
                            file: "Cargo.toml".to_string(),
                            line: 8,
                            kind: EvidenceKind::Structural,
                            text: "Cargo workspace member: auth-svc".to_string(),
                        },
                        Evidence {
                            content_hash: String::new(),
                            file: "src/main.rs".to_string(),
                            line: 1,
                            kind: EvidenceKind::Lexical,
                            text: "Module root".to_string(),
                        },
                    ],
                },
                Container {
                    canonical_key: "api-gateway".to_string(),
                    name: "api-gateway".to_string(),
                    strategy: "dockerfile".to_string(),
                    confidence: 0.60,
                    merged_from: vec!["dockerfile".to_string()],
                    evidences: vec![Evidence {
                        content_hash: String::new(),
                        file: "services/api/Dockerfile".to_string(),
                        line: 1,
                        kind: EvidenceKind::Config,
                        text: "Dockerfile for api-gateway".to_string(),
                    }],
                },
            ],
            errors: vec![],
        };

        // Round-trip: Rust struct → JSON string → parsed Value
        let json_str =
            serde_json::to_string(&report).expect("DiscoverReport must serialise to JSON");
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("JSON must be parseable");

        // Validate against the embedded schema
        let schema_val: serde_json::Value = serde_json::from_str(DISCOVER_REPORT_SCHEMA)
            .expect("DISCOVER_REPORT_SCHEMA must be valid JSON");
        let validator = jsonschema::validator_for(&schema_val)
            .expect("DISCOVER_REPORT_SCHEMA must be a valid JSON Schema");
        let result = validator.validate(&parsed);
        assert!(
            result.is_ok(),
            "real Container must pass schema validation: {:?}",
            result.err()
        );
    }

    #[test]
    fn c4_language_label_dockerfile_variants() {
        assert_eq!(c4_language_label("Dockerfile"), "dockerfile");
        assert_eq!(c4_language_label("dockerfile"), "dockerfile");
        assert_eq!(c4_language_label("services/api/Dockerfile"), "dockerfile");
        assert_eq!(c4_language_label("SERVICES/DOCKERFILE"), "dockerfile");
    }

    #[test]
    fn c4_language_label_manifest_extensions() {
        assert_eq!(c4_language_label("Cargo.toml"), "toml");
        assert_eq!(c4_language_label("foo.TOML"), "toml");
        assert_eq!(c4_language_label("package.json"), "json");
        assert_eq!(c4_language_label("Chart.yaml"), "yaml");
        assert_eq!(c4_language_label("values.yml"), "yaml");
        assert_eq!(c4_language_label("CHART.YAML"), "yaml");
    }

    #[test]
    fn c4_language_label_falls_back_to_manifest() {
        assert_eq!(c4_language_label("Makefile"), "manifest");
        assert_eq!(c4_language_label("Procfile"), "manifest");
        assert_eq!(c4_language_label("no-extension"), "manifest");
        assert_eq!(c4_language_label("unknown.xyz"), "manifest");
    }
}
