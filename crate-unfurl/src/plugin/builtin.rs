//! Builtin plugins that may be used by the unfurler.

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
    util::AbyssSink,
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
            info!(
                plugin = %plugin.name(),
                kind = "http",
                "Selected plugin"
            );
            sinks.push(plugin.accept(unfurler, &head).await);
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
    async fn accept(&self, unfurler: &Unfurler, head: &Head<'_>) -> Box<dyn HttpSink> {
        let is_html = head
            .headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.contains("text/html"))
            .unwrap_or(false); // Default to false, some sites might be sneaky though
        if !is_html {
            return Box::new(AbyssSink);
        }

        let mut sinks = vec![];
        for plugin in &unfurler.html_plugins {
            info!(
                plugin = %plugin.name(),
                kind = "html",
                "Selected plugin"
            );
            sinks.push(plugin.create_sink(unfurler, head));
        }

        // the html parser is !Send due to Rc, so spawn a blocking task
        let (tx, mut rx) = mpsc::channel::<Bytes>(32);
        let parse_task = tokio::task::spawn_blocking(move || {
            let sink = MultiSink(RefCell::new(sinks));
            let tokenizer = Tokenizer::new(sink, TokenizerOpts::default());
            let mut queue = BufferQueue::default();

            let mut tail = Vec::new();

            while let Some(chunk) = rx.blocking_recv() {
                let s = decode_chunk(&mut tail, &chunk);
                queue.push_back(StrTendril::from_slice(&s));
                let _ = tokenizer.feed(&mut queue);
            }

            tokenizer.end();
            tokenizer.sink
        });

        Box::new(HtmlHttpSink {
            tx,
            parse_task,
            bytes_read: 0,
        })
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

/// Merges `tail` with `chunk`, returning the valid UTF-8 string and
/// storing any incomplete trailing bytes back into `tail`.
// TEMP: pub
fn decode_chunk<'a>(tail: &mut Vec<u8>, chunk: &[u8]) -> String {
    let bytes = if tail.is_empty() {
        // PERF: don't clone chunk
        chunk.to_vec()
    } else {
        let mut b = std::mem::take(tail);
        b.extend_from_slice(chunk);
        b
    };

    match std::str::from_utf8(&bytes) {
        Ok(s) => s.to_string(),
        Err(e) => {
            // FIXME: handle incomplete vs invalid byte sequences
            // currently, this code treats all errors as incomplete sequences.
            // this can cause all data to be stored in tail, rather than be
            // properly parsed.
            let valid = &bytes[..e.valid_up_to()];
            tail.extend_from_slice(&bytes[e.valid_up_to()..]);
            std::str::from_utf8(valid).unwrap().to_string()
        }
    }
}
