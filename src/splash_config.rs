//! TOML config structures for splash-page.
//!
//! Field semantics are documented on [`SplashConfig`] and [`SplashLine`].
//! Every optional field has a default, so the minimum config only needs
//! `enabled`, `duration_ms`, `width`, `height`, and
//! `background_image_base64`.

use serde::Deserialize;

/// Top-level config for the splash window.
#[derive(Debug, Clone, Deserialize)]
pub struct SplashConfig {
    /// Whether splash is enabled. When `false`, [`crate::init`] returns
    /// immediately.
    pub enabled: bool,
    /// Display duration in milliseconds.
    pub duration_ms: u64,
    /// Window width in pixels.
    pub width: u32,
    /// Window height in pixels.
    pub height: u32,
    /// Base64 background image, optionally with a `data:image/...;base64,`
    /// prefix.
    pub background_image_base64: String,
    /// Text lines. Defaults to empty.
    #[serde(default)]
    pub lines: Vec<SplashLine>,
}

/// Config for one text line.
#[derive(Debug, Clone, Deserialize)]
pub struct SplashLine {
    /// Text content. An empty string occupies a line's height but is not
    /// drawn.
    #[serde(default)]
    pub text: String,
    /// Text color as `#RRGGBB` or `RRGGBB`. Defaults to `#DDDDDD`.
    #[serde(default = "default_color")]
    pub color: String,
    /// Font family name, resolved via FLTK's `Font::by_name`.
    ///
    /// - Non-empty: used directly for this line.
    /// - Empty (default): falls back to the auto-detected CJK font,
    ///   selected by the `bold` flag.
    #[serde(default)]
    pub font_family: String,
    /// Font size in pixels. Defaults to 38.
    #[serde(default = "default_font_size")]
    pub font_size: u32,
    /// Horizontal alignment. Defaults to left.
    #[serde(default)]
    pub alignment: Alignment,
    /// Whether the text is bold. Defaults to `false`.
    #[serde(default)]
    pub bold: bool,
    /// URL opened in the system browser when the line is clicked; `None`
    /// means the line is not clickable.
    #[serde(default)]
    pub link: Option<String>,
}

/// Horizontal text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

fn default_color() -> String {
    "#DDDDDD".to_string()
}

fn default_font_size() -> u32 {
    38
}

impl SplashConfig {
    /// Parse the config from a TOML string.
    pub fn from_toml_str(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }

    /// Parse the config from a TOML file.
    ///
    /// # Errors
    ///
    /// The file cannot be read, or the TOML cannot be parsed.
    pub fn from_toml_file<P: AsRef<std::path::Path>>(
        path: P,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::from_toml_str(&content)?)
    }
}
