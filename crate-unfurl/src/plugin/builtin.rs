use async_trait::async_trait;
use url::Url;

use crate::{
    Unfurler,
    plugin::{Plugin, PluginUrl, UnfurlResult},
    unfurler::UnfurlerBuilder,
};

/// handle http(s) urls and forward responses to http plugins
pub struct SelectHttp;

/// handle html and forward data to html plugins
pub struct SelectHtml;

impl Plugin for SelectHttp {
    fn register(&self, builder: UnfurlerBuilder) -> UnfurlerBuilder {
        builder.add_plugin_url(SelectHttp)
    }
}

#[async_trait]
impl PluginUrl for SelectHttp {
    async fn handle(&self, unfurler: &Unfurler, url: &Url) -> UnfurlResult {
        if !matches!(url.scheme(), "http" | "https") {
            return UnfurlResult::skip();
        }

        let res = match unfurler
            .http_client()
            .get(url.clone())
            .send()
            .await
            .and_then(|res| res.error_for_status())
        {
            Ok(res) => res,
            Err(e) => {
                return UnfurlResult {
                    embeds: vec![],
                    errors: vec![e.into()],
                    stop: true,
                };
            }
        };

        todo!()
    }
}

// TODO: impl SelectHtml
