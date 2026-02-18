#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// maybe i need a version that explicitly drops some brightness and saturation (keeping hue)
/// a color
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Color {
    // Named(ColorNamed),
    // Semantic(ColorSemantic),
    // Srgb(ColorSrgb),
    /// sRGB (not linear) compatible with css
    // FIXME: this should be a hex code, not arbitrary string
    Srgb(String),
}

/// a color that changes depending on theme
///
/// COLOR NAMES CURRENTLY UNSTABLE AND MAY CHANGE
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ColorThemed {
    /// default text color
    FgMain,

    /// low priority text color
    FgDim,

    /// default background color (for this item)
    BgMain,

    /// default accent color
    Accent,

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
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

    /// very important to read, bad things may happen if you don't
    Warning,

    /// very important to read, dangerous things happen if you don't
    Danger,

    /// something went wrong
    Error,

    /// something went right
    Success,
}

/// a sRGB (not linear) color, compatible with css
// TODO: impl FromStr, Display. enforce hex format #rrggbb, allow #rgb in deserialize
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ColorSrgb(pub String);

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
