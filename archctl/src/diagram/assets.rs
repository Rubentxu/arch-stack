//! Embedded C4 icon assets.
//!
//! icons: stub 1×1 PNG placeholders; real C4 icons pending separate asset cycle.
//! ADR-011 blocks external URLs, so icons are embedded at compile time.

pub static CONTEXT_ICON: &[u8] = include_bytes!("icons/context.png");
pub static CONTAINER_ICON: &[u8] = include_bytes!("icons/container.png");
pub static COMPONENT_ICON: &[u8] = include_bytes!("icons/component.png");
pub static PERSON_ICON: &[u8] = include_bytes!("icons/person.png");
pub static EXTERNAL_PERSON_ICON: &[u8] = include_bytes!("icons/external_person.png");
pub static SOFTWARE_SYSTEM_ICON: &[u8] = include_bytes!("icons/software_system.png");

/// Returns the icon bytes for the given C4 element type, or `None` if unknown.
pub fn icon_for(kind: &str) -> Option<&'static [u8]> {
    match kind {
        "context" => Some(CONTEXT_ICON),
        "container" => Some(CONTAINER_ICON),
        "component" => Some(COMPONENT_ICON),
        "person" => Some(PERSON_ICON),
        "external_person" => Some(EXTERNAL_PERSON_ICON),
        "software_system" => Some(SOFTWARE_SYSTEM_ICON),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_for_known_kinds_returns_some() {
        for kind in ["context", "container", "component", "person", "external_person", "software_system"] {
            assert!(icon_for(kind).is_some(), "icon_for({kind}) should return Some");
        }
    }

    #[test]
    fn icon_for_unknown_kind_returns_none() {
        assert!(icon_for("unknown").is_none());
        assert!(icon_for("").is_none());
    }

    #[test]
    fn all_icon_constants_are_non_empty() {
        assert!(!CONTEXT_ICON.is_empty());
        assert!(!CONTAINER_ICON.is_empty());
        assert!(!COMPONENT_ICON.is_empty());
        assert!(!PERSON_ICON.is_empty());
        assert!(!EXTERNAL_PERSON_ICON.is_empty());
        assert!(!SOFTWARE_SYSTEM_ICON.is_empty());
    }
}
