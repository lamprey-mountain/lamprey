use serde::{Deserialize, Serialize};

use crate::MediaId;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

// TODO: switch to a common thing between room, thread, user, etc info?

/// a language
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Locale(String);

/// generic profile data thing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Profile {
    pub name: String,

    /// room = topic, user = status
    pub info_short: Option<String>,

    /// room = description, user = bio
    pub info_long: Option<String>,

    /// avatar, profile picture, icon
    pub avatar: Option<MediaId>,

    /// a larger background image
    pub banner: Option<MediaId>,

    /// list of preferred locales, in order of most to least preferred
    pub languages: Vec<Locale>,
}
