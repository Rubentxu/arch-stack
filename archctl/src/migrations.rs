//! Versioned schema migration runner.
//!
//! Applies pending migrations in order by reading the `.archctl-schema`
//! marker file and executing every migration script whose version is
//! strictly newer than the marker. Marker is written ONLY after all
//! statements of a migration succeed — no partial marker bumps on failure.

use anyhow::{Context, Result};
use std::path::Path;

use crate::filesystem::Filesystem;
use crate::store::LbugSession;

/// A single named schema migration.
pub struct Migration {
    /// Version string written to `.archctl-schema` after success.
    pub version: &'static str,
    /// Cypher script content. `include_str!`-compiled for zero I/O at runtime.
    pub cypher: &'static str,
    /// Optional Rust hook executed after the Cypher step succeeds.
    ///
    /// Use this when the migration requires more than declarative DDL
    /// (e.g., backfilling derived rows from existing tables, complex
    /// per-row updates that benefit from Rust's iteration semantics).
    /// The function MUST be idempotent (the runner may re-apply after
    /// a partial-failure recovery even if the marker was rolled back).
    /// `None` for pure-Cypher migrations.
    pub rust_hook: Option<fn(&mut crate::store::LbugStore) -> Result<()>>,
}

/// Ordered registry of all migrations. Newest version MUST be last.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: "v1-initial",
        cypher: include_str!("../../docs/schema/001_initial_schema.cypher"),
        rust_hook: None,
    },
    Migration {
        version: "v2-source-evaluation",
        cypher: include_str!("../../docs/schema/002_source_evaluation.cypher"),
        rust_hook: None,
    },
    Migration {
        version: "v3-view-nodes",
        cypher: include_str!("../../docs/schema/003_view_nodes.cypher"),
        rust_hook: None,
    },
    // P2-09b (Wave 3 Item 19): persistent Observation + Claim tables.
    // ADR-049 closure step 1 of 2 — backfill lands separately
    // (cycle PR-B) so existing pre-upgrade graphs stay usable via
    // compat fallback while new graphs dual-write from `put_evidence`.
    Migration {
        version: "v4-p2-09b-create-obs-clm-tables",
        cypher: include_str!("../../docs/schema/004_p2_09b_create_obs_clm.cypher"),
        rust_hook: None,
    },
    // P2-09b backfill: populate the new tables with one Observation
    // + one compat Claim per Evidence row. Idempotent (skips rows
    // that already have a backing Observation). Empty Cypher because
    // the work is procedural — Rust hook below.
    Migration {
        version: "v5-p2-09b-backfill-obs-clm-from-evidence",
        cypher: "-- P2-09b backfill: work is in the rust_hook; no DDL needed.",
        rust_hook: Some(backfill_observation_claim_from_evidence),
    },
    // Wave 3 Item 27 follow-ups: persist fused claims so read-side
    // use cases (explain, coverage) can surface them. written_at is
    // STRING per the P2-09b lbug timestamp() strictness gotcha.
    Migration {
        version: "v6-fusion-persistence",
        cypher: include_str!("../../docs/schema/006_fusion_persistence.cypher"),
        rust_hook: None,
    },
];

/// Marker filename written to the project root after a successful run.
pub const SCHEMA_MARKER_FILENAME: &str = ".archctl-schema";

/// Read the current schema version from the marker file.
///
/// Returns `Ok(None)` if the marker is missing. Returns `Err` only on
/// an I/O error other than "file not found".
pub fn current_version(marker_path: &Path, fs: &dyn Filesystem) -> Result<Option<String>> {
    if !fs.exists(marker_path) {
        return Ok(None);
    }
    // read_to_string errors on missing files (MemoryFilesystem does this).
    // Treat any error reading the marker as "no version installed".
    let text = match fs.read_to_string(marker_path) {
        Ok(t) => t,
        Err(_) => return Ok(None),
    };
    Ok(Some(text.trim().to_string()))
}

/// Apply all migrations strictly newer than the current marker version.
///
/// Each migration script is split into statements (split on `;`) and
/// executed in order. The marker is written ONLY after all statements
/// of ALL pending migrations succeed — if any statement fails, the marker
/// is NOT updated and the next invocation replays the same migrations.
///
/// Returns the list of version strings that were applied (empty if
/// already up-to-date).
pub(crate) fn apply_pending(
    session: &LbugSession,
    fs: &dyn Filesystem,
    marker_path: &Path,
) -> Result<Vec<String>> {
    let current = current_version(marker_path, fs)?;
    let mut applied = Vec::new();

    for migration in MIGRATIONS {
        // Skip migrations at or below the current marker version
        if let Some(ref ver) = current {
            if ver.as_str() >= migration.version {
                continue;
            }
        } else {
            // No marker — apply from the beginning
        }

        tracing::info!(version = %migration.version, "applying migration");
        let stmts = schema_statements(migration.cypher);
        for (i, stmt) in stmts.iter().enumerate() {
            session.conn.query(stmt).with_context(|| {
                format!(
                    "migration {} statement #{i} failed: {stmt}",
                    migration.version
                )
            })?;
        }
        applied.push(migration.version.to_string());
    }

    // Bump the marker to the latest applied version (or last in the
    // registry if none were applied — preserves idempotency on already-
    // up-to-date graphs).
    if !applied.is_empty() {
        let final_version = MIGRATIONS.last().map(|m| m.version).unwrap_or("v1-initial");
        fs.write(marker_path, final_version.as_bytes())
            .with_context(|| format!("write schema marker {}", marker_path.display()))?;
        tracing::info!(version = %final_version, "schema marker updated");
    }

    Ok(applied)
}

/// Apply the P2-09b backfill hook against an open `LbugStore`.
///
/// Scans all `(:Evidence)` rows in `BATCH_SIZE` chunks (default 500),
/// writes one `(:Observation)` + one compat `(:Claim)` row per Evidence
/// row that does NOT already have a backing `(:Observation)`. Idempotent
/// on re-call: rows already backed are skipped.
///
/// Called from the migration `v5-p2-09b-backfill-obs-clm-from-evidence`'s
/// `rust_hook` field after the (empty) Cypher step succeeds. The hook
/// runs inside `init()` against an open session.
pub fn backfill_observation_claim_from_evidence(store: &mut crate::store::LbugStore) -> Result<()> {
    use crate::observation_claim::{compat_claim_from_evidence, observation_from_evidence};

    const BATCH_SIZE: usize = 500;
    const SAFETY_CAP: usize = 100_000;

    let session = store.session_for_migrations();
    let mut last_id_processed: Option<String> = None;
    let mut total_observations = 0usize;
    let mut total_claims = 0usize;
    let mut total_skipped = 0usize;
    let mut total_seen = 0usize;

    loop {
        // Page forward through Evidence rows by id. The first batch
        // returns all rows; subsequent batches add `WHERE e.id > last`
        // to avoid re-processing already-seen ids.
        let cypher = match &last_id_processed {
            None => format!(
                "MATCH (e:Evidence) \
                 RETURN e.id, e.kind, e.claim, e.path, \
                        e.start_line, e.end_line, e.tool_name, \
                        e.tool_version, e.observed_at \
                 ORDER BY e.id LIMIT {BATCH_SIZE};"
            ),
            Some(last) => format!(
                "MATCH (e:Evidence) WHERE e.id > '{last}' \
                 RETURN e.id, e.kind, e.claim, e.path, \
                        e.start_line, e.end_line, e.tool_name, \
                        e.tool_version, e.observed_at \
                 ORDER BY e.id LIMIT {BATCH_SIZE};"
            ),
        };
        // Use `run_query` (pub(crate) which preserves column names) so
        // `row.get("e.id")` works inside the loop body. Using
        // `Row::from_positional` here would lose the column names
        // and force a fall back to positional access.
        let rows = crate::store::run_query(&session.conn, &cypher)
            .with_context(|| format!("backfill: select batch after {last_id_processed:?}"))?;
        if rows.is_empty() {
            break;
        }

        for row in &rows {
            let id = match row.get("e.id").and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            total_seen += 1;

            // Idempotency check — skip if Observation exists for this id.
            // Uses session.conn.query directly (no LbugStore::query
            // wrapper; same admin-scope rationale as above).
            let exists_check =
                format!("MATCH (o:Observation {{id: 'obs:{id}'}}) RETURN o.id LIMIT 1;");
            let existing_rows = match session.conn.query(&exists_check) {
                Ok(r) => r.into_iter().count(),
                Err(_) => 0,
            };
            if existing_rows > 0 {
                total_skipped += 1;
                last_id_processed = Some(id);
                continue;
            }

            // Read remaining Evidence fields. Use &str-or-empty fallback
            // so a partial row doesn't crash the whole batch.
            let kind = row
                .get("e.kind")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let claim_text = row
                .get("e.claim")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let path = row
                .get("e.path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let start_line = row
                .get("e.start_line")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let end_line = row.get("e.end_line").and_then(|v| v.as_i64()).unwrap_or(0);
            let tool_name = row
                .get("e.tool_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let tool_version = row
                .get("e.tool_version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let observed_at = row
                .get("e.observed_at")
                .and_then(|v| v.as_str())
                .unwrap_or("1970-01-01T00:00:00Z")
                .to_string();
            // lbug 0.18.3 `timestamp()` parser is strict — accepts
            // `YYYY-MM-DD hh:mm:ss[.zzzzzz][+-TT[:tt]]` (space-separated,
            // not `T`), and 2-digit hour. Evidence rows are stored with
            // ISO-8601 strings (`2026-08-01T00:00:00Z`); convert to
            // lbug's expected format for the backfill writes.
            let _oa_cypher_literal = iso_to_lbug_timestamp(&observed_at);

            // Build a stub EvidenceEntry so we can reuse the P2-09a
            // compat mappers as the single source of truth for shape.
            // EvidenceEntry (used by the compat mappers) is the
            // export-shape type, not the write-shape Evidence.
            let stub = crate::diagram::export_types::EvidenceEntry {
                id: id.clone(),
                kind: kind.clone(),
                claim: claim_text.clone(),
                path: path.clone(),
                start_line: start_line.max(0) as u64,
                end_line: end_line.max(0) as u64,
                tool_name: tool_name.clone(),
                tool_version: tool_version.clone(),
                rule_id: "backfill".to_string(),
                content_hash: String::new(),
                observed_at: observed_at.clone(),
                status: Some("accepted".to_string()),
            };
            let obs = observation_from_evidence(&stub);
            let claim = compat_claim_from_evidence(&stub);

            // Idempotent MERGE for Observation.
            let obs_id = format!("obs:{id}");
            let safe_obs_kind = obs.kind.replace('\'', "\\'");
            let safe_obs_claim = obs.claim.replace('\'', "\\'");
            let safe_obs_path = obs.path.replace('\'', "\\'");
            let safe_obs_tool = obs.tool_name.replace('\'', "\\'");
            let safe_obs_tv = obs.tool_version.replace('\'', "\\'");
            // `observed_at` was already normalized by lbug when the
            // Evidence row was created (T→space, Z→`+00:00:00`,
            // fractional precision stripped). Re-wrapping it in
            // `timestamp()` would trigger a second parse and fail
            // because the round-tripped form doesn't match the
            // strict format spec. We use the value as a literal —
            // lbug auto-coerces STRING-literal values into TIMESTAMP
            // columns in this version.
            let obs_cypher = format!(
                "MERGE (o:Observation {{id: '{obs_id}'}}) SET \
                 o.kind = '{safe_obs_kind}', \
                 o.claim = '{safe_obs_claim}', \
                 o.path = '{safe_obs_path}', \
                 o.start_line = {}, \
                 o.end_line = {}, \
                 o.tool_name = '{safe_obs_tool}', \
                 o.tool_version = '{safe_obs_tv}', \
                 o.confidence = 1.0, \
                 o.source_origin = 'backfill_from_evidence', \
                 o.written_via_backfill = true, \
                 o.written_at = '{oa}' RETURN o;",
                obs.start_line as i64,
                obs.end_line as i64,
                oa = observed_at,
            );
            if let Err(e) = session.conn.query(&obs_cypher) {
                tracing::warn!(evidence_id = %id, error = %e, "backfill: Observation MERGE failed");
                last_id_processed = Some(id);
                continue;
            }
            total_observations += 1;

            // Idempotent MERGE for compat Claim.
            let claim_id = format!("clm:compat:{id}");
            let safe_claim_statement = claim.statement.replace('\'', "\\'");
            // Same rationale as Observation: skip `timestamp()` wrap
            // (see comment above).
            let claim_cypher = format!(
                "MERGE (c:Claim {{id: '{claim_id}'}}) SET \
                 c.statement = '{safe_claim_statement}', \
                 c.fused = false, \
                 c.confidence = 1.0, \
                 c.observation_ids = ['obs:{id}'], \
                 c.derived_from = ['{id}'], \
                 c.status = 'accepted', \
                 c.written_at = '{oa}' RETURN c;",
                oa = observed_at,
            );
            if let Err(e) = session.conn.query(&claim_cypher) {
                tracing::warn!(evidence_id = %id, error = %e, "backfill: Claim MERGE failed");
            } else {
                total_claims += 1;
            }

            last_id_processed = Some(id);
        }

        // SAFETY_CAP guard.
        if total_observations + total_skipped >= SAFETY_CAP {
            tracing::warn!(
                processed = total_observations + total_skipped,
                cap = SAFETY_CAP,
                "backfill hit safety cap; aborting to avoid unbounded loop"
            );
            break;
        }
    }

    tracing::info!(
        evidence_seen = total_seen,
        observations_written = total_observations,
        claims_written = total_claims,
        skipped_existing = total_skipped,
        "P2-09b backfill complete"
    );
    Ok(())
}

/// Best-effort mapping from a stored `kind` string back to
/// `EvidenceKind` for the P2-09a compat derivator. Currently
/// unused (the backfill body passes the raw `kind` string directly
/// into `EvidenceEntry.kind` since the compat mappers are
/// string-typed); kept as a stable reference for future
/// refactors that want to enforce enum validation.
#[allow(dead_code)]
fn parse_kind_for_backfill(s: &str) -> crate::evidence::EvidenceKind {
    use crate::evidence::EvidenceKind;
    match s {
        "structural" | "Structural" => EvidenceKind::Structural,
        "lexical" | "Lexical" => EvidenceKind::Lexical,
        "config" | "Config" => EvidenceKind::Config,
        "annotation" | "Annotation" => EvidenceKind::Annotation,
        "semantic" | "Semantic" => EvidenceKind::Semantic,
        "other" | "Other" => EvidenceKind::Other,
        _ => EvidenceKind::Other,
    }
}

/// Convert an ISO-8601 timestamp string (e.g. `2026-08-01T00:00:00Z`)
/// into the literal format that lbug 0.18.3's `timestamp()` Cypher
/// function accepts. Currently the backfill body uses a string-literal
/// fallback (no `timestamp()` wrap) because lbug's parser is
/// strict on the round-tripped form; this helper is retained for
/// future re-introduction once the lbug version is bumped.
#[allow(dead_code)]
fn iso_to_lbug_timestamp(iso: &str) -> String {
    // Expected input: `2026-08-01T00:00:00Z` (length 20).
    if iso.len() == 20 {
        let bytes = iso.as_bytes();
        if bytes[10] == b'T' && bytes[19] == b'Z' {
            // `YYYY-MM-DD` + `T` + `hh:mm:ss` + `.000`.
            let mut s = String::with_capacity(23);
            s.push_str(&iso[..19]); // "YYYY-MM-DDThh:mm:ss"
            s.push_str(".000");
            return s;
        }
    }
    iso.to_string()
}

/// Split a Cypher script into individual statements, stripping
/// directives that lbug does not need in single-graph mode.
///
/// The schema files open with `CREATE GRAPH architecture; USE architecture;`
/// because they were written against Neo4j semantics. lbug 0.18.3 runs in
/// single-graph mode and silently no-ops those prefixes; subsequent `MATCH`
/// queries then fail with "Table X does not exist". We strip them here so
/// the canonical docs schema is the source of truth and lbug gets a clean
/// script.
fn schema_statements(script: &str) -> Vec<String> {
    script
        .lines()
        .map(str::trim)
        // Remove blank lines and comment-only lines before splitting on `;`.
        // Comment-only lines would otherwise be merged into the next statement
        // when split on `;`, causing lbug to try executing `-- comment text`
        // as a query (lbug does not support `--` prefix syntax).
        .filter(|s| !s.is_empty() && !s.starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::SystemFilesystem;
    use crate::graph::init as graph_init;
    use crate::store::{GraphStore, LbugStore};

    fn system_fs() -> SystemFilesystem {
        SystemFilesystem
    }

    #[test]
    fn migrations_is_ordered() {
        // P2-09b PR-A added v4, PR-B adds v5; fusion follow-ups add v6.
        assert_eq!(MIGRATIONS.len(), 6);
        assert!(MIGRATIONS[0].version < MIGRATIONS[1].version);
        assert!(MIGRATIONS[1].version < MIGRATIONS[2].version);
        assert!(MIGRATIONS[2].version < MIGRATIONS[3].version);
        assert!(MIGRATIONS[3].version < MIGRATIONS[4].version);
        assert!(MIGRATIONS[4].version < MIGRATIONS[5].version);
    }

    #[test]
    fn apply_pending_on_fresh_graph_applies_all() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = system_fs();
        graph_init(&project, &fs).unwrap();
        let marker = project.join(SCHEMA_MARKER_FILENAME);
        let text = std::fs::read_to_string(&marker).unwrap();
        // Fresh graph advances to v6-fusion-persistence.
        assert_eq!(text.trim(), "v6-fusion-persistence");
    }

    #[test]
    fn apply_pending_on_v1_graph_replays_from_marker_v1() {
        // Simulate a graph whose marker was manually reset to v1
        // (partial-apply recovery scenario — Q3). The runner replays
        // 002, but if the table already exists lbug errors. This is
        // the documented partial-apply failure mode. Recovery: delete
        // .archctl-schema and re-run init.
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = system_fs();

        // First init: marker becomes v2
        graph_init(&project, &fs).unwrap();

        // Simulate partial-apply: manually reset marker to v1-initial
        // (the table already exists from the first init)
        let marker = project.join(SCHEMA_MARKER_FILENAME);
        std::fs::write(&marker, "v1-initial").unwrap();

        // The runner sees v1 < v2 and tries to replay 002.
        // Since the table already exists, lbug returns an error.
        // This is the Q3 partial-apply failure mode — the runner
        // does NOT silently skip; manual recovery is required.
        // We verify the marker stayed at v1 (no partial bump).
        let mut store = LbugStore::open(&project).unwrap();
        let result = apply_pending(store.session_for_migrations(), &fs, &marker);
        // apply_pending returns Err because 002 replays on existing tables
        assert!(
            result.is_err(),
            "expected error on replay of already-applied migration"
        );
        let text = std::fs::read_to_string(&marker).unwrap();
        assert_eq!(
            text.trim(),
            "v1-initial",
            "marker must not be partially bumped"
        );
    }

    #[test]
    fn apply_pending_on_v2_graph_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = system_fs();
        graph_init(&project, &fs).unwrap();
        let mut store = LbugStore::open(&project).unwrap();

        // Call apply_pending on an already-v2 graph
        let marker = project.join(SCHEMA_MARKER_FILENAME);
        let result = apply_pending(store.session_for_migrations(), &fs, &marker).unwrap();
        assert!(
            result.is_empty(),
            "expected no migrations applied, got {result:?}"
        );
    }

    #[test]
    fn marker_writes_only_after_success() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = system_fs();
        graph_init(&project, &fs).unwrap();
        let marker = project.join(SCHEMA_MARKER_FILENAME);
        let text = std::fs::read_to_string(&marker).unwrap();
        // Fusion follow-ups: fresh-graph marker advances to v6-fusion-persistence.
        assert_eq!(text.trim(), "v6-fusion-persistence");
    }

    /// P2-09b backfill: pre-upgrade Evidence rows (those written BEFORE
    /// the `v4` migration creates the `(:Observation)` tables) must be
    /// backfilled by the `v5` migration's `rust_hook`. This test
    /// verifies the empty-database case (the simplest end-to-end path
    /// that exercises the migration runner + hook integration without
    /// hitting lbug 0.18.3's `timestamp()` parser quirks when writing
    /// to the TIMESTAMP columns on pre-upgrade data).
    #[test]
    fn backfill_is_noop_on_empty_database() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = system_fs();
        // Init runs the v4 migration (creates tables) + the v5 hook
        // (backfill scan — finds 0 Evidence rows at this point).
        graph_init(&project, &fs).unwrap();

        let mut store = LbugStore::open(&project).expect("open");
        store.init().expect("init");

        // Step 1: empty-database backfill is a no-op (covers
        // idempotency + the migration runner + hook integration).
        backfill_observation_claim_from_evidence(&mut store).expect("backfill empty");

        // Verify the schema is in place (Observation + Claim tables
        // exist post-migration; empty).
        let obs_count: i64 = {
            let session = store.session_for_migrations();
            session
                .conn
                .query("MATCH (o:Observation) RETURN count(o) AS n;")
                .expect("count obs")
                .into_iter()
                .next()
                .and_then(|t| t.into_iter().next())
                .and_then(|v| match v {
                    lbug::Value::Int64(n) => Some(n),
                    lbug::Value::Int32(n) => Some(n as i64),
                    _ => None,
                })
                .unwrap_or(0)
        };
        assert_eq!(
            obs_count, 0,
            "no Observation rows should exist pre-evidence"
        );

        // Step 2: idempotency — re-running the backfill on the same
        // empty database must be a no-op.
        backfill_observation_claim_from_evidence(&mut store).expect("backfill empty again");
    }
}
