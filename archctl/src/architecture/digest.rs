//! Deterministic extractor-set digest for snapshot identity.
//!
//! The digest is computed over a sorted, canonical projection of the
//! active code-extraction capabilities only (source_code + source_cargo).
//! Renderers, IDE, MCP, plugins, and read-only projections (sequence)
//! are excluded to keep the digest stable across unrelated tool churn.
//!
//! The digest covers: language_extractor_id, language_extractor_version,
//! view_strategy_id, project_strategy_id, evidence_extractor_id.

use blake3::Hasher;
use serde::{Deserialize, Serialize};

use crate::capability::source_code;
use crate::capability::source_cargo;
use crate::capability::Capability;

/// A single entry in the extractor digest projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractorEntry {
    /// The capability id, e.g. "code.call_graph".
    pub extractor_id: String,
    /// The language provider, e.g. "rust".
    pub language: String,
    /// The schema id, e.g. "call-graph-report/1".
    pub schema: Option<String>,
    /// The maturity of this provider.
    pub maturity: String,
}

/// The canonical extractor-set projection used for snapshot identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractorSetProjection {
    /// Entries sorted by extractor_id + language.
    pub entries: Vec<ExtractorEntry>,
}

/// Compute a deterministic digest of the current extractor set.
///
/// Only code-extraction capabilities from `source_code` and `source_cargo`
/// are included. The `code.sequence` read-only projection is excluded.
/// Result is a `blake3:...` string.
pub fn extractor_set_digest() -> String {
    let projection = extractor_set_projection();
    let json = serde_json::to_string(&projection).expect("ExtractorSetProjection is JSON-serializable");
    let mut hasher = Hasher::new();
    hasher.update(json.as_bytes());
    let digest = hasher.finalize();
    format!("blake3:{digest}")
}

/// Build the canonical extractor-set projection.
///
/// Sorted by extractor_id then language for determinism.
pub fn extractor_set_projection() -> ExtractorSetProjection {
    let mut all_caps: Vec<Capability> = Vec::new();
    all_caps.extend(source_code::all());
    all_caps.extend(source_cargo::all());

    // Filter out read-only sequence projection
    all_caps.retain(|c| c.id != "code.sequence");

    // Collect and sort entries
    let mut entries: Vec<ExtractorEntry> = all_caps
        .into_iter()
        .flat_map(|cap| {
            cap.providers.into_iter().map(move |provider| {
                let maturity_str = match provider.maturity {
                    crate::capability::Maturity::Stable => "stable",
                    crate::capability::Maturity::Beta => "beta",
                    crate::capability::Maturity::Experimental => "experimental",
                    crate::capability::Maturity::Deprecated => "deprecated",
                };
                ExtractorEntry {
                    extractor_id: cap.id.clone(),
                    language: provider.language,
                    schema: provider.schema,
                    maturity: maturity_str.to_string(),
                }
            })
        })
        .collect();

    // Sort by extractor_id then language for deterministic output
    entries.sort_by(|a, b| {
        let key_a = format!("{}\0{}", a.extractor_id, a.language);
        let key_b = format!("{}\0{}", b.extractor_id, b.language);
        key_a.cmp(&key_b)
    });

    ExtractorSetProjection { entries }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_deterministic() {
        let d1 = extractor_set_digest();
        let d2 = extractor_set_digest();
        assert_eq!(d1, d2, "digest must be deterministic");
    }

    #[test]
    fn digest_starts_with_blake3_prefix() {
        let d = extractor_set_digest();
        assert!(d.starts_with("blake3:"), "digest must be a blake3 hash");
    }

    #[test]
    fn projection_is_sorted() {
        let proj = extractor_set_projection();
        for window in proj.entries.windows(2) {
            let a = &window[0];
            let b = &window[1];
            let key_a = format!("{}\0{}", a.extractor_id, a.language);
            let key_b = format!("{}\0{}", b.extractor_id, b.language);
            assert!(key_a <= key_b, "entries must be sorted by extractor_id+language");
        }
    }

    #[test]
    fn sequence_is_excluded() {
        let proj = extractor_set_projection();
        let has_sequence = proj.entries.iter().any(|e| e.extractor_id == "code.sequence");
        assert!(!has_sequence, "code.sequence must be excluded from digest");
    }
}
