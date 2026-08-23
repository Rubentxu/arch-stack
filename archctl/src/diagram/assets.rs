//! Embedded C4 icon assets.
//!
//! Five canonical C4-level icons (per [ADR-013](docs/adr/ADR-013-viewer-ortogonal.md)
//! and the C4 model) are embedded as SVG strings at compile time and exposed as
//! `pub static` constants. `icon_for()` returns `Some` for exactly these five
//! element kinds; all other kinds return `None`.
//!
//! ADR-011 blocks external URLs, so icons are bundled into the bundle by
//! `run_export` and re-required by `run_validate` (single source of truth:
//! [`CANONICAL_C4_ICONS`]).
//!
//! SVG was chosen over PNG because:
//! - Text-based: small, diff-friendly, inspectable in source control.
//! - Resolution-independent: scales cleanly at any zoom in `archview`.
//! - Inline-stylable via CSS in the viewer (single source per icon).
//! - No external image library needed for rendering.
//!
//! The five levels and their visual encoding:
//! - **context**: a single bordered rectangle representing an entire system.
//! - **container**: a nested database cylinder and application box.
//! - **component**: three nested rectangles representing components inside a container.
//! - **dynamic**: numbered step circles connected by an arrow.
//! - **deployment**: a 3D cube representing a deployment node.

pub static CONTEXT_ICON: &str = include_str!("icons/context.svg");
pub static CONTAINER_ICON: &str = include_str!("icons/container.svg");
pub static COMPONENT_ICON: &str = include_str!("icons/component.svg");
pub static DYNAMIC_ICON: &str = include_str!("icons/dynamic.svg");
pub static DEPLOYMENT_ICON: &str = include_str!("icons/deployment.svg");

/// All canonical C4 icon filenames (without path or extension). Single source of
/// truth shared by `run_export` (which writes them) and `run_validate` (which
/// requires them).
pub const CANONICAL_C4_ICONS: &[&str] =
    &["context", "container", "component", "dynamic", "deployment"];

/// File extension for embedded C4 icons. Centralized so the exporter and
/// validator cannot drift.
pub const ICON_EXTENSION: &str = "svg";

/// Returns the SVG source for the given C4 element type, or `None` if not one
/// of the five canonical C4 levels (context/container/component/dynamic/
/// deployment).
pub fn icon_for(kind: &str) -> Option<&'static str> {
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

    #[test]
    fn all_icon_constants_are_valid_svg() {
        for kind in CANONICAL_C4_ICONS {
            let svg = icon_for(kind).unwrap();
            assert!(
                svg.contains("<svg") && svg.contains("</svg>"),
                "icon for {kind} should be a valid SVG document"
            );
        }
    }

    #[test]
    fn icon_extension_is_svg() {
        // Centralized extension contract — referenced by exporter and validator.
        assert_eq!(ICON_EXTENSION, "svg");
    }
}
