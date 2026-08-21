use crate::{
    v1::types::{EmbedId, misc::Color, util::truncate::truncate_with_ellipsis},
    v2::types::media::{Media, MediaReference},
};

use lamprey_macros::record;
use url::Url;

// maybe allow iframes for some sites? probably could be done client side though
#[record]
#[derive(Default, PartialEq, Eq)]
pub enum EmbedType {
    /// this is a piece of media, ie. an image, video, or audio
    Media,

    /// this is from a webpage
    Link,

    /// this is manually specified from a bot
    #[default]
    Custom,
}

#[record]
pub struct Embed {
    pub id: EmbedId,

    /// what kind of thing this is
    #[serde(default, rename = "type")]
    pub ty: EmbedType,

    /// the url this embed was requested for
    // FIXME: validate length
    pub url: Option<Url>,

    /// the final resolved url, after redirects and canonicalization. If None, its the same as `url`.
    pub canonical_url: Option<Url>,

    #[schema(min_length = 1, max_length = 256)]
    #[validate(length(min = 1, max = 256))]
    pub title: Option<String>,

    #[schema(min_length = 1, max_length = 4096)]
    #[validate(length(min = 1, max = 4096))]
    pub description: Option<String>,

    /// the theme color of the site, as a hex string (`#rrggbb`)
    pub color: Option<Color>,

    pub media: Option<Media>,
    pub thumbnail: Option<Media>,

    #[schema(min_length = 1, max_length = 256)]
    #[validate(length(min = 1, max = 256))]
    pub author_name: Option<String>,
    // TODO: validate length
    pub author_url: Option<Url>,
    pub author_avatar: Option<Media>,

    /// the name of the website
    #[schema(min_length = 1, max_length = 256)]
    #[validate(length(min = 1, max = 256))]
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

// TODO: rename to EmbedGenerate
#[record]
#[derive(PartialEq, Eq)]
// #[cfg_attr(feature = "validator", derive(Validate))]
pub struct EmbedRequest {
    pub url: Url,
}

#[record]
#[derive(Default, PartialEq, Eq)]
pub struct EmbedCreate {
    /// the url this embed was requested for
    // FIXME: validate max length
    pub url: Option<Url>,

    #[schema(min_length = 1, max_length = 256)]
    #[validate(length(min = 1, max = 256))]
    pub title: Option<String>,

    #[schema(min_length = 1, max_length = 4096)]
    #[validate(length(min = 1, max = 4096))]
    pub description: Option<String>,

    /// the theme color of the site, as a hex string (`#rrggbb`)
    pub color: Option<Color>,
    pub media: Option<MediaReference>,
    pub thumbnail: Option<MediaReference>,

    #[schema(min_length = 1, max_length = 256)]
    #[validate(length(min = 1, max = 256))]
    pub author_name: Option<String>,

    pub author_url: Option<Url>,

    pub author_avatar: Option<MediaReference>,
}

impl EmbedCreate {
    /// set the title of the embed
    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    /// set the color of the embed
    pub fn color<C: Into<Color>>(mut self, color: C) -> Self {
        self.color = Some(color.into());
        self
    }

    /// set the description of the embed
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// set the media of the embed
    pub fn media(mut self, media_ref: MediaReference) -> Self {
        self.media = Some(media_ref);
        self
    }

    /// set the url of the embed
    pub fn url(mut self, url: Url) -> Self {
        self.url = Some(url);
        self
    }

    /// set the thumbnail of the embed
    pub fn thumbnail(mut self, thumbnail: MediaReference) -> Self {
        self.thumbnail = Some(thumbnail);
        self
    }

    /// set the author name of the embed
    pub fn author_name<S: Into<String>>(mut self, author_name: S) -> Self {
        self.author_name = Some(author_name.into());
        self
    }

    /// set the author url of the embed
    pub fn author_url(mut self, author_url: Url) -> Self {
        self.author_url = Some(author_url);
        self
    }

    /// set the author avatar of the embed
    pub fn author_avatar(mut self, author_avatar: MediaReference) -> Self {
        self.author_avatar = Some(author_avatar);
        self
    }
}

impl Embed {
    pub fn builder() -> EmbedCreate {
        EmbedCreate::default()
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    #[test]
    fn test_embed_create_builder() {
        let url = Url::parse("https://example.com").unwrap();
        let color: Color = "#FF0000".parse().unwrap();
        let embed = Embed::builder()
            .url(url.clone())
            .title("Test Title")
            .description("Test Description")
            .color(color.clone())
            .author_name("Author Name")
            .author_url(url.clone());

        assert_eq!(embed.url, Some(url.clone()));
        assert_eq!(embed.title, Some("Test Title".to_string()));
        assert_eq!(embed.description, Some("Test Description".to_string()));
        assert_eq!(embed.color, Some(color));
        assert_eq!(embed.author_name, Some("Author Name".to_string()));
        assert_eq!(embed.author_url, Some(url));
    }
}
