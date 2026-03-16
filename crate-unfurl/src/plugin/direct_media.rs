use async_trait::async_trait;
use lamprey_common::v1::types::{EmbedType, Mime};
use reqwest::Response;
use url::Url;

use crate::{
    error::UnfurlError,
    plugin::UnfurlPlugin,
    unfurler::EmbedGeneration,
    util::{EmbedGenerationTemplate, EmbedMedia, EmbedMediaPending},
};

pub struct DirectMediaPlugin;

#[async_trait]
impl UnfurlPlugin for DirectMediaPlugin {
    fn name(&self) -> &'static str {
        "DirectMediaPlugin"
    }

    fn accepts_response(&self, res: &Response) -> bool {
        if let Some(content_type) = res.headers().get(reqwest::header::CONTENT_TYPE) {
            let ct = content_type.to_str().unwrap_or_default();
            ct.starts_with("image/") || ct.starts_with("video/") || ct.starts_with("audio/")
        } else {
            false
        }
    }

    async fn process_response(
        &self,
        url: &Url,
        res: Response,
    ) -> Result<Vec<EmbedGeneration>, UnfurlError> {
        // Extract basic mime info
        let ct_str = res
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream");

        let mime: Mime = ct_str
            .parse()
            .unwrap_or_else(|_| "application/octet-stream".parse().unwrap());

        let media: EmbedMedia = EmbedMediaPending::new(url.clone()).mime_guess(mime).into();

        Ok(vec![EmbedGeneration {
            embed: EmbedGenerationTemplate {
                ty: EmbedType::Media,
                url: Some(url.clone()),
                canonical_url: Some(res.url().clone()),
                media: Some(media),
                title: None,
                description: None,
                color: None,
                thumbnail: None,
                author_name: None,
                author_url: None,
                author_avatar: None,
                site_name: None,
                site_avatar: None,
            },
        }])
    }
}
