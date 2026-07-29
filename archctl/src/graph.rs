use anyhow::{Context, Result};
use lbug::{Connection, Database, SystemConfig, Value};
use serde::Serialize;
use serde_json::Value as Json;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

const SCHEMA_CYPHER: &str = include_str!("../../docs/schema/001_initial_schema.cypher");

const BOOTSTRAP_VERSION: &str = "v1-initial";

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

pub fn init(project_dir: &Path) -> Result<PathBuf> {
    let path = database_path(project_dir);
    std::fs::create_dir_all(project_dir)
        .with_context(|| format!("mkdir {}", project_dir.display()))?;
    let db = Database::new(&path, SystemConfig::default())
        .with_context(|| format!("create database at {}", path.display()))?;
    let conn = Connection::new(&db).context("create connection")?;
    let marker = project_dir.join(".archctl-schema");
    if marker.exists() {
        let installed = std::fs::read_to_string(&marker).unwrap_or_default();
        if installed.trim() == BOOTSTRAP_VERSION {
            info!(version = %installed, "schema already bootstrapped");
            return Ok(path);
        }
    }
    info!("bootstrapping schema from docs/schema/001_initial_schema.cypher");
    for stmt in schema_statements(SCHEMA_CYPHER) {
        conn.query(&stmt).with_context(|| format!("apply schema statement: {stmt}"))?;
    }
    std::fs::write(&marker, BOOTSTRAP_VERSION).context("write schema marker")?;
    Ok(path)
}

/// Split a Cypher script into individual statements, stripping
/// `CREATE GRAPH` / `USE <graph>` directives that lbug does not need
/// (single-graph mode).
fn schema_statements(script: &str) -> Vec<String> {
    script
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter(|s| !s.to_ascii_uppercase().starts_with("CREATE GRAPH"))
        .filter(|s| !s.to_ascii_uppercase().starts_with("USE "))
        .map(|s| format!("{s};"))
        .collect()
}

/// Open a fresh Connection on a freshly-leaked Database. Each call leaks
/// a small amount of memory tied to the database handle; this matches the
/// cost of a one-shot CLI command. Long-running processes should switch
/// to a different ownership model.
fn open_conn(project_dir: &Path) -> Result<Connection<'static>> {
    let path = database_path(project_dir);
    if !path.exists() {
        std::fs::create_dir_all(project_dir)
            .with_context(|| format!("mkdir {}", project_dir.display()))?;
    }
    let db = Box::leak(Box::new(
        Database::new(&path, SystemConfig::default())
            .with_context(|| format!("open database at {}", path.display()))?,
    ));
    Connection::new(db).context("create connection")
}

pub fn stat(project_dir: &Path) -> Result<GraphStat> {
    let conn = open_conn(project_dir)?;
    Ok(GraphStat {
        elements: count_match(&conn, "MATCH (:Element) RETURN count(*)")?,
        relations: count_match(&conn, "MATCH (:SemanticRelation) RETURN count(*)")?,
        evidence: count_match(&conn, "MATCH (:Evidence) RETURN count(*)")?,
        metatypes: count_match(&conn, "MATCH (:MetaType) RETURN count(*)")?,
        predicates: count_match(&conn, "MATCH (:Predicate) RETURN count(*)")?,
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

fn validate_identifier(id: &str) -> Result<&str> {
    if id.is_empty() {
        anyhow::bail!("empty identifier");
    }
    if id.len() > 256 {
        anyhow::bail!("identifier too long ({} > 256)", id.len());
    }
    let ok = id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':' | '/'));
    if !ok {
        anyhow::bail!("invalid identifier (allowed: alphanumeric, . - _ : /)");
    }
    Ok(id)
}

pub fn query(project_dir: &Path, cypher: &str) -> Result<Vec<Json>> {
    let conn = open_conn(project_dir)?;
    debug!(%cypher, "graph query");
    run_query(&conn, cypher)
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

pub fn neighbours(project_dir: &Path, element_id: &str, depth: u8) -> Result<Vec<Json>> {
    let id = validate_identifier(element_id)?;
    let depth = depth.clamp(1, 4) as i64;
    let cypher = format!(
        "MATCH (e:Element {{id: '{id}'}})-[*1..{depth}]-(n) RETURN DISTINCT n.id AS id, labels(n) AS kinds;"
    );
    let conn = open_conn(project_dir)?;
    run_query(&conn, &cypher)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_then_query_round_trips() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        init(&project).unwrap();
        let conn = open_conn(&project).unwrap();
        conn.query("CREATE (:MetaType {id: 'mt.system', namespace: 'c4', name: 'system'});")
            .unwrap();
        let rows = query(&project, "MATCH (m:MetaType) RETURN m.id, m.name ORDER BY m.id;").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["m.id"], "mt.system");
        assert_eq!(rows[0]["m.name"], "system");
    }

    #[test]
    fn init_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        init(&project).unwrap();
        init(&project).unwrap();
        let stat = stat(&project).unwrap();
        assert_eq!(stat.elements, 0);
        assert_eq!(stat.metatypes, 0);
    }

    #[test]
    fn schema_marker_present_after_init() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        init(&project).unwrap();
        let marker = project.join(".archctl-schema");
        assert!(marker.exists());
        let text = std::fs::read_to_string(marker).unwrap();
        assert_eq!(text.trim(), BOOTSTRAP_VERSION);
    }

    #[test]
    fn validate_identifier_rejects_bad_chars() {
        assert!(validate_identifier("good_id.with:dots-and-slashes/path").is_ok());
        assert!(validate_identifier("").is_err());
        assert!(validate_identifier("has spaces").is_err());
        assert!(validate_identifier("quote'injection").is_err());
        assert!(validate_identifier("semi;colon").is_err());
    }
}
