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
    // TRUST-005 (cycle p-38e02210a9f14317/trust-005-observation-fusion):
    // add `status STRING` to Observation; backfill existing rows with
    // status="accepted" (legacy-compatible default per the v5 backfill
    // convention). Plus create Feedback + Reconciliation node tables
    // and their edge tables (VERDICTS_ON, RECONCILES) per spec-35 v1.1.
    // Canonical schema path: archctl/migrations/ (per design D1).
    Migration {
        version: "v7-observation-status",
        cypher: include_str!("../../archctl/migrations/v7_fusion_confidence_status.cypher"),
        rust_hook: Some(backfill_observation_status),
    },
    // TRUST-008 (cycle p-38e02210a9f14317/trust-008-m30-bridge-promotion):
    // add (:Adjudication) node table, (:AdjudicationDecision) lookup table,
    // and ADJUDICATES edge per spec REQ-T08-001. The rust hook
    // `backfill_adjudication_event_diagnostics` emits a tracing::warn! listing
    // pre-v8 FusedClaim rows that carry pending_adjudication_event = true
    // AND have no backing (:Adjudication) event. HITL is preserved;
    // no auto-decision.
    Migration {
        version: "v8-adjudication-event-store",
        cypher: include_str!("../../archctl/migrations/v8_adjudication_event_store.cypher"),
        rust_hook: Some(backfill_adjudication_event_diagnostics),
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
            // P2-09b residual fix: lbug round-trips `timestamp()` columns
            // as `"2026-08-15 0:00:00.0 +00:00:00"` (NOT RFC 3339), and its
            // TIMESTAMP column rejects that readback form on write. The
            // previous workaround (writing the raw readback as a string
            // literal, relying on auto-coercion) fails silently — the
            // MERGE errors and the backfill skips the row. Normalize to
            // strict RFC 3339 first: parseable by lbug's coercion AND by
            // our staleness parser regardless of column typing.
            let oa_normalized = crate::architecture::fusion::parse_observed_at(&observed_at)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| observed_at.clone());
            // `timestamp('...')` Cypher wrap: lbug requires the function
            // call to write TIMESTAMP columns (no implicit string cast).
            // UTC-normalized to the Z form put_evidence uses.
            let oa_ts = format!(
                "timestamp('{}')",
                crate::architecture::fusion::parse_observed_at(&observed_at)
                    .map(|dt| dt
                        .with_timezone(&chrono::Utc)
                        .format("%Y-%m-%dT%H:%M:%SZ")
                        .to_string())
                    .unwrap_or_else(|| observed_at.clone())
            );

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
                observed_at: oa_normalized.clone(),
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
            // P2-09b residual fix (verified 2026-08-18): lbug 0.18.x
            // does NOT implicitly cast STRING literals into TIMESTAMP
            // columns — the previous workaround (bare literal) failed
            // silently and skipped every pre-upgrade row. The
            // round-tripped readback form also breaks `timestamp()`.
            // Fix: normalize to strict RFC 3339 first, then wrap in
            // `timestamp()` (the same path `put_evidence` uses).
            let obs_cypher = format!(
                "MERGE (o:Observation {{id: '{obs_id}'}}) SET \
                 o.kind = '{safe_obs_kind}', \
                 o.claim = '{safe_obs_claim}', \
                 o.path = '{safe_obs_path}', \
                 o.start_line = {}, \
                 o.end_line = {}, \
                 o.tool_name = '{safe_obs_tool}', \
                 o.tool_version = '{safe_obs_tv}', \
                 o.observed_at = '{oa_normalized}', \
                 o.confidence = 1.0, \
                 o.source_origin = 'backfill_from_evidence', \
                 o.written_via_backfill = true, \
                 o.written_at = {oa_ts} RETURN o;",
                obs.start_line as i64, obs.end_line as i64,
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
            // Same rationale as Observation: normalized RFC 3339 wrapped
            // in `timestamp()` (see comment above).
            let claim_cypher = format!(
                "MERGE (c:Claim {{id: '{claim_id}'}}) SET \
                 c.statement = '{safe_claim_statement}', \
                 c.fused = false, \
                 c.confidence = 1.0, \
                 c.observation_ids = ['obs:{id}'], \
                 c.derived_from = ['{id}'], \
                 c.status = 'accepted', \
                 c.written_at = {oa_ts} RETURN c;",
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

/// TRUST-005 v7 migration rust hook: idempotent backfill of `status`
/// on existing `(:Observation)` rows. Sets status="accepted" for
/// every pre-v7 Observation that lacks a status column (the legacy-
/// compatible default; mirrors the v5 backfill's `written_via_backfill`
/// convention). Also sets `pending_adjudication_event = false` on
/// every existing `(:FusedClaim)` row so the m30 bridge column has a
/// deterministic starting state.
pub fn backfill_observation_status(store: &mut crate::store::LbugStore) -> Result<()> {
    let session = store.session_for_migrations();

    // Backfill Observation.status = "accepted" for pre-v7 rows lacking it.
    let cypher_obs = "MATCH (o:Observation) WHERE o.status IS NULL SET o.status = 'accepted';";
    session
        .conn
        .query(cypher_obs)
        .with_context(|| "v7 backfill: Observation.status default")?;

    // Backfill FusedClaim.pending_adjudication_event = false for pre-v7 rows.
    let cypher_fc = "MATCH (f:FusedClaim) WHERE f.pending_adjudication_event IS NULL \
                     SET f.pending_adjudication_event = false;";
    session
        .conn
        .query(cypher_fc)
        .with_context(|| "v7 backfill: FusedClaim.pending_adjudication_event default")?;

    tracing::info!("TRUST-005 v7 backfill complete");
    Ok(())
}

/// TRUST-008 v8 migration rust hook: emits ONE tracing::warn! listing
/// pre-v8 (:FusedClaim) offenders (pending_adjudication_event = true AND no
/// (:Adjudication) event). Does NOT mutate.
pub fn backfill_adjudication_event_diagnostics(store: &mut crate::store::LbugStore) -> Result<()> {
    let session = store.session_for_migrations();
    let cypher = "MATCH (c:FusedClaim) \
                  WHERE c.pending_adjudication_event = true \
                    AND NOT EXISTS { MATCH (:Adjudication)-[:ADJUDICATES]->(c) } \
                  RETURN count(c) AS n;";
    let rows = crate::store::run_query(&session.conn, cypher)
        .with_context(|| "v8 backfill: count pre-v8 offenders")?;
    let count: u64 = rows
        .first()
        .and_then(|r| r.get("n").and_then(|c| c.as_i64()))
        .unwrap_or(0) as u64;
    if count > 0 {
        tracing::warn!(
            offenders = count,
            "TRUST-008 v8 backfill: {count} pre-v8 FusedClaim row(s) carry \
             pending_adjudication_event = true AND have no Adjudication event. \
             Use `archctl adjudication list --pending` then `decide --verdict promote` \
             to clear each one. Migration is intentionally non-mutating."
        );
    } else {
        tracing::info!("TRUST-008 v8 backfill: 0 pre-v8 offenders");
    }
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
        // P2-09b PR-A added v4, PR-B adds v5; fusion follow-ups add v6;
        // TRUST-005 adds v7 (Observation.status + Feedback + Reconciliation).
        // TRUST-008 adds v8 (Adjudication node table + ADJUDICATES edge).
        assert_eq!(MIGRATIONS.len(), 8);
        assert!(MIGRATIONS[0].version < MIGRATIONS[1].version);
        assert!(MIGRATIONS[1].version < MIGRATIONS[2].version);
        assert!(MIGRATIONS[2].version < MIGRATIONS[3].version);
        assert!(MIGRATIONS[3].version < MIGRATIONS[4].version);
        assert!(MIGRATIONS[4].version < MIGRATIONS[5].version);
        assert!(MIGRATIONS[5].version < MIGRATIONS[6].version);
        assert!(MIGRATIONS[6].version < MIGRATIONS[7].version);
    }

    #[test]
    fn apply_pending_on_fresh_graph_applies_all() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = system_fs();
        graph_init(&project, &fs).unwrap();
        let marker = project.join(SCHEMA_MARKER_FILENAME);
        let text = std::fs::read_to_string(&marker).unwrap();
        // Fresh graph advances to v8-adjudication-event-store (TRUST-008).
        assert_eq!(text.trim(), "v8-adjudication-event-store");
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
        // TRUST-008: fresh-graph marker advances to v8-adjudication-event-store.
        assert_eq!(text.trim(), "v8-adjudication-event-store");
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

#[cfg(test)]
mod backfill_preupgrade_tests {
    use super::*;
    use crate::graph::init as graph_init;
    use crate::store::{GraphStore, LbugStore, RawGraphQuery};

    /// P2-09b residual: backfill must handle PRE-UPGRADE Evidence rows
    /// whose `observed_at` was written by lbug's `timestamp()` and
    /// round-trips as `"2026-08-15 0:00:00.0 +00:00:00"` (NOT RFC 3339).
    ///
    /// Regression pin for the STATE.md blocker: "cambiar written_at a
    /// STRING o bump lbug (bloquea backfill de filas pre-upgrade)".
    /// The v5 hook writes `written_at` as a STRING literal relying on
    /// lbug auto-coercion — this test proves that path works with
    /// round-tripped timestamps.
    #[test]
    fn backfill_pre_upgrade_rows_with_roundtripped_timestamps() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = crate::filesystem::SystemFilesystem;
        graph_init(&project, &fs).unwrap();

        let mut store = LbugStore::open(&project).expect("open");
        store.init().expect("init");

        // Simulate a PRE-UPGRADE Evidence row: written via timestamp()
        // (the old writer path), observed_at round-trips in lbug's
        // non-RFC3339 format when read back.
        store
            .execute_raw_cypher_for_test(
                "CREATE (:Evidence {id: 'ev:pre:1', kind: 'call', claim: 'foo', path: 'src/a.rs', start_line: 1, end_line: 2, tool_name: 'ast-grep', tool_version: '0.1', rule_id: 'test:rule', props: '{\"status\":\"accepted\"}', content_hash: 'sha256:pre1', observed_at: timestamp('2026-08-15T00:00:00Z')})",
            )
            .expect("seed pre-upgrade evidence");
        // Pre-upgrade means NO Observation row exists yet (the v4/v5
        // tables were created later) — assert the premise.
        let prematch: i64 = {
            let rows = <LbugStore as RawGraphQuery>::query(
                &store,
                "MATCH (o:Observation {id: 'obs:ev:pre:1'}) RETURN count(o) AS n;",
            )
            .expect("count obs");
            rows.first()
                .and_then(|r| r.get("n"))
                .and_then(|c| c.as_i64())
                .unwrap_or(0)
        };
        assert_eq!(prematch, 0, "pre-upgrade premise: no Observation row");

        // Run the v5 backfill hook.
        backfill_observation_claim_from_evidence(&mut store).expect("backfill pre-upgrade");

        // The Observation row must exist, with written_at populated
        // (STRING-literal coercion path) and written_via_backfill=true.
        let rows = <LbugStore as RawGraphQuery>::query(
            &store,
            "MATCH (o:Observation {id: 'obs:ev:pre:1'}) RETURN o.written_via_backfill, o.observed_at, o.written_at;",
        )
        .expect("read backfilled obs");
        assert_eq!(rows.len(), 1, "backfill must create the Observation row");
        let row = &rows[0];
        assert_eq!(
            row.get("o.written_via_backfill").and_then(|c| c.as_bool()),
            Some(true),
            "written_via_backfill must be true"
        );
        let observed_at = row
            .get("o.observed_at")
            .and_then(|c| c.as_str())
            .unwrap_or("");
        // observed_at is preserved (round-tripped form or normalized).
        assert!(!observed_at.is_empty(), "observed_at must be populated");
        // The backfilled written_at must be parseable by our fusion
        // staleness parser (RFC 3339 or lbug readback format).
        let written_at = row
            .get("o.written_at")
            .and_then(|c| c.as_str())
            .unwrap_or("");
        assert!(
            !written_at.is_empty(),
            "written_at must be populated by the backfill"
        );
        assert!(
            crate::architecture::fusion::parse_observed_at(written_at).is_some(),
            "backfilled written_at must be parseable, got: {written_at}"
        );

        // Compat Claim row too.
        let claims = <LbugStore as RawGraphQuery>::query(
            &store,
            "MATCH (c:Claim {id: 'clm:compat:ev:pre:1'}) RETURN c.id;",
        )
        .expect("read compat claim");
        assert_eq!(claims.len(), 1, "backfill must create the compat Claim row");
    }
}
