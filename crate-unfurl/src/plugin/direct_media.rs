use async_trait::async_trait;
use lamprey_common::v1::types::{EmbedType, Mime};
use reqwest::Response;
use url::Url;

use crate::{
    Plugin, PluginHttp, Unfurler,
    plugin::UnfurlResult,
    unfurler::{EmbedGeneration, UnfurlerBuilder},
    util::{EmbedGenerationTemplate, EmbedMedia, EmbedMediaPending},
};

pub struct DirectMediaPlugin;

impl Plugin for DirectMediaPlugin {
    fn register(self, builder: UnfurlerBuilder) -> UnfurlerBuilder {
        builder.add_plugin_http(self)
    }
}

#[async_trait]
impl PluginHttp for DirectMediaPlugin {
    async fn handle(&self, _unfurler: &Unfurler, url: &Url, res: &Response) -> UnfurlResult {
        let accepts_response =
            if let Some(content_type) = res.headers().get(reqwest::header::CONTENT_TYPE) {
                let ct = content_type.to_str().unwrap_or_default();
                ct.starts_with("image/") || ct.starts_with("video/") || ct.starts_with("audio/")
            } else {
                false
            };

        if !accepts_response {
            return UnfurlResult::skip();
        }

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

        let embed = EmbedGeneration {
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
        };

        UnfurlResult {
            embeds: vec![embed],
            errors: vec![],
            stop: true,
        }
    }
}
