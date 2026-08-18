//! Secret redaction for strict export profiles (ADR-055).
//!
//! Deny-by-default scanner: known secret shapes embedded in bundle
//! string fields are replaced with `[REDACTED:<kind>]` before a
//! strict bundle leaves the machine. Deterministic (same input →
//! same output), zero dependencies (no regex crate — matching is
//! manual substring scanning with context validation).
//!
//! Detection layers (ADR-055 phases):
//! - Phase 2: known patterns — AWS AKIA, GitHub/Slack tokens, private
//!   keys, JWTs, URLs with credentials, generic assignments.
//! - Phase 3: entropy heuristic — alphanumeric tokens ≥ 32 chars with
//!   Shannon entropy ≥ 4.0 bits/char are redacted as
//!   `[REDACTED:high-entropy]` (catches custom API keys with no known
//!   prefix). Hex hashes (blake3/sha256 ≈ 3.3 bits/char) and short
//!   identifiers survive.
//!
//! Field allowlist (what `redact_bundle` scans): every STRING field of
//! the bundle — manifest.view_selector, node name/description/
//! canonical_key/label_override, edge id/source/target/predicate/label,
//! evidence id/kind/claim/path/tool_name/tool_version/rule_id/
//! content_hash. NOT scanned (safe by construction): numeric fields
//! (start_line/end_line/confidence/x/y), hashes already validated by
//! schema pattern (baseRevision blake3, content_hash), timestamps
//! (generatedAt/observedAt). Evidence paths are relativized BEFORE
//! redaction (Item 28) so both protections compose.
//!
//! Deliberate limits (documented in ADR-055):
//! - Heuristic only — entropy is a bar, not a proof; exotic secrets
//!   below the bars leak by design (conservative false-negative).
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
    ("high-entropy", find_high_entropy),
];

/// Minimum token length for the entropy heuristic. Short tokens are
/// too common to be meaningful (e.g. a 24-char hex id has ~3.3
/// bits/char and would never pass the 4.0 bar anyway, but keep the
/// length gate explicit).
const ENTROPY_MIN_LEN: usize = 32;
/// Shannon entropy bar (bits per char). Random-looking secrets (API
/// keys, tokens without a known prefix) usually sit at 4.0–5.5;
/// hex ids and base32 identifiers sit below ~3.5.
const ENTROPY_MIN_BITS: f64 = 4.0;

/// Entropy heuristic (ADR-055 phase 3): any alphanumeric token of
/// ≥ [`ENTROPY_MIN_LEN`] chars with Shannon entropy ≥
/// [`ENTROPY_MIN_BITS`] bits/char is treated as a secret. Catches
/// API keys and tokens with no known prefix. Deliberately
/// conservative (long + high entropy only) to avoid false positives
/// on hashes (hex ≈ 3.3 bits/char) and long identifiers.
fn find_high_entropy(text: &str) -> Option<(usize, usize)> {
    let mut start = 0usize;
    let mut in_token = false;
    let mut token_start = 0usize;
    for (i, c) in text.char_indices() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            if !in_token {
                in_token = true;
                token_start = i;
            }
        } else if in_token {
            let end = i;
            if end - token_start >= ENTROPY_MIN_LEN
                && shannon_entropy(&text[token_start..end]) >= ENTROPY_MIN_BITS
            {
                return Some((token_start, end));
            }
            in_token = false;
            start = i;
        }
    }
    if in_token {
        let end = text.len();
        if end - token_start >= ENTROPY_MIN_LEN
            && shannon_entropy(&text[token_start..end]) >= ENTROPY_MIN_BITS
        {
            return Some((token_start, end));
        }
    }
    let _ = start;
    None
}

/// Shannon entropy of a token in bits per character.
fn shannon_entropy(s: &str) -> f64 {
    let mut counts: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
    let mut total = 0usize;
    for c in s.chars() {
        *counts.entry(c).or_insert(0) += 1;
        total += 1;
    }
    if total == 0 {
        return 0.0;
    }
    let t = total as f64;
    counts.values().fold(0.0, |acc, &n| {
        let p = n as f64 / t;
        acc - p * p.log2()
    })
}

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

#[cfg(test)]
mod entropy_tests {
    use super::*;

    /// High-entropy base64url-looking token (no known prefix) → redacted.
    #[test]
    fn redacts_high_entropy_token() {
        let token = "aZ9xQ7mK2vN8pL4cR6tW1yE3uI5oA0sD";
        assert!(token.len() >= ENTROPY_MIN_LEN);
        let redacted = redact_secrets(&format!("key {token} end"));
        assert!(
            redacted.contains("[REDACTED:high-entropy]"),
            "high-entropy token must be redacted, got: {redacted}"
        );
        assert!(!redacted.contains(token), "token must not leak");
    }

    /// Hex blake3/sha256 ids are NOT redacted by the entropy heuristic
    /// (hex entropy ≈ 3.3 bits/char < 4.0 bar) — identifiers survive.
    #[test]
    fn hex_hashes_survive_entropy_heuristic() {
        let hash = "blake3:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
        let redacted = redact_secrets(hash);
        assert_eq!(redacted, hash, "hex hashes must NOT be redacted");
    }

    /// Short tokens are never redacted by entropy.
    #[test]
    fn short_tokens_survive() {
        let text = "the quick brown fox 1234567890abcdef";
        assert_eq!(redact_secrets(text), text);
    }

    /// Determinism of the entropy heuristic.
    #[test]
    fn entropy_heuristic_is_deterministic() {
        let token = "kD8mP2xQ9vN4cL6rT1wY5uI3oA7sE0zB";
        let input = format!("prefix-{token}-suffix");
        assert_eq!(redact_secrets(&input), redact_secrets(&input));
    }
}
