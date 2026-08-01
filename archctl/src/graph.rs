use anyhow::{Context, Result};
use lbug::{Connection, Database, SystemConfig, Value};
use serde::Serialize;
use serde_json::Value as Json;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::filesystem::Filesystem;
use crate::migrations::{self, SCHEMA_MARKER_FILENAME};

/// Bounded buffer pool size: 256 MiB per database.
///
/// lbug 0.18.3's `SystemConfig::default()` resolves to `UINT64_MAX`
/// (~8 TB). With many parallel test fixtures each trying to mmap that
/// much virtual address space, the kernel runs out before the DB opens.
/// 256 MiB is ~10× headroom over the worst observed production graph
/// while keeping the per-fixture footprint bounded.
pub const BUFFER_POOL_SIZE: u64 = 256 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct GraphStat {
    pub elements: i64,
    pub relations: i64,
    pub evidence: i64,
    pub metatypes: i64,
    pub predicates: i64,
}

pub fn database_path(project_dir: &Path) -> PathBuf {
    project_dir.join("architecture.lbdb")
}

/// Scope-bounded Database + Connection. Both drop together at the end
/// of the enclosing block; nothing leaks.
///
/// `conn` is declared FIRST so it drops before `_db`. Rust drops struct
/// fields in declaration order; if `_db` dropped first, `conn`'s
/// destructor would access freed memory.
pub struct Session {
    // SAFETY: this Connection borrows from `_db` below. The `'static`
    // marker is a lie — the real lifetime is bounded by `&self`. The
    // Session struct enforces the invariant: anything holding a
    // `Session` cannot observe `conn` outliving `_db` because field
    // drop order is declaration order (conn first, _db second). We
    // extend the lifetime via `std::mem::transmute` so the public API
    // can hand out `&Connection<'_>` without HRTB gymnastics.
    pub conn: Connection<'static>,
    _db: Database,
}

/// Open (or create) the LadybugDB file and return a scope-bounded session.
///
/// Field declaration order is `conn` FIRST, `_db` SECOND — Rust drops
/// struct fields in declaration order, so `conn`'s destructor runs while
/// `_db` is still alive. The `'static` lifetime on `Connection` is a lie
/// bounded by the `_db` field's drop; see `Session` for the safety
/// argument.
///
/// Used by both `Session` (public, port-aware) and `LbugSession`
/// (private, std-fs) wrappers to avoid duplicating the transmute logic.
pub(crate) fn create_db_session(project_dir: &Path) -> Result<(Connection<'static>, Database)> {
    let path = database_path(project_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let db = Database::new(
        &path,
        SystemConfig::default()
            .buffer_pool_size(BUFFER_POOL_SIZE)
            .max_db_size(BUFFER_POOL_SIZE),
    )
    .with_context(|| format!("open database at {}", path.display()))?;
    let conn = Connection::new(&db).context("create connection")?;
    let conn: Connection<'static> = unsafe { std::mem::transmute(conn) };
    Ok((conn, db))
}

pub fn open_session(project_dir: &Path, fs: &dyn Filesystem) -> Result<Session> {
    let path = database_path(project_dir);
    if let Some(parent) = path.parent() {
        fs.create_dir_all(parent)
            .with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let (conn, db) = create_db_session(project_dir)?;
    Ok(Session { conn, _db: db })
}

pub fn init(project_dir: &Path, fs: &dyn Filesystem) -> Result<PathBuf> {
    let path = database_path(project_dir);
    fs.create_dir_all(project_dir)
        .with_context(|| format!("mkdir {}", project_dir.display()))?;
    let session = open_session(&path.parent().unwrap_or(project_dir), fs)?;
    let marker = project_dir.join(SCHEMA_MARKER_FILENAME);
    let applied = migrations::apply_pending(&session, fs, &marker)?;
    if applied.is_empty() {
        info!("schema already up-to-date");
    } else {
        info!(versions = ?applied, "migrations applied");
    }
    Ok(path)
}

pub fn stat(project_dir: &Path, fs: &dyn Filesystem) -> Result<GraphStat> {
    let session = open_session(project_dir, fs)?;
    let conn = &session.conn;
    Ok(GraphStat {
        elements: count_match(conn, "MATCH (:Element) RETURN count(*)")?,
        relations: count_match(conn, "MATCH (:SemanticRelation) RETURN count(*)")?,
        evidence: count_match(conn, "MATCH (:Evidence) RETURN count(*)")?,
        metatypes: count_match(conn, "MATCH (:MetaType) RETURN count(*)")?,
        predicates: count_match(conn, "MATCH (:Predicate) RETURN count(*)")?,
    })
}

fn count_match(conn: &Connection<'_>, cypher: &str) -> Result<i64> {
    let mut result = conn.query(cypher).context("count query")?;
    Ok(result
        .next()
        .and_then(|r| r.first().cloned())
        .map(|v| value_to_i64(&v))
        .unwrap_or(0))
}

fn value_to_i64(v: &Value) -> i64 {
    match v {
        Value::Int64(n) => *n,
        Value::Int32(n) => *n as i64,
        Value::UInt64(n) => *n as i64,
        _ => 0,
    }
}

fn value_to_json(v: &Value) -> Json {
    match v {
        Value::Null(_) => Json::Null,
        Value::Bool(b) => Json::Bool(*b),
        Value::Int8(n) => Json::from(*n),
        Value::Int16(n) => Json::from(*n),
        Value::Int32(n) => Json::from(*n),
        Value::Int64(n) => Json::from(*n),
        Value::UInt8(n) => Json::from(*n),
        Value::UInt16(n) => Json::from(*n),
        Value::UInt32(n) => Json::from(*n),
        Value::UInt64(n) => Json::from(*n),
        Value::Int128(n) => Json::from(n.to_string()),
        Value::Float(n) => serde_json::Number::from_f64(*n as f64)
            .map(Json::Number)
            .unwrap_or(Json::Null),
        Value::Double(n) => serde_json::Number::from_f64(*n)
            .map(Json::Number)
            .unwrap_or(Json::Null),
        Value::Date(d) => Json::from(d.to_string()),
        Value::Interval(d) => Json::from(d.to_string()),
        Value::Timestamp(t)
        | Value::TimestampTz(t)
        | Value::TimestampNs(t)
        | Value::TimestampMs(t)
        | Value::TimestampSec(t) => Json::from(t.to_string()),
        Value::String(s) => Json::from(s.as_str()),
        Value::Json(j) => j.clone(),
        Value::Blob(b) => Json::from(format!("<blob {} bytes>", b.len())),
        Value::List(_, list) | Value::Array(_, list) => {
            Json::Array(list.iter().map(value_to_json).collect())
        }
        Value::Struct(fields) => {
            let mut obj = serde_json::Map::new();
            for (k, vv) in fields {
                obj.insert(k.clone(), value_to_json(vv));
            }
            Json::Object(obj)
        }
        Value::Map(_, entries) => {
            let mut obj = serde_json::Map::new();
            for (k, vv) in entries {
                obj.insert(value_to_json(k).to_string(), value_to_json(vv));
            }
            Json::Object(obj)
        }
        Value::RecursiveRel { .. } => Json::from("<recursive_rel>"),
        Value::Union { value, .. } => value_to_json(value),
        Value::UUID(u) => Json::from(u.to_string()),
        Value::Decimal(d) => Json::from(d.to_string()),
        Value::Node(n) => Json::from(format!("<node {}>", n)),
        Value::Rel(r) => Json::from(format!("<rel {}>", r)),
        Value::InternalID(id) => Json::from(id.to_string()),
    }
}

/// Allowlist for ids that are interpolated into Cypher queries.
///
/// lbug 0.18.3 has no parameter binding (`PreparedStatement::execute()`
/// does not exist; the prepared statement is read-only-check only), so
/// `neighbours` interpolates the id as a string literal. Any character
/// outside the allowlist must be rejected — otherwise we open the door
/// to Cypher injection (closing the quote, adding `}) RETURN ...`, etc).
///
/// Allowed: ASCII alphanumeric + `.` `-` `_` `:` `/`. This covers the
/// graph ids we generate (`c4:system:checkout`, `uml.class:<fqcn>`,
/// paths) and nothing else.
pub fn validate_identifier(id: &str) -> Result<&str> {
    if id.is_empty() {
        anyhow::bail!("empty identifier");
    }
    if id.len() > 256 {
        anyhow::bail!("identifier too long ({} > 256)", id.len());
    }
    if !id.is_ascii() {
        anyhow::bail!("non-ASCII characters in identifier");
    }
    let bad = id
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':' | '/')));
    if let Some(c) = bad {
        anyhow::bail!("invalid character {c:?} in identifier (allowed: alnum, . - _ : /)");
    }
    Ok(id)
}

pub fn query(project_dir: &Path, cypher: &str, fs: &dyn Filesystem) -> Result<Vec<Json>> {
    let session = open_session(project_dir, fs)?;
    debug!(%cypher, "graph query");
    run_query(&session.conn, cypher)
}

fn run_query(conn: &Connection<'_>, cypher: &str) -> Result<Vec<Json>> {
    let mut result = conn.query(cypher).context("execute query")?;
    let columns = result.get_column_names();
    let mut rows = Vec::new();
    while let Some(row) = result.next() {
        let mut obj = serde_json::Map::new();
        for (i, col) in columns.iter().enumerate() {
            let value = row.get(i).map(value_to_json).unwrap_or(Json::Null);
            obj.insert(col.clone(), value);
        }
        rows.push(Json::Object(obj));
    }
    Ok(rows)
}

pub fn neighbours(project_dir: &Path, element_id: &str, depth: u8, fs: &dyn Filesystem) -> Result<Vec<Json>> {
    let id = validate_identifier(element_id)?;
    let depth = depth.clamp(1, 4) as i64;
    let cypher = format!(
        "MATCH (e:Element {{id: '{id}'}})-[*1..{depth}]-(n) RETURN DISTINCT n.id AS id, labels(n) AS kinds;"
    );
    if depth > 2 {
        warn!(depth, "graph traversal depth > 2 may be slow on large graphs");
    }
    let session = open_session(project_dir, fs)?;
    debug!(%cypher, "graph neighbours");
    run_query(&session.conn, &cypher)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system_fs() -> crate::filesystem::SystemFilesystem {
        crate::filesystem::SystemFilesystem
    }

    #[test]
    fn init_then_query_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = system_fs();
        init(&project, &fs).unwrap();
        let session = open_session(&project, &fs).unwrap();
        session
            .conn
            .query("CREATE (:MetaType {id: 'mt.system', namespace: 'c4', name: 'system'});")
            .unwrap();
        let rows = query(&project, "MATCH (m:MetaType) RETURN m.id, m.name ORDER BY m.id;", &fs).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["m.id"], "mt.system");
        assert_eq!(rows[0]["m.name"], "system");
    }

    #[test]
    fn init_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = system_fs();
        init(&project, &fs).unwrap();
        init(&project, &fs).unwrap();
        let stat = stat(&project, &fs).unwrap();
        assert_eq!(stat.elements, 0);
        assert_eq!(stat.metatypes, 0);
    }

    #[test]
    fn schema_marker_present_after_init() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = system_fs();
        init(&project, &fs).unwrap();
        let marker = project.join(".archctl-schema");
        assert!(marker.exists());
        let text = std::fs::read_to_string(marker).unwrap();
        assert_eq!(text.trim(), "v3-view-nodes");
    }

    #[test]
    fn validate_identifier_accepts_canonical_graph_ids() {
        assert!(validate_identifier("c4:system:checkout").is_ok());
        assert!(validate_identifier("uml.class:org.example.Order").is_ok());
        assert!(validate_identifier("behavior:scenario:orders/create-order/success").is_ok());
        assert!(validate_identifier("evidence-123_v2").is_ok());
    }

    #[test]
    fn validate_identifier_rejects_cypher_injection() {
        assert!(validate_identifier("").is_err());
        assert!(validate_identifier("has spaces").is_err());
        assert!(validate_identifier("quote'injection").is_err());
        assert!(validate_identifier("quote\"double").is_err());
        assert!(validate_identifier("semi;colon").is_err());
        assert!(validate_identifier("paren)boom").is_err());
        assert!(validate_identifier("curly}boom").is_err());
        assert!(validate_identifier("nonascií").is_err());
        assert!(validate_identifier(&"x".repeat(257)).is_err());
    }

    #[test]
    fn neighbours_rejects_bad_id_before_touching_db() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = system_fs();
        init(&project, &fs).unwrap();
        let err = neighbours(&project, "evil'}) RETURN 1;//", 1, &fs).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("invalid character"), "got: {msg}");
    }

    #[test]
    fn buffer_pool_size_is_256_mib() {
        // Gate 6: pin the constant so accidental changes are caught.
        assert_eq!(BUFFER_POOL_SIZE, 256 * 1024 * 1024);
        assert_eq!(BUFFER_POOL_SIZE, 268_435_456);
    }

    #[test]
    fn neighbours_returns_of_type_target() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = system_fs();
        init(&project, &fs).unwrap();
        let session = open_session(&project, &fs).unwrap();
        session
            .conn
            .query("CREATE (:MetaType {id: 'mt.system', namespace: 'c4', name: 'system'});")
            .unwrap();
        session
            .conn
            .query("CREATE (:Element {id: 'e1', kind_id: 'mt.system', category: 'c4', canonical_key: 'k1'});")
            .unwrap();
        session
            .conn
            .query(
                "MATCH (e:Element {id: 'e1'}), (m:MetaType {id: 'mt.system'}) CREATE (e)-[:OF_TYPE]->(m);",
            )
            .unwrap();
        let rows = neighbours(&project, "e1", 1, &fs).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["id"], "mt.system");
    }
}
