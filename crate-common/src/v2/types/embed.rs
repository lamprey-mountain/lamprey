use lamprey_macros::record;
use url::Url;

use crate::{
    v1::types::misc::Color,
    v2::types::{EmbedId, media::Media},
};

/// what type of embed this is for
#[record]
#[derive(Default, PartialEq, Eq)]
pub enum EmbedType {
    /// this is a piece of media, ie. an image, video, or audio
    Media,

    /// this is a preview of a webpage
    Link,

    // TODO: add
    // /// this for an invite url
    // Invite,
    // TODO(?): add embeds for other content too?
    /// this is manually created by a bot
    #[default]
    Custom,
}

/// an embed for some remote content
#[record]
pub struct Embed {
    pub id: EmbedId,

    /// what kind of thing this is
    #[serde(default, rename = "type")]
    pub ty: EmbedType,

    /// the url this embed was requested for
    // FIXME: validate length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,

    /// The final resolved url, after redirects and canonicalization
    ///
    /// If this is `None`, its the same as `url`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_url: Option<Url>,

    #[schema(min_length = 1, max_length = 256)]
    #[validate(length(min = 1, max = 256))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[schema(min_length = 1, max_length = 4096)]
    #[validate(length(min = 1, max = 4096))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// the theme color of the site
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<Media>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<Media>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<EmbedAuthor>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<EmbedSite>,
}

#[record]
pub struct EmbedAuthor {
    #[schema(min_length = 1, max_length = 256)]
    #[validate(length(min = 1, max = 256))]
    pub name: String,

    // FIXME: validate length
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<Media>,
}

#[record]
pub struct EmbedSite {
    /// the name of the website
    #[schema(min_length = 1, max_length = 256)]
    #[validate(length(min = 1, max = 256))]
    pub name: String,

    /// an icon that represents this website
    ///
    /// usually will be the favicon
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Media>,
}

/// a request to generate embeds for some urls
#[record]
pub struct EmbedGenerate {
    #[schema(min_length = 1, max_length = 8)]
    #[validate(length(min = 1, max = 8))]
    pub urls: Vec<Url>,
}

/// a request to create a new embed
#[record]
pub struct EmbedCreate {
    // TODO
}

// TODO: finish impl and use
