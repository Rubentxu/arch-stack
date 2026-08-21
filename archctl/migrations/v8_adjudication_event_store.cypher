-- v8-adjudication-event-store (TRUST-008 cycle p-38e02210a9f14317/trust-008-m30-bridge-promotion)
--
-- ADR-063 trust-first invariant, data-plane layer.
--
-- This migration:
-- 1. Creates (:Adjudication) node table per spec REQ-T08-001.
-- 2. Creates (:AdjudicationDecision) lookup node table (forward-compat for v1.2).
-- 3. Creates ADJUDICATES typed edge (Adjudication -> FusedClaim).
-- 4. Idempotent by construction (IF NOT EXISTS).
-- 5. The rust hook (`backfill_adjudication_event_diagnostics` in
--    archctl/src/migrations.rs) emits ONE tracing::warn! listing how many
--    pre-v8 (:FusedClaim) rows carry pending_adjudication_event = true
--    AND have no backing (:Adjudication) event. Does NOT auto-decide.

CREATE NODE TABLE IF NOT EXISTS Adjudication (
    id STRING PRIMARY KEY,
    target_fused_claim_id STRING,
    adjudicator STRING,
    evidence_refs STRING,
    decided_at STRING,
    decision STRING
);

CREATE NODE TABLE IF NOT EXISTS AdjudicationDecision (
    name STRING PRIMARY KEY
);

CREATE REL TABLE IF NOT EXISTS ADJUDICATES (FROM Adjudication TO FusedClaim);
