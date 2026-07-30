# Delta spec — Filesystem port

> **Change**: `refactor-1b-filesystem-port`
> **Cycle**: A-min (no design phase) · Path: A-min
> **Branch**: `feat/filesystem-port` @ `607ee64`
> **Status**: Completed and archived
>
> This file IS the main spec for the Filesystem port. No prior spec existed
> for this surface — this delta is the canonical record.

---

## What the Filesystem port is

The Filesystem port is a hexagonal abstraction layer that decouples all
domain code from direct `std::fs::*` calls. It follows the same pattern as
the existing `Clock` and `Environment` ports (`archctl/src/clock.rs`,
`archctl/src/environment.rs`).

**Trait** (`archctl/src/filesystem.rs`):

```rust
pub trait Filesystem: Send + Sync {
    fn read_to_string(&self, path: &Path) -> Result<String>;
    fn write(&self, path: &Path, contents: &[u8]) -> Result<()>;
    fn create_dir_all(&self, path: &Path) -> Result<()>;
    fn canonicalize(&self, path: &Path) -> Result<PathBuf>;
    fn read_dir(&self, path: &Path) -> Result<Vec<DirEntry>>;
    fn remove_file(&self, path: &Path) -> Result<()>;
    fn exists(&self, path: &Path) -> bool;
}
```

**Adapters**:

| Adapter | Purpose | Backing store |
|---|---|---|
| `SystemFilesystem` | Production | Delegates to `std::fs::*` with `.with_context()` on every error |
| `MemoryFilesystem` | Tests | `HashMap<PathBuf, Vec<u8>>` + `RwLock` interior mutability |

**Factories**:

```rust
pub fn system_filesystem() -> Arc<dyn Filesystem> { ... }
pub fn memory_filesystem() -> Arc<MemoryFilesystem> { ... }  // returns concrete so callers can chain .with_file()
```

**Re-exports** (`archctl/src/lib.rs`):
`Filesystem`, `SystemFilesystem`, `MemoryFilesystem`, `DirEntry`, `EntryKind`,
`system_filesystem`, `memory_filesystem`

---

## How to use it

### Production (real filesystem)

```rust
use archctl::{CliContext, system_filesystem};

let ctx = CliContext::production();
// ctx.fs is Arc<dyn Filesystem> backed by SystemFilesystem
let contents = ctx.fs.read_to_string(path)?;
```

### Tests (in-memory, no real I/O)

```rust
use archctl::{memory_filesystem, CliContext, FixedEnvironment};

let fs = memory_filesystem()
    .with_file("/tmp/proj/skills.lock.yaml".into(), b"lock: v1\n");
let env = Arc::new(FixedEnvironment::new());
let ctx = CliContext::for_test_with_fs(env, fs);
// Domain code reads from the in-memory map — no real filesystem touched
```

`MemoryFilesystem::with_file(path, bytes)` pre-loads a file.
`MemoryFilesystem::was_written_to(path)` returns `true` only for files
written via `fs.write(...)` during the test — NOT for pre-loaded files.

---

## The 8 migrated call sites

Every `std::fs::*` production call in the domain was threaded through
`&dyn Filesystem`:

| File | Line(s) | Function | Port method |
|---|---|---|---|
| `graph.rs` | 46, 58, 63, 78, 294 | `open_session`, `init`, `put_evidence` | `fs.read_to_string`, `fs.create_dir_all`, `fs.write` |
| `evidence.rs` | 176 | `extract` (production path) | `fs.read_to_string` |
| `skills.rs` | 39, 44, 76, 119, 122 | `load_lock`, `sync_skill`, `sync_skills`, `verify_skills`, `activate_skill` | `fs.read_to_string`, `fs.create_dir_all`, `fs.remove_file` |
| `render.rs` | 27, 30, 47 | render helpers | `fs.create_dir_all`, `fs.read_to_string`, `fs.write` |
| `tsg.rs` | 125 | `extract_with_rules` | `fs.read_to_string` |
| `identity.rs` | 49 | `safe_realpath(p, fs)` | `fs.canonicalize` |
| `xdg.rs` | 110 | `ensure_xdg(fs)` | `fs.create_dir_all` |
| `doctor.rs` | 40 | `check_*` family | `ctx.fs.create_dir_all` |

**Test fixtures exempt** (stay direct `std::fs::`): `inventory.rs:270-277`,
`evidence.rs:534-540`, `cli.rs:694-695`.

---

## Manifest and the `must_not_contain` gate

`manifests/filesystem.toml` registers the scope:

```toml
id = "filesystem"
version = "0.1.0"
cargo_dir = "archctl"
editable = ["archctl/src/filesystem.rs", "archctl/src/lib.rs"]
public_symbols = [
  "Filesystem", "DirEntry", "EntryKind",
  "SystemFilesystem", "MemoryFilesystem",
  "system_filesystem", "memory_filesystem",
]
must_hold = [
  "pub trait Filesystem",
  "pub struct SystemFilesystem",
  "pub struct MemoryFilesystem",
  "pub fn system_filesystem",
  "pub fn memory_filesystem",
  "with_context",
]
minimum_tests = 3
must_not_contain = ["use std::fs::"]
```

**Negative invariant** (`must_not_contain`): proves the port itself does not
import `std::fs`. The gate is implemented in `scope.rs` as
`ScopeGate::MustNotContainAbsent` — symmetric to `gate_must_hold_invariants`.
Both positive and negative branches are tested in `scope::tests`.

Verification: `archctl doctor --scopes filesystem` exits 0 (0 findings).

---

## Test results

| Metric | Baseline | Post-cycle | Delta |
|---|---|---|---|
| Unit tests | 89 | 107 | +18 |
| Doctests | 0 | 4 | +4 |
| **Total** | **89** | **111** | **+22** |

New tests: 16 in `filesystem::tests::` (one per trait method + positive/negative
for `was_written_to`), 2 in `scope::tests::gate_must_not_contain_*`,
2 doctests in `filesystem.rs`.

---

## Follow-up items (next cycle candidates)

1. **Migrate `scope.rs` to the Filesystem port** — ✅ DONE ✓
   (`refactor-1c-scope-port`, `feat/refactor-1c-scope-port` @ `87a2149`).
   Chicken-and-egg resolved: `manifests/scope.toml` declares its own gates
   independent of `manifests/filesystem.toml`. All 6 production `std::fs::*` sites
   in `scope.rs` migrated; see **Scope.rs migration** section below.
2. **Fix 3 remaining `Path::exists()` leaks** — `render.rs:8`, `skills.rs:80`,
   `skills.rs:187` bypass the port (C-W1 from debt-verify). Three 1-line edits
   to replace `path.exists()` with `fs.exists(path)`.
3. **ADR-016 B1** — Source + Evaluation types in graph; 4-6 hours.
4. **Extend existing manifests** (`clock`, `environment`, `identity`) with
   `must_not_contain` gates; 1 hour.
5. **CLI flag drift** — spec text says `--check-scope`, shipped CLI uses
   `--scopes`. Update spec text in next revision.
6. **`strip_cfg_test_blocks` unit test** — add targeted unit test for the
   state machine in `gate_must_not_contain_invariants`; out of scope per
   spec but flagged as future hardening.

---

## Scope.rs migration (`refactor-1c-scope-port`)

> Cycle: `refactor-1c-scope-port` · Branch: `feat/refactor-1c-scope-port` @ `87a2149`
> Path: A-min · Verify: PASS · Debt: PASS_WITH_WARNINGS (non-blocking YAGNI)

**Summary**: `scope.rs` is now the 9th migrated domain module. The chicken-and-egg
problem is resolved by `manifests/scope.toml` declaring its own gates independently
of `manifests/filesystem.toml`. Six production `std::fs::*` call sites in
`scope.rs` (lines 75, 87, 101, 244, 345, 374) are now port-routed. The new
manifest's `must_not_contain = ["use std::fs::"]` gate is guarded by
`strip_cfg_test_blocks` to prevent false positives on test-fixture data.

### The 6 migrated production call sites

| File:line | Function | Port method |
|---|---|---|
| `scope.rs:75` | `ScopeManifest::load` | `fs.read_to_string(&path)` |
| `scope.rs:87` | `ScopeManifest::load_all` | `fs.read_dir(&dir)` |
| `scope.rs:101` | `ScopeManifest::load_all` | `fs.read_to_string(&path)` |
| `scope.rs:244` | `gate_public_symbols_exist` | `fs.read_to_string(&project_root.join(path_str))` |
| `scope.rs:345` | `gate_must_hold_invariants` | `fs.read_to_string(&project_root.join(p))` |
| `scope.rs:374` | `gate_must_not_contain_invariants` | `fs.read_to_string(&project_root.join(p))` |

Gates that do **not** gain `fs` (D1 — they don't touch disk via `std::fs`):
`gate_editable_files_exist` (uses `Path::exists()` only), `gate_test_count_meets_minimum`
(uses `std::process::Command` only).

### New manifest

```toml
# manifests/scope.toml
id = "scope"
version = "0.1.0"
description = "Scope gate engine + manifest loader (migrated to Filesystem port)"
cargo_dir = "archctl"
editable = ["archctl/src/scope.rs", "archctl/src/doctor.rs"]
public_symbols = [
  "ScopeManifest", "ScopeCheckReport", "ScopeFinding", "ScopeSeverity",
  "ScopeGate", "check_scope", "check_all_scopes",
  "gate_editable_files_exist", "gate_public_symbols_exist",
  "gate_must_hold_invariants", "gate_must_not_contain_invariants",
  "gate_test_count_meets_minimum", "render_report_line",
]
must_hold = [
  "pub fn check_scope",
  "pub fn check_all_scopes",
  "pub fn gate_must_hold_invariants",
  "pub fn gate_must_not_contain_invariants",
]
minimum_tests = 8
must_not_contain = ["use std::fs::"]
```

### `strip_cfg_test_blocks` — post-audit fix (commit `87a2149`)

`manifests/scope.toml`'s `must_not_contain = ["use std::fs::"]` gate scans
`scope.rs` itself (which is in `editable_files`). Since `scope.rs` contains
`use std::fs::` inside its own `#[cfg(test)] mod tests` block (test fixtures
that set up tempdir state), the gate would self-trigger without this fix.

`strip_cfg_test_blocks` (`scope.rs:418-473`) is a per-line state machine that:
1. Detects `#[cfg(test)]` or `#[cfg_attr(..., test, ...)]` attributes
2. Skips the entire decorated item (mod/fn/struct) by brace-depth tracking
3. Returns the source text with test blocks removed before the substring search

Production code (lines 1-675 of `scope.rs`) is completely clean of `std::fs::*`;
all matches after line 676 are inside `#[cfg(test)]` blocks and are stripped
before the gate evaluates `must_not_contain`.

### Call chain (port threading verified end-to-end)

```
cli::run_inner (cli.rs:247)
  → doctor::check_scope(cwd, scope_ids, &*ctx.fs)          (cli.rs:255)
  → scope::check_all_scopes(cwd, fs)                         (doctor.rs:125)
  → ScopeManifest::load_all(cwd, fs)                        (scope.rs:649)
  → for each manifest: scope::check_scope(cwd, m, true, fs) (scope.rs:653)
  → gate_public_symbols_exist(cwd, m, fs)                   (scope.rs:635)
  → gate_must_hold_invariants(cwd, m, fs)                   (scope.rs:636)
  → gate_must_not_contain_invariants(cwd, m, fs)            (scope.rs:637)
```

Verification: `archctl doctor --scopes scope` exits 0 with `[OK  ] scope scope (0 findings)`.

### Cycle commits

| SHA | Subject |
|---|---|
| `adc9f0e` | refactor(scope): migrate std::fs calls to Filesystem port |
| `8fbdd9e` | chore(scope): register scope manifest with must_not_contain gate |
| `87a2149` | fix(scope): make must_not_contain gate ignore cfg(test) blocks |

### Test results

- **111 tests** (107 unit + 4 doctests, no regression from Refactor 1b baseline)
- 20 `scope::tests::*` all passing
- `archctl doctor --scopes scope` exits 0 with 0 findings

---

## Artifacts

### Refactor 1b — Filesystem port

| Artifact | Path |
|---|---|
| Spec | `sddk/refactor-1b-filesystem-port/spec.md` |
| Tasks | `sddk/refactor-1b-filesystem-port/tasks.md` |
| Verify report | `sddk/refactor-1b-filesystem-port/verify-report.md` |
| Debt report | `sddk/refactor-1b-filesystem-port/debt-report.md` |
| Archive report | `sddk/refactor-1b-filesystem-port/archive-report.md` |
| Manifest | `manifests/filesystem.toml` |
| Source | `archctl/src/filesystem.rs` |

### Refactor 1c — scope.rs migration

| Artifact | Path |
|---|---|
| Spec | `sddk/refactor-1c-scope-port/spec.md` |
| Tasks | `sddk/refactor-1c-scope-port/tasks.md` |
| Verify report | `sddk/refactor-1c-scope-port/verify-report.md` |
| Debt report | `sddk/refactor-1c-scope-port/debt-report.md` |
| Archive report | `sddk/refactor-1c-scope-port/archive-report.md` ← (this file) |
| Apply checkpoint | `sddk/refactor-1c-scope-port/apply-checkpoint.json` |
| Delta spec | `docs/specs/filesystem-port.md` (scope.rs section) |
| Manifest | `manifests/scope.toml` |
