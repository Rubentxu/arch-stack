//! `source.rs` — source file preview for evidence (ADR-041 §5).
//!
//! Provides `source_preview` which reads a file at a given line range,
//! validates path containment, and returns the content with metadata.

use super::workspace::WorkspaceError;
use super::workspace::validate_path_under_cwd;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Maximum lines returned in a single preview (safety limit).
pub const MAX_LINES: usize = 2000;

/// Source preview result with content and metadata.
#[derive(Debug, serde::Serialize)]
pub struct SourcePreview {
    pub file: String,
    pub start_line: u32,
    pub total_lines: u32,
    pub content: Vec<String>,
    pub truncated: bool,
}

/// Read a source file and return a preview around `line`.
///
/// `line` is 1-indexed. If `line` exceeds `total_lines`, it is clamped.
/// Returns at most `MAX_LINES` lines with `truncated: true` if the file
/// exceeds that limit.
pub fn source_preview(
    file: &Path,
    line: Option<u32>,
    cwd: &Path,
) -> Result<SourcePreview, SourceError> {
    // Validate path containment first (before any I/O).
    let validated = validate_path_under_cwd(file, cwd)?;

    // Check if it's a directory.
    if validated.is_dir() {
        return Err(SourceError::IsDirectory);
    }

    // Open and count lines.
    let f = fs::File::open(&validated)?;
    let reader = BufReader::new(f);
    let all_lines: Vec<String> = reader
        .lines()
        .map(|l| l.map_err(SourceError::Io))
        .collect::<Result<Vec<_>, _>>()?;
    let total_lines = all_lines.len() as u32;

    // Determine start_line (1-indexed, clamp to total).
    let start_line = match line {
        None => 1,
        Some(l) if l < 1 => 1,
        Some(l) if l > total_lines => total_lines,
        Some(l) => l,
    };

    // Compute window: ±2 lines centered on start_line, clamped to [1, total].
    let window_start = start_line.saturating_sub(2).max(1);
    let window_end = (start_line + 2).min(total_lines);

    // Extract content.
    let content: Vec<String> =
        all_lines[(window_start as usize - 1)..(window_end as usize)].to_vec();

    // Check truncation: if the file has more than MAX_LINES, truncate.
    let truncated = total_lines > MAX_LINES as u32;
    let content = if truncated {
        content.into_iter().take(MAX_LINES).collect()
    } else {
        content
    };

    let file_display = file.to_string_lossy().to_string();
    Ok(SourcePreview {
        file: file_display,
        start_line,
        total_lines,
        content,
        truncated,
    })
}

/// Error types for source preview operations.
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("path is outside the allowed scope")]
    OutsideScope,
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("path is a directory, not a file")]
    IsDirectory,
    #[error("path is invalid: {0}")]
    InvalidPath(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<WorkspaceError> for SourceError {
    fn from(e: WorkspaceError) -> Self {
        match e {
            WorkspaceError::PathOutsideScope { .. } => SourceError::OutsideScope,
            WorkspaceError::NotFound => SourceError::NotFound("file not found".into()),
            WorkspaceError::PathInvalid(s) => SourceError::InvalidPath(s),
            WorkspaceError::CwdInvalid(s) => SourceError::InvalidPath(s),
            _ => SourceError::Io(std::io::Error::other(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_file(path: &Path, lines: &[&str]) {
        let mut f = fs::File::create(path).unwrap();
        for line in lines {
            writeln!(f, "{}", line).unwrap();
        }
    }

    #[test]
    fn source_preview_within_cwd() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        let file = src_dir.join("main.rs");
        make_file(&file, &["fn main() {", "    println!(\"hello\");", "}"]);
        let result = source_preview(&file, Some(1), tmp.path());
        assert!(result.is_ok());
        let preview = result.unwrap();
        assert_eq!(preview.start_line, 1);
        assert_eq!(preview.total_lines, 3);
        assert!(!preview.truncated);
        assert!(preview.content[0].contains("fn main"));
    }

    #[test]
    fn source_preview_line_clamped_to_total() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        let file = src_dir.join("main.rs");
        make_file(&file, &["line1", "line2", "line3"]);
        // Request line 999, should clamp to 3.
        let result = source_preview(&file, Some(999), tmp.path());
        assert!(result.is_ok());
        let preview = result.unwrap();
        assert_eq!(preview.start_line, 3);
    }

    #[test]
    fn source_preview_file_outside_cwd() {
        // Two sibling tempdirs: `inner` is the cwd, `outer` is outside it.
        let outer = tempfile::tempdir().unwrap();
        let secret = outer.path().join("secret.rs");
        fs::write(&secret, "secret contents").unwrap();
        let inner = tempfile::tempdir().unwrap();
        let result = source_preview(&secret, Some(1), inner.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SourceError::OutsideScope));
    }

    #[test]
    fn source_preview_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("nonexistent.rs");
        let result = source_preview(&file, Some(1), tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn source_preview_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("src");
        fs::create_dir(&dir).unwrap();
        let result = source_preview(&dir, Some(1), tmp.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SourceError::IsDirectory));
    }

    #[test]
    fn source_preview_truncated_at_max_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("large.rs");
        let lines: Vec<String> = (0..3000).map(|i| format!("line {}", i)).collect();
        let lines_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        make_file(&file, &lines_refs);
        let result = source_preview(&file, Some(1), tmp.path());
        assert!(result.is_ok());
        let preview = result.unwrap();
        assert!(preview.truncated);
        assert!(preview.content.len() <= MAX_LINES);
    }

    #[test]
    fn source_preview_line_zero_or_negative() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("main.rs");
        make_file(&file, &["line1", "line2"]);
        // line 0 should be treated as 1.
        let result = source_preview(&file, Some(0), tmp.path());
        assert!(result.is_ok());
        let preview = result.unwrap();
        assert_eq!(preview.start_line, 1);
    }

    #[test]
    fn source_preview_no_line_param() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("main.rs");
        make_file(&file, &["line1", "line2", "line3"]);
        // No line param → defaults to 1.
        let result = source_preview(&file, None, tmp.path());
        assert!(result.is_ok());
        let preview = result.unwrap();
        assert_eq!(preview.start_line, 1);
    }

    #[test]
    fn source_preview_window_centering() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("main.rs");
        // 10 lines, built as Vec<String> then borrowed as &str for make_file.
        let owned: Vec<String> = (0..10).map(|i| format!("line{i}")).collect();
        let lines: Vec<&str> = owned.iter().map(String::as_str).collect();
        make_file(&file, &lines);
        // Request line 5 → window should be [3, 7] (2 above, 2 below).
        let result = source_preview(&file, Some(5), tmp.path());
        assert!(result.is_ok());
        let preview = result.unwrap();
        assert_eq!(preview.start_line, 5);
        assert_eq!(preview.content.len(), 5); // lines 3,4,5,6,7
        assert!(preview.content[0].contains("line2")); // 3-1=2 index 0
        assert!(preview.content[2].contains("line4")); // 5-1=4 index 2
    }
}
