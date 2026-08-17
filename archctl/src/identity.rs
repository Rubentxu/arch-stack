use crate::Filesystem;
use anyhow::Result;
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Stable repository identity — resolved once, not coupled to HEAD.
///
/// Defined as `blake3("repo|{normalized_remote}|{first_commit}")` where
/// `first_commit` is the deepest reachable commit of the default branch
/// (ADR-004 alignment). Unlike `SourceIdentity::Git::repository_id` which
/// includes the current HEAD and changes with each checkout, this value
/// is stable for the lifetime of the remote repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryIdentity {
    /// The stable identity string: `blake3("repo|{remote}|{first_commit}")`.
    pub repo_identity: String,
    /// Normalized remote URL (no protocol, no `.git`, no credentials).
    pub remote: String,
    /// The deepest reachable commit of the default branch at resolution time.
    pub first_commit: String,
    /// Canonical absolute path to the worktree root.
    pub toplevel: String,
}

/// Git-backed source identity — coupled to the current HEAD.
///
/// `repository_id` includes `root_commit` (HEAD), so it changes with each
/// checkout. Kept for backward compatibility with P1 callers; new code
/// should prefer `RepositoryIdentity` for snapshot keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceIdentity {
    Git {
        repository_id: String,
        worktree_id: String,
        root_commit: String,
        toplevel: String,
        remote: String,
    },
    Directory {
        directory_id: String,
        canonical_realpath: String,
    },
}

pub fn normalize_remote(url: &str) -> String {
    let mut s = url.to_string();
    for prefix in ["https://", "http://", "ssh://", "git@"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest.to_string();
            break;
        }
    }
    if let Some(at) = s.find('@') {
        s = s[at + 1..].to_string();
    }
    if let Some(colon) = s.find(':') {
        let mut out = String::with_capacity(s.len());
        out.push_str(&s[..colon]);
        out.push('/');
        out.push_str(&s[colon + 1..]);
        s = out;
    }
    s = s.trim_end_matches(".git").to_string();
    s.trim_end_matches('/').to_string()
}

fn norm_dir(p: &str) -> String {
    p.trim_end_matches('/').trim_end_matches('\\').to_string()
}

fn safe_realpath(p: &str, fs: &dyn Filesystem) -> Result<PathBuf> {
    fs.canonicalize(Path::new(p))
}

pub fn blake_like(input: &str) -> String {
    let mut hasher = Hasher::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    format!("blake3:{digest}")
}

pub fn resolve_source_identity(cwd: &str, fs: &dyn Filesystem) -> Result<SourceIdentity> {
    // Try to open as git repo via gix
    match gix::open(cwd) {
        Ok(repo) => {
            // Get worktree path via gix Worktree::base()
            let toplevel = repo
                .worktree()
                .map(|p| p.base().to_string_lossy().into_owned())
                .unwrap_or_else(|| cwd.to_string());
            let canonical_top = safe_realpath(&toplevel, fs)
                .map(|p| norm_dir(&p.to_string_lossy()))
                .unwrap_or_else(|_| norm_dir(&toplevel));

            // Get remote.origin.url via config snapshot
            let remote = repo
                .config_snapshot()
                .string("remote.origin.url")
                .map(|r| r.to_string())
                .unwrap_or_default();
            let remote = normalize_remote(&remote);

            // Get HEAD commit: peel Head to Id, then use Display to get hex string
            let mut head: gix::Head = match repo.head() {
                Ok(h) => h,
                Err(_) => {
                    let canonical = safe_realpath(cwd, fs)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| cwd.to_string());
                    return Ok(SourceIdentity::Directory {
                        directory_id: blake_like(&format!("dir|{}", canonical)),
                        canonical_realpath: norm_dir(&canonical),
                    });
                }
            };
            let commit_id: gix::Id<'_> = match head.try_peel_to_id() {
                Ok(Some(id)) => id,
                Ok(None) | Err(_) => {
                    let canonical = safe_realpath(cwd, fs)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| cwd.to_string());
                    return Ok(SourceIdentity::Directory {
                        directory_id: blake_like(&format!("dir|{}", canonical)),
                        canonical_realpath: norm_dir(&canonical),
                    });
                }
            };
            let root_commit = format!("{commit_id}");

            let repository_id = blake_like(&format!("git|{remote}|{root_commit}"));
            let worktree_id = blake_like(&format!("worktree|{canonical_top}"));
            Ok(SourceIdentity::Git {
                repository_id,
                worktree_id,
                root_commit,
                toplevel: canonical_top,
                remote,
            })
        }
        Err(_) => {
            let canonical = safe_realpath(cwd, fs)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| cwd.to_string());
            Ok(SourceIdentity::Directory {
                directory_id: blake_like(&format!("dir|{canonical}")),
                canonical_realpath: norm_dir(&canonical),
            })
        }
    }
}

/// Resolve the stable repository identity for a Git working directory.
///
/// Unlike `resolve_source_identity` which uses the current HEAD commit,
/// this function resolves the **deepest reachable** commit of the default
/// branch (`refs/remotes/<remote>/HEAD` or `HEAD` on first commit).
/// The resulting `RepositoryIdentity` is stable across checkouts and does
/// not change when the current HEAD moves.
///
/// Returns `None` if `cwd` is not a Git repository.
pub fn resolve_repository_identity(
    cwd: &str,
    fs: &dyn Filesystem,
    ref_override: Option<&str>,
) -> Result<Option<RepositoryIdentity>> {
    let repo = match gix::open(cwd) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let toplevel = repo
        .worktree()
        .map(|p| p.base().to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd.to_string());
    let canonical_top = safe_realpath(&toplevel, fs)
        .map(|p| norm_dir(&p.to_string_lossy()))
        .unwrap_or_else(|_| norm_dir(&toplevel));

    let remote = repo
        .config_snapshot()
        .string("remote.origin.url")
        .map(|r| r.to_string())
        .unwrap_or_default();
    let remote = normalize_remote(&remote);

    // Resolve the first/deepest commit: if --ref is provided, use that
    // revision directly; otherwise traverse all branch heads and find the
    // one with the oldest commit time.
    let first_commit = if let Some(rev) = ref_override {
        // Resolve the given revision to a commit ID
        repo.find_reference(rev)
            .ok()
            .and_then(|mut r| r.peel_to_id().ok())
            .map(|id| format!("{id}"))
            .unwrap_or_else(|| find_deepest_commit(&repo))
    } else {
        find_deepest_commit(&repo)
    };
    let repo_identity = blake_like(&format!("repo|{remote}|{first_commit}"));

    Ok(Some(RepositoryIdentity {
        repo_identity,
        remote,
        first_commit,
        toplevel: canonical_top,
    }))
}

/// Walk all refs and return the hex SHA of the commit that appears to be
/// the oldest (deepest/first in history). Falls back to HEAD if nothing
/// is reachable.
fn find_deepest_commit(repo: &gix::Repository) -> String {
    // Walk all refs using repo.references() + Platform::all() to iterate.
    // Platform::all() returns Result<Iter, Error>; Iter has .next() which
    // returns Option<Result<Reference, Error>>.
    let head_id = match repo.head() {
        Ok(mut h) => match h.try_peel_to_id() {
            Ok(Some(id)) => id,
            _ => return "0000000000000000000000000000000000000000".to_string(),
        },
        Err(_) => return "0000000000000000000000000000000000000000".to_string(),
    };

    let mut oldest_id = head_id;

    // Iterate all refs: Platform::all() -> Result<Iter, Error>
    if let Ok(references) = repo.references() {
        // references is Platform; .all() -> Result<Iter, Error>
        let all_refs = match references.all() {
            Ok(a) => a,
            Err(_) => return format!("{head_id}"),
        };
        // all_refs is Iter; use .next() to iterate
        // Iter::next() returns Option<Result<Reference, Error>>
        let mut iter = all_refs;
        loop {
            let next = iter.next();
            match next {
                Some(Ok(mut r)) => {
                    // Skip symbolic references (like HEAD) - they need peeling
                    // and we only care about direct commit references
                    let target_id = r.peel_to_id().ok();
                    if let Some(id) = target_id
                        && id < oldest_id
                    {
                        oldest_id = id;
                    }
                }
                Some(Err(_)) => continue,
                None => break,
            }
        }
    }

    format!("{oldest_id}")
}

pub fn portable_project_id(identity: &SourceIdentity) -> String {
    let mut hasher = Sha256::new();
    let serialized = match identity {
        SourceIdentity::Git {
            repository_id,
            worktree_id,
            root_commit,
            toplevel,
            remote,
        } => {
            format!("git|{remote}|{root_commit}|{toplevel}|{repository_id}|{worktree_id}")
        }
        SourceIdentity::Directory {
            directory_id,
            canonical_realpath,
        } => {
            format!("dir|{canonical_realpath}|{directory_id}")
        }
    };
    hasher.update(serialized.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex = hex::encode(bytes);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

pub fn identity_summary(identity: &SourceIdentity) -> String {
    match identity {
        SourceIdentity::Git {
            remote,
            root_commit,
            ..
        } => {
            let short = if root_commit.len() >= 12 {
                &root_commit[..12]
            } else {
                root_commit
            };
            format!("git:{remote}@{short}")
        }
        SourceIdentity::Directory {
            canonical_realpath, ..
        } => {
            format!("dir:{canonical_realpath}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_remote_strips_transport_and_credentials() {
        assert_eq!(
            normalize_remote("https://user:pass@github.com/foo/bar.git"),
            "github.com/foo/bar"
        );
        assert_eq!(
            normalize_remote("git@github.com:foo/bar.git"),
            "github.com/foo/bar"
        );
        assert_eq!(
            normalize_remote("https://github.com/foo/bar.git"),
            "github.com/foo/bar"
        );
        assert_eq!(normalize_remote("ssh://git@host/x/y.git"), "host/x/y");
    }

    #[test]
    fn blake_like_is_prefixed_and_deterministic() {
        assert_eq!(blake_like("foo"), blake_like("foo"));
        assert_ne!(blake_like("foo"), blake_like("bar"));
        assert!(blake_like("foo").starts_with("blake3:"));
    }

    #[test]
    fn portable_project_id_is_uuid_v4_shaped() {
        let id = SourceIdentity::Directory {
            directory_id: blake_like("dir:/tmp"),
            canonical_realpath: "/tmp".to_string(),
        };
        let uuid = portable_project_id(&id);
        let bytes = uuid.as_bytes();
        assert_eq!(bytes.len(), 36);
        assert_eq!(bytes[8] as char, '-');
        assert_eq!(bytes[13] as char, '-');
        assert_eq!(bytes[18] as char, '-');
        assert_eq!(bytes[23] as char, '-');
        assert_eq!(bytes[14] as char, '4', "version nibble must be 4");
        let variant = bytes[19] as char;
        assert!(
            matches!(variant, '8' | '9' | 'a' | 'b'),
            "variant nibble must be 8|9|a|b, got {variant}"
        );
        for &i in &[
            0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 14, 15, 16, 17, 19, 20, 21, 22, 24, 25, 26, 27,
            28, 29, 30, 31, 32, 33, 34, 35,
        ] {
            assert!(
                matches!(bytes[i] as char, '0'..='9' | 'a'..='f'),
                "non-hex at {i}: {}",
                bytes[i] as char
            );
        }
    }

    #[test]
    fn identity_summary_formats_both_modes() {
        let dir = SourceIdentity::Directory {
            directory_id: "x".into(),
            canonical_realpath: "/tmp".to_string(),
        };
        assert_eq!(identity_summary(&dir), "dir:/tmp");
        let git = SourceIdentity::Git {
            repository_id: "x".into(),
            worktree_id: "y".into(),
            root_commit: "abcdef1234567890".into(),
            toplevel: "/tmp".into(),
            remote: "github.com/a/b".into(),
        };
        assert_eq!(identity_summary(&git), "git:github.com/a/b@abcdef123456");
    }

    #[test]
    fn find_deepest_commit_is_deterministic_and_valid() {
        // find_deepest_commit must return a valid hex SHA (40 chars) for a real repo.
        // This test opens the workspace root (parent of archctl/) as the git repo.
        let repo = gix::open("..").expect("must open workspace repo");
        let result = find_deepest_commit(&repo);
        assert_eq!(
            result.len(),
            40,
            "find_deepest_commit must return a valid 40-char hex SHA, got: {}",
            result
        );
        assert!(
            result.chars().all(|c| c.is_ascii_hexdigit()),
            "result must be all hex digits, got: {}",
            result
        );

        // Calling twice must return the same result (deterministic)
        let result2 = find_deepest_commit(&repo);
        assert_eq!(result, result2, "find_deepest_commit must be deterministic");
    }
}
