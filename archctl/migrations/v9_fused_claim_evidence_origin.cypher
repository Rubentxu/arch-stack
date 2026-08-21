-- v9-fused-claim-evidence-origin (TRUST-008 cycle p-38e02210a9f14317/trust-008-m30-bridge-promotion)
--
-- REQ-T08-004 / REQ-M25-006: m30 bridge promotion from soft-warn to hard-fail.
--
-- Adds `evidence_origin STRING` column to the `FusedClaim` node table.
-- The column stores the trust-classifying source origin of the first member
-- Observation (sufficient for the m30 bridge predicate since all members
-- of a fused group share provenance).
--
-- Pre-v9 graphs are permissive: rows without `evidence_origin` skip the
-- m30 bridge check in `put_feedback`. No data migration needed.
ALTER TABLE FusedClaim ADD IF NOT EXISTS evidence_origin STRING;

