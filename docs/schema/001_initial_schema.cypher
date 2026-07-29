CREATE GRAPH architecture;
USE architecture;

CREATE NODE TABLE MetaType (
    id STRING PRIMARY KEY,
    namespace STRING,
    name STRING,
    category STRING,
    schema_version INT64,
    property_schema JSON,
    validation_rules JSON,
    renderer_hints JSON,
    description STRING
);

CREATE NODE TABLE Predicate (
    id STRING PRIMARY KEY,
    namespace STRING,
    name STRING,
    directed BOOLEAN,
    transitive BOOLEAN,
    symmetric BOOLEAN,
    schema_version INT64,
    allowed_pairs JSON,
    property_schema JSON,
    validation_rules JSON,
    renderer_hints JSON,
    description STRING
);

CREATE NODE TABLE Element (
    id STRING PRIMARY KEY,
    kind_id STRING,
    category STRING,
    canonical_key STRING,
    current_version_id STRING,
    current_name STRING,
    current_status STRING,
    current_origin STRING,
    current_confidence DOUBLE,
    current_order_key STRING,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE NODE TABLE ElementVersion (
    id STRING PRIMARY KEY,
    element_id STRING,
    name STRING,
    description STRING,
    status STRING,
    origin STRING,
    confidence DOUBLE,
    order_key STRING,
    content_hash STRING,
    props JSON,
    created_at TIMESTAMP
);

CREATE NODE TABLE SemanticRelation (
    id STRING PRIMARY KEY,
    predicate_id STRING,
    source_id STRING,
    target_id STRING,
    canonical_key STRING,
    current_version_id STRING,
    current_label STRING,
    current_status STRING,
    current_origin STRING,
    current_confidence DOUBLE,
    current_order_key STRING,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE NODE TABLE RelationVersion (
    id STRING PRIMARY KEY,
    relation_id STRING,
    label STRING,
    status STRING,
    origin STRING,
    confidence DOUBLE,
    order_key STRING,
    content_hash STRING,
    props JSON,
    created_at TIMESTAMP
);

CREATE NODE TABLE Snapshot (
    id STRING PRIMARY KEY,
    sequence INT64,
    kind STRING,
    commit_hash STRING,
    worktree_id STRING,
    schema_version INT64,
    created_at TIMESTAMP,
    props JSON
);

CREATE NODE TABLE Evidence (
    id STRING PRIMARY KEY,
    kind STRING,
    classification STRING,
    claim STRING,
    confidence DOUBLE,
    path STRING,
    start_line INT64,
    end_line INT64,
    commit_hash STRING,
    content_hash STRING,
    tool_name STRING,
    tool_version STRING,
    rule_id STRING,
    props JSON,
    observed_at TIMESTAMP
);

CREATE NODE TABLE SourceArtifact (
    id STRING PRIMARY KEY,
    kind STRING,
    relative_path STRING,
    language STRING,
    content_hash STRING,
    commit_hash STRING,
    generated BOOLEAN,
    props JSON
);

CREATE NODE TABLE ToolRun (
    id STRING PRIMARY KEY,
    tool_name STRING,
    tool_version STRING,
    adapter_version STRING,
    command_hash STRING,
    configuration_hash STRING,
    started_at TIMESTAMP,
    finished_at TIMESTAMP,
    status STRING,
    props JSON
);

CREATE NODE TABLE Artifact (
    id STRING PRIMARY KEY,
    kind STRING,
    format STRING,
    path STRING,
    content_hash STRING,
    renderer STRING,
    renderer_version STRING,
    status STRING,
    props JSON,
    created_at TIMESTAMP
);

CREATE NODE TABLE AnalysisRun (
    id STRING PRIMARY KEY,
    request STRING,
    kind STRING,
    status STRING,
    started_at TIMESTAMP,
    finished_at TIMESTAMP,
    props JSON
);

CREATE REL TABLE OF_TYPE (
    FROM Element TO MetaType
);

CREATE REL TABLE RELATION_TYPE (
    FROM SemanticRelation TO Predicate
);

CREATE REL TABLE VERSION_OF (
    FROM ElementVersion TO Element
);

CREATE REL TABLE CURRENT_VERSION (
    FROM Element TO ElementVersion
);

CREATE REL TABLE RELATION_VERSION_OF (
    FROM RelationVersion TO SemanticRelation
);

CREATE REL TABLE CURRENT_RELATION_VERSION (
    FROM SemanticRelation TO RelationVersion
);

CREATE REL TABLE AT_SNAPSHOT (
    FROM ElementVersion TO Snapshot,
    FROM RelationVersion TO Snapshot,
    FROM Evidence TO Snapshot,
    FROM Artifact TO Snapshot
);

CREATE REL TABLE PARENT_SNAPSHOT (
    FROM Snapshot TO Snapshot
);

CREATE REL TABLE REL_SOURCE (
    FROM Element TO SemanticRelation
);

CREATE REL TABLE REL_TARGET (
    FROM SemanticRelation TO Element
);

CREATE REL TABLE SEMANTIC_EDGE (
    FROM Element TO Element,
    relation_id STRING,
    relation_version_id STRING,
    predicate_id STRING,
    active BOOLEAN,
    order_key STRING,
    props JSON
);

CREATE REL TABLE SUPPORTED_BY (
    FROM ElementVersion TO Evidence,
    FROM RelationVersion TO Evidence,
    FROM Artifact TO Evidence,
    role STRING
);

CREATE REL TABLE CONTRADICTED_BY (
    FROM ElementVersion TO Evidence,
    FROM RelationVersion TO Evidence,
    role STRING
);

CREATE REL TABLE EXTRACTED_FROM (
    FROM Evidence TO SourceArtifact
);

CREATE REL TABLE PRODUCED_BY (
    FROM Evidence TO ToolRun,
    FROM Artifact TO ToolRun
);

CREATE REL TABLE DERIVED_FROM_EVIDENCE (
    FROM Evidence TO Evidence
);

CREATE REL TABLE GENERATED_FROM (
    FROM Artifact TO Element,
    role STRING
);

CREATE REL TABLE GENERATED_FROM_RELATION (
    FROM Artifact TO SemanticRelation,
    role STRING
);

CREATE REL TABLE DERIVED_ARTIFACT (
    FROM Artifact TO Artifact,
    transformation STRING
);

CREATE REL TABLE RUN_INPUT_SNAPSHOT (
    FROM AnalysisRun TO Snapshot
);

CREATE REL TABLE RUN_OUTPUT_SNAPSHOT (
    FROM AnalysisRun TO Snapshot
);

CREATE REL TABLE RUN_USED_TOOL (
    FROM AnalysisRun TO ToolRun
);

CREATE REL TABLE RUN_PRODUCED_ARTIFACT (
    FROM AnalysisRun TO Artifact
);
