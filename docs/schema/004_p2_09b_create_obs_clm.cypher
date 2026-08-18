-- B1 Block - P2-09b: persistent Observation + Claim tables (v4-p2-09b-create-obs-clm-tables).
--
-- Persists the Wave 3 Item 19 dual-write target surfaces. These are
-- the derived projections over `Evidence` that P2-09a shipped as
-- compat-only mappers in `archctl/src/observation_claim.rs`.
--
-- After this migration:
-- - `(:Observation)` rows exist per Evidence id (1:1, namespaced
--   `obs:<evidence_id>`).
-- - `(:Claim)` compat rows exist per Evidence id (1:1, namespaced
--   `clm:compat:<evidence_id>`, fused=false).
--
-- ADR-049 (Evidence/Observation/Claim/Confidence model) closes its
-- `Aceptado (parcial)` once both tables are populated.
--
-- Future migrations (PR-B cycle): backfill existing Evidence rows
-- into Observation + Claim. Currently the tables are EMPTY post-create;
-- read paths fall back to P2-09a compat derivation.

CREATE NODE TABLE Observation (
    id STRING PRIMARY KEY,
    kind STRING,
    claim STRING,
    path STRING,
    start_line INT64,
    end_line INT64,
    tool_name STRING,
    tool_version STRING,
    confidence DOUBLE,
    source_origin STRING,
    written_via_backfill BOOLEAN,
    written_at TIMESTAMP
);

CREATE NODE TABLE Claim (
    id STRING PRIMARY KEY,
    text STRING,
    fused BOOLEAN,
    confidence DOUBLE,
    evidence_ids STRING[],
    written_at TIMESTAMP
);
