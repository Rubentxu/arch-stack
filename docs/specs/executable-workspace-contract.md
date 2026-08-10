# Spec — Executable Workspace Contract

> **Status:** stub — mirrors the condensed spec from `sddk/h1-durable-workspace-state/spec.md`.
> This is the executable contract for the workspace state API (ADR-041).

## 1. Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/workspace` | Load workspace state from XDG |
| PUT | `/api/workspace` | Save workspace state atomically |
| GET | `/api/source?file=&line=` | Read source file with path validation |
| POST | `/api/open-editor` | Spawn user's editor at file:line |

## 2. GET /api/workspace

**Request:** no body, no query params.

**Response:**
- `200 { "workspace": <WorkspaceState>, "version": "1.0" }` if file exists
- `200 { "workspace": null, "version": "1.0" }` if no file yet
- `500 { "error": "xdg_inaccessible" }` if XDG dir not accessible

## 3. PUT /api/workspace

**Request:** `Content-Type: application/json` with body validating against `workspace-state.schema.json`.

**Response:**
- `204` on success (atomic write: temp + rename)
- `400 { "error": "invalid_schema", "details": "..." }` if body doesn't match schema
- `500` if write fails

## 4. GET /api/source

**Request:** query params `file` (required) and `line` (optional, 1-indexed).

**Path validation (before any I/O):**
- `canonicalize(resolve(cwd, file))` must have prefix `canonicalize(cwd)`
- Trailing `..`, absolute paths outside cwd → `403 { "error": "path_outside_scope" }`
- File not found → `404 { "error": "file_not_found" }`
- Path is directory → `400 { "error": "is_directory" }`

**Response:**
```json
{
  "file": "<path>",
  "start_line": <n>,
  "total_lines": <total>,
  "content": ["<line n>", ...],
  "truncated": <bool>
}
```

- `line` clamped to `total_lines` if exceeded
- Max 2000 lines returned; `truncated: true` if file exceeds limit

## 5. POST /api/open-editor

**Request:** `Content-Type: application/json`:
```json
{ "file": "<path>", "line": <number> }
```

**Path validation:** same as `/api/source`.

**Editor resolution (in order):**
1. `$EDITOR` env var
2. `$VISUAL` env var
3. `xdg-open` (Linux) / `open` (macOS)

**Response:**
- `204` if editor spawned successfully
- `503 { "error": "no_editor_configured", "hint": "set $EDITOR or $VISUAL" }` if no editor found
- `403` if path escapes cwd
- `400` if body invalid

**Spawn:** `Command::new(editor).arg(file).arg("+{line}")` — no shell, no wait.

## 6. Concurrency

- Writes to `workspace.json` use atomic write: write to temp file, then rename (POSIX atomic)
- Dedicated `workspace.json.lock` flock (separate from `.lbdb` flock per ADR-010)
- Lock acquisition order if both needed: workspace first, `.lbdb` second (deadlock prevention)

## 7. Schema

`schemas/workspace-state.schema.json` — JSON Schema 2020-12 with:

```json
{
  "version": "1.0",
  "project_hash": "<64-char blake3 hex>",
  "workspace": {
    "camera": { "x": <number>, "y": <number> },
    "zoom": <0-100>,
    "filters": [{ "kind": "...", "predicate": "..." }],
    "selection": { "kind": "...", "id": "..." } | null
  },
  "updated_at": "<ISO-8601>"
}
```

## 8. Scenarios (from spec.md)

| ID | Description |
|----|-------------|
| S1 | PUT valid → 204 + atomic write + GET returns same |
| S2 | PUT invalid schema → 400, no file modified |
| S3 | GET with no file → 200 null |
| S4 | GET source within cwd → 200 + content |
| S5 | GET source traversal → 403 |
| S6 | GET source line > total → adjusted to total |
| S7 | POST editor with $EDITOR → 204 + spawn |
| S8 | POST editor without editor → 503 |
| S9 | Concurrent PUTs serialized by flock |
| S10 | Cross-port restore (different ports, same XDG) |

## 9. Out of Scope

- Bundle URL in workspace state
- Per-view state
- WebSocket/SSE reactive sync
- localStorage/sessionStorage
- Daemon mode

## 10. References

- [ADR-041](../../docs/adr/ADR-041-workspace-state-persistence.md) — full ADR
- [spec.md](../../sddk/h1-durable-workspace-state/spec.md) — full spec
- [design.md](../../sddk/h1-durable-workspace-state/design.md) — design decisions
