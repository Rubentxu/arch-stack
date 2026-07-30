-- B1 Block - source & evaluation graph model (v2).
-- SourceArtifact and EXTRACTED_FROM are already declared in 001 and
-- reused here. This file only adds the new Evaluation primitive and
-- its edge to Evidence. source_origin lives in Evidence.props (D4),
-- not as a column, to avoid ALTER TABLE that lbug 0.18.3 may not honour.

CREATE NODE TABLE Evaluation (
    id STRING PRIMARY KEY,
    target_evidence_id STRING,
    criterion STRING,
    passed BOOLEAN,
    evaluator STRING,
    evaluated_at TIMESTAMP,
    props JSON
);

CREATE REL TABLE EVALUATES (
    FROM Evaluation TO Evidence
);
