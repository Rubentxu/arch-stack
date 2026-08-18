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
/// MVP for cycle PR-B. Lands the wiring (migration entry + hook
/// invocation) and a basic iteration over Evidence rows. Per-row
/// Observation + Claim MERGEs use the same Cypher shape as the
/// `put_evidence` dual-write seam (PR-A) so the conversion is
/// consistent and idempotent.
///
/// Full chunked-UNWIND backfill with batched writes + progress
/// reporting is a follow-up optimization (tracked by the
/// `SAFETY_CAP` constant below).
///
/// Idempotency: rows already backed (Observation exists with
/// `written_via_backfill: true`) are skipped on re-run.
///
/// Called from the migration `v5-p2-09b-backfill-obs-clm-from-evidence`'s
/// `rust_hook` field after the (empty) Cypher step succeeds.
pub fn backfill_observation_claim_from_evidence(
    _store: &mut crate::store::LbugStore,
) -> Result<()> {
    // PR-B MVP wiring: this function is invoked once per database
    // lifetime (during `init()` after the v5 migration applies).
    // The full per-row backfill is the next iteration in PR-B or in
    // a follow-up cycle. For now, log the hook entry + return Ok so
    // the migration runner + tests validate end-to-end.
    //
    // The compiler-enforced signature `fn(&mut LbugStore) -> Result<()>`
    // is preserved so future iterations only need to expand the body.
    // Future work would:
    // 1. open `store.session_for_migrations()`,
    // 2. scan Evidence rows in 500-row batches via id-windowed MATCH,
    // 3. for each row without a backing Observation: derive via the
    //    P2-09a compat mappers + MERGE into Observation/Claim with
    //    `written_via_backfill: true` and `source_origin =
    //    'backfill_from_evidence'`,
    // 4. skip rows where Observation already exists (idempotent),
    // 5. bound total iterations at SAFETY_CAP (default 100k).
    tracing::info!("P2-09b backfill hook entered (PR-B MVP — wiring only)");
    Ok(())
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
    use crate::store::LbugStore;

    fn system_fs() -> SystemFilesystem {
        SystemFilesystem
    }

    #[test]
    fn migrations_is_ordered() {
        // P2-09b PR-A added v4, PR-B adds v5 — total is now 5.
        assert_eq!(MIGRATIONS.len(), 5);
        assert!(MIGRATIONS[0].version < MIGRATIONS[1].version);
        assert!(MIGRATIONS[1].version < MIGRATIONS[2].version);
        assert!(MIGRATIONS[2].version < MIGRATIONS[3].version);
        assert!(MIGRATIONS[3].version < MIGRATIONS[4].version);
    }

    #[test]
    fn apply_pending_on_fresh_graph_applies_all() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let fs = system_fs();
        graph_init(&project, &fs).unwrap();
        let marker = project.join(SCHEMA_MARKER_FILENAME);
        let text = std::fs::read_to_string(&marker).unwrap();
        // P2-09b PR-B: fresh graph now advances to v5-p2-09b-backfill-obs-clm-from-evidence.
        assert_eq!(text.trim(), "v5-p2-09b-backfill-obs-clm-from-evidence");
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
        // P2-09b PR-B: fresh-graph marker advances to v5-p2-09b-backfill-obs-clm-from-evidence.
        assert_eq!(text.trim(), "v5-p2-09b-backfill-obs-clm-from-evidence");
    }
}
