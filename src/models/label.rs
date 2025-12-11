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
pub fn validate_color(color: &str) -> Result<()> {
    if color.len() != 6 {
        return Err(Error::InvalidColor(color.to_string()));
    }

    if !color.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::InvalidColor(color.to_string()));
    }

    Ok(())
}

/// Generate a color from a label name using a simple hash.
/// Produces pleasant, saturated colors in HSL space then converts to hex.
pub fn generate_color(name: &str) -> String {
    // Simple hash: sum of bytes with position weighting
    let hash: u32 = name
        .to_lowercase()
        .bytes()
        .enumerate()
        .fold(0u32, |acc, (i, b)| {
            acc.wrapping_add((b as u32).wrapping_mul((i as u32).wrapping_add(1)))
        });

    // Use hash to pick hue (0-360), keep saturation and lightness fixed for pleasant colors
    let hue = (hash % 360) as f32;
    let saturation = 0.65;
    let lightness = 0.45;

    // Convert HSL to RGB
    let (r, g, b) = hsl_to_rgb(hue, saturation, lightness);

    format!("{:02x}{:02x}{:02x}", r, g, b)
}

/// Convert HSL to RGB (h: 0-360, s: 0-1, l: 0-1) -> (r, g, b) as u8
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
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

    #[test]
    fn generate_color_is_valid_hex() {
        let color = generate_color("bug");
        assert_eq!(color.len(), 6);
        assert!(validate_color(&color).is_ok());
    }

    #[test]
    fn generate_color_is_deterministic() {
        assert_eq!(generate_color("bug"), generate_color("bug"));
        assert_eq!(generate_color("feature"), generate_color("feature"));
    }

    #[test]
    fn generate_color_is_case_insensitive() {
        assert_eq!(generate_color("Bug"), generate_color("bug"));
        assert_eq!(generate_color("BUG"), generate_color("bug"));
    }

    #[test]
    fn generate_color_different_names_different_colors() {
        let bug = generate_color("bug");
        let feature = generate_color("feature");
        let urgent = generate_color("urgent");
        // Different names should produce different colors
        assert_ne!(bug, feature);
        assert_ne!(bug, urgent);
        assert_ne!(feature, urgent);
    }
}
