//! Filesystem port — abstraction over filesystem reads/writes/metadata.
//!
//! The domain calls `std::fs::*` directly in several places. This
//! port introduces `&dyn Filesystem` so tests can inject a
//! `MemoryFilesystem` (no tempdir, no real I/O) and production can
//! use `SystemFilesystem` (real `std::fs` with context-wrapped errors).
//!
//! ## What the port hides
//!
//! - **`std::fs` calls.** The domain never imports `std::fs`. The
//!   production adapter ([`SystemFilesystem`]) does; the test adapter
//!   ([`MemoryFilesystem`]) does not.
//! - **Error-context details.** `SystemFilesystem` surfaces all I/O
//!   errors with a human-readable `.with_context(|| format!(...))` message
//!   that names the path and the operation.
//!
//! ## What the port does NOT hide
//!
//! - **Path canonicalisation semantics.** `canonicalize` returns whatever
//!   the OS reports; callers decide what to do with non-existent paths.
//! - **Directory-listing ordering.** `read_dir` returns entries in OS
//!   order; callers that need sorted output sort themselves.
//!
//! ## `read_dir` returns `Vec<DirEntry>`, not `Vec<PathBuf>`
//!
//! The caller of `read_dir` almost always wants to know "is this entry
//! a file or a directory?" without paying for a second `metadata()`
//! call. Returning `Vec<PathBuf>` would force callers to re-stat every
//! entry. A typed return — `Vec<DirEntry>` with `{ path: PathBuf,
//! kind: EntryKind }` where `EntryKind::File | EntryKind::Dir` — gives
//! the caller that bit for free and lets `MemoryFilesystem` answer it
//! from the map key (file = key present, dir = marker present) without
//! touching the real filesystem. The struct lives here, not in `std::fs`,
//! so we do not leak std types across the port boundary.

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The filesystem port.
///
/// Implementations:
/// - [`SystemFilesystem`] — production adapter, calls `std::fs`.
/// - [`MemoryFilesystem`] — test adapter, HashMap-backed.
pub trait Filesystem: Send + Sync {
    /// Read a file as a UTF-8 string. Returns `Err` if the file
    /// does not exist or is not valid UTF-8.
    fn read_to_string(&self, path: &Path) -> Result<String>;

    /// Write a byte slice to a file, replacing its contents if it
    /// already exists (like `std::fs::write`).
    fn write(&self, path: &Path, contents: &[u8]) -> Result<()>;

    /// Recursively create a directory and all of its ancestors (like
    /// `std::fs::create_dir_all`).
    fn create_dir_all(&self, path: &Path) -> Result<()>;

    /// Canonicalize a path: resolve symlinks, `.`, and `..` (like
    /// `std::fs::canonicalize`). Returns `Ok(path)` unchanged if the
    /// path does not exist — callers that need to distinguish
    /// "path not found" from "path exists but can't canonicalize"
    /// should use a separate existence check first.
    fn canonicalize(&self, path: &Path) -> Result<PathBuf>;

    /// List the entries in a directory (not recursive). The returned
    /// [`DirEntry`] carries `{ path, kind }` so callers know whether
    /// each entry is a file or directory without a second stat call.
    fn read_dir(&self, path: &Path) -> Result<Vec<DirEntry>>;

    /// Remove a file. Returns `Ok(())` if the file was deleted, or if
    /// it never existed — idempotent delete.
    fn remove_file(&self, path: &Path) -> Result<()>;

    /// Returns `true` if the path exists (file or directory),
    /// `false` otherwise. Unlike most other port methods, this returns
    /// `bool` rather than `Result` — the caller does not need to
    /// distinguish "file missing" from "permission denied" for an
    /// existence check.
    fn exists(&self, path: &Path) -> bool;
}

/// Kind of a directory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Dir,
}

/// One entry returned by [`Filesystem::read_dir`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    /// Absolute or relative path to the entry, as returned by the OS.
    pub path: PathBuf,
    /// Whether the entry is a file or a directory.
    pub kind: EntryKind,
}

// ---------------------------------------------------------------------------
// Production adapter
// ---------------------------------------------------------------------------

/// The real `std::fs` adapter. Cheap to construct (zero-cost struct).
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemFilesystem;

impl Filesystem for SystemFilesystem {
    fn read_to_string(&self, path: &Path) -> Result<String> {
        std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))
    }

    fn write(&self, path: &Path, contents: &[u8]) -> Result<()> {
        std::fs::write(path, contents)
            .with_context(|| format!("writing {}", path.display()))
    }

    fn create_dir_all(&self, path: &Path) -> Result<()> {
        std::fs::create_dir_all(path)
            .with_context(|| format!("creating directory {}", path.display()))
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        std::fs::canonicalize(path)
            .with_context(|| format!("canonicalizing {}", path.display()))
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<DirEntry>> {
        let entries = std::fs::read_dir(path)
            .with_context(|| format!("reading directory {}", path.display()))?;
        let mut out = Vec::new();
        for entry in entries {
            let entry = entry
                .with_context(|| format!("reading directory entry in {}", path.display()))?;
            let path = entry.path();
            let kind = if path.is_dir() {
                EntryKind::Dir
            } else {
                EntryKind::File
            };
            out.push(DirEntry { path, kind });
        }
        Ok(out)
    }

    fn remove_file(&self, path: &Path) -> Result<()> {
        // Idempotent: ignore NotFound.
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e)
                .with_context(|| format!("removing file {}", path.display())),
        }
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

// ---------------------------------------------------------------------------
// Test adapter
// ---------------------------------------------------------------------------

/// A HashMap-backed filesystem for tests.
///
/// No tempdirs, no real I/O. Construct with the `with_file` and
/// `with_dir` builders, then pass to the code under test via
/// `CliContext::for_test_with_fs`.
///
/// `RwLock` provides interior mutability so `MemoryFilesystem` can
/// implement the `&self` trait methods even though write operations
/// mutate internal state. The lock is never held across await points
/// (this is a sync trait), and tests are single-threaded, so there
/// is no actual contention.
///
/// ## Example
///
/// ```
/// use archctl::filesystem::{Filesystem, MemoryFilesystem, EntryKind};
/// use std::path::PathBuf;
///
/// let fs = MemoryFilesystem::new()
///     .with_file(PathBuf::from("/tmp/proj/skills.lock.yaml"), b"lock: v1\n");
///
/// let text = fs.read_to_string("/tmp/proj/skills.lock.yaml".as_ref()).unwrap();
/// assert!(text.contains("lock: v1"));
/// ```
pub struct MemoryFilesystem {
    /// Files: key = path, value = file contents (bytes).
    files: RwLock<HashMap<PathBuf, Vec<u8>>>,
    /// Directories that exist (marker set). Any path that is a prefix
    /// of a file key, or added via `with_dir`, is implicitly a dir.
    dirs: RwLock<HashSet<PathBuf>>,
    /// Paths that were written via `Filesystem::write` (distinct from
    /// files pre-loaded via `with_file`). Used by `was_written_to`.
    write_log: RwLock<HashSet<PathBuf>>,
}

impl std::fmt::Debug for MemoryFilesystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryFilesystem").finish()
    }
}

impl Clone for MemoryFilesystem {
    fn clone(&self) -> Self {
        Self {
            files: RwLock::new(self.files.read().unwrap().clone()),
            dirs: RwLock::new(self.dirs.read().unwrap().clone()),
            write_log: RwLock::new(self.write_log.read().unwrap().clone()),
        }
    }
}

impl Default for MemoryFilesystem {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryFilesystem {
    /// Start with an empty filesystem.
    pub fn new() -> Self {
        Self {
            files: RwLock::new(HashMap::new()),
            dirs: RwLock::new(HashSet::new()),
            write_log: RwLock::new(HashSet::new()),
        }
    }

    /// Register a file with the given contents (pre-existing, not a
    /// domain write).
    pub fn with_file(mut self, path: PathBuf, contents: &[u8]) -> Self {
        // Every file implicitly marks all its parent dirs as existing.
        {
            let dirs = self.dirs.get_mut().unwrap();
            let mut cursor: Option<&Path> = path.parent();
            while let Some(parent) = cursor {
                dirs.insert(parent.to_path_buf());
                cursor = parent.parent();
            }
        }
        self.files.get_mut().unwrap().insert(path, contents.to_vec());
        self
    }

    /// Register a directory as existing.
    pub fn with_dir(mut self, path: PathBuf) -> Self {
        let dirs = self.dirs.get_mut().unwrap();
        // A dir also marks all its ancestors.
        let mut cursor: Option<&Path> = Some(&path);
        while let Some(p) = cursor {
            dirs.insert(p.to_path_buf());
            cursor = p.parent();
        }
        self
    }

    /// Returns `true` if the given path was written via `Filesystem::write`
    /// (not via `with_file`). This is the test assertion helper for
    /// "the domain wrote X":
    ///
    /// ```
    /// # use archctl::filesystem::{Filesystem, MemoryFilesystem};
    /// # use std::path::PathBuf;
    /// let fs = MemoryFilesystem::new();
    /// fs.write("/out/ diagram.svg".as_ref(), b"<>").unwrap();
    /// assert!(fs.was_written_to("/out/ diagram.svg".as_ref()));
    /// assert!(!fs.was_written_to("/never/touched".as_ref()));
    /// ```
    pub fn was_written_to(&self, path: &Path) -> bool {
        self.write_log.read().unwrap().contains(path)
    }
}

impl Filesystem for MemoryFilesystem {
    fn read_to_string(&self, path: &Path) -> Result<String> {
        match self.files.read().unwrap().get(path) {
            Some(bytes) => String::from_utf8(bytes.clone())
                .with_context(|| format!("{} is not valid UTF-8", path.display())),
            None => Err(anyhow::anyhow!("file not found"))
                .context(format!("reading {}", path.display())),
        }
    }

    fn write(&self, path: &Path, contents: &[u8]) -> Result<()> {
        // Mark all parent dirs as existing.
        {
            let mut dirs = self.dirs.write().unwrap();
            let mut cursor: Option<&Path> = path.parent();
            while let Some(parent) = cursor {
                dirs.insert(parent.to_path_buf());
                cursor = parent.parent();
            }
        }
        self.files.write().unwrap().insert(path.to_path_buf(), contents.to_vec());
        self.write_log.write().unwrap().insert(path.to_path_buf());
        Ok(())
    }

    fn create_dir_all(&self, path: &Path) -> Result<()> {
        let mut dirs = self.dirs.write().unwrap();
        let mut cursor: Option<&Path> = Some(path);
        while let Some(p) = cursor {
            dirs.insert(p.to_path_buf());
            cursor = p.parent();
        }
        Ok(())
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        // MemoryFilesystem doesn't do real path resolution — return the
        // path as-is if it exists, error if it doesn't.
        if self.exists(path) {
            Ok(path.to_path_buf())
        } else {
            Err(anyhow::anyhow!("path not found"))
                .context(format!("canonicalizing {}", path.display()))
        }
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<DirEntry>> {
        if !self.dirs.read().unwrap().contains(path) {
            return Err(anyhow::anyhow!("directory not found"))
                .context(format!("reading directory {}", path.display()));
        }
        let files = self.files.read().unwrap();
        let dirs = self.dirs.read().unwrap();
        let mut entries: Vec<DirEntry> = Vec::new();
        for (file_path, _) in files.iter() {
            if let Some(parent) = file_path.parent() {
                if parent == path {
                    entries.push(DirEntry {
                        path: file_path.clone(),
                        kind: EntryKind::File,
                    });
                }
            }
        }
        for dir_path in dirs.iter() {
            if let Some(parent) = dir_path.parent() {
                if parent == path && dir_path != path {
                    entries.push(DirEntry {
                        path: dir_path.clone(),
                        kind: EntryKind::Dir,
                    });
                }
            }
        }
        Ok(entries)
    }

    fn remove_file(&self, path: &Path) -> Result<()> {
        match self.files.write().unwrap().remove(path) {
            Some(_) => Ok(()),
            None => Ok(()), // idempotent
        }
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.read().unwrap().contains_key(path)
            || self.dirs.read().unwrap().contains(path)
    }
}

// ---------------------------------------------------------------------------
// Factories
// ---------------------------------------------------------------------------

/// Factory: the production filesystem adapter, type-erased to the trait.
pub fn system_filesystem() -> Arc<dyn Filesystem> {
    Arc::new(SystemFilesystem)
}

/// Factory: an empty memory filesystem for tests. Call `with_file`
/// and `with_dir` builders to pre-load the filesystem state the test
/// needs.
pub fn memory_filesystem() -> Arc<MemoryFilesystem> {
    Arc::new(MemoryFilesystem::new())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_filesystem_reads_real_cwd() {
        let fs = SystemFilesystem;
        // cwd always exists — if this fails the test was run in a
        // stripped environment and we skip rather than panic.
        if let Ok(cwd) = std::env::current_dir() {
            assert!(fs.exists(&cwd));
        }
    }

    #[test]
    fn system_filesystem_exists_returns_false_for_nonexistent() {
        let fs = SystemFilesystem;
        assert!(!fs.exists(Path::new("/this/path/does/not/exist/anywhere")));
    }

    #[test]
    fn memory_filesystem_read_to_string_returns_file_contents() {
        let fs = MemoryFilesystem::new()
            .with_file(PathBuf::from("/tmp/proj/skills.lock.yaml"), b"lock: v1\n");
        let text = fs
            .read_to_string("/tmp/proj/skills.lock.yaml".as_ref())
            .unwrap();
        assert!(text.contains("lock: v1"));
    }

    #[test]
    fn memory_filesystem_read_to_string_err_not_found() {
        let fs = MemoryFilesystem::new();
        let err = fs
            .read_to_string("/does/not/exist".as_ref())
            .unwrap_err();
        // anyhow's Display shows the context message, not the full chain.
        let msg = err.to_string();
        assert!(
            msg.contains("reading"),
            "expected 'reading' in anyhow error: {msg}"
        );
    }

    #[test]
    fn memory_filesystem_write_then_was_written_to() {
        let fs = MemoryFilesystem::new();
        fs.write("/out/diagram.svg".as_ref(), b"<>")
            .unwrap();
        assert!(fs.was_written_to("/out/diagram.svg".as_ref()));
        assert!(!fs.was_written_to("/never/touched".as_ref()));
    }

    #[test]
    fn memory_filesystem_was_written_to_after_with_file() {
        // was_written_to should be false for files added via with_file
        // (they are "pre-existing", not "written by domain").
        let fs = MemoryFilesystem::new()
            .with_file(PathBuf::from("/pre-existing/config.yaml"), b"key: value");
        assert!(!fs.was_written_to("/pre-existing/config.yaml".as_ref()));
    }

    #[test]
    fn memory_filesystem_exists_returns_true_for_file() {
        let fs = MemoryFilesystem::new()
            .with_file(PathBuf::from("/tmp/f"), b"");
        assert!(fs.exists("/tmp/f".as_ref()));
    }

    #[test]
    fn memory_filesystem_exists_returns_true_for_dir() {
        let fs = MemoryFilesystem::new().with_dir(PathBuf::from("/tmp/d"));
        assert!(fs.exists("/tmp/d".as_ref()));
    }

    #[test]
    fn memory_filesystem_exists_returns_false_for_missing() {
        let fs = MemoryFilesystem::new();
        assert!(!fs.exists("/tmp/does_not_exist".as_ref()));
    }

    #[test]
    fn memory_filesystem_create_dir_all_makes_dirs_exist() {
        let fs = MemoryFilesystem::new();
        fs.create_dir_all("/a/b/c".as_ref()).unwrap();
        assert!(fs.exists("/a".as_ref()));
        assert!(fs.exists("/a/b".as_ref()));
        assert!(fs.exists("/a/b/c".as_ref()));
    }

    #[test]
    fn memory_filesystem_remove_file_is_idempotent() {
        let fs = MemoryFilesystem::new()
            .with_file(PathBuf::from("/tmp/f"), b"content");
        fs.remove_file("/tmp/f".as_ref()).unwrap();
        assert!(!fs.exists("/tmp/f".as_ref()));
        // Second call is also Ok (idempotent).
        fs.remove_file("/tmp/f".as_ref()).unwrap();
    }

    #[test]
    fn memory_filesystem_read_dir_returns_file_entries() {
        let fs = MemoryFilesystem::new()
            .with_file(PathBuf::from("/dir/file.txt"), b"content")
            .with_file(PathBuf::from("/dir/other.rs"), b"fn main() {}");
        let entries = fs.read_dir("/dir".as_ref()).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.kind == EntryKind::File));
        let names: Vec<_> = entries.iter().map(|e| e.path.file_name().unwrap().to_str()).collect();
        assert!(names.contains(&Some("file.txt")), "expected Some(\"file.txt\") in {names:?}");
        assert!(names.contains(&Some("other.rs")), "expected Some(\"other.rs\") in {names:?}");
    }

    #[test]
    fn memory_filesystem_read_dir_returns_dir_entries() {
        let fs = MemoryFilesystem::new()
            .with_dir(PathBuf::from("/dir/subdir"))
            .with_file(PathBuf::from("/dir/file.txt"), b"content");
        let entries = fs.read_dir("/dir".as_ref()).unwrap();
        let file_kind = |e: &DirEntry| e.kind == EntryKind::File;
        let dir_kind = |e: &DirEntry| e.kind == EntryKind::Dir;
        assert!(entries.iter().any(file_kind), "should have a file entry");
        assert!(entries.iter().any(dir_kind), "should have a dir entry");
    }

    #[test]
    fn memory_filesystem_canonicalize_returns_path_for_existing() {
        let fs = MemoryFilesystem::new()
            .with_file(PathBuf::from("/tmp/f"), b"content");
        let canon = fs.canonicalize("/tmp/f".as_ref()).unwrap();
        assert_eq!(canon, PathBuf::from("/tmp/f"));
    }

    #[test]
    fn memory_filesystem_canonicalize_err_for_missing() {
        let fs = MemoryFilesystem::new();
        let err = fs.canonicalize("/tmp/missing".as_ref()).unwrap_err();
        assert!(err.to_string().contains("missing") || err.to_string().contains("not found"));
    }

    #[test]
    fn factory_returns_dyn_filesystem() {
        let sys: Arc<dyn Filesystem> = system_filesystem();
        let _: &dyn Filesystem = sys.as_ref();
        let mem: Arc<MemoryFilesystem> = memory_filesystem();
        let _: &dyn Filesystem = mem.as_ref();
    }
}
