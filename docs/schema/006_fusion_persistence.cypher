-- v6-fusion-persistence (Wave 3 Item 27 follow-ups): persist fused claims.
--
-- Write-side of the fusion engine: `archctl architecture fuse --persist`
-- materializes the in-memory `FusedClaim` projection as graph rows so
-- read-side use cases (explain, coverage) can surface them without
-- recomputing fusion from observations.
--
-- Design notes:
-- - `written_at` is STRING (lbug 0.18.3 `timestamp()` is strict for
--   pre-upgrade backfill; P2-09b learned rule, see ADR-049).
-- - `observation_ids` / `derived_from` arrays mirror the carrier so
--   the canonical read path reconstructs the struct 1:1 from row
--   values (same convention as `:Claim` in 004_p2_09b).
-- - `CONTRADICTS` materializes the `conflicts_with` cross-links
--   (both directions, written by the store layer).
-- - `FUSED_FROM` keeps member provenance navigable in the graph
--   (best-effort: member `:Observation` rows may be absent on
--   compat-only derivations pre-backfill).
--
-- Idempotent by construction: the ADR-017 runner is marker-gated
-- (`.archctl-schema`); MERGE semantics handle re-application.

CREATE NODE TABLE FusedClaim (
    id STRING PRIMARY KEY,
    kind STRING,
    statement STRING,
    confidence DOUBLE,
    supports INT64,
    status STRING,
    stale BOOLEAN,
    observation_ids STRING[],
    derived_from STRING[],
    version_id STRING,
    written_at STRING
);

CREATE REL TABLE CONTRADICTS (FROM FusedClaim TO FusedClaim);

CREATE REL TABLE FUSED_FROM (FROM FusedClaim TO Observation);
