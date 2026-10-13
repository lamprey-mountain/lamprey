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

use crate::{
    Plugin, PluginHttp, Unfurler,
    plugin::{Head, HtmlSink, HttpSink, UnfurlResult},
    unfurler::UnfurlerBuilder,
};

/// a plugin which forwards html responses to html plugins
#[derive(Debug)]
pub struct ForwardHtmlPlugin {
    // TODO: let end user configure this
    // TODO: maybe move body size limit to the http plugin?
    pub max_bytes: usize,
}

impl Default for ForwardHtmlPlugin {
    fn default() -> Self {
        Self {
            max_bytes: 1024 * 1024 * 4,
        }
    }
}

impl Plugin for ForwardHtmlPlugin {
    fn register(self, builder: UnfurlerBuilder) -> UnfurlerBuilder {
        builder.add_plugin_http(self)
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
            let mut out = String::new();

            while let Some(chunk) = rx.blocking_recv() {
                let max_len = decoder
                    .max_utf8_buffer_length(chunk.len())
                    .expect("chunk way too big!");
                out.clear();
                out.reserve(max_len);
                let _ = decoder.decode_to_string(&chunk, &mut out, false);
                queue.push_back(StrTendril::from_slice(&out));
                let _ = tokenizer.feed(&mut queue);
            }

            out.clear();
            let _ = decoder.decode_to_string(&[], &mut out, true);
            if !out.is_empty() {
                queue.push_back(StrTendril::from_slice(&out));
                let _ = tokenizer.feed(&mut queue);
            }

            tokenizer.end();
            tokenizer.sink
        });

        Some(Box::new(HtmlHttpSink {
            tx,
            parse_task,
            bytes_read: 0,
            max_bytes: self.max_bytes,
        }))
    }
}

struct HtmlHttpSink {
    tx: mpsc::Sender<Bytes>,
    parse_task: tokio::task::JoinHandle<MultiSink>,
    bytes_read: usize,
    max_bytes: usize,
}

#[async_trait]
impl HttpSink for HtmlHttpSink {
    async fn push(&mut self, chunk: Bytes) -> ControlFlow<UnfurlResult> {
        self.bytes_read += chunk.len();
        if let Err(_) = self.tx.send(chunk).await {
            debug!("channel closed, stopping");
        }

        if self.bytes_read > self.max_bytes {
            debug!("max bytes exceeded, stopping");
            return ControlFlow::Break(UnfurlResult {
                embeds: vec![],
                errors: vec![],
                stop: true,
            });
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
            let s = sink.finish().await; // PERF: join in parallel
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
