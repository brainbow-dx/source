//! Smallest useful slice of `spec/.agents/proposals/styleguide-frontmatter.md`'s design: a
//! markdown file with a YAML frontmatter block naming colors, dimensions, and text sizes, parsed
//! once into a flat lookup table. Deliberately not the full W3C Design Tokens shape that proposal
//! targets
//! long-term (no `$type`/`$value` tagging, no `{token.path}` aliasing, no component-dependency
//! declarations — that question is still open) — this exists so `apps/anvil`'s terminal UI and
//! native AppKit chrome can read the *same* named colors today instead of each hardcoding its own
//! palette, not to be the final schema.

use std::collections::HashMap;
use std::fmt;

#[derive(serde::Deserialize, Default)]
struct RawStyleguide {
    #[serde(default)]
    colors: HashMap<String, String>,
    #[serde(default)]
    dimensions: HashMap<String, f64>,
    /// Named font sizes, in points — kept separate from `dimensions` even though both are just
    /// `f64` under the hood, so a consumer can tell "this number sizes text" from "this number
    /// spaces/sizes a box" without the name alone having to carry that distinction.
    #[serde(default)]
    text: HashMap<String, f64>,
}

/// A parsed styleguide: named colors (as `(r, g, b)`, 0-255), named dimensions, and named text
/// sizes (both as raw `f64` point values — no unit system yet, callers currently treat every
/// number as points/pixels).
#[derive(Debug, Clone, Default)]
pub struct Styleguide {
    colors: HashMap<String, (u8, u8, u8)>,
    dimensions: HashMap<String, f64>,
    text: HashMap<String, f64>,
}

#[derive(Debug)]
pub enum StyleguideError {
    /// The document doesn't open with a `---`-delimited YAML block.
    MissingFrontmatter,
    Yaml(serde_yaml::Error),
    /// A `colors` entry wasn't a `#rrggbb` (or `rrggbb`) hex string.
    InvalidColor(String),
}

impl fmt::Display for StyleguideError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StyleguideError::MissingFrontmatter => write!(f, "no YAML frontmatter found (expected a leading `---` block)"),
            StyleguideError::Yaml(error) => write!(f, "invalid frontmatter YAML: {error}"),
            StyleguideError::InvalidColor(raw) => write!(f, "invalid color {raw:?}: expected `#rrggbb`"),
        }
    }
}

impl std::error::Error for StyleguideError {}

impl Styleguide {
    /// Parses a whole styleguide document (frontmatter + markdown body — the body itself is
    /// prose for humans, not read by this parser).
    pub fn parse(document: &str) -> Result<Self, StyleguideError> {
        let body = document.strip_prefix("---").ok_or(StyleguideError::MissingFrontmatter)?;
        let body = body.strip_prefix('\n').or_else(|| body.strip_prefix("\r\n")).unwrap_or(body);
        let end = body.find("\n---").ok_or(StyleguideError::MissingFrontmatter)?;
        let frontmatter = &body[..end];

        let raw: RawStyleguide = serde_yaml::from_str(frontmatter).map_err(StyleguideError::Yaml)?;

        let mut colors = HashMap::with_capacity(raw.colors.len());
        for (name, hex) in raw.colors {
            let rgb = parse_hex(&hex).ok_or_else(|| StyleguideError::InvalidColor(hex.clone()))?;
            colors.insert(name, rgb);
        }

        Ok(Styleguide { colors, dimensions: raw.dimensions, text: raw.text })
    }

    pub fn color(&self, name: &str) -> Option<(u8, u8, u8)> {
        self.colors.get(name).copied()
    }

    pub fn dimension(&self, name: &str) -> Option<f64> {
        self.dimensions.get(name).copied()
    }

    /// A named font size, in points.
    pub fn text_size(&self, name: &str) -> Option<f64> {
        self.text.get(name).copied()
    }
}

fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_colors_and_dimensions() {
        let doc = "---\ncolors:\n  accent: \"#7aa2f7\"\ndimensions:\n  radius: 6\ntext:\n  body: 13\n---\n# Body\n";
        let guide = Styleguide::parse(doc).unwrap();
        assert_eq!(guide.color("accent"), Some((122, 162, 247)));
        assert_eq!(guide.dimension("radius"), Some(6.0));
        assert_eq!(guide.text_size("body"), Some(13.0));
        assert_eq!(guide.color("missing"), None);
    }

    #[test]
    fn rejects_missing_frontmatter() {
        assert!(matches!(Styleguide::parse("# just markdown"), Err(StyleguideError::MissingFrontmatter)));
    }
}
