use crate::v1::types::{
    media::Media, misc::Color, util::truncate::truncate_with_ellipsis, EmbedId,
};

use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use super::media::MediaRef;

// maybe allow iframes for some sites? probably could be done client side though
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Embed {
    pub id: EmbedId,

    /// the url this embed was requested for
    pub url: Option<Url>,

    /// the final resolved url, after redirects and canonicalization. If None, its the same as `url`.
    pub canonical_url: Option<Url>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 256))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub title: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 4096)))]
    pub description: Option<String>,

    /// the theme color of the site, as a hex string (`#rrggbb`)
    pub color: Option<Color>,

    pub media: Option<Media>,
    pub thumbnail: Option<Media>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 256))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub author_name: Option<String>,
    pub author_url: Option<Url>,
    pub author_avatar: Option<Media>,

    /// the name of the website
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 256))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub site_name: Option<String>,

    /// aka favicon
    pub site_avatar: Option<Media>,
    // /// what kind of thing this is
    // pub kind: UrlTargetKind,
    // pub timestamp: Option<Time>,
    // pub footer: Option<String>,

    // // discord compatibility? these aren't really used for url embeds though, and
    // // from my experience seem somewhat rarely used for bots. i could probably do
    // // something better with the rich text system, but idk.
    // pub field: Vec<name, value, inline?>
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
// #[cfg_attr(feature = "validator", derive(Validate))]
pub struct EmbedRequest {
    pub url: Url,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct EmbedCreate {
    /// the url this embed was requested for
    pub url: Option<Url>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 256))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub title: Option<String>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 4096))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 4096)))]
    pub description: Option<String>,

    /// the theme color of the site, as a hex string (`#rrggbb`)
    pub color: Option<String>,

    // TODO: allow using a url directly
    pub media: Option<MediaRef>,
    pub thumbnail: Option<MediaRef>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 256))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub author_name: Option<String>,

    pub author_url: Option<Url>,

    pub author_avatar: Option<MediaRef>,
}

impl Embed {
    pub fn truncate(self) -> Self {
        let title = self
            .title
            .map(|t| truncate_with_ellipsis(&t, 256).into_owned());
        let description = self
            .description
            .map(|s| truncate_with_ellipsis(&s, 4096).into_owned());
        let author_name = self
            .author_name
            .map(|t| truncate_with_ellipsis(&t, 256).into_owned());
        let site_name = self
            .site_name
            .map(|t| truncate_with_ellipsis(&t, 256).into_owned());
        Self {
            title,
            description,
            author_name,
            site_name,

            // no way to truncate urls safely
            url: self.url,
            canonical_url: self.canonical_url,
            author_url: self.author_url,

            // already truncated media filenames
            media: self.media,
            thumbnail: self.thumbnail,
            author_avatar: self.author_avatar,
            site_avatar: self.site_avatar,
            ..self
        }
    }
}
