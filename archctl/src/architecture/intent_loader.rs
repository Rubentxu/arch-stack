//! TOML loader for intent documents.

use std::path::Path;

use crate::architecture::intent::{IntentDeclaration, IntentError};

/// Load an intent document from a TOML file.
pub fn load_intent(path: &Path) -> Result<IntentDeclaration, IntentError> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| IntentError::InvalidIntent(format!("read {}: {e}", path.display())))?;
    toml::from_str(&raw)
        .map_err(|e| IntentError::InvalidIntent(format!("parse {}: {e}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn s8_invalid_toml_rejected() {
        let mut tmp = NamedTempFile::with_suffix(".toml").unwrap();
        writeln!(tmp, "this is not valid toml [[elements").unwrap();

        let result = load_intent(tmp.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, IntentError::InvalidIntent(_)));
    }

    #[test]
    fn valid_toml_loaded() {
        let mut tmp = NamedTempFile::with_suffix(".toml").unwrap();
        writeln!(
            tmp,
            r#"schemaVersion = "1.0"
capability = "test"
[[elements]]
id = "c4:container:a"
kind = "container"
category = "c4"
"#
        )
        .unwrap();

        let result = load_intent(tmp.path()).unwrap();
        assert_eq!(result.schema_version, "1.0");
        assert_eq!(result.capability, "test");
        assert_eq!(result.elements.len(), 1);
        assert_eq!(result.elements[0].id, "c4:container:a");
    }
}
