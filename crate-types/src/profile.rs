use serde::{Deserialize, Serialize};
use url::Url;

use crate::{misc::Color, text::Language, MediaId};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// TODO: switch to a common profile structure between room, thread, user, etc info?

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

    // a color? could be useful, unsure what it would be used for. needs ui design.
    pub color: Color,

    /// links to other websites
    pub links: Vec<ProfileUrl>,
}

/// minor profile for things like tags/roles
// NOTE: what do i remove, what do i keep? is it this even necessary to have a separate struct?
// well, i guess banner and languages don't make much sense for roles and tags.
// or i guess i could create separate "topic"(?) pages for them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ProfileMinor {
    pub name: String,

    pub description: Option<String>,

    /// always a square image
    pub icon: Option<MediaId>,

    // a color? could be useful, unsure what it would be used for. needs ui design.
    pub color: Color,
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
// maybe parse some types of urls, extract usernames, generate embeds, idk
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ProfileUrl {
    pub url: Url,
    pub verified: bool,
}
