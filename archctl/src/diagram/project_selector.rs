//! ProjectSelector for `diagram project` — independent from C4Kind used by `diagram export`.
//!
//! Grammar: `<kind>:<scope>`
//! - `kind` ∈ {c4-context, c4-container, c4-component, class, sequence, state, usecase}
//! - `scope` ∈ {`*` (all), or an identifier string}
//!
//! Per ADR-028: ViewKind is separate from C4Kind because `export` → viewer-bundle
//! (archview) while `project` → editable DSL source. Audiences differ; sharing
//! ScopeFilter is fine, diverging in kinds is intentional.

use crate::graph::validate_identifier;
use std::fmt;

/// View kinds for `diagram project`. Independent from C4Kind (ADR-028).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewKind {
    C4Context,
    C4Container,
    C4Component,
    Class,
    Sequence,
    State,
    UseCase,
}

impl ViewKind {
    /// Parse a ViewKind from a string, or return `None` if unknown.
    pub fn parse(s: &str) -> Option<ViewKind> {
        match s {
            "c4-context" => Some(ViewKind::C4Context),
            "c4-container" => Some(ViewKind::C4Container),
            "c4-component" => Some(ViewKind::C4Component),
            "class" => Some(ViewKind::Class),
            "sequence" => Some(ViewKind::Sequence),
            "state" => Some(ViewKind::State),
            "usecase" => Some(ViewKind::UseCase),
            _ => None,
        }
    }

    /// Category for graph queries: "c4", "uml", or "behavior".
    pub fn category(&self) -> &'static str {
        match self {
            ViewKind::C4Context | ViewKind::C4Container | ViewKind::C4Component => "c4",
            ViewKind::Class | ViewKind::Sequence | ViewKind::State | ViewKind::UseCase => "uml",
        }
    }

    /// Metatype filter for element queries, if applicable.
    pub fn metatype_filter(&self) -> Option<&'static str> {
        match self {
            ViewKind::Class => Some("uml.class"),
            ViewKind::State => Some("uml.state"),
            _ => None,
        }
    }
}

impl fmt::Display for ViewKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewKind::C4Context => write!(f, "c4-context"),
            ViewKind::C4Container => write!(f, "c4-container"),
            ViewKind::C4Component => write!(f, "c4-component"),
            ViewKind::Class => write!(f, "class"),
            ViewKind::Sequence => write!(f, "sequence"),
            ViewKind::State => write!(f, "state"),
            ViewKind::UseCase => write!(f, "usecase"),
        }
    }
}

/// Scope filter for a project selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeFilter {
    /// Match all elements in the given view.
    All,
    /// Match only the element with the exact given identifier.
    Exact(String),
}

/// A parsed project selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSelector {
    pub view: ViewKind,
    pub scope: ScopeFilter,
}

impl ProjectSelector {
    /// Parse a project selector string into a `ProjectSelector`.
    ///
    /// Grammar: `<kind>:<scope>`
    ///
    /// # Errors
    ///
    /// Returns an error if the string is malformed, the kind is unknown,
    /// or the scope contains invalid identifier characters.
    pub fn parse(s: &str) -> anyhow::Result<ProjectSelector> {
        let s = s.trim();

        let colon_pos = s
            .find(':')
            .ok_or_else(|| anyhow::anyhow!("project selector must contain ':' (got: {s})"))?;

        if colon_pos == 0 {
            anyhow::bail!("project selector must have a view kind before ':'");
        }

        let kind_str = &s[..colon_pos];
        let scope_str = &s[colon_pos + 1..];

        let view =
            ViewKind::parse(kind_str).ok_or_else(|| {
                anyhow::anyhow!(
                    "unknown view kind: \"{kind_str}\" (expected: c4-context, c4-container, c4-component, class, sequence, state, usecase)"
                )
            })?;

        let scope = parse_scope(scope_str)?;

        Ok(ProjectSelector { view, scope })
    }

    /// Return the graph category for queries.
    pub fn category(&self) -> &'static str {
        self.view.category()
    }

    /// Return the scope identifier (None for All).
    pub fn scope_ident(&self) -> Option<&str> {
        match &self.scope {
            ScopeFilter::All => None,
            ScopeFilter::Exact(s) => Some(s.as_str()),
        }
    }
}

fn parse_scope(s: &str) -> anyhow::Result<ScopeFilter> {
    if s.is_empty() {
        anyhow::bail!("scope cannot be empty");
    }

    if s == "*" {
        return Ok(ScopeFilter::All);
    }

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
    fn parse_class_exact() {
        let vs = ProjectSelector::parse("class:orders").unwrap();
        assert!(matches!(vs.view, ViewKind::Class));
        assert!(matches!(vs.scope, ScopeFilter::Exact(s) if s == "orders"));
    }

    #[test]
    fn parse_class_all() {
        let vs = ProjectSelector::parse("class:*").unwrap();
        assert!(matches!(vs.view, ViewKind::Class));
        assert!(matches!(vs.scope, ScopeFilter::All));
    }

    #[test]
    fn parse_state_all() {
        let vs = ProjectSelector::parse("state:*").unwrap();
        assert!(matches!(vs.view, ViewKind::State));
        assert!(matches!(vs.scope, ScopeFilter::All));
    }

    #[test]
    fn parse_c4_container_exact() {
        let vs = ProjectSelector::parse("c4-container:orders").unwrap();
        assert!(matches!(vs.view, ViewKind::C4Container));
        assert!(matches!(vs.scope, ScopeFilter::Exact(s) if s == "orders"));
    }

    #[test]
    fn parse_usecase_all() {
        let vs = ProjectSelector::parse("usecase:*").unwrap();
        assert!(matches!(vs.view, ViewKind::UseCase));
        assert!(matches!(vs.scope, ScopeFilter::All));
    }

    #[test]
    fn parse_all_view_kinds() {
        for (kind_str, expected) in [
            ("c4-context", ViewKind::C4Context),
            ("c4-container", ViewKind::C4Container),
            ("c4-component", ViewKind::C4Component),
            ("class", ViewKind::Class),
            ("sequence", ViewKind::Sequence),
            ("state", ViewKind::State),
            ("usecase", ViewKind::UseCase),
        ] {
            let vs = ProjectSelector::parse(&format!("{kind_str}:foo")).unwrap();
            assert!(
                matches!(vs.view, v if std::mem::discriminant(&v) == std::mem::discriminant(&expected)),
                "expected {:?} for {kind_str}",
                expected
            );
        }
    }

    #[test]
    fn parse_rejects_empty_scope() {
        assert!(ProjectSelector::parse("class:").is_err());
    }

    #[test]
    fn parse_rejects_missing_colon() {
        assert!(ProjectSelector::parse("class").is_err());
        assert!(ProjectSelector::parse("class*").is_err());
    }

    #[test]
    fn parse_rejects_unknown_kind() {
        assert!(ProjectSelector::parse("unknown:foo").is_err());
        assert!(ProjectSelector::parse("actor:foo").is_err());
        assert!(ProjectSelector::parse(":foo").is_err());
    }

    #[test]
    fn parse_rejects_space_in_scope() {
        assert!(ProjectSelector::parse("class:a b").is_err());
    }

    #[test]
    fn parse_rejects_oversize_scope() {
        let long_scope = "a".repeat(65);
        assert!(ProjectSelector::parse(&format!("class:{long_scope}")).is_err());
    }

    #[test]
    fn parse_rejects_cypher_injection_chars() {
        for bad in ["a'b", "a\"b", "a;b", "a)b", "a}b"] {
            assert!(ProjectSelector::parse(&format!("class:{bad}")).is_err());
        }
    }

    #[test]
    fn parse_idempotent() {
        let input = "class:orders-api";
        let a = ProjectSelector::parse(input).unwrap();
        let b = ProjectSelector::parse(input).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn view_kind_category() {
        assert_eq!(ViewKind::C4Context.category(), "c4");
        assert_eq!(ViewKind::C4Container.category(), "c4");
        assert_eq!(ViewKind::C4Component.category(), "c4");
        assert_eq!(ViewKind::Class.category(), "uml");
        assert_eq!(ViewKind::Sequence.category(), "uml");
        assert_eq!(ViewKind::State.category(), "uml");
        assert_eq!(ViewKind::UseCase.category(), "uml");
    }

    #[test]
    fn view_kind_metatype_filter() {
        assert_eq!(ViewKind::Class.metatype_filter(), Some("uml.class"));
        assert_eq!(ViewKind::State.metatype_filter(), Some("uml.state"));
        assert_eq!(ViewKind::C4Container.metatype_filter(), None);
        assert_eq!(ViewKind::Sequence.metatype_filter(), None);
    }

    #[test]
    fn project_selector_scope_ident() {
        let all = ProjectSelector::parse("class:*").unwrap();
        assert_eq!(all.scope_ident(), None);

        let exact = ProjectSelector::parse("class:orders").unwrap();
        assert_eq!(exact.scope_ident(), Some("orders"));
    }
}
