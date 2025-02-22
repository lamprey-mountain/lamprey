use crate::{Media, UrlEmbedId};

use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct UrlEmbed {
    pub id: UrlEmbedId,

    /// the url this embed was requested for
    pub url: Url,

    /// the final resolved url, after redirects and canonicalization. If None, its the same as `url`.
    pub canonical_url: Option<Url>,

    pub title: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,

    pub media: Option<Media>,

    /// if `media` should be displayed as a small thumbnail or as a full size
    pub media_is_thumbnail: bool,
    // pub media_extra: Vec<Media>, // maybe sites have extra media?
    pub author_url: Option<Url>,
    pub author_name: Option<String>,
    pub author_avatar: Option<Media>,

    pub site_name: Option<String>,

    /// aka favicon
    pub site_avatar: Option<Media>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
// #[cfg_attr(feature = "validator", derive(Validate))]
pub struct UrlEmbedRequest {
    pub url: Url,
}
