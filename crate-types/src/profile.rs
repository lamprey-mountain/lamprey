use serde::{Deserialize, Serialize};
use url::Url;

use crate::{text::Language, MediaId};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// TODO: switch to a common profile structure between room, thread, user, etc info?

// what format is this? hex code? css-style linear srgb?
// maybe i need a version that explicitly drops some brightness and saturation (keeping hue)
// since in frontend i'll want to modify colors to be readable despite the theme
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Color(pub String);

/// generic profile data thing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Profile {
    pub name: String,

    pub description: Option<String>,

    /// avatar, profile picture, icon. always square.
    pub avatar: Option<MediaId>,

    /// a larger background image
    // TODO: does it have a constant aspect ratio? if so, what is it?
    pub banner: Option<MediaId>,

    /// list of preferred locales, in order of most to least preferred
    pub languages: Vec<Language>,

    // a color? could be useful, unsure what it would be used for
    pub color: Color,

    /// links to other websites
    pub links: Vec<ProfileUrl>,
}

// does it make sense to allow overriding to `null`? eg. description, avatar, etc
// whats the ui design like? overrides existing for both rooms and threads might be confusing
// do overrides persist (keep name, avatar, etc) for historical messages? again, might be cool, might be annoying
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ProfileOverride {
    pub override_name: Option<String>,
    pub override_description: Option<String>,
    pub override_avatar: Option<MediaId>,
    pub override_banner: Option<MediaId>,
    pub override_color: Option<Color>,

    /// prepended instead of replaced
    pub extra_languages: Vec<Language>,

    /// prepended (in a new section?) instead of replaced
    pub extra_links: Vec<ProfileUrl>,
}

/// a link to another website
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ProfileUrl {
    pub url: Url,
    pub verified: bool,
}
