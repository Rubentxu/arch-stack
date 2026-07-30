# Delta spec — Source & Evaluation graph types

> **Change**: `b1-source-evaluation-types`
> **Cycle**: `b1-source-evaluation-types` · Path: A-full
> **Branch**: `feat/b1-source-evaluation-types` @ `1264f9e` (11 commits from base `22b1066`)
> **Status**: Completed and archived
> **Mode**: This file IS the main spec for the Source and Evaluation domain types.
>   No prior spec existed for this surface — this delta is the canonical record.

---

## Overview

This cycle introduces two new domain node types (`SourceArtifact`, `Evaluation`) and their
graph edges, completing the infrastructure for ADR-016 §3 (Bloque B1). The
`SourceArtifact` node represents a versioned file observed by the pipeline; the
`Evaluation` node represents a judgement applied to an evidence row. Both types are
persisted via new `GraphStore` port methods and composed through the `put_with_source`
use-case in `evidence.rs`.

**Out of scope** (punted to later cycles): `drafted → accepted` lifecycle,
file-change invalidation query, backfill of legacy `Evidence` rows.

---

## Domain types

### SourceArtifact (`archctl/src/source.rs`)

```rust
pub struct SourceArtifact {
    pub id: String,                       // "src:" + blake3(relative_path + content_hash)[..16]
    pub kind: String,                     // always "source_file" in B1
    pub relative_path: String,            // forward-slash, relative to workspace root
    pub language: String,                 // "rust" | "python" | ... (from inventory label)
    pub content_hash: String,             // "sha256:<hex>" — reused from evidence::content_hash_of, NOT recomputed
    pub commit_hash: Option<String>,       // None for non-git workspaces
    pub generated: bool,                  // false for source files
    pub props: serde_json::Map<String, serde_json::Value>,
}
```

**Identifier derivation** (`source.rs:91-96`):
```
id = "src:" + blake3(relative_path + content_hash)[..16]
```

**Props** (populated by `SourceArtifact::from_content`):
| Key | Type | Source |
|-----|------|--------|
| `first_seen_at` | string (RFC3339) | `clock.now_rfc3339()` |
| `extractor` | string | constant `"archctl:evidence"` |
| `extractor_version` | string | `env!("CARGO_PKG_VERSION")` |

**Note on naming**: The spec used `sa:` prefix; the design and implementation use
`src:` per tasks.md Q4 naming-harmonisation resolution. This matches the design
§Interfaces decision and avoids confusion with `Evidence` id prefix `ev:`.

### Evaluation (`archctl/src/evaluation.rs`)

```rust
pub struct Evaluation {
    pub id: String,                       // "eval:" + blake3(criterion + target_evidence_id + evaluated_at)[..16]
    pub target_evidence_id: String,
    pub criterion: String,                 // "min_occurrence" | "user_accepted" | ...
    pub passed: bool,
    pub evaluator: String,               // "archctl:threshold_v1" | "human:<id>"
    pub evaluated_at: String,             // RFC3339 from Clock
    pub props: serde_json::Map<String, serde_json::Value>,
}
```

**Constructors** (`evaluation.rs:42-62`):
- `Evaluation::accept(criterion, target_evidence_id, evaluator, clock) → Self` — sets `passed = true`
- `Evaluation::reject(criterion, target_evidence_id, evaluator, clock) → Self` — sets `passed = false`

**Identifier derivation** (`evaluation.rs:87-93`):
```
id = "eval:" + blake3(criterion + target_evidence_id + evaluated_at)[..16]
```

**Props**:
| Key | Type | Source |
|-----|------|--------|
| `criterion_params` | JSON object | caller-supplied (e.g. `{"min": 3}`) |
| `observed_value` | any JSON | caller-supplied |
| `notes` | string | optional |

**D3 — Evaluation optional in B1**: The `put_with_source` wrapper accepts
`Option<&Evaluation>`. Existing `put_evidence` signature is unchanged.

---

## Graph edges

### `EXTRACTED_FROM` (Evidence → SourceArtifact)

Declared in `docs/schema/001_initial_schema.cypher:231-233` (existing, not modified).

Direction: `(:Evidence)-[:EXTRACTED_FROM]->(:SourceArtifact)`

### `EVALUATES` (Evaluation → Evidence)

Declared in `docs/schema/002_source_evaluation.cypher:105-107` (new, B1).

Direction: `(:Evaluation)-[:EVALUATES]->(:Evidence)`

Note: The spec used `EVALUATED_BY` (inverse direction). The implementation uses
`EVALUATES` from Evaluation to Evidence, matching the design's directional choice
(Evaluation is the source node of the edge, consistent with how `link_evaluates`
is called from `put_with_source`).

---

## Schema migrations

### Migration versions

| Version | File | Contents |
|---------|------|----------|
| `v1-initial` | `docs/schema/001_initial_schema.cypher` | Existing schema (SourceArtifact, EXTRACTED_FROM, Evidence) |
| `v2-source-evaluation` | `docs/schema/002_source_evaluation.cypher` | Evaluation node table + EVALUATES relationship |

### Marker

File: `<project_dir>/.archctl-schema`
Format: plain text version string (e.g. `v2-source-evaluation\n`)
Reader: `migrations::current_version(marker_path, fs) → Result<Option<String>>`
Writer: `migrations::apply_pending` writes only after ALL migrations succeed

### Migration runner (`archctl/src/migrations.rs`)

```rust
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: "v1-initial",
        script: include_str!("../../docs/schema/001_initial_schema.cypher"),
    },
    Migration {
        version: "v2-source-evaluation",
        script: include_str!("../../docs/schema/002_source_evaluation.cypher"),
    },
];

pub const SCHEMA_MARKER_FILENAME: &str = ".archctl-schema";

pub fn apply_pending(
    marker_path: &Path,
    fs: &dyn Filesystem,
    conn: &Connection,
) -> Result<Vec<String>>;
```

**Layer**: Session (not behind `GraphStore::execute_raw`). Both `graph::init`
and `LbugStore::init` delegate to `migrations::apply_pending`.

**Idempotence**: Strict-greater-than comparison (`current >= migration.version → skip`).
Marker written only when `!applied.is_empty()`.

**D5 — No backfill**: Pre-B1 `Evidence` rows remain without `EXTRACTED_FROM` edges.
No `ALTER TABLE` needed; `source_origin` lives in `Evidence.props` JSON.

---

## GraphStore port additions

### Trait methods (all `&mut self`, all return `Result<()>`)

| Method | Description |
|--------|-------------|
| `put_source(&mut self, source: &SourceArtifact)` | MERGE SourceArtifact node by `id`. Idempotent on `(relative_path, content_hash)`. |
| `put_evaluation(&mut self, evaluation: &Evaluation)` | MERGE Evaluation node by `id`. Idempotent. |
| `link_extracted_from(&mut self, evidence_id: &str, source_id: &str)` | Create `EXTRACTED_FROM` edge. Idempotent (MERGE on REL TABLE; fallback MATCH+CREATE). |
| `link_evaluates(&mut self, evaluation_id: &str, evidence_id: &str)` | Create `EVALUATES` edge. Idempotent (same MERGE/fallback pattern). |

**Note on naming**: The spec used `create_source_artifact` / `create_evaluation` /
`link_evidence_to_source`. Implementation uses `put_source` / `put_evaluation` /
`link_extracted_from` per tasks.md Q4 naming-harmonisation (design wins over spec).

### lbug 0.18.3 MERGE-on-REL-TABLE fallback

`lbug 0.18.3` rejects `MERGE` on REL TABLE. Adapter implements:
1. Try `MERGE (e:Evidence {id})-[:EXTRACTED_FROM]->(s:SourceArtifact {id})`
2. If error → fallback `MATCH` for both endpoints + `CREATE` edge

This is documented in ADR-017 §"Nota técnica".

---

## Evidence.props — source_origin injection (D4)

`source_origin` is persisted in `Evidence.props` JSON, not as a schema column.
Follows the existing pattern for `language`, `start_byte`, `end_byte`, `text_preview`.

**Injection points** (domain layer, not adapter):
- `evidence_from_match` (`evidence.rs:284-287`): `props.insert("source_origin", SourceOrigin::UserInput.as_str())`
- `from_tsg_node` (`evidence.rs:431-434`): `props.insert("source_origin", SourceOrigin::ToolOutput.as_str())`

**Props key**: `"source_origin"` → `"user_workspace"` or `"tool_output"` (string, not enum)

**Query path**: `MATCH (e:Evidence) RETURN e.props.source_origin AS so`
(lbug JSON path access; not queryable as a top-level column)

---

## Use-case wrapper (`evidence.rs`)

```rust
pub fn put_with_source(
    project_dir: &Path,
    evidence: &[Evidence],
    sources: Option<&[SourceArtifact]>,
    evaluation: Option<&Evaluation>,
    _clock: &dyn Clock,
) -> Result<usize>
```

**Composition order** (spec-mandated, D6):
1. `put_source` (if `sources.is_some()`) — deduplicates by `HashSet<&source.id>`
2. `put_evidence` (existing method)
3. `link_extracted_from` (per evidence × source pair)
4. `put_evaluation` + `link_evaluates` (if `evaluation.is_some()`) — **last**, so the EVALUATES edge can find the Evidence row

**Non-transactional**: Failure in step 4 does NOT roll back steps 1–3 (per spec contract).

**Existing signatures unchanged**: `put_evidence` keeps `fn(&[Evidence]) → Result<usize>`.

---

## Hash scheme (D2)

| Purpose | Algorithm | Output |
|---------|-----------|--------|
| `Evidence.content_hash` | SHA-256 (from `evidence::content_hash_of`) | `"sha256:<hex>"` (already computed in pipeline) |
| `SourceArtifact.id` | blake3(relative_path + content_hash) | `"src:<16-byte-hex>"` |
| `Evaluation.id` | blake3(criterion + target_evidence_id + evaluated_at) | `"eval:<16-byte-hex>"` |
| `Evidence.id` (pre-existing) | blake3(path + start_byte + end_byte + text) | `"ev:<16-byte-hex>"` |

**No new hash algorithm introduced.** blake3 already in `Cargo.toml` (used by `evidence_id`).
SHA-256 already computed by `evidence::content_hash_of`.

---

## Manifests (commit 8, 10)

| File | Scope | New symbols | Notes |
|------|-------|-------------|-------|
| `manifests/source.toml` | `source.rs` | `SourceArtifact`, `from_content`, `id_for` | NEW |
| `manifests/evaluation.toml` | `evaluation.rs` | `Evaluation`, `accept`, `reject` | NEW |
| `manifests/evidence.toml` | `evidence.rs` | updated `public_symbols` | `SourceArtifact`/`Evaluation` removed from cross-module symbols (commit 10 fix) |
| `manifests/store.toml` | `store.rs` | `put_source`, `put_evaluation`, `link_extracted_from`, `SourceArtifact`, `Evaluation` | trait methods listed as `fn <name>` without `pub` prefix (commit 10 fix) |

---

## Decisions captured

| ID | Decision | Reference |
|----|----------|-----------|
| D1 | Versioned migration runner at Session layer (not behind GraphStore port) | ADR-017 §D1 |
| D2 | `SourceArtifact.id = "src:" + blake3(relative_path + content_hash)` | ADR-017 §D2 |
| D3 | Evaluation is optional in B1; `put_with_source` accepts `Option<&Evaluation>` | ADR-017 §D3 |
| D4 | `source_origin` lives in `Evidence.props` JSON (not a schema column) | ADR-017 §D4 |
| D5 | No backfill of legacy `Evidence` rows | ADR-017 §D5 |
| D6 | Extend `GraphStore` with new methods; do NOT create a new port | ADR-017 §D6 |
| Q1 | Reuse SHA-256 from `content_hash_of`; blake3 only for id derivation | tasks.md Q1 |
| Q2 | Try MERGE on REL TABLE first; fallback MATCH+CREATE on lbug error | tasks.md Q2 |
| Q3 | Marker-gated replay without catch-and-skip; manual recovery documented | tasks.md Q3 |
| Q4 | Naming: `put_source`/`put_evaluation`/`link_extracted_from` + `src:` prefix (design wins) | tasks.md Q4 |

---

## Files produced by this cycle

| Path | Role |
|------|------|
| `archctl/src/source.rs` | SourceArtifact domain type |
| `archctl/src/evaluation.rs` | Evaluation domain type |
| `archctl/src/migrations.rs` | Versioned migration runner |
| `archctl/src/store.rs` | 4 new GraphStore port methods + adapter |
| `archctl/src/evidence.rs` | source_origin injection + put_with_source wrapper |
| `archctl/src/graph.rs` | init delegates to migrations::apply_pending |
| `archctl/src/lib.rs` | Module exports |
| `docs/schema/002_source_evaluation.cypher` | Evaluation table + EVALUATES edge |
| `manifests/source.toml` | Scope manifest for source.rs |
| `manifests/evaluation.toml` | Scope manifest for evaluation.rs |
| `manifests/evidence.toml` | Updated (cross-module symbols removed in commit 10) |
| `manifests/store.toml` | Updated (trait methods listed correctly in commit 10) |
| `docs/adr/ADR-017-schema-migration-runner.md` | Captures D1–D6 + lbug quirks |

---

## Breaking changes

**None.** All existing public signatures are unchanged:
- `put_evidence` → still `fn(&[Evidence]) → Result<usize>`
- `evidence::put_with_clock` → unchanged
- `GraphStore` existing 7 methods → unchanged

---

## Test count

| Phase | Count |
|-------|-------|
| Baseline (22b1066) | 111 |
| After B1 | 124 |
| New tests (B1 files) | 13 (3 source + 3 evaluation + 5 migrations + 2 store) + 6 in evidence.rs = 19 |

All 124 tests pass under `--test-threads=1`. 10 pre-existing parallel failures
(lbug 0.18.3 mmap race) are out of scope for B1.

---

**End of delta spec — b1-source-evaluation-types**
