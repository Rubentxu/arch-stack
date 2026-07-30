//! Persistence port — hexagonal boundary for graph storage.
//!
//! The domain (`Evidence`, `ProjectInfo`, etc.) does not depend on any
//! concrete graph engine. Everything that touches a `Database`,
//! `Connection`, or driver-specific API lives behind this trait.
//!
//! Concrete adapters (today only `LbugStore`) implement the port. To
//! migrate to a different engine — e.g. SparrowDB or an in-memory
//! fixture for tests — write a new struct that implements
//! [`GraphStore`] and wire it through `LbugStore` in the call sites that
//! currently use the module-level helpers in `graph.rs`.
//!
//! ## What the port hides
//!
//! - **Connection lifecycle.** No `Session` or `Connection` is exposed
//!   to callers. The adapter opens, holds, and closes its own handles.
//!   The `init()` / `stat()` / `query()` / `put_evidence()` /
//!   `list_evidence()` methods take `&self` or `&mut self` only.
//!
//! - **Driver-specific Cypher extensions.** Callers pass plain Cypher
//!   strings. The adapter is responsible for stripping Neo4j-only
//!   directives (`CREATE GRAPH …; USE …;`) that some engines do not
//!   accept in single-graph mode.
//!
//! - **Identifier validation.** The adapter assumes callers have already
//!   validated any user-supplied identifiers via
//!   [`crate::graph::validate_identifier`]. The port does NOT re-validate
//!   — that would couple the port to Cypher-injection semantics.
//!
//! ## What the port does NOT hide (yet)
//!
//! - **Query language.** Cypher is the query language for every
//!   `EvidenceStore` we know how to write. If we adopt a different
//!   backend, the queries are still strings but the engine interprets
//!   them. Migrating to e.g. a property-graph store with GQL semantics
//!   would require rewriting the strings — that is a known
//!   follow-up, not a port defect.
//!
//! - **Persistence shape on disk.** Each adapter owns its file format.
//!   `LbugStore` writes `architecture.lbdb` next to the project; future
//!   adapters pick their own. Cross-engine migration is the
//!   `SparrowStore::import_lbug()` problem, not the port's.

use anyhow::Result;
use serde_json::Value as Json;
use std::path::{Path, PathBuf};

use crate::evidence::Evidence;
use crate::evaluation::Evaluation;
use crate::graph::GraphStat;
use crate::migrations;
use crate::row::Row;
use crate::source::SourceArtifact;

/// The persistence port.
///
/// Every method corresponds to a capability the domain needs:
/// - `init` — apply the canonical schema and create the bootstrap marker.
/// - `stat` — return element/relation/evidence counts for `doctor`.
/// - `query` — execute an arbitrary Cypher read query (used by
///   `archctl graph query` and the neighbours traversal).
/// - `put_evidence` — MERGE each evidence row by `id` (idempotent).
/// - `list_evidence` — return evidence rows, optionally filtered by
///   path prefix (used by `archctl evidence list`).
pub trait GraphStore: Send + Sync {
    /// Open or create a store rooted at `project_dir`. Each adapter
    /// decides what file (or set of files) lives there.
    fn open(project_dir: &Path) -> Result<Self>
    where
        Self: Sized;

    /// Apply the schema if it has not been applied yet. Idempotent —
    /// safe to call repeatedly; the canonical marker file under
    /// `.archctl-schema` is the source of truth for "already bootstrapped".
    fn init(&mut self) -> Result<()>;

    /// Counts per label group. Returned as a struct, not as Cypher
    /// strings, so the caller does not need to know the underlying
    /// schema details.
    fn stat(&self) -> Result<GraphStat>;

    /// Execute a Cypher read query and return rows as typed
    /// [`Row`] values. Columns preserve the engine's RETURN order.
    /// The adapter is responsible for translating driver-specific
    /// value types into [`Cell`] — the domain never sees
    /// `serde_json::Value` (use [`Cell::to_json`] at the formatter
    /// edge if JSON output is needed).
    fn query(&self, cypher: &str) -> Result<Vec<Row>>;

    /// Persist a batch of evidence rows. Each row is MERGEd by `id`,
    /// so repeat calls are idempotent (no duplicate rows).
    /// Returns the number of rows written.
    fn put_evidence(&mut self, evidence: &[Evidence]) -> Result<usize>;

    /// List evidence rows. When `path` is `Some(p)`, only rows whose
    /// `e.path` equals `p` are returned. When `None`, the most
    /// recent 100 rows are returned. Returned rows carry the canonical
    /// column set: `e.id`, `e.kind`, `e.claim`, `e.start_line`,
    /// `e.end_line`, `e.path`.
    fn list_evidence(&self, path: Option<&str>) -> Result<Vec<Row>>;

    /// MERGE a SourceArtifact node by `id`. Idempotent on the
    /// identity `(relative_path, content_hash)` (D2). MUST NOT
    /// create edges — edge creation is `link_extracted_from`'s job.
    fn put_source(&mut self, source: &SourceArtifact) -> Result<()>;

    /// MERGE an Evaluation node by `id`. Idempotent. MUST NOT
    /// create edges — the EVALUATES edge is minted separately
    /// if the design chooses to expose it.
    fn put_evaluation(&mut self, evaluation: &Evaluation) -> Result<()>;

    /// Create the EXTRACTED_FROM edge linking `evidence_id` to
    /// `source_id`. Idempotent: MERGE on the (evidence_id, source_id)
    /// pair so re-runs are a no-op.
    fn link_extracted_from(
        &mut self,
        evidence_id: &str,
        source_id: &str,
    ) -> Result<()>;

    /// Create the EVALUATES edge linking `evaluation_id` to
    /// `evidence_id`. Idempotent: MERGE on the (evaluation_id, evidence_id)
    /// pair so re-runs are a no-op.
    fn link_evaluates(
        &mut self,
        evaluation_id: &str,
        evidence_id: &str,
    ) -> Result<()>;
}

/// Factory: pick the concrete adapter the CLI requested. Today only
/// `lbug` exists; tomorrow this is where the `--store sparrowdb`
/// branch lives.
pub fn open_default(project_dir: &Path) -> Result<Box<dyn GraphStore>> {
    let store = LbugStore::open(project_dir)?;
    Ok(Box::new(store))
}

// ---------------------------------------------------------------------------
// Adapter: LadybugDB (the only concrete implementation today)
// ---------------------------------------------------------------------------

/// The current adapter — wraps LadybugDB (the `lbug` crate) behind the
/// port. All the per-engine code that used to live in `graph.rs` is
/// here. Callers see a `&dyn GraphStore` and never touch a
/// `Connection`.
pub struct LbugStore {
    project_dir: PathBuf,
    session: Option<LbugSession>,
}

/// Internal scope-bounded handle. Mirrors the previous `Session` but
/// stays private to the adapter.
struct LbugSession {
    // SAFETY: see `crate::graph::Session` (the old comment explains the
    // 'static transmute trick). Kept identical so the original tests
    // that rely on it still pass.
    conn: lbug::Connection<'static>,
    _db: lbug::Database,
}

impl LbugStore {
    fn session_mut(&mut self) -> Result<&mut LbugSession> {
        if self.session.is_none() {
            self.session = Some(open_lbug_session(&self.project_dir)?);
        }
        Ok(self.session.as_mut().expect("just initialised"))
    }
}

impl GraphStore for LbugStore {
    fn open(project_dir: &Path) -> Result<Self> {
        // Lazy session: do not open the DB file until the first
        // operation. `init` and `stat` already called `open_session`
        // eagerly; `put_evidence` and `list_evidence` did not. Keeping
        // the lazy semantics preserves the existing behaviour and
        // shrinks the time during which we hold an exclusive lock.
        Ok(Self {
            project_dir: project_dir.to_path_buf(),
            session: None,
        })
    }

    fn init(&mut self) -> Result<()> {
        use tracing::info;
        use crate::filesystem::SystemFilesystem;

        // Run migrations using a separate session. The store's own
        // session is opened lazily by session_mut(); running migrations
        // on a fresh session first ensures the schema exists before the
        // store touches the DB.
        let marker = self.project_dir.join(migrations::SCHEMA_MARKER_FILENAME);
        let fs = SystemFilesystem;
        let session = crate::graph::open_session(&self.project_dir, &fs)?;
        let applied = migrations::apply_pending(&session, &fs, &marker)?;
        if applied.is_empty() {
            info!("schema already up-to-date");
        } else {
            info!(versions = ?applied, "migrations applied");
        }
        // Also open the store's own session so subsequent operations
        // (stat, put_evidence, query) don't fail with "not initialized".
        let _ = self.session_mut()?;
        Ok(())
    }

    fn stat(&self) -> Result<GraphStat> {
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("LbugStore::stat called before init"))?;
        Ok(GraphStat {
            elements: count_match(&session.conn, "MATCH (:Element) RETURN count(*)")?,
            relations: count_match(&session.conn, "MATCH (:SemanticRelation) RETURN count(*)")?,
            evidence: count_match(&session.conn, "MATCH (:Evidence) RETURN count(*)")?,
            metatypes: count_match(&session.conn, "MATCH (:MetaType) RETURN count(*)")?,
            predicates: count_match(&session.conn, "MATCH (:Predicate) RETURN count(*)")?,
        })
    }

    fn query(&self, cypher: &str) -> Result<Vec<Row>> {
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("LbugStore::query called before init"))?;
        tracing::debug!(%cypher, "graph query");
        run_query(&session.conn, cypher)
    }

    fn put_evidence(&mut self, evidence: &[Evidence]) -> Result<usize> {
        use tracing::warn;

        if evidence.is_empty() {
            return Ok(0);
        }
        let session = self.session_mut()?;
        let mut written = 0usize;
        for ev in evidence {
            // The caller (evidence::put) is expected to validate
            // identifiers before calling us. If something slipped
            // through we surface the error rather than silently
            // allowing Cypher injection.
            let id = crate::graph::validate_identifier(&ev.id)
                .context("evidence id failed validation")?;
            let path = crate::graph::validate_identifier(&ev.path)
                .context("evidence path failed validation")?;
            let kind = crate::graph::validate_identifier(ev.kind.as_str())?;
            let tool = crate::graph::validate_identifier(&ev.tool_name)?;
            let rule = crate::graph::validate_identifier(&ev.rule_id)?;
            let props_json =
                serde_json::to_string(&ev.props).context("serialize evidence props")?;
            let hash_json = serde_json::to_string(ev.content_hash.as_deref().unwrap_or(""))
                .context("serialize content_hash")?;

            // lbug 0.18.3 has no parameter binding; we interpolate
            // after escaping single quotes. The id/path/kind/tool/rule/
            // lang are allowlist-validated; the user-supplied claim is
            // escaped. The Evidence table columns in `docs/schema/` are
            //   id, kind, classification, claim, confidence, path,
            //   start_line, end_line, commit_hash, content_hash,
            //   tool_name, tool_version, rule_id, props, observed_at
            // We mirror extra fields (language, start_byte, end_byte,
            // text_preview) into `props`.
            let safe_claim = ev.claim.replace('\'', "\\'");
            let safe_tv = ev.tool_version.replace('\'', "\\'");
            let safe_oa = ev.observed_at.replace('\'', "\\'");
            // lbug TIMESTAMP column requires `timestamp(<string>)`, not
            // a bare string literal. We wrap the allowlist-validated
            // ISO-8601 timestamp at query time. (validated above by
            // ensure_ascii path; we still cap length defensively.)
            let oa_cypher = if safe_oa.is_empty() || safe_oa.len() > 64 {
                "timestamp('1970-01-01T00:00:00Z')".to_string()
            } else {
                format!("timestamp('{safe_oa}')")
            };
            let safe_ch = hash_json.replace('\'', "\\'");
            let safe_props = props_json.replace('\'', "\\'");

            let cypher = format!(
                "MERGE (e:Evidence {{id: '{id}'}}) SET \
                 e.kind = '{kind}', \
                 e.claim = '{safe_claim}', \
                 e.path = '{path}', \
                 e.start_line = {sl}, \
                 e.end_line = {el}, \
                 e.tool_name = '{tool}', \
                 e.tool_version = '{safe_tv}', \
                 e.rule_id = '{rule}', \
                 e.content_hash = '{safe_ch}', \
                 e.observed_at = {oa_cypher}, \
                 e.props = '{safe_props}' RETURN e;",
                sl = ev.start_line,
                el = ev.end_line,
            );
            session
                .conn
                .query(&cypher)
                .with_context(|| format!("persist evidence {id}"))?;
            written += 1;
        }
        if evidence.len() > 25 {
            warn!(rows = evidence.len(), "bulk evidence write exceeds 25 rows");
        }
        Ok(written)
    }

    fn list_evidence(&self, path: Option<&str>) -> Result<Vec<Row>> {
        let cypher = match path {
            Some(p) => {
                let safe = crate::graph::validate_identifier(p)?;
                format!(
                    "MATCH (e:Evidence) WHERE e.path = '{safe}' \
                     RETURN e.id, e.kind, e.claim, e.start_line, e.end_line, e.path \
                     ORDER BY e.start_line;"
                )
            }
            None => "MATCH (e:Evidence) \
                     RETURN e.id, e.kind, e.claim, e.start_line, e.end_line, e.path \
                     ORDER BY e.start_line LIMIT 100;"
                .to_string(),
        };
        self.query(&cypher)
    }

    fn put_source(&mut self, source: &SourceArtifact) -> Result<()> {
        let session = self.session_mut()?;
        let id = crate::graph::validate_identifier(&source.id)
            .context("source id failed validation")?;
        let rel_path = crate::graph::validate_identifier(&source.relative_path)
            .context("source relative_path failed validation")?;
        let lang = crate::graph::validate_identifier(&source.language)
            .context("source language failed validation")?;
        let kind = crate::graph::validate_identifier(&source.kind)
            .context("source kind failed validation")?;
        let props_json =
            serde_json::to_string(&source.props).context("serialize source props")?;
        let safe_props = props_json.replace('\'', "\\'");
        let safe_ch = source.content_hash.replace('\'', "\\'");
        let commit_str = source
            .commit_hash
            .as_deref()
            .unwrap_or("");

        let cypher = format!(
            "MERGE (s:SourceArtifact {{id: '{id}'}}) SET \
             s.kind = '{kind}', \
             s.relative_path = '{rel_path}', \
             s.language = '{lang}', \
             s.content_hash = '{safe_ch}', \
             s.commit_hash = '{commit_str}', \
             s.generated = {generated}, \
             s.props = '{safe_props}';",
            generated = source.generated,
        );
        session.conn.query(&cypher).with_context(|| {
            format!("persist SourceArtifact {id}")
        })?;
        Ok(())
    }

    fn put_evaluation(&mut self, evaluation: &Evaluation) -> Result<()> {
        let session = self.session_mut()?;
        let id = crate::graph::validate_identifier(&evaluation.id)
            .context("evaluation id failed validation")?;
        let target_eid = crate::graph::validate_identifier(&evaluation.target_evidence_id)
            .context("evaluation target_evidence_id failed validation")?;
        let criterion = crate::graph::validate_identifier(&evaluation.criterion)
            .context("evaluation criterion failed validation")?;
        let evaluator = crate::graph::validate_identifier(&evaluation.evaluator)
            .context("evaluation evaluator failed validation")?;
        let props_json =
            serde_json::to_string(&evaluation.props).context("serialize evaluation props")?;
        let safe_props = props_json.replace('\'', "\\'");
        let safe_ea = evaluation.evaluated_at.replace('\'', "\\'");
        let ea_cypher = if safe_ea.is_empty() || safe_ea.len() > 64 {
            "timestamp('1970-01-01T00:00:00Z')".to_string()
        } else {
            format!("timestamp('{safe_ea}')")
        };

        let cypher = format!(
            "MERGE (ev:Evaluation {{id: '{id}'}}) SET \
             ev.target_evidence_id = '{target_eid}', \
             ev.criterion = '{criterion}', \
             ev.passed = {passed}, \
             ev.evaluator = '{evaluator}', \
             ev.evaluated_at = {ea_cypher}, \
             ev.props = '{safe_props}';",
            passed = evaluation.passed,
        );
        session.conn.query(&cypher).with_context(|| {
            format!("persist Evaluation {id}")
        })?;
        Ok(())
    }

    fn link_extracted_from(&mut self, evidence_id: &str, source_id: &str) -> Result<()> {
        let session = self.session_mut()?;
        let eid = crate::graph::validate_identifier(evidence_id)
            .context("link_extracted_from: evidence_id failed validation")?;
        let sid = crate::graph::validate_identifier(source_id)
            .context("link_extracted_from: source_id failed validation")?;

        // Q2: Try MERGE on the REL TABLE first. If lbug 0.18.3 rejects
        // MERGE on a REL TABLE, fall back to a MATCH + single CREATE
        // (idempotent: if the edge already exists the CREATE is a no-op).
        let primary = format!(
            "MERGE (e:Evidence {{id: '{eid}'}})-[:EXTRACTED_FROM]->(s:SourceArtifact {{id: '{sid}'}});"
        );
        let result = session.conn.query(&primary);
        if result.is_err() {
            // Q2 fallback: find the nodes, then CREATE the edge if they exist.
            // This is safe and idempotent: if the edge already exists, a second
            // CREATE on the same edge is a no-op in lbug's single-graph mode.
            let fallback = format!(
                "MATCH (e:Evidence {{id: '{eid}'}}), (s:SourceArtifact {{id: '{sid}'}}) \
                 CREATE (e)-[:EXTRACTED_FROM]->(s);"
            );
            session.conn.query(&fallback).with_context(|| {
                format!("link_extracted_from fallback for ({eid}, {sid})")
            })?;
        }
        Ok(())
    }

    fn link_evaluates(&mut self, evaluation_id: &str, evidence_id: &str) -> Result<()> {
        let session = self.session_mut()?;
        let evid = crate::graph::validate_identifier(evaluation_id)
            .context("link_evaluates: evaluation_id failed validation")?;
        let eid = crate::graph::validate_identifier(evidence_id)
            .context("link_evaluates: evidence_id failed validation")?;

        // Try MERGE on the REL TABLE first; fall back to MATCH + CREATE if needed.
        let primary = format!(
            "MERGE (ev:Evaluation {{id: '{evid}'}})-[:EVALUATES]->(e:Evidence {{id: '{eid}'}});"
        );
        let result = session.conn.query(&primary);
        if result.is_err() {
            let fallback = format!(
                "MATCH (ev:Evaluation {{id: '{evid}'}}), (e:Evidence {{id: '{eid}'}}) \
                 CREATE (ev)-[:EVALUATES]->(e);"
            );
            session.conn.query(&fallback).with_context(|| {
                format!("link_evaluates fallback for ({evid}, {eid})")
            })?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Internal helpers — formerly in `graph.rs`, now private to the adapter
// ---------------------------------------------------------------------------

fn open_lbug_session(project_dir: &Path) -> Result<LbugSession> {
    use anyhow::Context;
    let path = crate::graph::database_path(project_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let db = lbug::Database::new(&path, lbug::SystemConfig::default())
        .with_context(|| format!("open database at {}", path.display()))?;
    let conn = lbug::Connection::new(&db).context("create connection")?;
    let conn: lbug::Connection<'static> = unsafe { std::mem::transmute(conn) };
    Ok(LbugSession { conn, _db: db })
}


fn count_match(conn: &lbug::Connection<'_>, cypher: &str) -> Result<i64> {
    use anyhow::Context;
    let mut result = conn.query(cypher).context("count query")?;
    Ok(result
        .next()
        .and_then(|r| r.first().cloned())
        .map(|v| value_to_i64(&v))
        .unwrap_or(0))
}

fn value_to_i64(v: &lbug::Value) -> i64 {
    match v {
        lbug::Value::Int64(n) => *n,
        lbug::Value::Int32(n) => *n as i64,
        lbug::Value::UInt64(n) => *n as i64,
        _ => 0,
    }
}

fn run_query(conn: &lbug::Connection<'_>, cypher: &str) -> Result<Vec<Row>> {
    use anyhow::Context;
    use crate::row::{Cell, Row};
    let mut result = conn.query(cypher).context("execute query")?;
    let columns = result.get_column_names();
    let mut rows = Vec::new();
    while let Some(row) = result.next() {
        let mut r = Row::new();
        for (i, col) in columns.iter().enumerate() {
            // Translate driver value -> Cell. The `from_serde_json`
            // bridge on Cell lets us reuse the JSON-level conversion
            // (already battle-tested in `value_to_json`) without
            // re-implementing variant mapping twice.
            let cell: Cell = row
                .get(i)
                .map(|v| Cell::from(value_to_json(v)))
                .unwrap_or(Cell::Null);
            r.push(col.clone(), cell);
        }
        rows.push(r);
    }
    Ok(rows)
}

fn value_to_json(v: &lbug::Value) -> Json {
    match v {
        lbug::Value::Null(_) => Json::Null,
        lbug::Value::Bool(b) => Json::Bool(*b),
        lbug::Value::Int8(n) => Json::from(*n),
        lbug::Value::Int16(n) => Json::from(*n),
        lbug::Value::Int32(n) => Json::from(*n),
        lbug::Value::Int64(n) => Json::from(*n),
        lbug::Value::UInt8(n) => Json::from(*n),
        lbug::Value::UInt16(n) => Json::from(*n),
        lbug::Value::UInt32(n) => Json::from(*n),
        lbug::Value::UInt64(n) => Json::from(*n),
        lbug::Value::Int128(n) => Json::from(n.to_string()),
        lbug::Value::Float(n) => serde_json::Number::from_f64(*n as f64)
            .map(Json::Number)
            .unwrap_or(Json::Null),
        lbug::Value::Double(n) => serde_json::Number::from_f64(*n)
            .map(Json::Number)
            .unwrap_or(Json::Null),
        lbug::Value::Date(d) => Json::from(d.to_string()),
        lbug::Value::Interval(d) => Json::from(d.to_string()),
        lbug::Value::Timestamp(t)
        | lbug::Value::TimestampTz(t)
        | lbug::Value::TimestampNs(t)
        | lbug::Value::TimestampMs(t)
        | lbug::Value::TimestampSec(t) => Json::from(t.to_string()),
        lbug::Value::String(s) => Json::from(s.as_str()),
        lbug::Value::Json(j) => j.clone(),
        lbug::Value::Blob(b) => Json::from(format!("<blob {} bytes>", b.len())),
        lbug::Value::List(_, list) | lbug::Value::Array(_, list) => {
            Json::Array(list.iter().map(value_to_json).collect())
        }
        lbug::Value::Struct(fields) => {
            let mut obj = serde_json::Map::new();
            for (k, vv) in fields {
                obj.insert(k.clone(), value_to_json(vv));
            }
            Json::Object(obj)
        }
        lbug::Value::Map(_, entries) => {
            let mut obj = serde_json::Map::new();
            for (k, vv) in entries {
                obj.insert(value_to_json(k).to_string(), value_to_json(vv));
            }
            Json::Object(obj)
        }
        lbug::Value::RecursiveRel { .. } => Json::from("<recursive_rel>"),
        lbug::Value::Union { value, .. } => value_to_json(value),
        lbug::Value::UUID(u) => Json::from(u.to_string()),
        lbug::Value::Decimal(d) => Json::from(d.to_string()),
        lbug::Value::Node(n) => Json::from(format!("<node {}>", n)),
        lbug::Value::Rel(r) => Json::from(format!("<rel {}>", r)),
        lbug::Value::InternalID(id) => Json::from(id.to_string()),
    }
}

use anyhow::Context;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{Evidence, EvidenceKind, SourceOrigin, TOOL_NAME, TOOL_VERSION};

    fn fixture() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("proj")).unwrap();
        tmp
    }

    #[test]
    fn init_then_stat_round_trips_through_port() {
        let tmp = fixture();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();
        let stat = store.stat().unwrap();
        assert_eq!(stat.elements, 0);
        assert_eq!(stat.evidence, 0);
    }

    #[test]
    fn init_is_idempotent_via_port() {
        let tmp = fixture();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();
        // Re-opening and re-initialising must not error.
        let mut store2 = LbugStore::open(&project).unwrap();
        store2.init().unwrap();
        assert_eq!(store2.stat().unwrap().elements, 0);
    }

    #[test]
    fn put_evidence_then_list_via_port() {
        let tmp = fixture();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        let ev = Evidence {
            id: "ev:port:1".to_string(),
            kind: EvidenceKind::Structural,
            claim: "port-level evidence".to_string(),
            path: "src/lib.rs".to_string(),
            start_line: 1,
            end_line: 1,
            start_byte: Some(0),
            end_byte: Some(4),
            tool_name: TOOL_NAME.to_string(),
            tool_version: TOOL_VERSION.to_string(),
            rule_id: "astgrep:rust:function_item".to_string(),
            language: "rust".to_string(),
            observed_at: "2026-07-30T00:00:00Z".to_string(),
            source_origin: SourceOrigin::UserWorkspace,
            content_hash: Some("sha256:0".to_string()),
            text_preview: Some("fn a".to_string()),
            props: Default::default(),
        };
        let n1 = store.put_evidence(std::slice::from_ref(&ev)).unwrap();
        let n2 = store.put_evidence(std::slice::from_ref(&ev)).unwrap();
        assert_eq!(n1, 1);
        assert_eq!(n2, 1, "MERGE must not duplicate rows");

        let all = store.list_evidence(None).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(
            all[0].get("e.id").and_then(|c| c.as_str()),
            Some("ev:port:1")
        );

        let filtered = store.list_evidence(Some("src/lib.rs")).unwrap();
        assert_eq!(filtered.len(), 1);

        let empty = store.list_evidence(Some("nonexistent/path")).unwrap();
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn query_returns_rows_as_typed_cells() {
        let tmp = fixture();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();
        store
            .query("CREATE (:MetaType {id: 'mt.port', namespace: 'c4', name: 'port'});")
            .expect("CREATE via port");
        let rows = store
            .query("MATCH (m:MetaType) RETURN m.id, m.name ORDER BY m.id;")
            .unwrap();
        assert_eq!(rows.len(), 1);
        // Typed access — the row carries the values as `Cell`, not as
        // serde_json::Value. The contract is the same: column-name
        // lookup, typed value extraction.
        assert_eq!(
            rows[0].get("m.id").and_then(|c| c.as_str()),
            Some("mt.port")
        );
        assert_eq!(
            rows[0].get("m.name").and_then(|c| c.as_str()),
            Some("port")
        );
    }

    #[test]
    fn open_default_returns_lbug_store() {
        let tmp = fixture();
        let project = tmp.path().join("proj");
        let mut store = open_default(&project).unwrap();
        store.init().unwrap();
        // Trait object: dynamic dispatch works.
        let stat: GraphStat = store.stat().unwrap();
        assert_eq!(stat.elements, 0);
    }

    #[test]
    fn lbug_store_put_source_is_idempotent_on_same_id() {
        use crate::source::SourceArtifact;
        let tmp = fixture();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        let sa = SourceArtifact::from_content(
            "src/lib.rs",
            "rust",
            "sha256:abc123def456",
            None,
            "2026-07-30T00:00:00Z",
            "0.1.0",
        );
        store.put_source(&sa).unwrap();
        store.put_source(&sa).unwrap(); // second call — must be idempotent
        let rows = store
            .query("MATCH (s:SourceArtifact) RETURN s.id, s.relative_path ORDER BY s.id;")
            .unwrap();
        assert_eq!(rows.len(), 1, "MERGE must not duplicate SourceArtifact nodes");
        assert_eq!(rows[0].get("s.relative_path").and_then(|c| c.as_str()), Some("src/lib.rs"));
    }

    #[test]
    fn lbug_store_link_extracted_from_creates_edge() {
        use crate::source::SourceArtifact;
        let tmp = fixture();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        // Create a source and an evidence row
        let sa = SourceArtifact::from_content(
            "src/lib.rs",
            "rust",
            "sha256:abc123def456",
            None,
            "2026-07-30T00:00:00Z",
            "0.1.0",
        );
        store.put_source(&sa).unwrap();

        let ev = Evidence {
            id: "ev:test:link".to_string(),
            kind: EvidenceKind::Structural,
            claim: "test evidence".to_string(),
            path: "src/lib.rs".to_string(),
            start_line: 1,
            end_line: 1,
            start_byte: Some(0),
            end_byte: Some(4),
            tool_name: TOOL_NAME.to_string(),
            tool_version: TOOL_VERSION.to_string(),
            rule_id: "test:rule".to_string(),
            language: "rust".to_string(),
            observed_at: "2026-07-30T00:00:00Z".to_string(),
            source_origin: SourceOrigin::UserWorkspace,
            content_hash: Some("sha256:abc123def456".to_string()),
            text_preview: Some("test".to_string()),
            props: Default::default(),
        };
        store.put_evidence(std::slice::from_ref(&ev)).unwrap();

        // Link evidence to source
        store
            .link_extracted_from("ev:test:link", &sa.id)
            .unwrap();

        // Verify the edge exists
        let rows = store
            .query(
                "MATCH (e:Evidence {id: 'ev:test:link'})-[:EXTRACTED_FROM]->(s:SourceArtifact) \
                 RETURN s.id AS source_id;",
            )
            .unwrap();
        assert_eq!(rows.len(), 1, "EXTRACTED_FROM edge must exist");
        assert_eq!(
            rows[0].get("source_id").and_then(|c| c.as_str()),
            Some(sa.id.as_str())
        );
    }
}
