//! View-selector parser for C4 diagram exports.
//!
//! Grammar: `<c4-kind>:<scope>`
//! - `c4-kind` ∈ {context, container, component, dynamic, deployment}
//! - `scope` ∈ {`*` (all), or an identifier string}

use crate::graph::validate_identifier;
use std::fmt;

/// Parse a view selector string.
///
/// Shorthand for `ViewSelector::parse`.
pub fn parse(s: &str) -> anyhow::Result<ViewSelector> {
    ViewSelector::parse(s)
}

/// C4 diagram kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum C4Kind {
    Context,
    Container,
    Component,
    Dynamic,
    Deployment,
}

impl C4Kind {
    /// Parse a C4 kind from a string, or return `None` if unknown.
    pub fn parse(s: &str) -> Option<C4Kind> {
        match s {
            "context" => Some(C4Kind::Context),
            "container" => Some(C4Kind::Container),
            "component" => Some(C4Kind::Component),
            "dynamic" => Some(C4Kind::Dynamic),
            "deployment" => Some(C4Kind::Deployment),
            _ => None,
        }
    }
}

impl fmt::Display for C4Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            C4Kind::Context => write!(f, "context"),
            C4Kind::Container => write!(f, "container"),
            C4Kind::Component => write!(f, "component"),
            C4Kind::Dynamic => write!(f, "dynamic"),
            C4Kind::Deployment => write!(f, "deployment"),
        }
    }
}

/// Scope filter for a view selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeFilter {
    /// Match all elements in the given C4 kind.
    All,
    /// Match only the element with the exact given identifier.
    Exact(String),
}

/// A parsed view selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewSelector {
    pub kind: C4Kind,
    pub scope: ScopeFilter,
}

impl ViewSelector {
    /// Parse a view selector string into a `ViewSelector`.
    ///
    /// Grammar: `<c4-kind>:<scope>`
    ///
    /// # Errors
    ///
    /// Returns an error if the string is malformed, the kind is unknown,
    /// or the scope contains invalid identifier characters.
    pub fn parse(s: &str) -> anyhow::Result<ViewSelector> {
        let s = s.trim();

        // Must contain a ':'
        let colon_pos = s
            .find(':')
            .ok_or_else(|| anyhow::anyhow!("view selector must contain ':' (got: {s})"))?;

        if colon_pos == 0 {
            anyhow::bail!("view selector must have a C4 kind before ':'");
        }

        let kind_str = &s[..colon_pos];
        let scope_str = &s[colon_pos + 1..];

        let kind = C4Kind::parse(kind_str).ok_or_else(|| {
            anyhow::anyhow!(
                "unknown C4 kind: {kind_str} (expected: context, container, component, dynamic, deployment)"
            )
        })?;

        let scope = parse_scope(scope_str)?;

        Ok(ViewSelector { kind, scope })
    }
}

fn parse_scope(s: &str) -> anyhow::Result<ScopeFilter> {
    if s.is_empty() {
        anyhow::bail!("scope cannot be empty");
    }

    if s == "*" {
        return Ok(ScopeFilter::All);
    }

    // Validate scope as an identifier for injection safety
    validate_identifier(s)?;

    if s.len() > 64 {
        anyhow::bail!("scope exceeds maximum length of 64 characters");
    }

    Ok(ScopeFilter::Exact(s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_container_exact() {
        let vs = ViewSelector::parse("container:orders").unwrap();
        assert!(matches!(vs.kind, C4Kind::Container));
        assert!(matches!(vs.scope, ScopeFilter::Exact(s) if s == "orders"));
    }

    #[test]
    fn parse_container_all() {
        let vs = ViewSelector::parse("container:*").unwrap();
        assert!(matches!(vs.kind, C4Kind::Container));
        assert!(matches!(vs.scope, ScopeFilter::All));
    }

    #[test]
    fn parse_all_c4_kinds() {
        for kind in ["context", "container", "component", "dynamic", "deployment"] {
            let vs = ViewSelector::parse(&format!("{kind}:foo")).unwrap();
            assert!(matches!(vs.kind, k if k.to_string() == kind));
        }
    }

    #[test]
    fn parse_rejects_empty_scope() {
        assert!(ViewSelector::parse("container:").is_err());
    }

    #[test]
    fn parse_rejects_missing_colon() {
        assert!(ViewSelector::parse("container").is_err());
        assert!(ViewSelector::parse("contain").is_err());
    }

    #[test]
    fn parse_rejects_unknown_kind() {
        assert!(ViewSelector::parse("actor:foo").is_err());
        assert!(ViewSelector::parse("system:foo").is_err());
        assert!(ViewSelector::parse(":foo").is_err());
    }

    #[test]
    fn parse_rejects_space_in_scope() {
        assert!(ViewSelector::parse("container:a b").is_err());
    }

    #[test]
    fn parse_rejects_oversize_scope() {
        let long_scope = "a".repeat(65);
        assert!(ViewSelector::parse(&format!("container:{long_scope}")).is_err());
    }

    #[test]
    fn parse_rejects_cypher_injection_chars() {
        // validate_identifier rejects these characters
        for bad in ["a'b", "a\"b", "a;b", "a)b", "a}b"] {
            assert!(ViewSelector::parse(&format!("container:{bad}")).is_err());
        }
    }

    #[test]
    fn parse_idempotent() {
        let input = "container:orders-api";
        let a = ViewSelector::parse(input).unwrap();
        let b = ViewSelector::parse(input).unwrap();
        assert_eq!(a, b);
    }
}
