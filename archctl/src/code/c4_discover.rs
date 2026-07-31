//! C4 Container boundary inference engine + report types.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::code::strategies::Strategy;
use crate::filesystem::Filesystem;

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
}
