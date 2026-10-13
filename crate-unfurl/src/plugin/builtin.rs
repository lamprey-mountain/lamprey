//! Builtin plugins that may be used by the unfurler.

use encoding_rs::UTF_8;
use std::{cell::RefCell, ops::ControlFlow};

use async_trait::async_trait;
use bytes::Bytes;
use html5ever::{
    tendril::StrTendril,
    tokenizer::{BufferQueue, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts},
};
use tokio::sync::mpsc;
use tracing::{debug, info};
use url::Url;

use crate::{
    Plugin, PluginHttp, PluginUrl, Unfurler,
    plugin::{Head, HtmlSink, HttpSink, UnfurlResult},
    unfurler::UnfurlerBuilder,
};

/// a plugin which forwards http responses to http plugins
#[derive(Debug, Default)]
pub struct ForwardHttpPlugin;

/// a plugin which forwards html responses to html plugins
#[derive(Debug, Default)]
pub struct ForwardHtmlPlugin;

impl Plugin for ForwardHttpPlugin {
    fn register(self, builder: UnfurlerBuilder) -> UnfurlerBuilder {
        builder.add_plugin_url(self)
    }
}

impl Plugin for ForwardHtmlPlugin {
    fn register(self, builder: UnfurlerBuilder) -> UnfurlerBuilder {
        builder.add_plugin_http(self)
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
        while let Some(chunk) = match res.chunk().await {
            Ok(c) => c,
            Err(e) => return UnfurlResult::error(e.into()),
        } {
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

#[async_trait]
impl PluginHttp for ForwardHtmlPlugin {
    async fn accept(&self, unfurler: &Unfurler, head: &Head<'_>) -> Option<Box<dyn HttpSink>> {
        let is_html = head
            .headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.contains("text/html"))
            .unwrap_or(false); // Default to false, some sites might be sneaky though
        if !is_html {
            return None;
        }

        let mut sinks = vec![];
        for plugin in &unfurler.html_plugins {
            if let Some(sink) = plugin.accept(unfurler, head).await {
                info!(
                    plugin = %plugin.name(),
                    kind = "html",
                    "Select plugin"
                );
                sinks.push(sink);
            }
        }

        if sinks.is_empty() {
            return None;
        }

        // the html parser is !Send due to Rc, so spawn a blocking task
        let (tx, mut rx) = mpsc::channel::<Bytes>(32);
        let parse_task = tokio::task::spawn_blocking(move || {
            let sink = MultiSink(RefCell::new(sinks));
            let tokenizer = Tokenizer::new(sink, TokenizerOpts::default());
            let mut queue = BufferQueue::default();
            let mut decoder = UTF_8.new_decoder();

            while let Some(chunk) = rx.blocking_recv() {
                let mut output = String::with_capacity(chunk.len());
                let (_result, _read, _had_errors) = decoder.decode_to_string(&chunk, &mut output, false);
                queue.push_back(StrTendril::from_slice(&output));
                let _ = tokenizer.feed(&mut queue);
            }

            tokenizer.end();
            tokenizer.sink
        });

        Some(Box::new(HtmlHttpSink {
            tx,
            parse_task,
            bytes_read: 0,
        }))
    }
}

struct HtmlHttpSink {
    tx: mpsc::Sender<Bytes>,
    parse_task: tokio::task::JoinHandle<MultiSink>,
    bytes_read: usize,
}

#[async_trait]
impl HttpSink for HtmlHttpSink {
    async fn push(&mut self, chunk: Bytes) -> ControlFlow<UnfurlResult> {
        self.bytes_read += chunk.len();
        if let Err(_) = self.tx.send(chunk).await {
            debug!("channel closed, stopping");
            // TODO: break if closed
            // return ControlFlow::Break(boxed_self.finish());
        }

        // TODO: configurable self.max_bytes
        if self.bytes_read > 1024 * 1024 * 4 {
            debug!("max bytes exceeded, stopping");
            // TODO: break here
            // return ControlFlow::Break(self.finish());
        }

        ControlFlow::Continue(())
    }

    async fn finish(self: Box<Self>) -> UnfurlResult {
        drop(self.tx);

        let mut result = UnfurlResult::skip();
        let sinks = match self.parse_task.await {
            Ok(s) => s.0.into_inner(),
            Err(e) => return UnfurlResult::error(e.into()),
        };
        for sink in sinks {
            let s = sink.finish();
            result.embeds.extend(s.embeds);
            result.errors.extend(s.errors);
            result.stop = s.stop;

            if result.stop {
                break;
            }
        }

        result
    }
}

struct MultiSink(RefCell<Vec<Box<dyn HtmlSink>>>);

impl TokenSink for MultiSink {
    type Handle = ();

    fn process_token(&self, token: Token, line_number: u64) -> TokenSinkResult<Self::Handle> {
        for sink in self.0.borrow_mut().iter_mut() {
            if sink.push(&token, line_number).is_break() {
                // TODO: handle break
            }
        }
        TokenSinkResult::Continue
    }
}
