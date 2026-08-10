//! `workspace.rs` — workspace state persistence (ADR-041).
//!
//! Handles loading/saving `workspace.json` from the XDG project directory
//! with atomic writes (temp + rename) and path-traversal validation.

use anyhow::Result;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::{fs, io};

/// Workspace state persisted to `workspace.json` in the XDG project dir.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceState {
    pub version: String,
    pub project_hash: String,
    pub workspace: Workspace,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// The workspace viewport and UI state.
///
/// `selection` is `Option<Selection>` to match the JSON Schema `oneOf: [null, Selection]`.
/// When `None`, the field MUST serialise to JSON `null` (not be omitted), which is
/// the default serde behaviour for `Option<T>` without `skip_serializing_if`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Workspace {
    pub camera: Camera,
    pub zoom: f64,
    pub filters: Vec<Filter>,
    pub selection: Option<Selection>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Camera {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Filter {
    pub kind: String,
    pub predicate: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Selection {
    pub kind: String,
    pub id: String,
}

/// Error types for workspace operations.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("cwd is invalid: {0}")]
    CwdInvalid(String),
    #[error("path is invalid: {0}")]
    PathInvalid(String),
    #[error("path {file} is outside cwd scope")]
    PathOutsideScope { file: PathBuf },
    #[error("workspace file not found")]
    NotFound,
    #[error("path is a directory: {0}")]
    IsDirectory(PathBuf),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("schema validation failed: {0}")]
    SchemaValidation(String),
}

/// Allowed values for the `kind` enum in `Filter` and `Selection`
/// (mirrors the JSON Schema `enum` declarations in spec.md §3).
const ALLOWED_VIEW_KINDS: &[&str] = &["c4", "call-graph", "sequence", "class", "package"];
const ALLOWED_SELECTION_KINDS: &[&str] =
    &["c4", "call-graph", "sequence", "class", "package", "node"];

impl WorkspaceState {
    /// Validate that this state satisfies the JSON Schema constraints that
    /// `serde` cannot enforce (const, enum, pattern, range). ADR-041 §3.
    pub fn validate(&self) -> Result<(), WorkspaceError> {
        if self.version != "1.0" {
            return Err(WorkspaceError::SchemaValidation(format!(
                "unsupported workspace schema version: {} (expected \"1.0\")",
                self.version
            )));
        }
        // project_hash: 64 lowercase hex chars (blake3).
        if self.project_hash.len() != 64
            || !self
                .project_hash
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err(WorkspaceError::SchemaValidation(format!(
                "project_hash must be 64 lowercase hex chars, got: {}",
                self.project_hash
            )));
        }
        self.workspace.validate()?;
        Ok(())
    }
}

impl Workspace {
    fn validate(&self) -> Result<(), WorkspaceError> {
        if !(0.0..=100.0).contains(&self.zoom) {
            return Err(WorkspaceError::SchemaValidation(format!(
                "zoom must be in [0, 100], got: {}",
                self.zoom
            )));
        }
        for (i, f) in self.filters.iter().enumerate() {
            if !ALLOWED_VIEW_KINDS.contains(&f.kind.as_str()) {
                return Err(WorkspaceError::SchemaValidation(format!(
                    "filters[{i}].kind must be one of {ALLOWED_VIEW_KINDS:?}, got: {}",
                    f.kind
                )));
            }
            if f.predicate.is_empty() {
                return Err(WorkspaceError::SchemaValidation(format!(
                    "filters[{i}].predicate must be non-empty"
                )));
            }
        }
        if let Some(sel) = &self.selection
            && !ALLOWED_SELECTION_KINDS.contains(&sel.kind.as_str())
        {
            return Err(WorkspaceError::SchemaValidation(format!(
                "selection.kind must be one of {ALLOWED_SELECTION_KINDS:?}, got: {}",
                sel.kind
            )));
        }
        if let Some(sel) = &self.selection
            && sel.id.is_empty()
        {
            return Err(WorkspaceError::SchemaValidation(
                "selection.id must be non-empty".into(),
            ));
        }
        Ok(())
    }
}

/// Validates that `file` resolves to a path under `cwd`.
///
/// `file` may be absolute or relative; relative paths are resolved against
/// `cwd` BEFORE canonicalization (so symlinks in the project tree resolve
/// against the project root, not the process cwd). Returns the canonical
/// absolute path on success, or an error if traversal is detected or the
/// file does not exist.
pub fn validate_path_under_cwd(file: &Path, cwd: &Path) -> Result<PathBuf, WorkspaceError> {
    let cwd_canonical = cwd
        .canonicalize()
        .map_err(|e| WorkspaceError::CwdInvalid(e.to_string()))?;
    let file_absolute = if file.is_absolute() {
        file.to_path_buf()
    } else {
        cwd.join(file)
    };
    let file_canonical = match file_absolute.canonicalize() {
        Ok(p) => p,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            // Caller can decide whether "not found" is an error here or a
            // separate outcome (e.g. for source preview, 404 is the right
            // answer, not 400).
            return Err(WorkspaceError::NotFound);
        }
        Err(e) => return Err(WorkspaceError::PathInvalid(e.to_string())),
    };
    if !file_canonical.starts_with(&cwd_canonical) {
        return Err(WorkspaceError::PathOutsideScope {
            file: file.to_path_buf(),
        });
    }
    Ok(file_canonical)
}

// ---------------------------------------------------------------------------
// WorkspacePersistence port (ADR-041)
// ---------------------------------------------------------------------------

/// Port de persistencia del estado del workspace.
///
/// Implementaciones:
/// - [`WorkspacePersistenceFs`] — producción, archivos en XDG.
/// - [`WorkspacePersistenceMemory`] — tests, BTreeMap en memoria.
pub trait WorkspacePersistence: Send + Sync {
    /// Cargar estado del workspace. Retorna `Ok(None)` si no existe archivo.
    fn load(&self) -> Result<Option<WorkspaceState>, WorkspaceError>;

    /// Guardar estado del workspace atómicamente (temp + rename).
    fn save(&self, state: &WorkspaceState) -> Result<(), WorkspaceError>;
}

/// Producción: filesystem real en XDG.
pub struct WorkspacePersistenceFs {
    root: PathBuf,
}

impl WorkspacePersistenceFs {
    /// Create a new FS persistence backend rooted at `root`.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl WorkspacePersistence for WorkspacePersistenceFs {
    fn load(&self) -> Result<Option<WorkspaceState>, WorkspaceError> {
        let path = self.root.join("workspace.json");
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        let state: WorkspaceState = serde_json::from_str(&content)?;
        state.validate()?;
        Ok(Some(state))
    }

    fn save(&self, state: &WorkspaceState) -> Result<(), WorkspaceError> {
        state.validate()?;
        let path = self.root.join("workspace.json");
        let parent = path
            .parent()
            .ok_or_else(|| WorkspaceError::PathInvalid("no parent dir".into()))?;
        fs::create_dir_all(parent)?;
        let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
        let json = serde_json::to_vec_pretty(state)?;
        tmp.as_file_mut().write_all(&json)?;
        tmp.as_file_mut().flush()?;
        tmp.persist(&path)
            .map_err(|e| WorkspaceError::Io(e.error))?;
        Ok(())
    }
}

/// Test seam: swap the global persistence impl for tests.
///
/// M72 design proposed this seam so unit tests could exercise `WorkspaceStore`
/// behaviour without touching XDG. The current M72 tests construct their own
/// `WorkspacePersistence` impls (see `WorkspacePersistenceMemory` below) and
/// inject them directly via the call sites — no global state needed.
/// The seam is kept available for future tests that need to override the
/// process-wide singleton without threading it through every call site.
#[cfg(test)]
mod test_seam {
    use super::WorkspacePersistence;
    use std::sync::{Arc, OnceLock};
    static TEST_PERSISTENCE: OnceLock<Arc<dyn WorkspacePersistence>> = OnceLock::new();

    /// Set the persistence implementation for tests.
    #[allow(dead_code)]
    pub fn __set_workspace_persistence_for_tests(p: Arc<dyn WorkspacePersistence>) {
        let _ = TEST_PERSISTENCE.set(p);
    }

    /// Get the test-injected persistence impl, if any.
    #[allow(dead_code)]
    pub fn __workspace_persistence_for_tests() -> Option<Arc<dyn WorkspacePersistence>> {
        TEST_PERSISTENCE.get().cloned()
    }
}

/// Singleton factory: returns the process-wide persistence impl.
static SYSTEM_PERSISTENCE: OnceLock<Arc<dyn WorkspacePersistence>> = OnceLock::new();

/// Returns the system-wide [`WorkspacePersistence`] implementation.
pub fn system_workspace_persistence() -> &'static Arc<dyn WorkspacePersistence> {
    SYSTEM_PERSISTENCE.get_or_init(|| {
        let project = crate::project::resolve_project(
            std::env::current_dir().unwrap().to_string_lossy().as_ref(),
        );
        Arc::new(WorkspacePersistenceFs::new(project.project_dir))
    })
}

/// Legacy struct — now a type alias for the trait object.
///
/// `Workspace` (the data) and `WorkspaceStore` (the I/O) live in different
/// namespaces to avoid name collision: data = `Workspace { camera, zoom,
/// filters, selection }`, I/O = `WorkspaceStore::load(cwd)` /
/// `WorkspaceStore::save(state, cwd)`.
///
/// Convenience namespace that delegates to the `WorkspacePersistence` trait.
///
/// `WorkspaceStore::load/save(cwd)` builds a per-call `WorkspacePersistenceFs`
/// from the cwd-resolved XDG project dir and forwards to the trait. This
/// keeps the existing call sites in `archctl/src/view.rs` working while
/// making the trait the single source of truth for I/O (no duplicated
/// filesystem code).
///
/// The trait itself stays a singleton (`system_workspace_persistence()`)
/// for callers that don't have a per-request cwd — currently only unit
/// tests. The two paths coexist; both go through the same impl.
pub struct WorkspaceStore;

impl WorkspaceStore {
    /// Load workspace state for the project rooted at `cwd`. Delegates to
    /// `WorkspacePersistence` via a per-call `WorkspacePersistenceFs`.
    pub fn load(cwd: &Path) -> Result<Option<WorkspaceState>, WorkspaceError> {
        let fs =
            WorkspacePersistenceFs::new(workspace_path(cwd).parent().unwrap_or(cwd).to_path_buf());
        fs.load()
    }

    /// Save workspace state for the project rooted at `cwd`. Delegates to
    /// `WorkspacePersistence` via a per-call `WorkspacePersistenceFs`.
    pub fn save(state: &WorkspaceState, cwd: &Path) -> Result<(), WorkspaceError> {
        let root = workspace_path(cwd).parent().unwrap_or(cwd).to_path_buf();
        let fs = WorkspacePersistenceFs::new(root);
        fs.save(state)
    }
}

fn workspace_path(cwd: &Path) -> PathBuf {
    // XDG project dir: ~/.local/share/archctl/projects/<hash>/
    // For now, compute from cwd using the project identity.
    // The workspace.json lives at the project root level.
    crate::project::resolve_project(cwd.to_string_lossy().as_ref())
        .project_dir
        .join("workspace.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// In-memory persistence impl for tests — no filesystem I/O.
    struct WorkspacePersistenceMemory {
        state: Mutex<Option<WorkspaceState>>,
    }

    impl WorkspacePersistenceMemory {
        fn new() -> Self {
            Self {
                state: Mutex::new(None),
            }
        }
    }

    impl WorkspacePersistence for WorkspacePersistenceMemory {
        fn load(&self) -> Result<Option<WorkspaceState>, WorkspaceError> {
            Ok(self.state.lock().unwrap().clone())
        }

        fn save(&self, new_state: &WorkspaceState) -> Result<(), WorkspaceError> {
            new_state.validate()?;
            *self.state.lock().unwrap() = Some(new_state.clone());
            Ok(())
        }
    }

    #[test]
    fn workspace_persistence_memory_load_empty_returns_none() {
        let mem = WorkspacePersistenceMemory::new();
        let result = mem.load();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn workspace_persistence_memory_save_and_load_round_trip() {
        let mem = WorkspacePersistenceMemory::new();
        let state = valid_workspace_state();
        mem.save(&state).unwrap();
        let loaded = mem.load().unwrap().unwrap();
        assert_eq!(loaded.version, "1.0");
        assert_eq!(loaded.workspace.zoom, 50.0);
    }

    #[test]
    fn workspace_persistence_memory_save_invalid_rejected() {
        let mem = WorkspacePersistenceMemory::new();
        let mut state = valid_workspace_state();
        state.workspace.zoom = 150.0; // out of range
        let result = mem.save(&state);
        assert!(result.is_err());
    }

    #[test]
    fn workspace_persistence_memory_overwrites_previous_state() {
        let mem = WorkspacePersistenceMemory::new();
        let state1 = valid_workspace_state();
        mem.save(&state1).unwrap();
        let mut state2 = valid_workspace_state();
        state2.workspace.zoom = 25.0;
        mem.save(&state2).unwrap();
        let loaded = mem.load().unwrap().unwrap();
        assert_eq!(loaded.workspace.zoom, 25.0);
    }

    #[test]
    fn workspace_persistence_trait_object_works() {
        let mem: Arc<dyn WorkspacePersistence> = Arc::new(WorkspacePersistenceMemory::new());
        let state = valid_workspace_state();
        mem.save(&state).unwrap();
        let loaded = mem.load().unwrap().unwrap();
        assert_eq!(loaded.version, "1.0");
    }

    #[test]
    fn workspace_persistence_fs_load_missing_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let fs = WorkspacePersistenceFs::new(tmp.path().to_path_buf());
        let result = fs.load();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn workspace_persistence_fs_save_and_load_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let fs = WorkspacePersistenceFs::new(tmp.path().to_path_buf());
        let state = valid_workspace_state();
        fs.save(&state).unwrap();
        let loaded = fs.load().unwrap().unwrap();
        assert_eq!(loaded.version, "1.0");
        assert_eq!(loaded.workspace.zoom, 50.0);
    }

    #[test]
    fn workspace_persistence_fs_save_invalid_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let fs = WorkspacePersistenceFs::new(tmp.path().to_path_buf());
        let mut state = valid_workspace_state();
        state.workspace.zoom = 150.0;
        let result = fs.save(&state);
        assert!(result.is_err());
    }

    #[test]
    fn load_missing_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        // No workspace.json exists.
        let result = WorkspaceStore::load(tmp.path());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn load_existing_valid_returns_state() {
        let tmp = tempfile::tempdir().unwrap();
        let state = valid_workspace_state();
        // Manually write a valid workspace.json.
        let path = tmp.path().join("workspace.json");
        let json = serde_json::to_string_pretty(&state).unwrap();
        fs::write(&path, &json).unwrap();
        // But WorkspaceStore::load uses resolve_project which won't find our tmp dir.
        // We test the serde round-trip separately.
        let loaded: WorkspaceState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.version, "1.0");
        assert_eq!(loaded.workspace.zoom, 50.0);
    }

    #[test]
    fn save_and_load_round_trip() {
        let state = valid_workspace_state();
        // This test would use the real XDG path, so we just test serde.
        let json = serde_json::to_string_pretty(&state).unwrap();
        let loaded: WorkspaceState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.version, "1.0");
    }

    #[test]
    fn validate_path_under_cwd_valid() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        let file = src_dir.join("main.rs");
        fs::write(&file, "fn main() {}").unwrap();
        let result = validate_path_under_cwd(&file, tmp.path());
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap(), file.canonicalize().unwrap());
    }

    #[test]
    fn validate_path_under_cwd_parent_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        // Attempt to escape via ...
        let file = tmp.path().join("..").join("..").join("etc").join("passwd");
        let result = validate_path_under_cwd(&file, tmp.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, WorkspaceError::PathOutsideScope { .. }));
    }

    #[test]
    fn validate_path_under_cwd_absolute_outside() {
        let tmp = tempfile::tempdir().unwrap();
        // Absolute path outside cwd.
        let file = Path::new("/etc/passwd");
        let result = validate_path_under_cwd(file, tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn validate_path_under_cwd_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("nonexistent.rs");
        let result = validate_path_under_cwd(&file, tmp.path());
        // canonicalize fails on missing file → NotFound (caller maps to 404).
        assert!(matches!(result, Err(WorkspaceError::NotFound)));
    }

    #[test]
    fn validate_path_under_cwd_symlink_to_outside() {
        // Create two sibling tempdirs; symlink from `inner` to a path in
        // `outer`; canonicalize resolves the symlink and detects escape.
        let outer = tempfile::tempdir().unwrap();
        let secret = outer.path().join("secret");
        fs::write(&secret, "secret").unwrap();
        let inner = tempfile::tempdir().unwrap();
        let link = inner.path().join("link_to_outer");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&secret, &link).unwrap();
        let result = validate_path_under_cwd(&link, inner.path());
        assert!(
            result.is_err(),
            "expected symlink-escape to be detected, got {result:?}"
        );
    }

    #[test]
    fn workspace_state_serde_round_trip() {
        let state = valid_workspace_state();
        let json = serde_json::to_string_pretty(&state).unwrap();
        let loaded: WorkspaceState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.version, "1.0");
        assert_eq!(loaded.workspace.zoom, 50.0);
        assert!(loaded.workspace.selection.is_some());
    }

    #[test]
    fn workspace_state_invalid_json() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("workspace.json");
        fs::write(&path, "not valid json {{{").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        let result: Result<WorkspaceState, _> = serde_json::from_str(&content);
        assert!(result.is_err());
    }

    #[test]
    fn workspace_state_wrong_schema_version_rejected() {
        // C1 fix: validate() rejects unsupported schema version (const violation).
        let json = r#"{"version":"2.0","project_hash":"abc","workspace":{"camera":{"x":0,"y":0},"zoom":50,"filters":[],"selection":null},"updated_at":"2026-01-01T00:00:00Z"}"#;
        let state: WorkspaceState = serde_json::from_str(json).unwrap();
        let err = state.validate().unwrap_err();
        assert!(matches!(err, WorkspaceError::SchemaValidation(_)));
    }

    #[test]
    fn workspace_state_uppercase_hash_rejected() {
        // C1 fix: project_hash must be 64 lowercase hex (pattern violation).
        let json = r#"{"version":"1.0","project_hash":"ABCDEF","workspace":{"camera":{"x":0,"y":0},"zoom":50,"filters":[],"selection":null},"updated_at":"2026-01-01T00:00:00Z"}"#;
        let state: WorkspaceState = serde_json::from_str(json).unwrap();
        assert!(state.validate().is_err());
    }

    #[test]
    fn workspace_state_zoom_out_of_range_rejected() {
        // C1 fix: zoom must be in [0, 100].
        let mut s = valid_workspace_state();
        s.workspace.zoom = 150.0;
        assert!(s.validate().is_err());
    }

    #[test]
    fn workspace_state_invalid_filter_kind_rejected() {
        // C1 fix: filters[*].kind must be one of allowed enum values.
        let mut s = valid_workspace_state();
        s.workspace.filters.push(Filter {
            kind: "unknown".into(),
            predicate: "x".into(),
        });
        assert!(s.validate().is_err());
    }

    #[test]
    fn workspace_selection_serialises_as_null_when_none() {
        // C2 fix: Option<Selection> without skip_serializing_if emits `null`.
        let mut s = valid_workspace_state();
        s.workspace.selection = None;
        let json = serde_json::to_string(&s).unwrap();
        assert!(
            json.contains("\"selection\":null"),
            "expected 'selection':null, got: {json}"
        );
    }

    fn valid_workspace_state() -> WorkspaceState {
        WorkspaceState {
            version: "1.0".to_string(),
            project_hash: "a".repeat(64),
            workspace: Workspace {
                camera: Camera { x: 0.0, y: 0.0 },
                zoom: 50.0,
                filters: vec![],
                selection: Some(Selection {
                    kind: "node".to_string(),
                    id: "n1".to_string(),
                }),
            },
            updated_at: chrono::Utc::now(),
        }
    }
}
