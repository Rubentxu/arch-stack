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
use fs2::FileExt;
use serde_json::Value as Json;
use std::path::{Path, PathBuf};

use crate::clock::Clock;
use crate::evidence::{Evidence, EvidenceStatus};
use crate::evaluation::Evaluation;
use crate::graph::GraphStat;
use crate::migrations;
use crate::row::{Cell, Row};
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

    /// Promote Evidence from `Drafted` to `Accepted`.
    ///
    /// Idempotent on already-`Accepted` (returns Ok, no new Evaluation).
    /// Errors if the evidence does not exist.
    /// Errors if the evidence is `Superseded` (must reinstate first).
    /// Side effect (D4): creates Evaluation node + EVALUATES edge
    /// (best-effort audit; Evaluation write failure does NOT roll back
    /// the status flip).
    fn accept_evidence(
        &mut self,
        evidence_id: &str,
        clock: &dyn Clock,
    ) -> Result<()>;

    /// Mark Evidence as `Superseded`. Idempotent on already-`Superseded`.
    ///
    /// Errors if the evidence does not exist.
    /// The caller is responsible for creating the replacement via
    /// `put_evidence` BEFORE invoking this. No Evaluation node is created.
    fn supersede_evidence(&mut self, old_evidence_id: &str) -> Result<()>;

    /// List evidence rows filtered by lifecycle status (D5).
    ///
    /// Returns the same column set as `list_evidence`:
    /// `e.id, e.kind, e.claim, e.start_line, e.end_line, e.path`.
    /// The `e.props` column is fetched for filtering but dropped from
    /// returned rows. Filters in Rust (D6 — no native JSON WHERE in lbug).
    /// When `path` is `Some(p)`, only rows with `e.path = p` are returned.
    /// When `path` is `None`, caps at 100 rows (consistent with `list_evidence`).
    fn list_evidence_by_status(
        &self,
        status: EvidenceStatus,
        path: Option<&str>,
    ) -> Result<Vec<Row>>;

    /// MERGE a Diagram node by `id`. Idempotent — re-running with the
    /// same id and a different revision updates in place.
    fn put_diagram(&mut self, diagram: &crate::diagram::view_types::Diagram) -> Result<()>;

    /// Fetch a Diagram by `id`. Errors if not found.
    fn get_diagram(&self, id: &str) -> Result<crate::diagram::view_types::Diagram>;

    /// MERGE a ViewMember node by `id`. Idempotent.
    fn put_view_member(&mut self, member: &crate::diagram::view_types::ViewMember) -> Result<()>;

    /// Create MEMBER_OF edge. Idempotent via MATCH+CREATE fallback
    /// (lbug 0.18.3 rejects MERGE on REL TABLE).
    fn link_member_of(&mut self, member_id: &str, diagram_id: &str) -> Result<()>;

    /// Create RENDERS edge. Idempotent via MATCH+CREATE fallback.
    fn link_renders(&mut self, member_id: &str, element_id: &str) -> Result<()>;

    /// MERGE a ViewGroup node by `id`. Idempotent.
    fn put_view_group(&mut self, group: &crate::diagram::view_types::ViewGroup) -> Result<()>;

    /// Create GROUP_CONTAINS edge. Idempotent via MATCH+CREATE fallback.
    fn link_group_contains(&mut self, group_id: &str, member_id: &str) -> Result<()>;

    /// Fetch all ViewMembers for a given diagram_id.
    fn get_view_members(&self, diagram_id: &str) -> Result<Vec<crate::diagram::view_types::ViewMember>>;
}

/// Factory: pick the concrete adapter the CLI requested. Today only
/// `lbug` exists; tomorrow this is where the `--store sparrowdb`
/// branch lives.
pub fn open_default(project_dir: &Path) -> Result<Box<dyn GraphStore>> {
    let store = LbugStore::open(project_dir)?;
    Ok(Box::new(store))
}

// ---------------------------------------------------------------------------
// DB lock errors
// ---------------------------------------------------------------------------

/// Error returned when the project DB is already locked by another process.
#[derive(Debug)]
pub enum LockError {
    /// Another `archctl` process holds the lock.
    AnotherArchctlRunning,
    /// I/O error while acquiring the lock.
    Io(std::io::Error),
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::AnotherArchctlRunning => {
                write!(f, "another archctl process is running for this project")
            }
            LockError::Io(e) => write!(f, "lock I/O error: {e}"),
        }
    }
}

impl std::error::Error for LockError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LockError::AnotherArchctlRunning => None,
            LockError::Io(e) => Some(e),
        }
    }
}

// ---------------------------------------------------------------------------
// Adapter: LadybugDB (the only concrete implementation today)
// ---------------------------------------------------------------------------

/// The current adapter — wraps LadybugDB (the `lbug` crate) behind the
/// port. Callers see a `&dyn GraphStore` and never touch a
/// `Connection`.
pub struct LbugStore {
    project_dir: PathBuf,
    session: Option<LbugSession>,
    /// File descriptor for the exclusive flock on `.lbdb`.
    /// Its Drop releases the kernel-managed lock.
    #[allow(dead_code)]
    lock_fd: std::fs::File,
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
    /// Open (or create) a store, acquiring an exclusive flock on `.lbdb`.
    /// Returns `Err(LockError::AnotherArchctlRunning)` if another process
    /// already holds the lock. The lock is released when the store is dropped.
    pub fn open(project_dir: &Path) -> Result<Self, LockError> {
        let lock_path = crate::graph::database_path(project_dir);
        // Ensure the project directory exists before creating the lock file.
        if let Some(parent) = lock_path.parent() {
            std::fs::create_dir_all(parent).map_err(LockError::Io)?;
        }
        let lock_fd = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&lock_path)
            .map_err(LockError::Io)?;
        // Try to acquire an exclusive lock. `WouldBlock` means another
        // process holds it (kernel-managed, no stale recovery code needed).
        match lock_fd.try_lock_exclusive() {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return Err(LockError::AnotherArchctlRunning);
            }
            Err(e) => return Err(LockError::Io(e)),
        }
        Ok(Self {
            project_dir: project_dir.to_path_buf(),
            session: None,
            lock_fd,
        })
    }

    fn session_mut(&mut self) -> Result<&mut LbugSession> {
        if self.session.is_none() {
            self.session = Some(open_lbug_session(&self.project_dir)?);
        }
        Ok(self.session.as_mut().expect("just initialised"))
    }
}

impl GraphStore for LbugStore {
    fn open(project_dir: &Path) -> Result<Self> {
        LbugStore::open(project_dir)
            .map_err(|e| anyhow::anyhow!("failed to acquire DB lock: {e}"))
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

    fn accept_evidence(
        &mut self,
        evidence_id: &str,
        clock: &dyn Clock,
    ) -> Result<()> {
        let session = self.session_mut()?;

        // Step 1: read current props
        let eid = crate::graph::validate_identifier(evidence_id)
            .context("accept_evidence: evidence_id failed validation")?;
        let read_cypher = format!(
            "MATCH (e:Evidence {{id: '{eid}'}}) RETURN e.props;"
        );
        let rows = run_query(&session.conn, &read_cypher)
            .with_context(|| format!("accept_evidence: failed to read {eid}"))?;
        if rows.is_empty() {
            anyhow::bail!("evidence not found: {eid}");
        }
        // e.props can be stored as a JSON string (Cell::String) or as a
        // parsed JSON object (Cell::Object) depending on the engine.
        let props_json: serde_json::Map<String, serde_json::Value> = rows
            .first()
            .and_then(|r| r.get("e.props"))
            .map(cell_to_json_map)
            .unwrap_or_default();

        // Step 2: check current status
        let current = EvidenceStatus::from_props(&props_json);
        if current == EvidenceStatus::Accepted {
            // Idempotent: already accepted
            return Ok(());
        }
        if current == EvidenceStatus::Superseded {
            anyhow::bail!(
                "cannot accept superseded evidence: {eid} — reinstate first"
            );
        }
        // current == Drafted: proceed

        // Step 3: flip status in props
        let mut new_props = props_json;
        new_props.insert(
            "status".to_string(),
            serde_json::Value::String(EvidenceStatus::Accepted.as_str().to_string()),
        );
        let safe_props =
            serde_json::to_string(&new_props).context("serialize updated props")?;
        let safe_props_escaped = safe_props.replace('\'', "\\'");

        // Step 4: write updated props back
        let write_cypher = format!(
            "MATCH (e:Evidence {{id: '{eid}'}}) SET e.props = '{safe_props_escaped}';"
        );
        session
            .conn
            .query(&write_cypher)
            .with_context(|| format!("accept_evidence: failed to update props for {eid}"))?;

        // Step 5: create Evaluation node + EVALUATES edge (best-effort)
        let eval = Evaluation::accept(
            evidence_id,
            "user_accepted",
            "archctl:lifecycle_v1",
            clock,
        );
        // Best-effort: failure here does NOT roll back the status flip
        if let Err(e) = self.put_evaluation(&eval) {
            tracing::warn!(err = %e, eval_id = %eval.id, "accept_evidence: put_evaluation failed, continuing");
        } else if let Err(e) = self.link_evaluates(&eval.id, evidence_id) {
            tracing::warn!(err = %e, eval_id = %eval.id, "accept_evidence: link_evaluates failed, continuing");
        }

        Ok(())
    }

    fn supersede_evidence(&mut self, old_evidence_id: &str) -> Result<()> {
        let session = self.session_mut()?;

        // Step 1: read current props
        let eid = crate::graph::validate_identifier(old_evidence_id)
            .context("supersede_evidence: old_evidence_id failed validation")?;
        let read_cypher = format!(
            "MATCH (e:Evidence {{id: '{eid}'}}) RETURN e.props;"
        );
        let rows = run_query(&session.conn, &read_cypher)
            .with_context(|| format!("supersede_evidence: failed to read {eid}"))?;
        if rows.is_empty() {
            anyhow::bail!("evidence not found: {eid}");
        }
        let props_json: serde_json::Map<String, serde_json::Value> = rows
            .first()
            .and_then(|r| r.get("e.props"))
            .map(cell_to_json_map)
            .unwrap_or_default();

        // Step 2: check current status
        let current = EvidenceStatus::from_props(&props_json);
        if current == EvidenceStatus::Superseded {
            // Idempotent: already superseded
            return Ok(());
        }

        // Step 3: flip status to superseded
        let mut new_props = props_json;
        new_props.insert(
            "status".to_string(),
            serde_json::Value::String(EvidenceStatus::Superseded.as_str().to_string()),
        );
        let safe_props =
            serde_json::to_string(&new_props).context("serialize updated props")?;
        let safe_props_escaped = safe_props.replace('\'', "\\'");

        // Step 4: write updated props back
        let write_cypher = format!(
            "MATCH (e:Evidence {{id: '{eid}'}}) SET e.props = '{safe_props_escaped}';"
        );
        session
            .conn
            .query(&write_cypher)
            .with_context(|| format!("supersede_evidence: failed to update props for {eid}"))?;

        Ok(())
    }

    fn list_evidence_by_status(
        &self,
        status: EvidenceStatus,
        path: Option<&str>,
    ) -> Result<Vec<Row>> {
        // Build the Cypher query — fetch e.props for filtering, plus the 6 canonical columns
        let cypher = match path {
            Some(p) => {
                let safe = crate::graph::validate_identifier(p)?;
                format!(
                    "MATCH (e:Evidence) WHERE e.path = '{safe}' \
                     RETURN e.id, e.kind, e.claim, e.start_line, e.end_line, e.path, e.props \
                     ORDER BY e.start_line;"
                )
            }
            None => "MATCH (e:Evidence) \
                     RETURN e.id, e.kind, e.claim, e.start_line, e.end_line, e.path, e.props \
                     ORDER BY e.start_line LIMIT 100;"
                .to_string(),
        };
        let rows = self.query(&cypher)?;

        // Filter in Rust: keep rows where EvidenceStatus::from_props matches requested status
        let filtered: Vec<Row> = rows
            .into_iter()
            .filter(|r| {
                let props_map: serde_json::Map<String, serde_json::Value> = r
                    .get("e.props")
                    .map(cell_to_json_map)
                    .unwrap_or_default();
                EvidenceStatus::from_props(&props_map) == status
            })
            .map(|mut r| {
                // Drop the e.props column so returned shape matches list_evidence
                r.remove("e.props");
                r
            })
            .collect();

        Ok(filtered)
    }

    fn put_diagram(&mut self, diagram: &crate::diagram::view_types::Diagram) -> Result<()> {
        let session = self.session_mut()?;
        let id = crate::graph::validate_identifier(&diagram.id)
            .context("put_diagram: diagram.id failed validation")?;
        let safe_revision = diagram.revision.replace('\'', "\\'");
        let safe_selector = diagram.selector.replace('\'', "\\'");
        let props_json =
            serde_json::to_string(&diagram.props).context("serialize diagram props")?;
        let safe_props = props_json.replace('\'', "\\'");
        let now = chrono::Utc::now().to_rfc3339();

        let cypher = format!(
            "MERGE (d:Diagram {{id: '{id}'}}) SET \
             d.revision = '{safe_revision}', \
             d.selector = '{safe_selector}', \
             d.props = '{safe_props}', \
             d.updated_at = timestamp('{now}'), \
             d.created_at = COALESCE(d.created_at, timestamp('{now}'));"
        );
        session.conn.query(&cypher).with_context(|| {
            format!("put_diagram: failed to persist Diagram {id}")
        })?;
        Ok(())
    }

    fn get_diagram(&self, id: &str) -> Result<crate::diagram::view_types::Diagram> {
        use crate::diagram::view_types::Diagram;
        let validated_id = crate::graph::validate_identifier(id)
            .context("get_diagram: id failed validation")?;
        let rows = self.query(&format!(
            "MATCH (d:Diagram {{id: '{validated_id}'}}) \
             RETURN d.id, d.revision, d.selector, d.props, d.created_at, d.updated_at;"
        ))?;
        if rows.is_empty() {
            anyhow::bail!("diagram not found: {id}");
        }
        let row = rows.into_iter().next().unwrap();
        let cell_to_str = |col: &str| -> String {
            row.get(col)
                .and_then(|c| c.as_str())
                .map(String::from)
                .unwrap_or_default()
                .replace("\\'", "'")
        };
        let cell_to_json = |col: &str| -> serde_json::Value {
            row.get(col)
                .and_then(|c| c.as_str())
                .map(|s| {
                    // Props are stored escaped, unescape single quotes
                    serde_json::from_str(&s.replace("\\'", "'")).ok()
                })
                .flatten()
                .unwrap_or(serde_json::Value::Null)
        };
        Ok(Diagram {
            id: cell_to_str("d.id"),
            revision: cell_to_str("d.revision"),
            selector: cell_to_str("d.selector"),
            props: cell_to_json("d.props"),
            created_at: Some(cell_to_str("d.created_at")).filter(|s| !s.is_empty()),
            updated_at: Some(cell_to_str("d.updated_at")).filter(|s| !s.is_empty()),
        })
    }

    fn put_view_member(&mut self, member: &crate::diagram::view_types::ViewMember) -> Result<()> {
        let session = self.session_mut()?;
        let id = crate::graph::validate_identifier(&member.id)
            .context("put_view_member: member.id failed validation")?;
        let diagram_id = crate::graph::validate_identifier(&member.diagram_id)
            .context("put_view_member: diagram_id failed validation")?;
        let element_id = crate::graph::validate_identifier(&member.element_id)
            .context("put_view_member: element_id failed validation")?;
        let safe_label = member.label.replace('\'', "\\'");
        let props_json =
            serde_json::to_string(&member.props).context("serialize view_member props")?;
        let safe_props = props_json.replace('\'', "\\'");
        let now = chrono::Utc::now().to_rfc3339();

        let cypher = format!(
            "MERGE (vm:ViewMember {{id: '{id}'}}) SET \
             vm.diagram_id = '{diagram_id}', \
             vm.element_id = '{element_id}', \
             vm.label = '{safe_label}', \
             vm.x = {x}, \
             vm.y = {y}, \
             vm.collapsed = {collapsed}, \
             vm.props = '{safe_props}', \
             vm.updated_at = timestamp('{now}'), \
             vm.created_at = COALESCE(vm.created_at, timestamp('{now}'));",
            x = member.x,
            y = member.y,
            collapsed = member.collapsed,
        );
        session.conn.query(&cypher).with_context(|| {
            format!("put_view_member: failed to persist ViewMember {id}")
        })?;
        Ok(())
    }

    fn link_member_of(&mut self, member_id: &str, diagram_id: &str) -> Result<()> {
        let session = self.session_mut()?;
        let mid = crate::graph::validate_identifier(member_id)
            .context("link_member_of: member_id failed validation")?;
        let did = crate::graph::validate_identifier(diagram_id)
            .context("link_member_of: diagram_id failed validation")?;

        // ADR-017 §"Nota técnica": MERGE on REL TABLE is rejected by lbug 0.18.3.
        // Fall back to MATCH + CREATE (idempotent: if edge exists, second CREATE is no-op).
        let primary = format!(
            "MATCH (vm:ViewMember {{id: '{mid}'}}), (d:Diagram {{id: '{did}'}}) \
             MERGE (vm)-[:MEMBER_OF]->(d);"
        );
        let result = session.conn.query(&primary);
        if result.is_err() {
            let fallback = format!(
                "MATCH (vm:ViewMember {{id: '{mid}'}}), (d:Diagram {{id: '{did}'}}) \
                 CREATE (vm)-[:MEMBER_OF]->(d);"
            );
            session.conn.query(&fallback).with_context(|| {
                format!("link_member_of fallback for ({mid}, {did})")
            })?;
        }
        Ok(())
    }

    fn link_renders(&mut self, member_id: &str, element_id: &str) -> Result<()> {
        let mid = crate::graph::validate_identifier(member_id)
            .context("link_renders: member_id failed validation")?;
        let eid = crate::graph::validate_identifier(element_id)
            .context("link_renders: element_id failed validation")?;

        // ADR-017 §"Nota técnica": MERGE on REL TABLE rejected by lbug 0.18.3.
        // Check element existence first (immutable borrow of self).
        let elem_rows = self.query(&format!(
            "MATCH (e:Element {{id: '{eid}'}}) RETURN e.id;"
        ))?;
        if elem_rows.is_empty() {
            anyhow::bail!("element not found: {eid}");
        }

        // Now acquire mutable session for the edge write.
        let session = self.session_mut()?;
        let primary = format!(
            "MATCH (vm:ViewMember {{id: '{mid}'}}), (e:Element {{id: '{eid}'}}) \
             MERGE (vm)-[:RENDERS]->(e);"
        );
        let result = session.conn.query(&primary);
        if result.is_err() {
            let fallback = format!(
                "MATCH (vm:ViewMember {{id: '{mid}'}}), (e:Element {{id: '{eid}'}}) \
                 CREATE (vm)-[:RENDERS]->(e);"
            );
            session.conn.query(&fallback).with_context(|| {
                format!("link_renders fallback for ({mid}, {eid})")
            })?;
        }
        Ok(())
    }

    fn put_view_group(&mut self, group: &crate::diagram::view_types::ViewGroup) -> Result<()> {
        let session = self.session_mut()?;
        let id = crate::graph::validate_identifier(&group.id)
            .context("put_view_group: group.id failed validation")?;
        let diagram_id = crate::graph::validate_identifier(&group.diagram_id)
            .context("put_view_group: diagram_id failed validation")?;
        let safe_label = group.label.replace('\'', "\\'");
        let props_json =
            serde_json::to_string(&group.props).context("serialize view_group props")?;
        let safe_props = props_json.replace('\'', "\\'");
        let now = chrono::Utc::now().to_rfc3339();

        let cypher = format!(
            "MERGE (vg:ViewGroup {{id: '{id}'}}) SET \
             vg.diagram_id = '{diagram_id}', \
             vg.label = '{safe_label}', \
             vg.props = '{safe_props}', \
             vg.updated_at = timestamp('{now}'), \
             vg.created_at = COALESCE(vg.created_at, timestamp('{now}'));"
        );
        session.conn.query(&cypher).with_context(|| {
            format!("put_view_group: failed to persist ViewGroup {id}")
        })?;
        Ok(())
    }

    fn link_group_contains(&mut self, group_id: &str, member_id: &str) -> Result<()> {
        let session = self.session_mut()?;
        let gid = crate::graph::validate_identifier(group_id)
            .context("link_group_contains: group_id failed validation")?;
        let mid = crate::graph::validate_identifier(member_id)
            .context("link_group_contains: member_id failed validation")?;

        // ADR-017 §"Nota técnica": MERGE on REL TABLE rejected by lbug 0.18.3.
        let primary = format!(
            "MATCH (vg:ViewGroup {{id: '{gid}'}}), (vm:ViewMember {{id: '{mid}'}}) \
             MERGE (vg)-[:GROUP_CONTAINS]->(vm);"
        );
        let result = session.conn.query(&primary);
        if result.is_err() {
            let fallback = format!(
                "MATCH (vg:ViewGroup {{id: '{gid}'}}), (vm:ViewMember {{id: '{mid}'}}) \
                 CREATE (vg)-[:GROUP_CONTAINS]->(vm);"
            );
            session.conn.query(&fallback).with_context(|| {
                format!("link_group_contains fallback for ({gid}, {mid})")
            })?;
        }
        Ok(())
    }

    fn get_view_members(&self, diagram_id: &str) -> Result<Vec<crate::diagram::view_types::ViewMember>> {
        use crate::diagram::view_types::ViewMember;
        let did = crate::graph::validate_identifier(diagram_id)
            .context("get_view_members: diagram_id failed validation")?;
        let rows = self.query(&format!(
            "MATCH (vm:ViewMember) WHERE vm.diagram_id = '{did}' \
             RETURN vm.id, vm.diagram_id, vm.element_id, vm.label, \
                    vm.x, vm.y, vm.collapsed, \
                    vm.props, vm.created_at, vm.updated_at;"
        ))?;
        let members: Vec<ViewMember> = rows
            .into_iter()
            .map(|row| {
                let cell_to_str = |col: &str| -> String {
                    row.get(col)
                        .and_then(|c| c.as_str())
                        .map(String::from)
                        .unwrap_or_default()
                        .replace("\\'", "'")
                };
                let cell_to_i64 = |col: &str| -> i64 {
                    row.get(col).and_then(|c| c.as_i64()).unwrap_or(0)
                };
                let cell_to_bool = |col: &str| -> bool {
                    row.get(col).and_then(|c| c.as_bool()).unwrap_or(false)
                };
                let cell_to_json = |col: &str| -> serde_json::Value {
                    row.get(col)
                        .and_then(|c| c.as_str())
                        .map(|s| serde_json::from_str(&s.replace("\\'", "'")).ok())
                        .flatten()
                        .unwrap_or(serde_json::Value::Null)
                };
                ViewMember {
                    id: cell_to_str("vm.id"),
                    diagram_id: cell_to_str("vm.diagram_id"),
                    element_id: cell_to_str("vm.element_id"),
                    label: cell_to_str("vm.label"),
                    x: cell_to_i64("vm.x"),
                    y: cell_to_i64("vm.y"),
                    collapsed: cell_to_bool("vm.collapsed"),
                    props: cell_to_json("vm.props"),
                    created_at: Some(cell_to_str("vm.created_at")).filter(|s| !s.is_empty()),
                    updated_at: Some(cell_to_str("vm.updated_at")).filter(|s| !s.is_empty()),
                }
            })
            .collect();
        Ok(members)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers — formerly in `graph.rs`, now private to the adapter
// ---------------------------------------------------------------------------

/// Convert a `Cell` value (typically `e.props` from a Cypher result)
/// into a `serde_json::Map<String, serde_json::Value>`. Handles
/// `Cell::Object` (preserve string key-value pairs), `Cell::String`
/// (parse as JSON if valid), and `Cell::Null` (return empty map).
///
/// Only `Object` entries whose value is `Cell::String` are inserted;
/// non-string object values are intentionally skipped because
/// `e.props` payloads today arrive either as parseable JSON strings
/// or as `Object`s with string-typed values. Expansion to `Int`,
/// `Bool`, `Float` is a one-liner inside the inner match when needed.
fn cell_to_json_map(cell: &Cell) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    match cell {
        Cell::Object(kvs) => {
            for (k, v) in kvs {
                if let Cell::String(s) = v {
                    m.insert(k.clone(), serde_json::Value::String(s.clone()));
                }
                // Future: handle Cell::Int, Cell::Bool, etc.
            }
        }
        Cell::String(s) => {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
                if let Some(obj) = parsed.as_object() {
                    return obj.clone();
                }
            }
        }
        Cell::Null => {}
        _ => {}
    }
    m
}

fn open_lbug_session(project_dir: &Path) -> Result<LbugSession> {
    use anyhow::Context;
    let path = crate::graph::database_path(project_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let db = lbug::Database::new(&path, lbug::SystemConfig::default().buffer_pool_size(crate::graph::BUFFER_POOL_SIZE).max_db_size(crate::graph::BUFFER_POOL_SIZE))
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
    use crate::evidence::{Evidence, EvidenceKind, EvidenceStatus, SourceOrigin, TOOL_NAME, TOOL_VERSION};

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
        // Drop the first store to release the lock before re-opening.
        drop(store);
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
            status: EvidenceStatus::Accepted,
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
            status: EvidenceStatus::Accepted,
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

    // -------------------------------------------------------------------------
    // Lifecycle tests (commit 3 of b1-lifecycle-drafted-accepted)
    // -------------------------------------------------------------------------

    fn make_evidence(id: &str, status: EvidenceStatus) -> Evidence {
        Evidence {
            id: id.to_string(),
            kind: EvidenceKind::Structural,
            claim: "test claim".to_string(),
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
            content_hash: Some("sha256:abc123".to_string()),
            text_preview: Some("fn a".to_string()),
            props: {
                let mut m = serde_json::Map::new();
                m.insert(
                    "status".to_string(),
                    serde_json::Value::String(status.as_str().to_string()),
                );
                m
            },
            status,
        }
    }

    #[test]
    fn accept_evidence_promotes_drafted_to_accepted() {
        let tmp = fixture();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        let ev = make_evidence("ev:accept:1", EvidenceStatus::Drafted);
        store.put_evidence(std::slice::from_ref(&ev)).unwrap();

        let clock: &dyn Clock = &crate::clock::FixedClock::new("2026-07-30T12:00:00Z");
        store.accept_evidence("ev:accept:1", clock).unwrap();

        // Verify status is now accepted
        let rows = store
            .query("MATCH (e:Evidence {id: 'ev:accept:1'}) RETURN e.props;")
            .unwrap();
        let props: serde_json::Map<String, serde_json::Value> = rows
            .first()
            .and_then(|r| r.get("e.props"))
            .and_then(|c| match c {
                crate::row::Cell::String(s) => serde_json::from_str(s).ok(),
                crate::row::Cell::Object(fields) => {
                    let mut m = serde_json::Map::new();
                    for (k, v) in fields {
                        let json_val = match v {
                            crate::row::Cell::String(s) => {
                                serde_json::Value::String(s.clone())
                            }
                            crate::row::Cell::Int(n) => {
                                serde_json::Value::Number(serde_json::Number::from(*n))
                            }
                            crate::row::Cell::Bool(b) => serde_json::Value::Bool(*b),
                            crate::row::Cell::Float(f) => {
                                serde_json::Number::from_f64(*f)
                                    .map(serde_json::Value::Number)
                                    .unwrap_or(serde_json::Value::Null)
                            }
                            _ => serde_json::Value::Null,
                        };
                        m.insert(k.clone(), json_val);
                    }
                    Some(serde_json::Map::from(m))
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(
            EvidenceStatus::from_props(&props),
            EvidenceStatus::Accepted
        );
    }

    #[test]
    fn accept_evidence_creates_evaluation_with_user_accepted_criterion() {
        let tmp = fixture();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        let ev = make_evidence("ev:accept:eval", EvidenceStatus::Drafted);
        store.put_evidence(std::slice::from_ref(&ev)).unwrap();

        let clock: &dyn Clock = &crate::clock::FixedClock::new("2026-07-30T12:00:00Z");
        store.accept_evidence("ev:accept:eval", clock).unwrap();

        // Verify Evaluation node was created
        let eval_rows = store
            .query("MATCH (ev:Evaluation) RETURN ev.criterion AS c, ev.passed AS p;")
            .unwrap();
        assert_eq!(eval_rows.len(), 1);
        assert_eq!(
            eval_rows[0].get("c").and_then(|c| c.as_str()),
            Some("user_accepted")
        );
        assert_eq!(
            eval_rows[0].get("p").and_then(|c| c.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn accept_evidence_is_idempotent_on_already_accepted() {
        let tmp = fixture();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        // Start with status = Accepted (already accepted)
        let ev = make_evidence("ev:accept:idemp", EvidenceStatus::Accepted);
        store.put_evidence(std::slice::from_ref(&ev)).unwrap();

        let clock: &dyn Clock = &crate::clock::FixedClock::new("2026-07-30T12:00:00Z");
        // Both calls should be idempotent (early return, no Evaluation created)
        store.accept_evidence("ev:accept:idemp", clock).unwrap();
        store.accept_evidence("ev:accept:idemp", clock).unwrap();

        // Zero Evaluations: accept on already-accepted returns early without creating one
        let eval_rows = store
            .query("MATCH (ev:Evaluation) RETURN count(ev) AS n;")
            .unwrap();
        assert_eq!(
            eval_rows[0].get("n").and_then(|c| c.as_i64()).unwrap_or(0),
            0,
            "accept on already-accepted must not create any Evaluation"
        );
    }

    #[test]
    fn accept_rejects_superseded() {
        let tmp = fixture();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        let ev = make_evidence("ev:accept:superseded", EvidenceStatus::Superseded);
        store.put_evidence(std::slice::from_ref(&ev)).unwrap();

        let clock: &dyn Clock = &crate::clock::FixedClock::new("2026-07-30T12:00:00Z");
        let result = store.accept_evidence("ev:accept:superseded", clock);
        assert!(result.is_err(), "accept on superseded must return error");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("cannot accept superseded evidence"),
            "error message must mention supersession: {err}"
        );
    }

    #[test]
    fn accept_unknown_id_returns_err() {
        let tmp = fixture();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        let clock: &dyn Clock = &crate::clock::FixedClock::new("2026-07-30T12:00:00Z");
        let result = store.accept_evidence("ev:nonexistent", clock);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("evidence not found"),
            "error must say not found: {err}"
        );
    }

    #[test]
    fn supersede_marks_status_superseded() {
        let tmp = fixture();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        let ev = make_evidence("ev:supersede:1", EvidenceStatus::Accepted);
        store.put_evidence(std::slice::from_ref(&ev)).unwrap();

        store.supersede_evidence("ev:supersede:1").unwrap();

        // Verify status is now superseded
        let rows = store
            .query("MATCH (e:Evidence {id: 'ev:supersede:1'}) RETURN e.props;")
            .unwrap();
        let props: serde_json::Map<String, serde_json::Value> = rows
            .first()
            .and_then(|r| r.get("e.props"))
            .and_then(|c| match c {
                crate::row::Cell::String(s) => serde_json::from_str(s).ok(),
                crate::row::Cell::Object(fields) => {
                    let mut m = serde_json::Map::new();
                    for (k, v) in fields {
                        let json_val = match v {
                            crate::row::Cell::String(s) => {
                                serde_json::Value::String(s.clone())
                            }
                            crate::row::Cell::Int(n) => {
                                serde_json::Value::Number(serde_json::Number::from(*n))
                            }
                            crate::row::Cell::Bool(b) => serde_json::Value::Bool(*b),
                            crate::row::Cell::Float(f) => {
                                serde_json::Number::from_f64(*f)
                                    .map(serde_json::Value::Number)
                                    .unwrap_or(serde_json::Value::Null)
                            }
                            _ => serde_json::Value::Null,
                        };
                        m.insert(k.clone(), json_val);
                    }
                    Some(serde_json::Map::from(m))
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(
            EvidenceStatus::from_props(&props),
            EvidenceStatus::Superseded
        );
    }

    #[test]
    fn supersede_is_idempotent() {
        let tmp = fixture();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        let ev = make_evidence("ev:supersede:idemp", EvidenceStatus::Superseded);
        store.put_evidence(std::slice::from_ref(&ev)).unwrap();

        store.supersede_evidence("ev:supersede:idemp").unwrap();
        store.supersede_evidence("ev:supersede:idemp").unwrap(); // second call

        // Should succeed both times (idempotent)
        let rows = store
            .query("MATCH (e:Evidence {id: 'ev:supersede:idemp'}) RETURN e.props;")
            .unwrap();
        let props: serde_json::Map<String, serde_json::Value> = rows
            .first()
            .and_then(|r| r.get("e.props"))
            .and_then(|c| match c {
                crate::row::Cell::String(s) => serde_json::from_str(s).ok(),
                crate::row::Cell::Object(fields) => {
                    let mut m = serde_json::Map::new();
                    for (k, v) in fields {
                        let json_val = match v {
                            crate::row::Cell::String(s) => {
                                serde_json::Value::String(s.clone())
                            }
                            crate::row::Cell::Int(n) => {
                                serde_json::Value::Number(serde_json::Number::from(*n))
                            }
                            crate::row::Cell::Bool(b) => serde_json::Value::Bool(*b),
                            crate::row::Cell::Float(f) => {
                                serde_json::Number::from_f64(*f)
                                    .map(serde_json::Value::Number)
                                    .unwrap_or(serde_json::Value::Null)
                            }
                            _ => serde_json::Value::Null,
                        };
                        m.insert(k.clone(), json_val);
                    }
                    Some(serde_json::Map::from(m))
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(
            EvidenceStatus::from_props(&props),
            EvidenceStatus::Superseded
        );
    }

    #[test]
    fn list_by_status_filters_correctly_and_includes_legacy_rows_as_accepted() {
        let tmp = fixture();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        // Insert mixed-status rows
        store
            .put_evidence(std::slice::from_ref(&make_evidence(
                "ev:status:accepted",
                EvidenceStatus::Accepted,
            )))
            .unwrap();
        store
            .put_evidence(std::slice::from_ref(&make_evidence(
                "ev:status:drafted",
                EvidenceStatus::Drafted,
            )))
            .unwrap();
        store
            .put_evidence(std::slice::from_ref(&make_evidence(
                "ev:status:superseded",
                EvidenceStatus::Superseded,
            )))
            .unwrap();

        // Legacy row: no status key in props — should read as Accepted
        let legacy: Evidence = {
            let mut ev = make_evidence("ev:status:legacy", EvidenceStatus::Accepted);
            ev.props.remove("status");
            ev.status = EvidenceStatus::Accepted;
            ev
        };
        store
            .put_evidence(std::slice::from_ref(&legacy))
            .unwrap();

        // list_evidence_by_status(Accepted) — should return accepted + legacy
        let accepted = store
            .list_evidence_by_status(EvidenceStatus::Accepted, None)
            .unwrap();
        let accepted_ids: Vec<_> = accepted
            .iter()
            .filter_map(|r| r.get("e.id").and_then(|c| c.as_str()))
            .collect();
        assert!(
            accepted_ids.contains(&"ev:status:accepted"),
            "must include accepted row"
        );
        assert!(
            accepted_ids.contains(&"ev:status:legacy"),
            "must include legacy row (read-time default)"
        );
        assert!(
            !accepted_ids.contains(&"ev:status:drafted"),
            "must NOT include drafted row"
        );

        // list_evidence_by_status(Drafted)
        let drafted = store
            .list_evidence_by_status(EvidenceStatus::Drafted, None)
            .unwrap();
        let drafted_ids: Vec<_> = drafted
            .iter()
            .filter_map(|r| r.get("e.id").and_then(|c| c.as_str()))
            .collect();
        assert!(
            drafted_ids.contains(&"ev:status:drafted"),
            "must include drafted row"
        );
        assert_eq!(drafted_ids.len(), 1);

        // list_evidence_by_status(Superseded)
        let superseded = store
            .list_evidence_by_status(EvidenceStatus::Superseded, None)
            .unwrap();
        let superseded_ids: Vec<_> = superseded
            .iter()
            .filter_map(|r| r.get("e.id").and_then(|c| c.as_str()))
            .collect();
        assert!(
            superseded_ids.contains(&"ev:status:superseded"),
            "must include superseded row"
        );
        assert_eq!(superseded_ids.len(), 1);
    }

    #[test]
    fn lbug_store_open_succeeds_when_no_holder() {
        // Opening a fresh project should succeed.
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let result = LbugStore::open(&project);
        assert!(result.is_ok());
    }

    #[test]
    fn lbug_store_open_fails_when_holder_exists() {
        // Hold the lock directly on the .lbdb file, then try to open
        // LbugStore — it should fail with AnotherArchctlRunning.
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        // First open to create the .lbdb file.
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();
        // Drop the store to keep the .lbdb file but release our lock.
        drop(store);
        // Re-open the .lbdb file directly and hold an exclusive lock.
        let lock_path = crate::graph::database_path(&project);
        let holder_fd = std::fs::OpenOptions::new()
            .write(true)
            .open(&lock_path)
            .unwrap();
        holder_fd.try_lock_exclusive().unwrap();
        // Now LbugStore::open should fail because we hold the lock.
        let result = LbugStore::open(&project);
        assert!(matches!(result, Err(LockError::AnotherArchctlRunning)));
    }

    #[test]
    fn put_diagram_is_merge_on_id() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        let diag1 = crate::diagram::view_types::Diagram {
            id: "d1".into(),
            revision: "rev1".into(),
            selector: r#"{"kind":"container"}"#.into(),
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        };
        store.put_diagram(&diag1).unwrap();

        // Update with different revision
        let diag2 = crate::diagram::view_types::Diagram {
            id: "d1".into(),
            revision: "rev2".into(),
            selector: r#"{"kind":"container"}"#.into(),
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        };
        store.put_diagram(&diag2).unwrap();

        // Should have exactly one diagram with rev2
        let rows = store
            .query("MATCH (d:Diagram) RETURN d.id, d.revision;")
            .unwrap();
        assert_eq!(rows.len(), 1, "expected exactly one Diagram row");
        let rev = rows[0]
            .get("d.revision")
            .and_then(|c| c.as_str())
            .unwrap();
        assert_eq!(rev, "rev2", "revision should be updated to rev2");
    }

    #[test]
    fn get_diagram_errors_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        let result = store.get_diagram("nonexistent");
        assert!(result.is_err(), "expected error for missing diagram");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("diagram not found:"),
            "error should contain 'diagram not found:', got: {err_msg}"
        );
    }

    #[test]
    fn put_view_member_is_merge_on_id() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        let vm1 = crate::diagram::view_types::ViewMember {
            id: "vm1".into(),
            diagram_id: "d1".into(),
            element_id: "el1".into(),
            label: "Label1".into(),
            x: 100,
            y: 200,
            collapsed: false,
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        };
        store.put_view_member(&vm1).unwrap();

        // Update with different label
        let vm2 = crate::diagram::view_types::ViewMember {
            id: "vm1".into(),
            diagram_id: "d1".into(),
            element_id: "el1".into(),
            label: "Label2".into(),
            x: 100,
            y: 200,
            collapsed: false,
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        };
        store.put_view_member(&vm2).unwrap();

        // Should have exactly one ViewMember with Label2
        let rows = store
            .query("MATCH (vm:ViewMember) RETURN vm.id, vm.label;")
            .unwrap();
        assert_eq!(rows.len(), 1, "expected exactly one ViewMember row");
        let label = rows[0]
            .get("vm.label")
            .and_then(|c| c.as_str())
            .unwrap();
        assert_eq!(label, "Label2", "label should be updated to Label2");
    }

    #[test]
    fn put_view_member_persists_x_y_collapsed() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        let vm = crate::diagram::view_types::ViewMember {
            id: "vm-pos".into(),
            diagram_id: "d1".into(),
            element_id: "el1".into(),
            label: "Pos".into(),
            x: 240,
            y: 160,
            collapsed: true,
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        };
        store.put_view_member(&vm).unwrap();

        let members = store.get_view_members("d1").unwrap();
        assert_eq!(members.len(), 1, "expected one view member");
        let read = &members[0];
        assert_eq!(read.id, "vm-pos");
        assert_eq!(read.x, 240, "x must persist across put/get");
        assert_eq!(read.y, 160, "y must persist across put/get");
        assert!(read.collapsed, "collapsed must persist across put/get");
    }

    #[test]
    fn link_member_of_is_idempotent_via_fallback() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        // Seed a Diagram and ViewMember.
        store.put_diagram(&crate::diagram::view_types::Diagram {
            id: "d1".into(),
            revision: "r1".into(),
            selector: "{}".into(),
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        }).unwrap();
        store.put_view_member(&crate::diagram::view_types::ViewMember {
            id: "vm1".into(),
            diagram_id: "d1".into(),
            element_id: "el1".into(),
            label: "L".into(),
            x: 0,
            y: 0,
            collapsed: false,
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        }).unwrap();

        // Link twice.
        store.link_member_of("vm1", "d1").unwrap();
        store.link_member_of("vm1", "d1").unwrap();

        // Should have exactly one MEMBER_OF edge.
        let rows = store
            .query("MATCH (vm:ViewMember)-[:MEMBER_OF]->(d:Diagram) RETURN vm.id;")
            .unwrap();
        assert_eq!(rows.len(), 1, "expected exactly one MEMBER_OF edge");
    }

    #[test]
    fn link_renders_errors_when_element_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        // Seed a ViewMember but no Element.
        store.put_view_member(&crate::diagram::view_types::ViewMember {
            id: "vm1".into(),
            diagram_id: "d1".into(),
            element_id: "nonexistent-element".into(),
            label: "L".into(),
            x: 0,
            y: 0,
            collapsed: false,
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        }).unwrap();

        let result = store.link_renders("vm1", "nonexistent-element");
        assert!(result.is_err(), "expected error when element missing");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("element not found:"),
            "error should contain 'element not found:', got: {err_msg}"
        );
    }

    #[test]
    fn link_group_contains_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        // Seed a ViewGroup and ViewMember.
        store.put_view_group(&crate::diagram::view_types::ViewGroup {
            id: "vg1".into(),
            diagram_id: "d1".into(),
            label: "Backend".into(),
            collapsed: false,
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        }).unwrap();
        store.put_view_member(&crate::diagram::view_types::ViewMember {
            id: "vm1".into(),
            diagram_id: "d1".into(),
            element_id: "el1".into(),
            label: "L".into(),
            x: 0,
            y: 0,
            collapsed: false,
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        }).unwrap();

        // Link twice.
        store.link_group_contains("vg1", "vm1").unwrap();
        store.link_group_contains("vg1", "vm1").unwrap();

        // Should have exactly one GROUP_CONTAINS edge.
        let rows = store
            .query("MATCH (vg:ViewGroup)-[:GROUP_CONTAINS]->(vm:ViewMember) RETURN vg.id;")
            .unwrap();
        assert_eq!(rows.len(), 1, "expected exactly one GROUP_CONTAINS edge");
    }

    #[test]
    fn get_view_members_returns_empty_when_no_members() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let mut store = LbugStore::open(&project).unwrap();
        store.init().unwrap();

        // Diagram exists but has no ViewMembers.
        store.put_diagram(&crate::diagram::view_types::Diagram {
            id: "d1".into(),
            revision: "r1".into(),
            selector: "{}".into(),
            props: serde_json::json!({}),
            created_at: None,
            updated_at: None,
        }).unwrap();

        let members = store.get_view_members("d1").unwrap();
        assert!(members.is_empty(), "expected empty vec for diagram with no members");
    }
}
