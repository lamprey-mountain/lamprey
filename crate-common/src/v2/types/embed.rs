use lamprey_macros::record;
use url::Url;

use crate::{
    v1::types::misc::Color,
    v2::types::{EmbedId, media::Media},
};

/// the maximum length of a url in an embed
#[cfg(any(feature = "utoipa", feature = "validator"))]
const MAX_URL_LENGTH: usize = 2048;

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
    // TODO: deprecate?
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
    #[schema(max_length = 2048)]
    #[validate(custom(function = "validate_url_length"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<Url>,

    /// The final resolved url, after redirects and canonicalization
    ///
    /// If this is `None`, its the same as `url`.
    #[schema(max_length = 2048)]
    #[validate(custom(function = "validate_url_length"))]
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

    #[schema(max_length = 2048)]
    #[validate(custom(function = "validate_url_length"))]
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
    #[validate(length(min = 1, max = 8), custom(function = "validate_urls_length"))]
    pub urls: Vec<Url>,
}

#[cfg(feature = "validator")]
fn validate_url_length(url: &impl AsRef<str>) -> Result<(), validator::ValidationError> {
    if url.as_ref().len() > MAX_URL_LENGTH {
        let mut err = validator::ValidationError::new("url_length");
        err.add_param("max".into(), &MAX_URL_LENGTH);
        return Err(err);
    }

    Ok(())
}

#[cfg(feature = "validator")]
fn validate_urls_length(urls: &[Url]) -> Result<(), validator::ValidationError> {
    for url in urls {
        validate_url_length(url)?;
    }

    Ok(())
}

/// a request to create a new embed
#[record]
pub struct EmbedCreate {
    // TODO
}

#[cfg(all(test, feature = "validator"))]
mod tests {
    use super::*;
    use validator::Validate;

    fn url_of_length(len: usize) -> Url {
        let base = "https://example.com/";
        Url::parse(&format!("{base}{}", "a".repeat(len - base.len()))).unwrap()
    }

    #[test]
    fn embed_author_url_length() {
        let author = EmbedAuthor {
            name: "author".to_string(),
            url: Some(url_of_length(MAX_URL_LENGTH)),
            avatar: None,
        };
        assert!(author.validate().is_ok());

        let author = EmbedAuthor {
            url: Some(url_of_length(MAX_URL_LENGTH + 1)),
            ..author
        };
        assert!(author.validate().is_err());
    }

    #[test]
    fn embed_generate_url_length() {
        let req = EmbedGenerate {
            urls: vec![url_of_length(MAX_URL_LENGTH)],
        };
        assert!(req.validate().is_ok());

        let req = EmbedGenerate {
            urls: vec![url_of_length(MAX_URL_LENGTH + 1)],
        };
        assert!(req.validate().is_err());
    }
}

// TODO: finish impl and use
