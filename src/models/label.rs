use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// A label that can be applied to issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
}

/// Label view for JSON output (without internal id, per PLAN.md schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelView {
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
}

impl From<Label> for LabelView {
    fn from(label: Label) -> Self {
        Self {
            name: label.name,
            color: label.color,
            description: label.description,
        }
    }
}

impl From<&Label> for LabelView {
    fn from(label: &Label) -> Self {
        Self {
            name: label.name.clone(),
            color: label.color.clone(),
            description: label.description.clone(),
        }
    }
}

/// Validate a hex color string (6 characters, no # prefix)
#[allow(dead_code)] // Will be used in label creation
pub fn validate_color(color: &str) -> Result<()> {
    if color.len() != 6 {
        return Err(Error::InvalidColor(color.to_string()));
    }

    if !color.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::InvalidColor(color.to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_hex_colors() {
        assert!(validate_color("ff0000").is_ok());
        assert!(validate_color("000000").is_ok());
        assert!(validate_color("ffffff").is_ok());
        assert!(validate_color("a2eeef").is_ok());
        assert!(validate_color("AABBCC").is_ok());
    }

    #[test]
    fn invalid_hex_colors_with_hash() {
        assert!(validate_color("#ff0000").is_err());
    }

    #[test]
    fn invalid_hex_colors_too_short() {
        assert!(validate_color("ff000").is_err());
    }

    #[test]
    fn invalid_hex_colors_too_long() {
        assert!(validate_color("ff00000").is_err());
    }

    #[test]
    fn invalid_hex_colors_invalid_chars() {
        assert!(validate_color("gggggg").is_err());
        assert!(validate_color("zzzzzz").is_err());
    }

    #[test]
    fn invalid_hex_colors_empty() {
        assert!(validate_color("").is_err());
    }

    #[test]
    fn label_serializes_to_json() {
        let label = Label {
            id: 1,
            name: "bug".to_string(),
            description: Some("Something is broken".to_string()),
            color: Some("d73a4a".to_string()),
        };

        let json = serde_json::to_string(&label).unwrap();
        assert!(json.contains("\"name\":\"bug\""));
        assert!(json.contains("\"color\":\"d73a4a\""));
    }
}
