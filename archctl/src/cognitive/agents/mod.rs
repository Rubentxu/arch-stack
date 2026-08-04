//! Cognitive agents — ReactiveObserver implementations.
//!
//! v1.0: heuristic-only, deterministic agents for architecture analysis
//! and projection selection. No LLM dependency.

mod architecture;
mod projection;

pub use architecture::ArchitectureAgent;
pub use projection::ProjectionAgent;

/// Common suffix denylist for name normalization in connascence detection.
const DENYLIST_SUFFIXES: &[&str] = &[
    "Service",
    "Manager",
    "Impl",
    "Plugin",
    "Controller",
    "Handler",
    "Adapter",
    "Factory",
    "Repository",
    "Gateway",
];

/// Strip common architectural suffixes from a name for comparison.
fn strip_suffix(name: &str) -> String {
    let mut result = name.to_string();
    for suffix in DENYLIST_SUFFIXES {
        if result.ends_with(suffix) {
            result.truncate(result.len() - suffix.len());
        }
    }
    result
}

/// Compute a simple similarity score between two names (0.0 – 1.0).
/// Uses stripped-name edit distance normalized by max stripped length.
/// Stripping reduces noise from generic suffixes; the raw edit distance
/// on stripped names gives the true structural similarity.
fn name_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let sa = strip_suffix(a);
    let sb = strip_suffix(b);

    if sa == sb {
        return 1.0;
    }

    let len = sa.len().max(sb.len()) as f64;
    let dist = levenshtein(sa.as_bytes(), sb.as_bytes()) as f64;
    (1.0 - dist / len).clamp(0.0, 1.0)
}

/// Levenshtein distance for byte slices — Wagner-Fischer algorithm.
#[allow(clippy::needless_range_loop)]
fn levenshtein(a: &[u8], b: &[u8]) -> usize {
    let m = a.len();
    let n = b.len();
    let mut matrix = vec![vec![0usize; n + 1]; m + 1];

    for i in 0..=m {
        matrix[i][0] = i;
    }
    for j in 0..=n {
        matrix[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1) // deletion
                .min(matrix[i][j - 1] + 1) // insertion
                .min(matrix[i - 1][j - 1] + cost); // substitution
        }
    }

    matrix[m][n]
}

/// C4 kind_ids that represent architecturally relevant elements.
fn is_arch_relevant(kind_id: &str) -> bool {
    matches!(kind_id, "mt.system" | "mt.container" | "mt.component")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_suffix() {
        assert_eq!(strip_suffix("UserService"), "User");
        assert_eq!(strip_suffix("OrderManager"), "Order");
        assert_eq!(strip_suffix("AuthPlugin"), "Auth");
        assert_eq!(strip_suffix("PlainName"), "PlainName");
    }

    #[test]
    fn test_name_similarity() {
        // When stripped forms converge (both "User"), similarity is 1.0
        // since they refer to the same conceptual entity.
        assert!((name_similarity("UserService", "UserManager") - 1.0).abs() < 0.01);
        assert!((name_similarity("UserService", "UserService") - 1.0).abs() < 0.01);
        // Distinct base names should be low
        assert!(
            name_similarity("UserService", "OrderService") < 0.5,
            "got {}",
            name_similarity("UserService", "OrderService")
        );
    }

    #[test]
    fn test_is_arch_relevant() {
        assert!(is_arch_relevant("mt.container"));
        assert!(is_arch_relevant("mt.component"));
        assert!(is_arch_relevant("mt.system"));
        assert!(!is_arch_relevant("code.function"));
        assert!(!is_arch_relevant("uml.class"));
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein(b"kitten", b"kitten"), 0);
        assert_eq!(levenshtein(b"kitten", b"sitting"), 3);
    }
}
