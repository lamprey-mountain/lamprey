use std::ops::ControlFlow;

use async_trait::async_trait;
use bytes::Bytes;
use lamprey_common::v1::types::{EmbedType, Mime};
use url::Url;

use crate::{
    Plugin, PluginHttp, Unfurler,
    plugin::{Head, HttpSink, UnfurlResult},
    unfurler::{EmbedGeneration, UnfurlerBuilder},
    util::{AbyssSink, EmbedGenerationTemplate, EmbedMedia, EmbedMediaPending},
};

/// a plugin which handles links to media files
pub struct DirectMediaPlugin;

impl Plugin for DirectMediaPlugin {
    fn register(self, builder: UnfurlerBuilder) -> UnfurlerBuilder {
        builder.add_plugin_http(self)
    }
}

#[async_trait]
impl PluginHttp for DirectMediaPlugin {
    async fn accept(&self, _unfurler: &Unfurler, head: &Head<'_>) -> Box<dyn HttpSink> {
        let accepts_response =
            if let Some(content_type) = head.headers.get(reqwest::header::CONTENT_TYPE) {
                let ct = content_type.to_str().unwrap_or_default();
                ct.starts_with("image/") || ct.starts_with("video/") || ct.starts_with("audio/")
            } else {
                false
            };

        if accepts_response {
            Box::new(DirectMediaSink {
                url: head.url.clone(),
                content_type: head
                    .headers
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string()),
            })
        } else {
            Box::new(AbyssSink)
        }
    }
}

struct DirectMediaSink {
    url: Url,
    content_type: Option<String>,
}

#[async_trait]
impl HttpSink for DirectMediaSink {
    async fn push(&mut self, _chunk: Bytes) -> ControlFlow<UnfurlResult> {
        ControlFlow::Continue(())
    }

    async fn finish(self: Box<Self>) -> UnfurlResult {
        let ct_str = self
            .content_type
            .as_deref()
            .unwrap_or("application/octet-stream");

        let mime: Mime = ct_str
            .parse()
            .unwrap_or_else(|_| "application/octet-stream".parse().unwrap());

        let media: EmbedMedia = EmbedMediaPending::new(self.url.clone())
            .mime_guess(mime)
            .into();

        let embed = EmbedGeneration {
            embed: EmbedGenerationTemplate {
                ty: EmbedType::Media,
                url: Some(self.url.clone()),
                canonical_url: Some(self.url.clone()),
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
