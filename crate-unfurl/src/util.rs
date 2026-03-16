use lamprey_common::{
    v1::types::{misc::Color, EmbedType, MediaId, Mime},
    v2::types::media::Media,
};
use url::Url;

#[derive(Debug, Clone)]
pub struct EmbedGenerationTemplate {
    pub ty: EmbedType,
    pub url: Option<Url>,
    pub canonical_url: Option<Url>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub color: Option<Color>,
    pub media: Option<EmbedMedia>,
    pub thumbnail: Option<EmbedMedia>,
    pub author_name: Option<String>,
    pub author_url: Option<Url>,
    pub author_avatar: Option<EmbedMedia>,
    pub site_name: Option<String>,
    pub site_avatar: Option<EmbedMedia>,
}

#[derive(Debug, Clone)]
pub enum EmbedMedia {
    /// the client should fetch this media
    Pending(EmbedMediaPending),

    /// media created, currently downloading
    Downloading(Media),

    /// done downloading and processing
    Finished(Media),

    /// failed to fetch for some reason
    Failed(EmbedMediaFailed),
}

#[derive(Debug, Clone)]
pub struct EmbedMediaPending {
    pub placeholder_media_id: MediaId,
    pub url: Url,
    pub mime_guess: Option<Mime>,
}

#[derive(Debug, Clone)]
pub struct EmbedMediaFailed {
    pub message: String,
}

impl EmbedMediaPending {
    pub fn new(url: Url) -> Self {
        EmbedMediaPending {
            placeholder_media_id: MediaId::new(),
            url,
            mime_guess: None,
        }
    }

    pub fn mime_guess(mut self, m: Mime) -> Self {
        self.mime_guess = Some(m);
        self
    }
}

impl From<EmbedMediaPending> for EmbedMedia {
    fn from(value: EmbedMediaPending) -> Self {
        EmbedMedia::Pending(value)
    }
}
