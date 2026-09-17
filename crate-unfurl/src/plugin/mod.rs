use async_trait::async_trait;
use reqwest::Response;
use url::Url;

use crate::{error::UnfurlError, unfurler::EmbedGeneration};

pub mod direct_media;
pub mod html;
pub mod wikipedia;

#[async_trait]
pub trait UnfurlPlugin: Send + Sync {
    /// The name of the plugin for debugging
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Intercept and manually process a url
    ///
    /// Use this for custom protocols (`magnet://`) or specific API targets (`youtube.com`).
    /// Return `Ok(Some(EmbedGeneration))` to short-circuit the HTTP request entirely.
    // TODO: return Option<Result<Vec<EmbedGeneration>, UnfurlError>> instead? {
    async fn process_url(&self, _url: &Url) -> Result<Option<Vec<EmbedGeneration>>, UnfurlError> {
        Ok(None)
    }

    /// Check whether this plugin can accept this http response
    fn accepts_response(&self, _res: &Response) -> bool {
        false
    }

    /// Generate an embed from this http response.
    ///
    /// This takes ownership of the `reqwest::Response` stream.
    async fn process_response(
        &self,
        url: &Url,
        res: Response,
    ) -> Result<Vec<EmbedGeneration>, UnfurlError>;
}
