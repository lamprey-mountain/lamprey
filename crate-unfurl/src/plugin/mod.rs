//! the unfurler uses a plugin based system

use async_trait::async_trait;
use reqwest::{Response, StatusCode, header::HeaderMap};
use url::Url;

use crate::{
    Unfurler,
    error::UnfurlError,
    unfurler::{EmbedGeneration, UnfurlerBuilder},
};

// TODO: impl and use these
pub(crate) mod builtin;

pub mod direct_media;
pub mod html;
pub mod wikipedia;

// TODO: remove
#[async_trait]
#[deprecated = "use Plugin instead"]
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

/// the result of an unfurling
#[derive(Debug)]
pub struct UnfurlResult {
    /// generated embeds
    pub embeds: Vec<EmbedGeneration>,

    /// errors that occured while unfurling a url
    pub errors: Vec<UnfurlError>,

    /// whether to halt
    pub stop: bool,
}

impl UnfurlResult {
    /// skip this plugin
    pub fn skip() -> Self {
        Self {
            embeds: vec![],
            errors: vec![],
            stop: false,
        }
    }
}

/// A plugin for the unfurler
#[async_trait]
pub trait Plugin: Send + Sync {
    /// The name of this plugin, for debugging
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn register(&self, builder: UnfurlerBuilder) -> UnfurlerBuilder;
}

/// an unfurler plugin that handles urls
///
/// Use this for custom protocols (`magnet://`). Only return `Some` if embeds
/// are being generated, `Some([])` explicitly says that this url has no embeds.
#[async_trait]
pub trait PluginUrl: Send + Sync {
    /// attempt to handle a url
    async fn handle(&self, unfurler: &Unfurler, url: &Url) -> UnfurlResult;
}

/// an unfurler plugin that handles http responses
#[async_trait]
pub trait PluginHttp: Send + Sync {
    /// attempt to handle a http response
    async fn handle(&self, unfurler: &Unfurler, url: &Url, res: &Response) -> UnfurlResult;
}

/// an unfurler plugin that handles html responses
pub trait PluginHtml: Send + Sync {
    // TODO(?): associated type for PluginHtml
    // type Handler: HtmlSink;

    /// create a new html sink
    fn create_sink(
        &self,
        unfurler: &Unfurler,
        url: &Url,
        status: StatusCode,
        headers: &HeaderMap,
    ) -> Box<dyn HtmlSink>;
}

#[async_trait]
pub trait HtmlSink: Send {
    fn process_token(&mut self, token: &html5ever::tokenizer::Token, line_number: u64);
    fn finish(self) -> UnfurlResult;
}
