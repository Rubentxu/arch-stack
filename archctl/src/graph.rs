use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::filesystem::Filesystem;
use crate::migrations::{self, SCHEMA_MARKER_FILENAME};
use crate::store::open_admin_session;

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

#[derive(Debug, Clone)]
pub struct Element {
    pub id: String,
    pub kind_id: String,
    pub category: String,
    pub canonical_key: String,
    pub current_name: String,
    pub current_status: String,
    pub current_confidence: f64,
    pub current_version_id: String,
}

#[derive(Debug, Clone)]
pub struct ElementVersion {
    pub id: String,
    pub element_id: String,
    pub name: String,
    pub status: String,
    pub origin: String,
    pub confidence: f64,
    pub props: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct StructuralEvidence {
    pub id: String,
    pub kind: String,
    pub claim: String,
    pub file: String,
    pub line: u64,
    pub confidence: f64,
    pub rule_id: String,
    pub props: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct ElementRow {
    pub id: String,
    pub kind_id: String,
    pub category: String,
    pub canonical_key: String,
    pub current_name: String,
    pub current_status: String,
    pub current_confidence: f64,
    pub current_version_id: String,
}

#[derive(Debug, Clone)]
pub struct SemanticEdgeRow {
    pub relation_id: String,
    pub predicate_id: String,
    pub source_id: String,
    pub target_id: String,
    pub order_key: String,
    pub props: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct VersionPropsRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub props: serde_json::Map<String, serde_json::Value>,
}

/// A row from the SemanticRelation node table.
///
/// Mirrors `ElementRow` for the relation side of the graph.
/// Used by the `explain` use case to read relation metadata.
#[derive(Debug, Clone)]
pub struct RelationRow {
    pub id: String,
    pub current_version_id: String,
    pub current_label: String,
}

pub fn database_path(project_dir: &Path) -> PathBuf {
    project_dir.join("architecture.lbdb")
}

pub fn init(project_dir: &Path, fs: &dyn Filesystem) -> Result<PathBuf> {
    let path = database_path(project_dir);
    fs.create_dir_all(project_dir)
        .with_context(|| format!("mkdir {}", project_dir.display()))?;
    // Open a session directly without acquiring the LbugStore flock
    // (graph.rs is the admin boundary — see ADR-010). Multiple parallel
    // `archctl graph *` admin commands may run; they don't need to be
    // serialized against each other or against regular writes.
    let session = open_admin_session(&path)?;
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
    let _ = fs;
    let session = open_admin_session(&database_path(project_dir))?;
    let conn = &session.conn;
    Ok(GraphStat {
        elements: admin_count(conn, "MATCH (:Element) RETURN count(*)")?,
        relations: admin_count(conn, "MATCH ()-[r:SEMANTIC_EDGE]->() RETURN count(r)")?,
        evidence: admin_count(conn, "MATCH (:Evidence) RETURN count(*)")?,
        metatypes: admin_count(conn, "MATCH (:MetaType) RETURN count(*)")?,
        predicates: admin_count(conn, "MATCH (:Predicate) RETURN count(*)")?,
    })
}

fn admin_count(conn: &lbug::Connection<'_>, cypher: &str) -> Result<i64> {
    let mut result = conn.query(cypher).context("count query")?;
    let v = result
        .next()
        .and_then(|r| r.first().cloned())
        .unwrap_or(lbug::Value::Int64(0));
    Ok(match v {
        lbug::Value::Int64(n) => n,
        lbug::Value::Int32(n) => n as i64,
        lbug::Value::UInt64(n) => n as i64,
        _ => 0,
    })
}

/// Allowlist for ids that are interpolated into Cypher queries.
///
/// `GraphStore::prepare` + `execute` (M51) are now available with
/// parameter binding — but `prepare/execute` returns positional rows
/// without column names (lbug does not expose them through
/// `QueryResult`). `neighbours` needs column names (e.g. `m.id`,
/// `r.predicate`) for the export queries, so it continues to
/// interpolate the id as a string literal via `query`. Any character
/// outside the allowlist must be rejected — otherwise we open the
/// door to Cypher injection (closing the quote, adding `}) RETURN ...`,
/// etc). M51's prepare/execute is reserved for batched writers
/// (call_graph / class_diagram apply paths) that don't need column
/// names in result rows.
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

pub fn query(
    project_dir: &Path,
    cypher: &str,
    fs: &dyn Filesystem,
) -> Result<Vec<serde_json::Value>> {
    let _ = fs;
    let session = open_admin_session(&database_path(project_dir))?;
    debug!(%cypher, "graph query");
    admin_run_query(&session.conn, cypher)
}

pub fn neighbours(
    project_dir: &Path,
    element_id: &str,
    depth: u8,
    fs: &dyn Filesystem,
) -> Result<Vec<serde_json::Value>> {
    let _ = fs;
    let id = validate_identifier(element_id)?;
    let depth = depth.clamp(1, 4) as i64;
    let cypher = format!(
        "MATCH (e:Element {{id: '{id}'}})-[*1..{depth}]-(n) RETURN DISTINCT n.id AS id, labels(n) AS kinds;"
    );
    if depth > 2 {
        warn!(
            depth,
            "graph traversal depth > 2 may be slow on large graphs"
        );
    }
    let session = open_admin_session(&database_path(project_dir))?;
    debug!(%cypher, "graph neighbours");
    admin_run_query(&session.conn, &cypher)
}

fn admin_run_query(conn: &lbug::Connection<'_>, cypher: &str) -> Result<Vec<serde_json::Value>> {
    let result = conn.query(cypher).context("execute query")?;
    let columns = result.get_column_names();
    let mut rows = Vec::new();
    for row in result {
        let mut obj = serde_json::Map::new();
        for (i, col) in columns.iter().enumerate() {
            let value = row
                .get(i)
                .map(admin_value_to_json)
                .unwrap_or(serde_json::Value::Null);
            obj.insert(col.clone(), value);
        }
        rows.push(serde_json::Value::Object(obj));
    }
    Ok(rows)
}

fn admin_value_to_json(v: &lbug::Value) -> serde_json::Value {
    match v {
        lbug::Value::Null(_) => serde_json::Value::Null,
        lbug::Value::Bool(b) => serde_json::Value::Bool(*b),
        lbug::Value::Int8(n) => serde_json::Value::from(*n),
        lbug::Value::Int16(n) => serde_json::Value::from(*n),
        lbug::Value::Int32(n) => serde_json::Value::from(*n),
        lbug::Value::Int64(n) => serde_json::Value::from(*n),
        lbug::Value::UInt8(n) => serde_json::Value::from(*n),
        lbug::Value::UInt16(n) => serde_json::Value::from(*n),
        lbug::Value::UInt32(n) => serde_json::Value::from(*n),
        lbug::Value::UInt64(n) => serde_json::Value::from(*n),
        lbug::Value::Int128(n) => serde_json::Value::from(n.to_string()),
        lbug::Value::Float(n) => serde_json::Number::from_f64(*n as f64)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        lbug::Value::Double(n) => serde_json::Number::from_f64(*n)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        lbug::Value::Date(d) => serde_json::Value::from(d.to_string()),
        lbug::Value::Interval(d) => serde_json::Value::from(d.to_string()),
        lbug::Value::Timestamp(t)
        | lbug::Value::TimestampTz(t)
        | lbug::Value::TimestampNs(t)
        | lbug::Value::TimestampMs(t)
        | lbug::Value::TimestampSec(t) => serde_json::Value::from(t.to_string()),
        lbug::Value::String(s) => serde_json::Value::from(s.as_str()),
        lbug::Value::Json(j) => j.clone(),
        lbug::Value::Blob(b) => serde_json::Value::from(format!("<blob {} bytes>", b.len())),
        lbug::Value::List(_, list) | lbug::Value::Array(_, list) => {
            serde_json::Value::Array(list.iter().map(admin_value_to_json).collect())
        }
        lbug::Value::Struct(fields) => {
            let mut obj = serde_json::Map::new();
            for (k, vv) in fields {
                obj.insert(k.clone(), admin_value_to_json(vv));
            }
            serde_json::Value::Object(obj)
        }
        lbug::Value::Map(_, entries) => {
            let mut obj = serde_json::Map::new();
            for (k, vv) in entries {
                obj.insert(admin_value_to_json(k).to_string(), admin_value_to_json(vv));
            }
            serde_json::Value::Object(obj)
        }
        lbug::Value::RecursiveRel { .. } => serde_json::Value::from("<recursive_rel>"),
        lbug::Value::Union { value, .. } => admin_value_to_json(value),
        lbug::Value::UUID(u) => serde_json::Value::from(u.to_string()),
        lbug::Value::Decimal(d) => serde_json::Value::from(d.to_string()),
        lbug::Value::Node(n) => serde_json::Value::from(format!("<node {}>", n)),
        lbug::Value::Rel(r) => serde_json::Value::from(format!("<rel {}>", r)),
        lbug::Value::InternalID(id) => serde_json::Value::from(id.to_string()),
    }
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
        let session = open_admin_session(&database_path(&project)).unwrap();
        session
            .conn
            .query("CREATE (:MetaType {id: 'mt.system', namespace: 'c4', name: 'system'});")
            .unwrap();
        let rows = query(
            &project,
            "MATCH (m:MetaType) RETURN m.id, m.name ORDER BY m.id;",
            &fs,
        )
        .unwrap();
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
        // P2-09b: fresh-graph marker advances to v4-p2-09b-create-obs-clm-tables.
        assert_eq!(text.trim(), "v4-p2-09b-create-obs-clm-tables");
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
        let session = open_admin_session(&database_path(&project)).unwrap();
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
