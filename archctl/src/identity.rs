use anyhow::Result;
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use crate::Filesystem;

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
            let toplevel = repo.worktree()
                .map(|p| p.base().to_string_lossy().into_owned())
                .unwrap_or_else(|| cwd.to_string());
            let canonical_top = safe_realpath(&toplevel, fs)
                .map(|p| norm_dir(&p.to_string_lossy()))
                .unwrap_or_else(|_| norm_dir(&toplevel));

            // Get remote.origin.url via config snapshot
            let remote = repo.config_snapshot()
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
            return Ok(SourceIdentity::Git {
                repository_id,
                worktree_id,
                root_commit,
                toplevel: canonical_top,
                remote,
            });
        }
        Err(_) => {
            let canonical = safe_realpath(cwd, fs)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| cwd.to_string());
            return Ok(SourceIdentity::Directory {
                directory_id: blake_like(&format!("dir|{canonical}")),
                canonical_realpath: norm_dir(&canonical),
            });
        }
    }
}

pub fn portable_project_id(identity: &SourceIdentity) -> String {
    let mut hasher = Sha256::new();
    let serialized = match identity {
        SourceIdentity::Git { repository_id, worktree_id, root_commit, toplevel, remote } => {
            format!("git|{remote}|{root_commit}|{toplevel}|{repository_id}|{worktree_id}")
        }
        SourceIdentity::Directory { directory_id, canonical_realpath } => {
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
        SourceIdentity::Git { remote, root_commit, .. } => {
            let short = if root_commit.len() >= 12 { &root_commit[..12] } else { root_commit };
            format!("git:{remote}@{short}")
        }
        SourceIdentity::Directory { canonical_realpath, .. } => {
            format!("dir:{canonical_realpath}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_remote_strips_transport_and_credentials() {
        assert_eq!(normalize_remote("https://user:pass@github.com/foo/bar.git"), "github.com/foo/bar");
        assert_eq!(normalize_remote("git@github.com:foo/bar.git"), "github.com/foo/bar");
        assert_eq!(normalize_remote("https://github.com/foo/bar.git"), "github.com/foo/bar");
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
        for &i in &[0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 14, 15, 16, 17, 19, 20, 21, 22, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35] {
            assert!(matches!(bytes[i] as char, '0'..='9' | 'a'..='f'), "non-hex at {i}: {}", bytes[i] as char);
        }
    }

    #[test]
    fn identity_summary_formats_both_modes() {
        let dir = SourceIdentity::Directory {
            directory_id: "x".into(),
            canonical_realpath: "/tmp".into(),
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
}
