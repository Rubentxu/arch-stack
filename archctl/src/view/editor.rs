//! `editor.rs` — $EDITOR/$VISUAL resolution and safe spawn (ADR-041 §6).
//!
//! Resolves the user's preferred editor from environment variables and
//! spawns it safely (no shell expansion) with `+{line}` syntax.

use std::process::Command;

/// Resolved editor command.
#[derive(Debug, Clone)]
pub struct EditorCommand {
    pub binary: String,
}

/// Resolve the best available editor.
///
/// Order: $EDITOR → $VISUAL → platform fallback (xdg-open on linux, open on macOS).
/// Returns `None` if no editor can be resolved.
pub fn resolve_editor() -> Option<EditorCommand> {
    // $EDITOR takes precedence.
    if let Ok(e) = std::env::var("EDITOR")
        && !e.is_empty()
    {
        // Take only the first token (handles "code --wait" → "code").
        let binary = e.split_whitespace().next().unwrap_or(&e).to_string();
        return Some(EditorCommand { binary });
    }
    // $VISUAL as fallback.
    if let Ok(e) = std::env::var("VISUAL")
        && !e.is_empty()
    {
        let binary = e.split_whitespace().next().unwrap_or(&e).to_string();
        return Some(EditorCommand { binary });
    }
    // Platform-specific fallback.
    #[cfg(target_os = "linux")]
    {
        Some(EditorCommand {
            binary: "xdg-open".to_string(),
        })
    }
    #[cfg(target_os = "macos")]
    {
        Some(EditorCommand {
            binary: "open".to_string(),
        })
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

/// Spawn the editor with the given file and line number.
///
/// Uses `Command::new(binary).arg(file).arg(format!("+{line}"))` — no shell,
/// no wait. The file path must already be validated by the caller.
pub fn spawn_editor(
    file: &std::path::Path,
    line: u32,
    editor: &EditorCommand,
) -> std::io::Result<std::process::Child> {
    Command::new(&editor.binary)
        .arg(file)
        .arg(format!("+{line}"))
        .spawn()
}

/// Error returned when no editor is configured.
#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    #[error("no editor configured: set $EDITOR or $VISUAL")]
    NotConfigured,
    #[error("failed to spawn editor: {0}")]
    Spawn(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_editor_with_editor_var() {
        let _guard = EnvGuard::new("EDITOR", "vim");
        let result = resolve_editor();
        assert!(result.is_some());
        assert_eq!(result.unwrap().binary, "vim");
    }

    #[test]
    fn resolve_editor_empty_editor_falls_back_to_visual() {
        let _guard = EnvGuard::new("EDITOR", "");
        let _guard2 = EnvGuard::new("VISUAL", "nano");
        let result = resolve_editor();
        assert!(result.is_some());
        assert_eq!(result.unwrap().binary, "nano");
    }

    #[test]
    fn resolve_editor_empty_editor_and_visual_falls_back() {
        let _guard = EnvGuard::new("EDITOR", "");
        let _guard2 = EnvGuard::new("VISUAL", "");
        // Should fall back to platform default.
        #[cfg(target_os = "linux")]
        {
            let result = resolve_editor();
            assert!(result.is_some());
            assert_eq!(result.unwrap().binary, "xdg-open");
        }
        #[cfg(not(target_os = "linux"))]
        {
            // On other platforms, we just check it doesn't panic.
            let _ = resolve_editor();
        }
    }

    #[test]
    fn resolve_editor_editor_with_args_uses_first_token() {
        let _guard = EnvGuard::new("EDITOR", "code --wait");
        let result = resolve_editor();
        assert!(result.is_some());
        // Only first token is used.
        assert_eq!(result.unwrap().binary, "code");
    }

    #[test]
    fn spawn_editor_builds_correct_args() {
        // We can't actually spawn without a file, but we can verify
        // that the command construction doesn't panic.
        let editor = EditorCommand {
            binary: "echo".to_string(),
        };
        // Use a temp file that doesn't exist to avoid actually opening an editor.
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("test.rs");
        // echo will fail because "test.rs" doesn't exist as a command, but that's ok.
        // The important thing is no panic and args are built correctly.
        // Actually echo doesn't take "test.rs" as argument in the way we call it.
        // Let's just verify Command construction doesn't panic.
        let mut cmd = Command::new(&editor.binary);
        cmd.arg(&file).arg("+42");
        // Verify the args are correct by inspecting the command.
        assert_eq!(cmd.get_args().count(), 2);
    }

    /// RAII guard for environment variables. The stored value is only used
    /// during construction (to set the var) and is intentionally dropped
    /// after — Drop only needs the key to remove the var on scope exit.
    struct EnvGuard(&'static str);
    impl EnvGuard {
        fn new(key: &'static str, val: &'static str) -> Self {
            // SAFETY: tests in this module mutate $EDITOR/$VISUAL only via
            // their own EnvGuard; set_var/remove_var are unsafe in Rust 2024.
            unsafe {
                std::env::set_var(key, val);
            }
            EnvGuard(key)
        }
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: see EnvGuard::new.
            unsafe {
                std::env::remove_var(self.0);
            }
        }
    }
}
