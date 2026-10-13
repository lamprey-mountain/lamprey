//! Builtin plugins that may be used by the unfurler.

use std::ops::ControlFlow;

use async_trait::async_trait;
use tracing::{debug, info};
use url::Url;

use crate::{
    Plugin, PluginUrl, Unfurler,
    plugin::{Head, UnfurlResult},
    unfurler::UnfurlerBuilder,
};

#[derive(Debug)]
pub struct ForwardHttpPlugin {
    pub max_bytes: usize,
}

impl Default for ForwardHttpPlugin {
    fn default() -> Self {
        Self {
            max_bytes: 1024 * 1024 * 4,
        }
    }
}

impl Plugin for ForwardHttpPlugin {
    fn register(self, builder: UnfurlerBuilder) -> UnfurlerBuilder {
        builder.add_plugin_url(self)
    }
}

#[async_trait]
impl PluginUrl for ForwardHttpPlugin {
    /// attempt to handle a url
    async fn handle(&self, unfurler: &Unfurler, url: &Url) -> UnfurlResult {
        if url.scheme() != "http" && url.scheme() != "https" {
            return UnfurlResult::skip();
        }

        let mut res = match unfurler.http_client().get(url.clone()).send().await {
            Ok(r) => r,
            Err(e) => return UnfurlResult::error(e.into()),
        };

        let head = Head {
            url: res.url(),
            status: res.status(),
            headers: res.headers(),
        };

        let mut sinks = vec![];
        for plugin in &unfurler.http_plugins {
            if let Some(sink) = plugin.accept(unfurler, &head).await {
                info!(
                    plugin = %plugin.name(),
                    kind = "http",
                    "Select plugin"
                );
                sinks.push(sink);
            }
        }

        if sinks.is_empty() {
            return UnfurlResult::skip();
        }

        let mut embeds = vec![];
        let mut bytes_read = 0;
        let max_bytes = self.max_bytes;

        while let Some(chunk) = match res.chunk().await {
            Ok(c) => c,
            Err(e) => return UnfurlResult::error(e.into()),
        } {
            bytes_read += chunk.len();
            if bytes_read > max_bytes {
                debug!("max bytes exceeded, stopping");
                return UnfurlResult {
                    embeds,
                    errors: vec![],
                    stop: true,
                };
            }

            for sink in &mut sinks {
                match sink.push(chunk.clone()).await {
                    ControlFlow::Continue(_) => continue,
                    ControlFlow::Break(res) => {
                        embeds.extend(res.embeds);
                        // TODO: handle res.errors
                        if res.stop {
                            return UnfurlResult {
                                embeds,
                                errors: vec![],
                                stop: true,
                            };
                        }
                    }
                }
            }
        }

        for sink in sinks {
            let res = sink.finish().await;
            embeds.extend(res.embeds);
            // TODO: handle res.errors

            if res.stop {
                break;
            }
        }

        UnfurlResult {
            embeds,
            errors: vec![],
            stop: false,
        }
    }
}
