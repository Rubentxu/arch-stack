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

1. **Migrate `scope.rs` to the Filesystem port** — chicken-and-egg with
   manifest loader; `scope.rs` currently uses `std::fs::read_to_string` to load
   the manifest it then validates. Future cycle: `refactor-1c-scope-port`.
2. **Fix 3 remaining `Path::exists()` leaks** — `render.rs:8`, `skills.rs:80`,
   `skills.rs:187` bypass the port (C-W1 from debt-verify). Three 1-line edits
   to replace `path.exists()` with `fs.exists(path)`.
3. **ADR-016 B1** — Source + Evaluation types in graph; 4-6 hours.
4. **Extend existing manifests** (`clock`, `environment`, `identity`) with
   `must_not_contain` gates; 1 hour.
5. **CLI flag drift** — spec text says `--check-scope`, shipped CLI uses
   `--scopes`. Update spec text in next revision.

---

## Artifacts

| Artifact | Path |
|---|---|
| Spec | `sddk/refactor-1b-filesystem-port/spec.md` |
| Tasks | `sddk/refactor-1b-filesystem-port/tasks.md` |
| Verify report | `sddk/refactor-1b-filesystem-port/verify-report.md` |
| Debt report | `sddk/refactor-1b-filesystem-port/debt-report.md` |
| Archive report | `sddk/refactor-1b-filesystem-port/archive-report.md` |
| Manifest | `manifests/filesystem.toml` |
| Source | `archctl/src/filesystem.rs` |
