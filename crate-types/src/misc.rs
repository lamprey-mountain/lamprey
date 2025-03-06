use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// maybe i need a version that explicitly drops some brightness and saturation (keeping hue)
/// a color
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(untagged)]
pub enum Color {
    // /// a builtin named colow
    // Named(ColorNamed),
    /// sRGB (not linear) compatible with css
    // FIXME: this should be a hex code, not arbitrary string
    Srgb(String),
    // with alpha as separate ver? Srgba(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ColorNamed {
    // Red, Green, Blue,
    // Error, Warning, Info
    // or both?
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

// the below types are unlikely to be added, but worth mentioning anyways

// /// maybe add a way to automatically create links from some text
// struct Autolink {
//     prefix: String,
//     template: Url,
// }

// /// not really planned, bots can be used instead
// /// maybe special compatibility message send endpoints and a ?token=... query parameter could be added
// /// or ?compat=[platform], or a header
// /// if this is added, it paobably would be another user type
// struct Webhook {
//     url: Url,
//     token: String,
//     name: String,
// }

// /// split out members/roles into its own type
// /// rooms are already meant for auth, but have extra stuff
// /// doesn't seem that useful, and if it is necessary it could be a new minimal room type
// struct Team {}
