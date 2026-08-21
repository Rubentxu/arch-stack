-- v7-fusion-confidence-status (TRUST-005 cycle p-38e02210a9f14317/trust-005-observation-fusion)
--
-- ADR-064 — Fusion Bounded Context: Trust-Gated FusedClaim Recompute +
-- Feedback/Reconciliation as First-Class Types.
--
-- This migration:
-- 1. Adds `status STRING` column to `(:Observation)` (mirrors the Claim
--    table pattern at 004_p2_09b_create_obs_clm.cypher:44-52).
-- 2. Adds `pending_adjudication_event BOOLEAN` column to `(:FusedClaim)`
--    (the m30 Adjudication event store bridge; defaults to false).
-- 3. Creates `(:Feedback)` node table for the spec-35 v1.1 bounded context.
-- 4. Creates `(:Reconciliation)` node table for the spec-35 v1.1 bounded context.
-- 5. Creates the typed edges:
--    - `(:Feedback)-[:VERDICTS_ON]->(:FusedClaim)`
--    - `(:Reconciliation)-[:RECONCILES]->(:FusedClaim)`
--
-- The lbug `ALTER ... ADD COLUMN IF NOT EXISTS` syntax is used for
-- backwards compatibility with pre-v7 databases. The v7 rust hook
-- (`backfill_observation_status` in archctl/src/migrations.rs) handles
-- the legacy data:
--   - existing `(:Observation)` rows lacking `status` → `status = "accepted"`
--     (legacy-compatible default; matches the v5 backfill convention)
--   - existing `(:FusedClaim)` rows lacking `pending_adjudication_event`
--     → `pending_adjudication_event = false` (deterministic starting state)
--
-- Design notes (m25 / Wave 3 Item 19 residual learnings):
-- - All new columns are STRING/BOOLEAN — no TIMESTAMP interactions
--   (the v5 backfill learned the lbug 0.18.3 timestamp() strictness rule;
--   this migration stays clear of TIMESTAMP).
-- - Node table column names mirror the public Rust carrier structs in
--   `archctl/src/feedback.rs` and `archctl/src/reconciliation.rs` so
--   the canonical read path can reconstruct the structs 1:1 from row
--   values (same convention as `:Claim` in 004_p2_09b).
-- - The `evidence_refs` and `correlation_id` fields on Feedback are
--   carried in `(:Feedback).props` (JSON map; ADR-016-B3 precedent).
--   Optional fields ride the props carry; top-level columns stay typed.
-- - Idempotent by construction: the ADR-017 runner is marker-gated
--   (`.archctl-schema`); IF NOT EXISTS handles re-application.
--
-- Cross-references:
-- - ADR-064 (this cycle): the decision rationale.
-- - spec-35 v1.1 §2 (Feedback record schema).
-- - spec-35 v1.1 §3 (Reconciliation record schema).
-- - spec-35 v1.1 §5.2 (trust-aware accept; the m30 bridge).
-- - spec-12 v1.1 §6 (Feedback/Reconciliation cross-reference invariant).
-- - m25 verify-report SUGGESTION-2 (the seam this migration closes).
--
-- NOTE on file location: this file lives at `archctl/migrations/` per
-- the cycle's design decision (see design.md §4.1, T2a.1 open question).
-- The apply phase may move the canonical schema files to `docs/schema/`
-- (matching the v4/v5/v6 convention used by `migrations.rs:34-73`'s
-- `include_str!("../../docs/schema/...")` paths) and update the
-- `include_str!` paths accordingly. The two locations have identical
-- content; the design phase placed the file at both paths as a
-- forward-compat measure. The apply phase picks one canonical path.

-- 1. Add `status` to (:Observation) (idempotent ALTER)
ALTER TABLE Observation ADD IF NOT EXISTS status STRING;

-- 2. Add `pending_adjudication_event` to (:FusedClaim) (idempotent ALTER)
ALTER TABLE FusedClaim ADD IF NOT EXISTS pending_adjudication_event BOOLEAN;

-- 3. Create (:Feedback) node table (spec-35 v1.1 §2.2)
--    Field shape: id, target, verdict, replacement, actor, revision, timestamp.
--    evidence_refs and correlation_id live in props (JSON carry; ADR-016-B3).
CREATE NODE TABLE IF NOT EXISTS Feedback (
    id STRING PRIMARY KEY,
    target STRING,
    verdict STRING,
    replacement STRING,
    actor STRING,
    revision STRING,
    timestamp STRING,
    props STRING
);

-- 4. Create (:Reconciliation) node table (spec-35 v1.1 §3.2)
--    Field shape: id, assertion_id, subject, predicate, object,
--    evidence_set (LIST<STRING>), computed_status, rationale, revision.
--    `planes: Vec<PlaneEvidence>` reserved for v1.2 (single-plane today).
CREATE NODE TABLE IF NOT EXISTS Reconciliation (
    id STRING PRIMARY KEY,
    assertion_id STRING,
    subject STRING,
    predicate STRING,
    object STRING,
    evidence_set STRING[],
    computed_status STRING,
    rationale STRING,
    revision STRING
);

-- 5. Create VERDICTS_ON edge (Feedback → FusedClaim)
CREATE REL TABLE IF NOT EXISTS VERDICTS_ON (FROM Feedback TO FusedClaim);

-- 6. Create RECONCILES edge (Reconciliation → FusedClaim)
CREATE REL TABLE IF NOT EXISTS RECONCILES (FROM Reconciliation TO FusedClaim);
