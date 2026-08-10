---
status: accepted
date: 2026-08-10
deciders: [sddk-cycle-m71, user]
consulted: [archctl-rust, archview-ts]
informed: []
supersedes: []
superseded_by: []
---

# ADR-041 — Workspace state persistence contract

## Context and Problem Statement

When a user runs `archctl view`, the workbench opens with a default viewport
and no persisted UI state. Closing the tab loses the selected node, the open
drawers, the viewport position, and any filter settings. On the next run,
everything resets to defaults — even though the underlying graph (LadybugDB)
has not changed.

This matters because the workbench is the primary interaction surface for
reviewing architecture diagrams. A user who spends 10 minutes navigating to a
specific node and zoom level expects to find that context back when they
reopen `archctl view`.

The problem is non-trivial because:

1. The workbench is a SolidJS SPA embedded in the `archctl` binary
   (ADR-033). It has no filesystem access and cannot write to XDG directly.
2. `archctl view` is a one-shot HTTP server (ADR-010). Each invocation
   gets a new ephemeral port. State cannot live in the server process.
3. LocalStorage / IndexedDB are ruled out by ADR-038 Invariant 3:
   "El estado persistente vive exclusivamente en XDG."
4. A WebSocket-based reactive sync (Tab A ↔ Tab B ↔ archctl) was evaluated
   and rejected in ADR-039: the complexity of maintaining a shared reactive
   session is unjustified for a one-shot local tool.

## Decision Drivers

- **ADR-038 Invariant 3**: persistence must be XDG-only, never localStorage.
- **ADR-010 / ADR-033**: one-shot server, no daemon, no shared state between
  invocations. State must survive process death.
- **ADR-005**: evidence is immutable in the graph. The workbench UI is a
  read-only projection; workspace state is cosmetic (ADR-038 Invariant 4).
- **ADR-039 anti-roadmap**: WebSocket reactive was explicitly evaluated and
  deferred. The scope of H1 is "restore workspace" not "multi-tab sync".
- **Minimalism**: H1 does NOT include collaborative editing, conflict
  resolution, or version history. Scope is bounded to what one user expects
  from a local CLI tool.

## Considered Options

### Option A: Rust-owned `workspace.json` via XDG (chosen)

`archctl view` acts as a thin HTTP bridge: the SolidJS workbench sends
`PUT /api/workspace` (viewport, selectedNode, openDrawers) and
`GET /api/workspace` on startup. Rust handles all filesystem I/O, writing to:

```
~/.local/share/archctl/projects/<hash>/workspace.json
```

Atomic writes via `tempfile::NamedTempFile::persist()`. Concurrency guarded
by a dedicated `workspace.json.lock` flock (separate from `.lbdb` flock).

The schema is commiteable: `schemas/workspace-state.schema.json` (JSON Schema
2020-12). TypeScript types are generated from this schema via
`json-schema-to-typescript`.

### Option B: localStorage client-side

SolidJS stores state in browser localStorage. Survives tab close but NOT
`archctl view` restart (the embedded browser state is fresh on each run).
Violates ADR-038 Invariant 3. Rejected.

### Option C: WebSocket reactive

A shared WebSocket session between all open `archctl view` tabs plus the
server. Any tab can push viewport changes; all tabs receive them in real time.
State lives in server memory, not on disk.

Rejected in ADR-039: unjustified complexity for a one-shot local tool.
Adds a long-lived background process (the WS server) that contradicts
ADR-010. Deferred indefinitely.

## Decision Outcome

**Chosen option: A**, because it satisfies all three constraints simultaneously:
ADR-038 (XDG-only), ADR-010 (no daemon, one-shot), and the user's
expectation of state persistence across invocations. Option B violates the XDG
invariant. Option C violates the one-shot constraint and adds complexity
without user-facing benefit in the H1 scope.

### Positive Consequences

- State persists across `archctl view` restarts, process kills, and reboots.
- No localStorage / IndexedDB: everything in XDG, audit-friendly.
- The SolidJS bundle stays pure and stateless; all filesystem I/O is
  in the Rust backend.
- The `workspace.json` file is human-readable and debuggable.
- Flock-based concurrency means only one `archctl view` can hold the lock
  at a time; no corrupted state from concurrent writes.

### Negative Consequences

- The user cannot have two `archctl view` processes open at the same time
  against the same project. The second one fails with a lock error. This is
  by design (ADR-010 single-process) but may surprise users who expect
  "tabs" in a browser-based tool.
- `EDITOR` environment variable with arguments (e.g. `code --wait`) is not
  supported. The `EditorCommand` struct takes only the first token as the
  binary. This is documented as a known limitation.

## Implementation Plan

**PR #1 (Rust backend, ~470 LOC):**

1. Create `schemas/workspace-state.schema.json` with JSON Schema 2020-12.
   Fields: `version` (const "1.0"), `viewport` (x, y, zoom), `selectedNode`,
   `openDrawers` (array of `{type, id}`), `lastModified` (ISO 8601).
2. Implement `archctl/src/view/workspace.rs`: `WorkspaceState` serde structs,
   `load()`, `save()`, `validate_path_under_cwd()` with canonicalize + prefix
   check. Atomic write via `NamedTempFile::persist()`.
3. Implement `archctl/src/view/source.rs`: `GET /api/source?file=&line=`
   reads `file:line-range` from disk, validates path under `--cwd`, returns
   `{file, line, content: string[], totalLines: number}`.
4. Implement `archctl/src/view/editor.rs`: `resolve_editor()` (EDITOR, VISUAL,
   xdg-open fallback), `open_in_editor(file, line)` via `Command::new().arg()`.
   No shell expansion.
5. Add `GET|PUT /api/workspace`, `GET /api/source`, `POST /api/open-editor`
   to `handle_request()` in `archctl/src/view.rs`.
6. Add `#[test]` modules: path validation edge cases, serde round-trip,
   flock behavior, HTTP error codes.
7. Add `archctl/tests/view_workspace.rs`: integration test — TempDir,
   PUT → GET round-trip, concurrent flock, path traversal rejection.

**PR #2 (TypeScript frontend, ~310 LOC):**

1. Generate `archview/src/lib/workspace.types.ts` from the committed
   `schemas/workspace-state.schema.json` using `json-schema-to-typescript`.
2. Implement `archview/src/lib/workspace.ts`: `useWorkspaceState()` hook —
   `GET /api/workspace` on mount, `PUT /api/workspace` on state change,
   debounced 500ms.
3. Implement `archview/src/components/SourceDrawer/SourceDrawer.tsx`: receives
   `EvidenceRef {file, line}`, renders header with path, body with line preview
   (±2 lines), footer with "Open in $EDITOR" button. Read-only.
4. Integrate `SourceDrawer` into `archview/src/components/Sidebar.tsx`:
   conditional render when an evidence node is selected.
5. Add `archview/src/__tests__/workspace.test.ts` and
   `SourceDrawer.test.tsx`.

**Release tag:** v1.32.0 (both PRs land in the same minor version).

## Verification

1. `cargo test --quiet` — all Rust tests green.
2. `cargo clippy --quiet -- -D warnings` — zero warnings.
3. `pnpm test` — all TS tests green.
4. `pnpm build` — build succeeds without errors.
5. Manual cross-port restore:
   - Run `archctl view --cwd /path/to/repo`.
   - Pan, zoom, select a node.
   - Close the tab.
   - Run `archctl view --cwd /path/to/repo` again.
   - Verify viewport, selection, and open drawers are restored.

## Pros and Cons of the Options

### Option A: Rust-owned workspace.json via XDG

**Pros:**
- Satisfies ADR-038 Invariant 3 (XDG-only).
- State survives process death and reboots.
- Schema is commiteable and testable.
- Rust is the natural place for filesystem I/O (existing patterns in
  `project.rs`, `xdg.rs`).
- Flock prevents concurrent corruption.

**Cons:**
- One process per project (second `archctl view` fails with lock error).
- EDITOR with arguments not supported (known limitation, documented).

### Option B: localStorage client-side

**Pros:**
- Zero backend complexity.
- Instant saves (no network round-trip).

**Cons:**
- Violates ADR-038 Invariant 3.
- State does not survive `archctl view` restarts (the embedded browser is
  fresh each time).
- Bundle state lives in the binary, not on disk — no audit trail.

### Option C: WebSocket reactive

**Pros:**
- Real-time multi-tab sync.
- No filesystem I/O.

**Cons:**
- Violates ADR-010 (one-shot, no daemon). The WS server is long-lived.
- High complexity: connection lifecycle, reconnect, message protocol.
- ADR-039 explicitly evaluated and deferred this decision.
- No user-facing benefit in H1 scope (single user, local tool).

## Schema

Located at `schemas/workspace-state.schema.json`. The canonical schema is defined in `spec.md §3` and `schemas/workspace-state.schema.json`. This ADR records the design decision; the schema itself is the source of truth.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://archctl.local/schemas/workspace-state.schema.json",
  "$comment": "Contrato versionado Rust↔TS para workspace state. schemaVersion bump → migration. Campos con rango 0+ sin constraints absurdos por tratarse de viewport coordinates.",
  "type": "object",
  "additionalProperties": false,
  "required": ["version", "project_hash", "workspace", "updated_at"],
  "properties": {
    "version": {
      "type": "string",
      "const": "1.0",
      "description": "Literal fijo. Bump para migraciones de schema."
    },
    "project_hash": {
      "type": "string",
      "pattern": "^[0-9a-f]{64}$",
      "description": "blake3 hex del project identity (64 chars)."
    },
    "workspace": { "$ref": "#/$defs/Workspace" },
    "updated_at": {
      "type": "string",
      "format": "date-time",
      "description": "ISO-8601 con timezone de la última escritura."
    }
  },
  "$defs": {
    "Workspace": {
      "type": "object",
      "additionalProperties": false,
      "required": ["camera", "zoom", "filters", "selection"],
      "properties": {
        "camera": {
          "type": "object",
          "additionalProperties": false,
          "required": ["x", "y"],
          "properties": {
            "x": { "type": "number", "minimum": 0 },
            "y": { "type": "number", "minimum": 0 }
          }
        },
        "zoom": {
          "type": "number",
          "minimum": 0,
          "maximum": 100,
          "description": "Nivel de zoom. Rango 0–100 por seguridad."
        },
        "filters": {
          "type": "array",
          "items": { "$ref": "#/$defs/Filter" }
        },
        "selection": {
          "oneOf": [
            { "type": "null" },
            { "$ref": "#/$defs/Selection" }
          ]
        }
      }
    },
    "Filter": {
      "type": "object",
      "additionalProperties": false,
      "required": ["kind", "predicate"],
      "properties": {
        "kind": {
          "type": "string",
          "enum": ["c4", "call-graph", "sequence", "class", "package"]
        },
        "predicate": {
          "type": "string",
          "description": "Predicado de filtro validado client-side."
        }
      }
    },
    "Selection": {
      "type": "object",
      "additionalProperties": false,
      "required": ["kind", "id"],
      "properties": {
        "kind": {
          "type": "string",
          "enum": ["c4", "call-graph", "sequence", "class", "package", "node"]
        },
        "id": {
          "type": "string",
          "description": "Identificador del elemento seleccionado en el grafo."
        }
      }
    }
  }
}
```

## Editor Handoff Contract

The `POST /api/open-editor` endpoint receives:

```json
{ "file": "src/main.rs", "line": 42 }
```

Path is validated against `--cwd` (same rules as `/api/source`).
The server resolves `$EDITOR` → `$VISUAL` → platform fallback and spawns:

```rust
Command::new(editor_binary)
    .arg(resolved_file_path)
    .arg(format!("+{line}"))
    .spawn()?;  // no shell, no wait
```

**Limitation:** if `$EDITOR` contains spaces or arguments (e.g.
`code --wait` or `"Visual Studio Code.app"`), only the first token is used
as the binary. The remainder is silently dropped. This is a known
limitation documented in this ADR. Users who need argument passthrough
should use a wrapper script in `$PATH` as the `$EDITOR` value.

## References

- [ADR-038](ADR-038-one-product-five-invariants.md) — Invariant 3: XDG persistence
- [ADR-005](ADR-005-ladybugdb-grafo-canonico-y-evidencias.md) — evidence policy
- [ADR-010](ADR-010-concurrencia-ladybugdb.md) — flock concurrency
- [ADR-011](ADR-011-renderers-locales-y-bloqueo-de-publicos.md) — local-only
- [ADR-033](ADR-033-archctl-view-embedded-workbench.md) — embedded workbench
- [ADR-039](ADR-039-renderer-reality-anti-roadmap.md) — WebSocket deferred
- ROADMAP §H1 — durable workspace state milestone
- M71 cycle artifacts (explore-report, proposal, spec, design)
- `schemas/workspace-state.schema.json` — the schema itself

## Changelog

- 2026-08-10 | proposed | initial draft
- 2026-08-10 | accepted | approved by sddk-cycle-m71
