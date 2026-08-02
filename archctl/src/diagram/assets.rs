//! Embedded C4 icon assets.
//!
//! icons: stub 1×1 PNG placeholders; real C4 icons pending separate asset cycle.
//! ADR-011 blocks external URLs, so icons are embedded at compile time.
//!
//! The 5 canonical C4 diagram levels (per ADR-013 / C4 model) are:
//! - context, container, component, dynamic, deployment
//!
//! `icon_for()` returns `Some` for exactly these 5 element kinds. The legacy
//! `person`/`external_person`/`software_system` PNGs remain on disk for
//! backwards compatibility with bundles produced by earlier prototypes but
//! are not emitted by `run_export` or required by `run_validate`.

pub static CONTEXT_ICON: &[u8] = include_bytes!("icons/context.png");
pub static CONTAINER_ICON: &[u8] = include_bytes!("icons/container.png");
pub static COMPONENT_ICON: &[u8] = include_bytes!("icons/component.png");
pub static DYNAMIC_ICON: &[u8] = include_bytes!("icons/dynamic.png");
pub static DEPLOYMENT_ICON: &[u8] = include_bytes!("icons/deployment.png");

/// All canonical C4 icon filenames (without path). Single source of truth shared
/// by `run_export` (which writes them) and `run_validate` (which requires them).
pub const CANONICAL_C4_ICONS: &[&str] =
    &["context", "container", "component", "dynamic", "deployment"];

/// Returns the icon bytes for the given C4 element type, or `None` if not one of
/// the 5 canonical C4 levels (context/container/component/dynamic/deployment).
pub fn icon_for(kind: &str) -> Option<&'static [u8]> {
    match kind {
        "context" => Some(CONTEXT_ICON),
        "container" => Some(CONTAINER_ICON),
        "component" => Some(COMPONENT_ICON),
        "dynamic" => Some(DYNAMIC_ICON),
        "deployment" => Some(DEPLOYMENT_ICON),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_for_canonical_kinds_returns_some() {
        for kind in CANONICAL_C4_ICONS {
            assert!(
                icon_for(kind).is_some(),
                "icon_for({kind}) should return Some for canonical C4 level"
            );
        }
    }

    #[test]
    fn icon_for_legacy_kinds_returns_none() {
        // Legacy kinds not in the canonical C4 set — gracefully return None.
        for kind in ["person", "external_person", "software_system"] {
            assert!(
                icon_for(kind).is_none(),
                "icon_for({kind}) should return None — legacy kind not in canonical C4"
            );
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
        assert!(!DYNAMIC_ICON.is_empty());
        assert!(!DEPLOYMENT_ICON.is_empty());
    }
}
