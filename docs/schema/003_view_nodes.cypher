-- B1 Block - view-node persistence (v3).
-- Persists view-level diagram state distinct from canonical Element.
-- This is the only schema change in the m9-archctl-export-apply cycle.
-- All runtime code for these tables is added in T5-T8.
--
-- ADR-007 §"Vista persistida": 4 NODE TABLEs + 3 REL TABLEs.
-- ADR-017 §"Nota técnica": REL TABLEs use MATCH+CREATE fallback,
-- not MERGE (lbug 0.18.3 rejects MERGE on REL TABLE).

-- Diagram: top-level view container, identified by project-scoped id.
-- revision is the blake3 content-hash of the exported bundle at apply-time.
CREATE NODE TABLE Diagram (
    id STRING PRIMARY KEY,
    revision STRING,
    selector STRING,
    props JSON,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

-- ViewMember: a canonical Element placed in a Diagram.
-- diagram_id is denormalised for indexed lookup (GET view_members BY diagram_id).
-- element_id is the foreign key to Element.id.
CREATE NODE TABLE ViewMember (
    id STRING PRIMARY KEY,
    diagram_id STRING,
    element_id STRING,
    label STRING,
    props JSON,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

-- ViewEdge: a directed edge between two ViewMembers within a Diagram.
-- Corresponds to a SemanticRelation override in the view layer.
CREATE NODE TABLE ViewEdge (
    id STRING PRIMARY KEY,
    diagram_id STRING,
    source_member_id STRING,
    target_member_id STRING,
    edge_label STRING,
    props JSON,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

-- ViewGroup: a named group of ViewMembers within a Diagram.
CREATE NODE TABLE ViewGroup (
    id STRING PRIMARY KEY,
    diagram_id STRING,
    label STRING,
    props JSON,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

-- MEMBER_OF: a ViewMember belongs to exactly one Diagram.
CREATE REL TABLE MEMBER_OF (
    FROM ViewMember TO Diagram
);

-- RENDERS: a ViewMember renders one canonical Element.
CREATE REL TABLE RENDERS (
    FROM ViewMember TO Element
);

-- GROUP_CONTAINS: a ViewGroup contains zero or more ViewMembers.
CREATE REL TABLE GROUP_CONTAINS (
    FROM ViewGroup TO ViewMember
);
