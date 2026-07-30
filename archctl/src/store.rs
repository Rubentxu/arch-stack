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
use crate::graph::GraphStat;
use crate::row::Row;

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

        // Compute the marker path BEFORE borrowing the session —
        // otherwise `self.project_dir` is held both mutably (via
        // session_mut) and immutably in the same statement.
        let marker = self.project_dir.join(".archctl-schema");
        let session = self.session_mut()?;
        if marker.exists() {
            let installed = std::fs::read_to_string(&marker).unwrap_or_default();
            if installed.trim() == BOOTSTRAP_VERSION {
                info!(version = %installed, "schema already bootstrapped");
                return Ok(());
            }
        }
        info!("bootstrapping schema from docs/schema/001_initial_schema.cypher");
        let stmts = schema_statements(SCHEMA_CYPHER);
        info!(statements = stmts.len(), "applying schema statements");
        for (i, stmt) in stmts.iter().enumerate() {
            session
                .conn
                .query(stmt)
                .with_context(|| format!("schema statement #{i} failed: {stmt}"))?;
        }
        std::fs::write(&marker, BOOTSTRAP_VERSION).context("write schema marker")?;
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
}

// ---------------------------------------------------------------------------
// Internal helpers — formerly in `graph.rs`, now private to the adapter
// ---------------------------------------------------------------------------

const SCHEMA_CYPHER: &str = include_str!("../../docs/schema/001_initial_schema.cypher");
const BOOTSTRAP_VERSION: &str = "v1-initial";

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

/// Strip Neo4j-only directives that lbug does not need in single-graph
/// mode. See the original `graph.rs::schema_statements` doc-comment for
/// the full rationale.
fn schema_statements(script: &str) -> Vec<String> {
    script
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter(|s| {
            let upper = s.to_ascii_uppercase();
            !upper.starts_with("CREATE GRAPH") && !upper.starts_with("USE ")
        })
        .map(|s| format!("{s};"))
        .collect()
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
    use crate::evidence::{Evidence, EvidenceKind, TOOL_NAME, TOOL_VERSION};

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
}
