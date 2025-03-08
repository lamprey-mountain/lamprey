//! url embeds/link previews

use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use super::super::{misc::Color, util::truncate::truncate_with_ellipsis, EmbedId};

use super::{MediaFile, MediaImage};

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

/// a preview of some remote content at a url
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

/// can be truncated to fit inside max length limits
pub trait Truncate {
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

impl Truncate for EmbedBase {
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
            ..self
        }
    }
}

impl Truncate for EmbedUrl {
    fn truncate(self) -> Self {
        Self {
            base: self.base.truncate(),
            site: self.site.truncate(),
            ..self
        }
    }
}

impl Truncate for EmbedCustom {
    fn truncate(self) -> Self {
        Self {
            base: self.base.truncate(),
            ..self
        }
    }
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
