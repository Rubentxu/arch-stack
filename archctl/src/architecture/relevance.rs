//! Architecture relevance use case — deterministic scored shortlist.
//!
//! Read-only query over the live architecture graph that produces a ranked
//! shortlist of elements and relations matching a free-text or exact-id query.
//! Pure function over `DiagramRepository`; no graph-store writes are performed.
//!
//! ## Public surface
//!
//! - `relevance` — the main use case function
//! - `RelevanceReport` — the JSON-serializable output carrier
//! - `RelevanceOptions` — configuration (top, max_hops)
//! - `RelevanceError` — domain errors

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::store::DiagramRepository;

// ─────────────────────────────────────────────────────────────────────────────
// Carriers
// ─────────────────────────────────────────────────────────────────────────────

/// The relevance-report/1 carrier — the output of the `relevance` use case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelevanceReport {
    /// Schema version of this report format.
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,

    /// Capability that produced this report.
    pub capability: String,

    /// The original query string.
    pub query: String,

    /// Scored elements, sorted by (score DESC, id ASC).
    pub elements: Vec<ScoredElement>,

    /// Scored relations where source or target is in the shortlist.
    pub relations: Vec<ScoredRelation>,

    /// Trace of the selection process.
    #[serde(rename = "selectionTrace")]
    pub selection_trace: SelectionTrace,
}

/// A scored element in the relevance shortlist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoredElement {
    /// Element id.
    pub id: String,

    /// Element kind id (e.g. "container", "component").
    #[serde(rename = "kindId")]
    pub kind_id: String,

    /// Element category (e.g. "c4", "uml", "behavior").
    pub category: String,

    /// Element name.
    pub name: String,

    /// Relevance score (0.0–1.0).
    pub score: f64,

    /// How the element was matched.
    #[serde(rename = "matchType")]
    pub match_type: String,

    /// Hop distance from the nearest seed (0 = seed).
    #[serde(rename = "hopDistance")]
    pub hop_distance: usize,
}

/// A scored relation in the relevance shortlist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoredRelation {
    /// Relation id.
    #[serde(rename = "relationId")]
    pub relation_id: String,

    /// Predicate id (e.g. "depends_on", "calls").
    #[serde(rename = "predicateId")]
    pub predicate_id: String,

    /// Source element id.
    #[serde(rename = "sourceId")]
    pub source_id: String,

    /// Target element id.
    #[serde(rename = "targetId")]
    pub target_id: String,

    /// Relevance score (0.0–1.0).
    pub score: f64,
}

/// Trace of the selection process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionTrace {
    /// Number of seed elements matched exactly or by name.
    #[serde(rename = "seedsMatched")]
    pub seeds_matched: usize,

    /// Number of candidate elements scanned.
    #[serde(rename = "candidatesScanned")]
    pub candidates_scanned: usize,

    /// Number of expansion edges followed during BFS.
    #[serde(rename = "expansionEdgesFollowed")]
    pub expansion_edges_followed: usize,
}

/// Configuration for the relevance use case.
#[derive(Debug, Clone)]
pub struct RelevanceOptions {
    /// Maximum number of elements and relations to return.
    pub top: usize,
    /// Maximum BFS expansion hop distance from seeds.
    pub max_hops: usize,
}

impl Default for RelevanceOptions {
    fn default() -> Self {
        Self {
            top: 10,
            max_hops: 1,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

/// Errors specific to relevance operations.
#[derive(Debug, Clone)]
pub enum RelevanceError {
    /// Query string was empty or whitespace-only.
    EmptyQuery,
    /// The store returned an error.
    Store(String),
}

impl std::fmt::Display for RelevanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelevanceError::EmptyQuery => write!(f, "empty relevance query"),
            RelevanceError::Store(msg) => write!(f, "relevance error: {msg}"),
        }
    }
}

impl std::error::Error for RelevanceError {}

impl From<anyhow::Error> for RelevanceError {
    fn from(e: anyhow::Error) -> Self {
        RelevanceError::Store(e.to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Text normalisation helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Apply ASCII-fold normalisation: strip diacritics and lowercase.
/// ñ→n, á→a, é→e, í→i, ó→o, ú→u, ü→u, then to_lowercase.
fn ascii_fold(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.to_lowercase().chars() {
        match c {
            'ñ' => result.push('n'),
            'á' | 'à' | 'ä' | 'â' => result.push('a'),
            'é' | 'è' | 'ë' | 'ê' => result.push('e'),
            'í' | 'ì' | 'ï' | 'î' => result.push('i'),
            'ó' | 'ò' | 'ö' | 'ô' => result.push('o'),
            'ú' | 'ù' | 'ü' | 'û' => result.push('u'),
            'ÿ' => result.push('y'),
            'ß' => result.push_str("ss"),
            c if !c.is_ascii_alphanumeric() => result.push(' '),
            c => result.push(c),
        }
    }
    result
}

/// English and Spanish stopwords to drop during tokenisation.
const STOPWORDS: &[&str] = &[
    // English common stopwords
    "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by", "from",
    "as", "is", "was", "are", "were", "been", "be", "have", "has", "had", "do", "does", "did",
    "will", "would", "could", "should", "may", "might", "must", "shall", "can", "need", "it",
    "its", "this", "that", "these", "those", // Spanish common stopwords
    "el", "la", "los", "las", "un", "una", "unos", "unas", "y", "o", "pero", "en", "de", "del",
    "al", "a", "por", "para", "con", "sin", "sobre", "es", "son", "era", "eran", "fue", "fueron",
    "ser", "estar", "ha", "han", "había", "habían", "he", "hemos", "se", "le", "les", "me", "te",
    "nos", "que", "cual", "cuales", "como", "cuando", "donde", "porque", "este", "esta", "estos",
    "estas", "ese", "esa", "esos", "esas", "mi", "tu", "su", "nuestro", "vuestro",
];

/// Tokenise a string: lowercase, ASCII-fold, split on non-alphanumeric,
/// drop tokens shorter than 2 chars and stopwords.
fn tokenize(s: &str) -> Vec<String> {
    ascii_fold(s)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|tok| tok.len() >= 2)
        .filter(|tok| !STOPWORDS.contains(tok))
        .map(|tok| tok.to_string())
        .collect()
}

/// Check if a relation predicate matches the query.
/// Matches if any query token is contained within the predicate string
/// (after folding). This allows "depends" to match "depends_on".
fn predicate_matches_query(predicate_folded: &str, query_tokens: &[String]) -> bool {
    query_tokens
        .iter()
        .any(|qt| predicate_folded.contains(qt.as_str()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Core relevance function
// ─────────────────────────────────────────────────────────────────────────────

/// Compute a relevance shortlist for the given query.
///
/// Iterates elements across `c4`, `uml`, and `behavior` categories.
/// Scoring:
/// - exact id match → 1.0 * max(0.1, confidence), matchType "exact-id", hop 0
/// - name/canonical_key substring match → 0.8 * max(0.1, confidence), matchType "name", hop 0
/// - multi-token: all tokens match → full score; partial → proportional
/// - BFS expansion: 0.5^hop * max(0.1, confidence), matchType "expansion"
/// - Relations: 0.5 * min(srcScore, tgtScore) when source or target is in shortlist
///
/// Sort: (score DESC, id ASC). Output capped at opts.top.
pub fn relevance(
    repo: &dyn DiagramRepository,
    query: &str,
    opts: &RelevanceOptions,
) -> Result<RelevanceReport, RelevanceError> {
    let query = query.trim();

    // S10: empty query → EmptyQuery
    if query.is_empty() {
        return Err(RelevanceError::EmptyQuery);
    }

    // Load all elements from target categories
    let mut all_elements: Vec<crate::graph::ElementRow> = Vec::new();
    for category in &["c4", "uml", "behavior", "code"] {
        let elems = repo.list_elements(category, None, None)?;
        all_elements.extend(elems);
    }

    // Load all semantic edges
    let mut all_edges: Vec<crate::graph::SemanticEdgeRow> = Vec::new();
    for category in &["c4", "uml", "behavior", "code"] {
        let edges = repo.list_semantic_edges(category)?;
        all_edges.extend(edges);
    }

    let candidates_scanned = all_elements.len();
    let query_lower = ascii_fold(query);
    let query_exact_lower = query.to_lowercase(); // case-fold only (no space replacement) for exact-id
    let query_tokens = tokenize(query);

    // Determine if the query has meaningful tokens (non-stopword, len >= 2)
    let has_meaningful_tokens = !query_tokens.is_empty();

    // Check if the query is a multi-char pure stopword (len >= 2 stopword).
    // Single-char stopwords like "a" can still match as identifiers via raw fallback.
    // Multi-char stopwords like "the", "for" etc. should be skipped.
    let raw_query_str = query.trim();
    let is_multi_char_stopword = !raw_query_str.is_empty()
        && raw_query_str.len() >= 2
        && STOPWORDS.contains(&raw_query_str.to_lowercase().as_str());

    // Track scored elements: element_id -> (score, match_type, hop_distance)
    let mut scored: BTreeMap<String, (f64, String, usize)> = BTreeMap::new();

    // Phase 1: score seeds (exact-id and name matches)
    let mut seeds_matched = 0_usize;

    for elem in &all_elements {
        let elem_id_lower = elem.id.to_lowercase();
        let elem_name_lower = ascii_fold(&elem.current_name);
        let confidence = elem.current_confidence;

        // Exact id match (case-insensitive, preserving all characters)
        if elem_id_lower == query_exact_lower {
            let score = 1.0 * confidence.max(0.1);
            scored.insert(elem.id.clone(), (score, "exact-id".to_string(), 0));
            seeds_matched += 1;
            continue;
        }

        // Name/canonical_key matching
        if has_meaningful_tokens {
            // Tokenized matching: all query tokens should match.
            // For multi-char tokens: substring match (query token contained in name/category token).
            // For single-char tokens: exact equality (avoid "a" matching "container" as substring).
            let name_tokens = tokenize(&elem.current_name);
            let canonical_tokens = tokenize(&elem.canonical_key);

            let all_name_tokens_match = query_tokens
                .iter()
                .all(|qt| name_tokens.iter().any(|nt| nt.contains(qt.as_str())));
            let all_canonical_tokens_match = query_tokens
                .iter()
                .all(|qt| canonical_tokens.iter().any(|ct| ct.contains(qt.as_str())));

            let matched_name = query_tokens
                .iter()
                .filter(|qt| name_tokens.iter().any(|nt| nt.contains(qt.as_str())))
                .count();
            let matched_canonical = query_tokens
                .iter()
                .filter(|qt| canonical_tokens.iter().any(|ct| ct.contains(qt.as_str())))
                .count();

            let (token_fraction, match_type) =
                if all_name_tokens_match || all_canonical_tokens_match {
                    (1.0, "name")
                } else if matched_name > 0 || matched_canonical > 0 {
                    let total_matched =
                        matched_name.max(matched_canonical) as f64 / query_tokens.len() as f64;
                    (total_matched, "name")
                } else {
                    continue;
                };

            let score = 0.8 * confidence.max(0.1) * token_fraction;
            let prev = scored.entry(elem.id.clone()).or_insert_with(|| {
                seeds_matched += 1;
                (score, match_type.to_string(), 0)
            });
            if score > prev.0 {
                *prev = (score, match_type.to_string(), 0);
            }
        } else if !is_multi_char_stopword && !query_lower.is_empty() {
            // Fallback: raw substring matching on element NAME only (not canonical_key).
            // This avoids false matches like query "a" incorrectly matching "c4:container:b".
            let raw_query = query_lower.trim();
            if !raw_query.is_empty() && elem_name_lower.contains(raw_query) {
                let score = 0.8 * confidence.max(0.1);
                let prev = scored.entry(elem.id.clone()).or_insert_with(|| {
                    seeds_matched += 1;
                    (score, "name".to_string(), 0)
                });
                if score > prev.0 {
                    *prev = (score, "name".to_string(), 0);
                }
            }
        }
    }

    // Phase 2: BFS expansion from seeds
    let mut expansion_edges_followed = 0_usize;
    let seed_ids: std::collections::HashSet<String> = scored.keys().cloned().collect();

    if opts.max_hops > 0 {
        let mut frontier: std::collections::HashSet<String> = seed_ids.clone();
        let mut next_frontier: std::collections::HashSet<String> = std::collections::HashSet::new();

        for hop in 1..=opts.max_hops {
            let hop_score_factor = 0.5_f64.powi(hop as i32);
            let mut hop_edges = 0_usize;

            for edge in &all_edges {
                // Forward: expand from source in frontier to target
                if frontier.contains(&edge.source_id)
                    && !scored.contains_key(&edge.target_id)
                    && !next_frontier.contains(&edge.target_id)
                    && let Some(tgt) = all_elements.iter().find(|e| e.id == edge.target_id)
                {
                    let score = hop_score_factor * tgt.current_confidence.max(0.1);
                    scored.insert(tgt.id.clone(), (score, "expansion".to_string(), hop));
                    next_frontier.insert(tgt.id.clone());
                    hop_edges += 1;
                }
                // Reverse: expand from target in frontier to source
                if frontier.contains(&edge.target_id)
                    && !scored.contains_key(&edge.source_id)
                    && !next_frontier.contains(&edge.source_id)
                    && let Some(src) = all_elements.iter().find(|e| e.id == edge.source_id)
                {
                    let score = hop_score_factor * src.current_confidence.max(0.1);
                    scored.insert(src.id.clone(), (score, "expansion".to_string(), hop));
                    next_frontier.insert(src.id.clone());
                    hop_edges += 1;
                }
            }
            expansion_edges_followed += hop_edges;
            if next_frontier.is_empty() {
                break;
            }
            frontier = next_frontier;
            next_frontier = std::collections::HashSet::new();
        }
    }

    // Phase 3: Sort and cap elements
    let mut sorted_elements: Vec<(String, (f64, String, usize))> =
        scored.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    sorted_elements.sort_by(|(a_id, (a_score, _, _)), (b_id, (b_score, _, _))| {
        b_score
            .partial_cmp(a_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a_id.cmp(b_id))
    });

    let top_elements: Vec<ScoredElement> = sorted_elements
        .iter()
        .take(opts.top)
        .filter_map(|(id, (score, match_type, hop))| {
            all_elements
                .iter()
                .find(|e| &e.id == id)
                .map(|elem| ScoredElement {
                    id: elem.id.clone(),
                    kind_id: elem.kind_id.clone(),
                    category: elem.category.clone(),
                    name: elem.current_name.clone(),
                    score: *score,
                    match_type: match_type.clone(),
                    hop_distance: *hop,
                })
        })
        .collect();

    // Phase 4: Score relations
    let shortlist_ids: std::collections::HashSet<String> =
        top_elements.iter().map(|e| e.id.clone()).collect();

    let mut scored_relations: Vec<(String, f64)> = Vec::new();
    for edge in &all_edges {
        let src_score = scored.get(&edge.source_id).map(|s| s.0);
        let tgt_score = scored.get(&edge.target_id).map(|s| s.0);
        let predicate_folded = ascii_fold(&edge.predicate_id);

        // Include relation if: endpoints are in shortlist, OR predicate matches query
        let in_shortlist =
            shortlist_ids.contains(&edge.source_id) || shortlist_ids.contains(&edge.target_id);
        let pred_match = predicate_matches_query(&predicate_folded, &query_tokens);

        if in_shortlist {
            // Standard: score based on endpoint relevance
            if let (Some(ss), Some(ts)) = (src_score, tgt_score) {
                let rel_score = 0.5 * ss.min(ts);
                scored_relations.push((edge.relation_id.clone(), rel_score));
            }
        } else if pred_match {
            // Predicate-matched: score based on endpoint confidences (use 0.1 if unscored)
            let ss = src_score.unwrap_or(0.1);
            let ts = tgt_score.unwrap_or(0.1);
            let rel_score = 0.5 * ss.min(ts);
            scored_relations.push((edge.relation_id.clone(), rel_score));
        }
    }

    // Sort relations by score DESC, id ASC
    scored_relations.sort_by(|(a_id, a_score), (b_id, b_score)| {
        b_score
            .partial_cmp(a_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a_id.cmp(b_id))
    });

    let top_relations: Vec<ScoredRelation> = scored_relations
        .iter()
        .take(opts.top)
        .filter_map(|(rel_id, rel_score)| {
            all_edges
                .iter()
                .find(|e| &e.relation_id == rel_id)
                .map(|edge| ScoredRelation {
                    relation_id: edge.relation_id.clone(),
                    predicate_id: edge.predicate_id.clone(),
                    source_id: edge.source_id.clone(),
                    target_id: edge.target_id.clone(),
                    score: *rel_score,
                })
        })
        .collect();

    Ok(RelevanceReport {
        schema_version: "1.0".to_string(),
        capability: "architecture-relevance-mvp".to_string(),
        query: query.to_string(),
        elements: top_elements,
        relations: top_relations,
        selection_trace: SelectionTrace {
            seeds_matched,
            candidates_scanned,
            expansion_edges_followed,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{ElementRepository, GraphStore, LbugStore, SemanticEdgeRepository};

    /// Builder-style seeder that persists the test fixture into a
    /// real `LbugStore` opened in a TempDir. Mirrors the previous
    /// FakeRepo builder ergonomics. `.build()` returns the
    /// `LbugStore` ready to pass to functions taking `&dyn
    /// DiagramRepository`.
    struct SeededStore {
        project_dir: std::path::PathBuf,
        elements: Vec<(String, String, f64, String)>, // id, name, confidence, category
        edges: Vec<(String, String, String, String)>, // rel, pred, src, tgt
    }

    impl SeededStore {
        fn new(project_dir: &std::path::Path) -> Self {
            Self {
                project_dir: project_dir.to_path_buf(),
                elements: vec![],
                edges: vec![],
            }
        }
        fn with_element(mut self, id: &str, name: &str, confidence: f64, category: &str) -> Self {
            self.elements.push((
                id.to_string(),
                name.to_string(),
                confidence,
                category.to_string(),
            ));
            self
        }
        fn with_edge(
            mut self,
            relation_id: &str,
            predicate_id: &str,
            source_id: &str,
            target_id: &str,
        ) -> Self {
            self.edges.push((
                relation_id.to_string(),
                predicate_id.to_string(),
                source_id.to_string(),
                target_id.to_string(),
            ));
            self
        }
        fn build(self) -> LbugStore {
            let mut store = LbugStore::open(&self.project_dir).expect("LbugStore::open");
            store.init().expect("LbugStore::init");
            for (id, name, confidence, category) in &self.elements {
                let version_id = format!("{id}-v1");
                let v = crate::graph::ElementVersion {
                    id: version_id.clone(),
                    element_id: id.clone(),
                    name: name.clone(),
                    status: "accepted".to_string(),
                    origin: "test".to_string(),
                    confidence: *confidence,
                    props: Default::default(),
                };
                store
                    .upsert_element_version(&v)
                    .expect("upsert_element_version");
                store
                    .link_current_version(id, &version_id)
                    .expect("link_current_version");
                let e = crate::graph::Element {
                    id: id.clone(),
                    kind_id: "container".to_string(),
                    category: category.clone(),
                    canonical_key: id.clone(),
                    current_name: name.clone(),
                    current_status: "active".to_string(),
                    current_confidence: *confidence,
                    current_version_id: version_id.clone(),
                };
                store.upsert_element(&e).expect("upsert_element");
            }
            for (rel, pred, src, tgt) in &self.edges {
                store
                    .link_semantic_edge(src, tgt, rel, pred, &serde_json::Map::new(), true)
                    .expect("link_semantic_edge");
            }
            store
        }
    }

    // -------------------------------------------------------------------------
    // S10: Empty query → RelevanceError::EmptyQuery
    // -------------------------------------------------------------------------

    #[test]
    fn relevance_empty_query_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path()).build();
        let result = relevance(&repo, "", &RelevanceOptions::default());
        assert!(matches!(result, Err(RelevanceError::EmptyQuery)));

        let result = relevance(&repo, "   ", &RelevanceOptions::default());
        assert!(matches!(result, Err(RelevanceError::EmptyQuery)));
    }

    // -------------------------------------------------------------------------
    // S1: Exact-id seed score 1.0
    // -------------------------------------------------------------------------

    #[test]
    fn relevance_exact_id_seed() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path())
            .with_element("c4:container:orders", "OrderService", 0.9, "c4")
            .build();

        let result = relevance(&repo, "c4:container:orders", &RelevanceOptions::default()).unwrap();

        assert_eq!(result.elements.len(), 1);
        let elem = &result.elements[0];
        assert_eq!(elem.id, "c4:container:orders");
        assert!((elem.score - 0.9).abs() < 1e-9);
        assert_eq!(elem.match_type, "exact-id");
        assert_eq!(elem.hop_distance, 0);
        assert!(result.selection_trace.seeds_matched >= 1);
    }

    // -------------------------------------------------------------------------
    // S2: Free-text name match sorted (score DESC, id ASC)
    // -------------------------------------------------------------------------

    #[test]
    fn relevance_name_match_ranking() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path())
            .with_element("c4:container:orders", "OrderService", 0.9, "c4")
            .with_element("c4:container:queue", "OrderQueue", 0.8, "c4")
            .build();

        let result = relevance(&repo, "Order", &RelevanceOptions::default()).unwrap();

        assert_eq!(result.elements.len(), 2);
        // OrderService (0.9) before OrderQueue (0.8)
        assert_eq!(result.elements[0].id, "c4:container:orders");
        assert!((result.elements[0].score - 0.72).abs() < 1e-9); // 0.8 * 0.9
        assert_eq!(result.elements[1].id, "c4:container:queue");
        assert!((result.elements[1].score - 0.64).abs() < 1e-9); // 0.8 * 0.8
    }

    // -------------------------------------------------------------------------
    // S3: 1-hop expansion at 0.5x
    // -------------------------------------------------------------------------

    #[test]
    fn relevance_expansion_0_5x() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path())
            .with_element("c4:container:a", "A", 0.9, "c4")
            .with_element("c4:container:b", "B", 0.8, "c4")
            .with_edge("rel-a-b", "depends_on", "c4:container:a", "c4:container:b")
            .build();

        let result = relevance(
            &repo,
            "a",
            &RelevanceOptions {
                top: 10,
                max_hops: 1,
            },
        )
        .unwrap();

        assert_eq!(result.elements.len(), 2);
        // B should appear via expansion with 0.5 * 0.8 = 0.4
        let b = result.elements.iter().find(|e| e.id == "c4:container:b");
        assert!(b.is_some());
        let b = b.unwrap();
        assert!((b.score - 0.4).abs() < 1e-9);
        assert_eq!(b.match_type, "expansion");
        assert_eq!(b.hop_distance, 1);
        assert!(result.selection_trace.expansion_edges_followed >= 1);
    }

    // -------------------------------------------------------------------------
    // S4: max_hops=0 disables expansion
    // -------------------------------------------------------------------------

    #[test]
    fn relevance_max_hops_zero_no_expansion() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path())
            .with_element("c4:container:a", "A", 0.9, "c4")
            .with_element("c4:container:b", "B", 0.8, "c4")
            .with_edge("rel-a-b", "depends_on", "c4:container:a", "c4:container:b")
            .build();

        let result = relevance(
            &repo,
            "a",
            &RelevanceOptions {
                top: 10,
                max_hops: 0,
            },
        )
        .unwrap();

        assert_eq!(result.elements.len(), 1);
        assert_eq!(result.elements[0].id, "c4:container:a");
        assert_eq!(result.selection_trace.expansion_edges_followed, 0);
    }

    // -------------------------------------------------------------------------
    // S7: --top N caps shortlist independently
    // -------------------------------------------------------------------------

    #[test]
    fn relevance_top_caps_shortlist() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut seeder = SeededStore::new(tmp.path());
        for i in 0..12 {
            seeder = seeder.with_element(
                &format!("c4:container:e{}", i),
                &format!("Element{}", i),
                0.9,
                "c4",
            );
        }
        let repo = seeder.build();

        let result = relevance(
            &repo,
            "e",
            &RelevanceOptions {
                top: 5,
                max_hops: 0,
            },
        )
        .unwrap();

        assert_eq!(result.elements.len(), 5);
    }

    // -------------------------------------------------------------------------
    // S9: ASCII-fold match (MañanasService ↔ mananas)
    // -------------------------------------------------------------------------

    #[test]
    fn relevance_ascii_fold_match() {
        // Production's validate_identifier rejects non-ASCII names
        // so the builder can't seed "MañanasService" through the
        // normal write ports. Raw Cypher bypasses validation so the
        // read-path ASCII-fold logic (ñ→n) gets exercised against a
        // realistic unicode element name.
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path()).build();
        let mut repo = repo;
        repo.execute_raw_cypher_for_test(
            "CREATE (:Element {id: 'c4:container:srv', kind_id: 'container', \
             category: 'c4', canonical_key: 'c4:container:srv', \
             current_name: 'MañanasService', current_status: 'active', \
             current_confidence: 0.9, current_version_id: 'c4:container:srv-v1'});",
        )
        .expect("seed unicode element");

        let result = relevance(&repo, "mananas", &RelevanceOptions::default()).unwrap();

        assert!(!result.elements.is_empty());
        assert_eq!(result.elements[0].id, "c4:container:srv");
    }

    // -------------------------------------------------------------------------
    // S6: Empty graph → empty arrays, exit 0 (no error)
    // -------------------------------------------------------------------------

    #[test]
    fn relevance_empty_graph() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path()).build();

        let result = relevance(&repo, "anything", &RelevanceOptions::default()).unwrap();

        assert!(result.elements.is_empty());
        assert!(result.relations.is_empty());
        assert_eq!(result.selection_trace.seeds_matched, 0);
        assert_eq!(result.selection_trace.candidates_scanned, 0);
    }

    // -------------------------------------------------------------------------
    // S5: Determinism — two calls produce byte-equal JSON
    // -------------------------------------------------------------------------

    #[test]
    fn relevance_determinism() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path())
            .with_element("c4:container:srv", "OrderService", 0.9, "c4")
            .with_element("c4:container:api", "OrderApi", 0.8, "c4")
            .build();

        let opts = RelevanceOptions::default();
        let json1 = serde_json::to_string(&relevance(&repo, "Order", &opts).unwrap()).unwrap();
        let json2 = serde_json::to_string(&relevance(&repo, "Order", &opts).unwrap()).unwrap();

        assert_eq!(json1, json2);
    }

    // -------------------------------------------------------------------------
    // S8: Relations scored by id/predicate match
    // -------------------------------------------------------------------------

    #[test]
    fn relevance_relation_predicate_match() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path())
            .with_element("c4:container:a", "A", 0.9, "c4")
            .with_element("c4:container:b", "B", 0.8, "c4")
            .with_edge("rel-a-b", "depends_on", "c4:container:a", "c4:container:b")
            .build();

        let result = relevance(&repo, "depends_on", &RelevanceOptions::default()).unwrap();

        // The edge should appear because its predicate matches
        // and source/target are in the shortlist
        assert!(!result.relations.is_empty() || !result.elements.is_empty());
    }

    // -------------------------------------------------------------------------
    // Stopword drop
    // -------------------------------------------------------------------------

    #[test]
    fn relevance_stopword_token_dropped() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = SeededStore::new(tmp.path())
            .with_element("c4:container:srv", "theservice", 0.9, "c4")
            .build();

        let result = relevance(&repo, "the", &RelevanceOptions::default()).unwrap();

        // "the" is a stopword and should be dropped, so no match
        // The element should not be found
        assert!(result.elements.is_empty() || result.selection_trace.seeds_matched == 0);
    }
}
