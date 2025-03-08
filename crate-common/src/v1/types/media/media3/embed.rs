//! url embeds/link previews

use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::v1::types::{
    misc::Color, text::Language, EmbedId, MediaId, MessageId, MessageVerId, UserId,
};

use super::{MediaFile, MediaImage, Mime};

/// base for all embeds
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct EmbedBase {
    /// an unique identifier for this embed
    // i might be able to remove this and use the Media's MediaId instead?
    // but then there's no way to link media back to embeds with MediaLink
    pub id: EmbedId,

    /// the url for this thing
    pub url: Url,

    /// the title or name of this thing
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 256))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub title: Option<String>,

    /// a longer, more detailed description of this thing
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 4096)))]
    pub description: Option<String>,

    /// the color representative of this thing, as a hex string (`#rrggbb`)
    pub color: Option<Color>,

    /// if this thing is media, this is the media
    pub media: Option<MediaFile>,

    /// a small image that represents this thing
    pub thumbnail: Option<MediaImage>,

    /// who made this thing
    #[cfg_attr(feature = "validator", validate(nested))]
    pub author: Author,
}

/// a preview of content at a url
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct EmbedUrl {
    #[serde(flatten)]
    pub base: EmbedBase,

    /// the final resolved url, after redirects and canonicalization. If None, its the same as `url`.
    pub canonical_url: Option<Url>,

    /// where did the embed come from
    #[cfg_attr(feature = "validator", validate(nested))]
    pub site: Website,
    // // should i these fields for discord compatibility? these aren't
    // // really used for url embeds though, from my experience they're
    // // mostly used by bots
    // pub timestamp: Option<Time>,
    // pub video: Option<MediaVideo>,
    // pub footer: Option<{ text, url, icon }>,
    // pub field: Vec<{ name, value, inline }>
}

/// a custom embed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct EmbedCustom {
    #[serde(flatten)]
    pub base: EmbedBase,
    // // should i these fields for discord compatibility? these aren't
    // // really used for url embeds though, from my experience they're
    // // mostly used by bots
    // pub timestamp: Option<Time>,
    // pub video: Option<MediaVideo>,
    // pub footer: Option<{ text, url, icon }>,
    // pub field: Vec<{ name, value, inline }>
    // any custom embed specific fields?
}

/// a preview of some remote content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum Embed {
    /// a generic website embed
    Website(Box<EmbedUrl>),

    /// something that is primarily a text document, from news, blogs, etc
    // currently not displayed differently in any way
    Article(Box<EmbedUrl>),

    /// a direct link to a file
    File(Box<super::MediaFile>),

    /// a custom embed
    Custom(Box<EmbedCustom>),
    // opengraph types. i don't have time/energy to design something for all of these, maybe they can be simplified somewhat?
    // MusicSong(Box<Embed>),
    // MusicAlbum(Box<Embed>),
    // MusicPlaylist(Box<Embed>),
    // MusicRadioStation(Box<Embed>),
    // VideoMovie(Box<Embed>),
    // VideoEpisode(Box<Embed>),
    // VideoOther(Box<Embed>),
    // Book(Box<Embed>),
    // Profile(Box<Embed>),
    // Object(Box<Embed>),
    // Other(Box<Embed>),
}

/// who created this thing
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Author {
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 256))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub name: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 2048))]
    #[cfg_attr(feature = "validator", validate(custom(function = sane_url_length)))]
    pub url: Option<Url>,

    pub avatar: Option<MediaImage>,
}

/// information about the website this url is for
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Website {
    /// the website's site_name. if None, fall back to the url hostname.
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 256))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub name: Option<String>,

    /// the website's favicon
    pub favicon: Option<MediaImage>,
}

#[cfg(feature = "validator")]
fn sane_url_length(url: &Url) -> Result<(), validator::ValidationError> {
    use serde_json::json;

    let l = url.as_str().len();
    if l >= 1 && l <= 2048 {
        Ok(())
    } else {
        let mut err = validator::ValidationError::new("length");
        err.add_param("max".into(), &json!(2048));
        err.add_param("min".into(), &json!(1));
        Err(err)
    }
}
