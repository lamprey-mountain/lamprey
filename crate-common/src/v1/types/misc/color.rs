use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// maybe i need a version that explicitly drops some brightness and saturation (keeping hue)
/// a color
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(untagged)]
pub enum Color {
    // Named(ColorNamed),
    /// sRGB (not linear) compatible with css
    // FIXME: this should be a hex code, not arbitrary string
    Srgb(String),
    // with alpha as separate ver? Srgba(String),
}

/// a color that changes depending on theme
/// color names currently unstable and may change
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ColorThemed {
    /// default text color
    FgMain,

    /// low priority text color
    FgDim,

    /// default background color (for this item)
    BgMain,

    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Orange,
    Teal,
}

/// color with semantic meaning
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ColorSemantic {
    /// something worth pointing out
    Note,

    /// something with useful information
    Info,

    /// instructions or tips
    Help,

    /// very important to read, generic
    Important,

    /// very important to read, bad things happen if you don't
    Warning,

    /// very important to read, dangerous things happen if you don't
    Danger,

    /// something went wrong
    Error,

    /// something went right
    Success,
}

impl Color {
    pub fn from_hex_string(s: String) -> Color {
        // TODO: ensure hex
        Color::Srgb(s)
    }
}

impl AsRef<str> for Color {
    fn as_ref(&self) -> &str {
        match self {
            Color::Srgb(c) => c.as_str(),
        }
    }
}
