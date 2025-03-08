use crate::v1::types::{
    media::{
        media3::{self, File, MediaFile},
        Media,
    },
    misc::Color,
    util::truncate::truncate_with_ellipsis,
    UrlEmbedId,
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

/// a preview of some remote content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct UrlEmbed {
    /// an unique identifier for this embed
    pub id: UrlEmbedId,

    /// the url this embed was requested for
    pub url: Url,

    /// the final resolved url, after redirects and canonicalization. If None, its the same as `url`.
    pub canonical_url: Option<Url>,

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
    pub media: Option<Media>,

    /// a small image that represents this thing
    pub thumbnail: Option<Media>,

    /// who made this thing
    #[cfg_attr(feature = "validator", validate(nested))]
    pub author: Author,

    /// if its a url embed, where did the embed come from
    #[cfg_attr(feature = "validator", validate(nested))]
    pub site: Website,
    // /// what kind of thing this is
    // pub kind: UrlTargetKind,
    // pub timestamp: Option<Time>,
    // pub image: Option<Media>,
    // pub thumbnail: Option<Media>,
    // pub video: Option<Media>,
    // pub footer: Option<String>,

    // // discord compatibility? these aren't really used for url embeds though, and
    // // from my experience seem somewhat rarely used for bots. i could probably do
    // // something better with the rich text system, but idk.
    // pub field: Vec<name, value, inline?>
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum EmbedType {
    /// a generic website embed
    Website(Box<UrlEmbed>),

    /// something that is primarily a text document, from news, blogs, etc
    Article(Box<UrlEmbed>),

    /// an image
    Image(Box<media3::MediaImage>),

    /// a video
    Video(Box<media3::MediaVideo>),

    /// a custom embed
    Custom(Box<UrlEmbed>),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Attachment {
    pub alt: Option<String>,
    pub file: MediaFile,
}

/*

uog:type values:

article - Namespace URI: https://ogp.me/ns/article#

    article:published_time - datetime - When the article was first published.
    article:modified_time - datetime - When the article was last changed.
    article:expiration_time - datetime - When the article is out of date after.
    article:author - profile array - Writers of the article.
    article:section - string - A high-level section name. E.g. Technology
    article:tag - string array - Tag words associated with this article.

book - Namespace URI: https://ogp.me/ns/book#

    book:author - profile array - Who wrote this book.
    book:isbn - string - The ISBN
    book:release_date - datetime - The date the book was released.
    book:tag - string array - Tag words associated with this book.

profile - Namespace URI: https://ogp.me/ns/profile#

    profile:first_name - string - A name normally given to an individual by a parent or self-chosen.
    profile:last_name - string - A name inherited from a family or marriage and by which the individual is commonly known.
    profile:username - string - A short unique string to identify them.
    profile:gender - enum(male, female) - Their gender.

*/

// og:image:url - Identical to og:image.
// og:image:secure_url - An alternate url to use if the webpage requires HTTPS.
// og:image:type - A MIME type for this image.
// og:image:width - The number of pixels wide.
// og:image:height - The number of pixels high.
// og:image:alt - A description of what is in the image (not a caption). If the page specifies an og:image it should specify og:image:alt.

/*

og:audio - A URL to an audio file to accompany this object.
og:description - A one to two sentence description of your object.
og:locale - The locale these tags are marked up in. Of the format language_TERRITORY. Default is en_US.
og:locale:alternate - An array of other locales this page is available in.
og:site_name - If your object is part of a larger web site, the name which should be displayed for the overall site. e.g., "IMDb".
og:video - A URL to a video file that complements this object.

For example (line-break solely for display purposes):
*/

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct Website {
    /// the website's site_name. if None, fall back to the url hostname.
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 256))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 256)))]
    pub name: Option<String>,

    /// the website's favicon
    pub favicon: Option<Media>,
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

    pub avatar: Option<Media>,
}

#[cfg(feature = "validator")]
fn sane_url_length(url: &Url) -> Result<(), validator::ValidationError> {
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

trait Truncate {
    /// trim to max len
    fn truncate(self) -> Self;
}

impl Truncate for Website {
    fn truncate(self) -> Self {
        let name = self
            .name
            .map(|t| truncate_with_ellipsis(&t, 256).into_owned());
        Self { name, ..self }
    }
}

impl Truncate for Author {
    fn truncate(self) -> Self {
        let name = self
            .name
            .map(|t| truncate_with_ellipsis(&t, 256).into_owned());
        Self { name, ..self }
    }
}

impl Truncate for UrlEmbed {
    fn truncate(self) -> Self {
        let title = self
            .title
            .map(|t| truncate_with_ellipsis(&t, 256).into_owned());
        let description = self
            .description
            .map(|s| truncate_with_ellipsis(&s, 4096).into_owned());
        Self {
            title,
            description,
            author: self.author.truncate(),
            site: self.site.truncate(),
            ..self
        }
    }
}
