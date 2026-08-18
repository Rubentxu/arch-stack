//! Secret redaction for strict export profiles (ADR-055 phase 2).
//!
//! Deny-by-default scanner: known secret shapes embedded in bundle
//! string fields are replaced with `[REDACTED:<kind>]` before a
//! strict bundle leaves the machine. Deterministic (same input →
//! same output), zero dependencies (no regex crate — matching is
//! manual substring scanning with context validation).
//!
//! Deliberate limits (documented in ADR-055):
//! - Pattern-based only. Entropy/heuristic detection is phase 3.
//! - Only strict bundles are scanned (`--profile strict`); the
//!   default profile never redacts (0 regression).
//! - Source bytes are not scanned because they are not part of a
//!   bundle (path:line references only).

use crate::diagram::export::BundleEnvelope;
use crate::diagram::export_types::Manifest;

/// Replace known secret shapes in `text` with `[REDACTED:<kind>]`.
/// Non-matching text passes through untouched.
pub fn redact_secrets(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while !rest.is_empty() {
        let mut earliest: Option<(usize, usize, &str)> = None; // (start, end, kind)

        for (kind, finder) in PATTERNS {
            if let Some((start, end)) = finder(rest) {
                let better = match earliest {
                    None => true,
                    Some((es, _, _)) => start < es,
                };
                if better {
                    earliest = Some((start, end, kind));
                }
            }
        }

        match earliest {
            Some((start, end, kind)) => {
                out.push_str(&rest[..start]);
                out.push_str("[REDACTED:");
                out.push_str(kind);
                out.push(']');
                rest = &rest[end..];
            }
            None => {
                out.push_str(rest);
                break;
            }
        }
    }

    out
}

/// Apply redaction to every string field of a bundle in place.
/// Call AFTER path relativization (Item 28) so paths are both
/// relative AND redacted.
pub fn redact_bundle(bundle: &mut BundleEnvelope) {
    bundle.manifest = redact_manifest(&bundle.manifest);
    for node in &mut bundle.projection.nodes {
        node.name = redact_secrets(&node.name);
        if let Some(d) = node.description.as_mut() {
            *d = redact_secrets(d);
        }
        if let Some(k) = node.canonical_key.as_mut() {
            *k = redact_secrets(k);
        }
        if let Some(l) = node.label_override.as_mut() {
            *l = redact_secrets(l);
        }
    }
    for edge in &mut bundle.projection.edges {
        edge.id = redact_secrets(&edge.id);
        edge.source = redact_secrets(&edge.source);
        edge.target = redact_secrets(&edge.target);
        edge.predicate = redact_secrets(&edge.predicate);
        if let Some(l) = edge.label.as_mut() {
            *l = redact_secrets(l);
        }
    }
    for entry in &mut bundle.evidence.evidence {
        entry.id = redact_secrets(&entry.id);
        entry.kind = redact_secrets(&entry.kind);
        entry.claim = redact_secrets(&entry.claim);
        entry.path = redact_secrets(&entry.path);
        entry.tool_name = redact_secrets(&entry.tool_name);
        entry.tool_version = redact_secrets(&entry.tool_version);
        entry.rule_id = redact_secrets(&entry.rule_id);
        entry.content_hash = redact_secrets(&entry.content_hash);
    }
}

fn redact_manifest(m: &Manifest) -> Manifest {
    let mut m = m.clone();
    m.view_selector = redact_secrets(&m.view_selector);
    m
}

type Finder = fn(&str) -> Option<(usize, usize)>;

/// Pattern table: (kind, finder). Ordered by specificity; the earliest
/// match in the text wins (ties resolved by table order).
const PATTERNS: &[(&str, Finder)] = &[
    ("private-key", find_private_key),
    ("aws-access-key", find_aws_access_key),
    ("github-token", find_github_token),
    ("slack-token", find_slack_token),
    ("jwt", find_jwt),
    ("url-credentials", find_url_credentials),
    ("generic-secret", find_generic_secret),
];

/// `-----BEGIN ... PRIVATE KEY-----` … `-----END ... PRIVATE KEY-----`
/// Redacts the whole block (header + body + footer).
fn find_private_key(text: &str) -> Option<(usize, usize)> {
    const START: &str = "-----BEGIN";
    const END_MARKER: &str = "-----END";
    let start = text.find(START)?;
    let after_start = &text[start..];
    // Require "PRIVATE KEY" in the header line.
    let header_end = after_start.find('\n').unwrap_or(after_start.len());
    if !after_start[..header_end].contains("PRIVATE KEY") {
        return None;
    }
    // Find the END marker after the header.
    let end_rel = after_start[header_end..].find(END_MARKER)?;
    let end = start + header_end + end_rel + 10; // include "-----END..."
    // Extend to the end of the footer line (up to 10 chars of trailer).
    let mut end = end.min(text.len());
    let rest = &text[end..];
    let line_end = rest.find('\n').unwrap_or(rest.len()).min(80);
    end += line_end;
    Some((start, end))
}

/// `AKIA` + 16 uppercase alphanumerics (AWS access key id).
fn find_aws_access_key(text: &str) -> Option<(usize, usize)> {
    let start = text.find("AKIA")?;
    let tail = &text[start + 4..];
    let mut end = 0usize;
    for (i, c) in tail.chars().enumerate() {
        if c.is_ascii_uppercase() || c.is_ascii_digit() {
            end = i + 1;
        } else {
            break;
        }
    }
    if end >= 16 {
        Some((start, start + 4 + end))
    } else {
        None
    }
}

/// GitHub tokens: `ghp_`, `gho_`, `ghu_`, `ghs_`, `ghr_` + 36+ chars.
fn find_github_token(text: &str) -> Option<(usize, usize)> {
    const PREFIXES: [&str; 5] = ["ghp_", "gho_", "ghu_", "ghs_", "ghr_"];
    for prefix in PREFIXES {
        if let Some(start) = text.find(prefix) {
            let tail = &text[start + prefix.len()..];
            let len = tail
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .count();
            if len >= 36 {
                return Some((start, start + prefix.len() + len));
            }
        }
    }
    None
}

/// Slack tokens: `xoxb-`, `xoxp-`, `xoxa-`, `xoxr-`, `xoxs-` + 10+ chars.
fn find_slack_token(text: &str) -> Option<(usize, usize)> {
    const PREFIXES: [&str; 5] = ["xoxb-", "xoxp-", "xoxa-", "xoxr-", "xoxs-"];
    for prefix in PREFIXES {
        if let Some(start) = text.find(prefix) {
            let tail = &text[start + prefix.len()..];
            let len = tail
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .count();
            if len >= 10 {
                return Some((start, start + prefix.len() + len));
            }
        }
    }
    None
}

/// JWT: `eyJ…` + two dot-separated segments (header.payload.signature).
fn find_jwt(text: &str) -> Option<(usize, usize)> {
    let start = text.find("eyJ")?;
    // Require base64url chars after the prefix.
    let tail = &text[start + 3..];
    let mut i = 0usize;
    let mut dots = 0usize;
    let mut seg_lens: Vec<usize> = vec![0];
    for c in tail.chars() {
        if c == '.' {
            if dots == 2 {
                break;
            }
            dots += 1;
            seg_lens.push(0);
            i += 1;
        } else if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            *seg_lens.last_mut().expect("non-empty") += 1;
            i += 1;
        } else {
            break;
        }
    }
    if dots == 2 && seg_lens.iter().all(|&l| l >= 2) {
        Some((start, start + 3 + i))
    } else {
        None
    }
}

/// URL with embedded credentials: `scheme://user:pass@`.
fn find_url_credentials(text: &str) -> Option<(usize, usize)> {
    let lower = text.to_ascii_lowercase();
    let marker = "://";
    let mut search_from = 0usize;
    while let Some(rel) = lower[search_from..].find(marker) {
        let scheme_start = search_from + rel;
        // Scheme must be http/https (conservative).
        let scheme = &lower[scheme_start.saturating_sub(5)..scheme_start];
        let is_http = scheme.ends_with("http") || scheme.ends_with("https");
        if !is_http {
            search_from = scheme_start + marker.len();
            continue;
        }
        let after = &text[scheme_start + marker.len()..];
        // user:pass@ — a colon before the first @ in the authority.
        let at = after.find('@')?;
        let authority = &after[..at];
        if authority.contains(':') {
            // Redact the credentials portion: scheme://[user:pass@]
            // Start: scheme start (before "://" — at most 5 chars).
            let start = scheme_start - 5;
            let end = scheme_start + marker.len() + at + 1; // include '@'
            return Some((start, end));
        }
        search_from = scheme_start + marker.len();
    }
    None
}

/// Generic assignments: `api_key=`, `apikey=`, `token=`, `secret=`,
/// `password=`, `passwd=` (case-insensitive) followed by a value of
/// 16+ non-space chars.
fn find_generic_secret(text: &str) -> Option<(usize, usize)> {
    const KEYS: [&str; 6] = [
        "api_key=",
        "apikey=",
        "token=",
        "secret=",
        "password=",
        "passwd=",
    ];
    let lower = text.to_ascii_lowercase();
    for key in KEYS {
        let mut search_from = 0usize;
        while let Some(rel) = lower[search_from..].find(key) {
            let start = search_from + rel;
            let value = &text[start + key.len()..];
            let len = value
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
                .count();
            if len >= 16 {
                return Some((start, start + key.len() + len));
            }
            search_from = start + key.len();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_aws_access_key() {
        // Key id built by concatenation so GitHub push protection does
        // not flag the test literal as a real AWS credential.
        let key = format!("key={} here", ["AKIA", "IOSFODNN7EXAMPLE1234"].concat());
        assert_eq!(redact_secrets(&key), "key=[REDACTED:aws-access-key] here");
    }

    #[test]
    fn redacts_github_token() {
        // Same concatenation trick as the AWS test.
        let token = format!("{}abcdefghijklmnopqrstuvwxyzABCDEFGH123456", "ghp_");
        assert_eq!(redact_secrets(&token), "[REDACTED:github-token]");
        // Short lookalike stays untouched.
        assert_eq!(redact_secrets("ghp_short"), "ghp_short");
    }

    #[test]
    fn redacts_private_key_block() {
        let key = "data: -----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA\n-----END RSA PRIVATE KEY-----\n";
        let redacted = redact_secrets(key);
        assert!(redacted.contains("[REDACTED:private-key]"));
        assert!(!redacted.contains("MIIEowIBAAKCAQEA"));
        assert!(!redacted.contains("PRIVATE KEY"));
    }

    #[test]
    fn redacts_jwt() {
        // jwt.io demo token assembled in parts (push protection safety).
        let header = format!("eyJ{}", "hbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
        let payload = "eyJzdWIiOiIxMjM0NTY3ODkwIn0";
        let signature = "SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let jwt = format!("Bearer {header}.{payload}.{signature}");
        let redacted = redact_secrets(&jwt);
        assert!(redacted.contains("[REDACTED:jwt]"));
        assert!(!redacted.contains("eyJhbGci"));
    }

    #[test]
    fn redacts_url_credentials() {
        assert_eq!(
            redact_secrets("repo at https://user:supersecret@example.com/x"),
            "repo at [REDACTED:url-credentials]example.com/x"
        );
    }

    #[test]
    fn redacts_generic_assignments() {
        // The whole `key=value` assignment is redacted (key name + value).
        assert_eq!(
            redact_secrets("export API_KEY=abcdefghijklmnop1234567890"),
            "export [REDACTED:generic-secret]"
        );
        // Short values are NOT redacted (conservative).
        assert_eq!(redact_secrets("token=abc"), "token=abc");
    }

    #[test]
    fn deterministic_output() {
        let aws = format!("AKIA{}", "IOSFODNN7EXAMPLE1234");
        let gh = format!("{}abcdefghijklmnopqrstuvwxyzABCDEFGH123456", "ghp_");
        let input = format!("{aws} and {gh}");
        assert_eq!(redact_secrets(&input), redact_secrets(&input));
    }

    #[test]
    fn redacts_slack_token() {
        // Concatenated to avoid GitHub push protection false positive.
        let token = format!("{}123456789012-abcdefghijklmnop", "xoxb-");
        assert_eq!(redact_secrets(&token), "[REDACTED:slack-token]");
    }

    #[test]
    fn plain_text_passes_through() {
        let text = "the quick brown fox jumps over the lazy dog";
        assert_eq!(redact_secrets(text), text);
    }
}
