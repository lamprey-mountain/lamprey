//! the unfurler uses a plugin based system

use std::ops::ControlFlow;

use async_trait::async_trait;
use bytes::Bytes;
use reqwest::{StatusCode, header::HeaderMap};
use url::Url;

use crate::{
    Unfurler,
    error::UnfurlError,
    unfurler::{EmbedGeneration, UnfurlerBuilder},
};

// TODO: impl and use these
pub(crate) mod builtin;

pub mod direct_media;
pub mod html; // TODO: split html module into opengraph, microdata, json_ld
pub mod wikipedia;
// pub mod microdata;
// pub mod json_ld;
// pub mod opengraph;

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

pub struct Head<'a> {
    pub url: &'a Url,
    pub status: StatusCode,
    pub headers: &'a HeaderMap,
}

/// A plugin for the unfurler
pub trait Plugin: Send + Sync {
    /// The name of this plugin, for debugging
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn register(self, builder: UnfurlerBuilder) -> UnfurlerBuilder;
}

// TODO: surely theres a better way than copying `fn name()` to every plugin trait?

/// an unfurler plugin that handles urls
///
/// Use this for custom protocols (`magnet://`). Only return `Some` if embeds
/// are being generated, `Some([])` explicitly says that this url has no embeds.
#[async_trait]
pub trait PluginUrl: Send + Sync {
    /// The name of this plugin, for debugging
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// attempt to handle a url
    async fn handle(&self, unfurler: &Unfurler, url: &Url) -> UnfurlResult;
}

/// an unfurler plugin that handles http responses
#[async_trait]
pub trait PluginHttp: Send + Sync {
    /// The name of this plugin, for debugging
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// attempt to handle a http response
    async fn accept(&self, unfurler: &Unfurler, head: &Head<'_>) -> Option<Box<dyn HttpSink>>;
}

/// an unfurler plugin that handles html responses
#[async_trait]
pub trait PluginHtml: Send + Sync {
    // TODO(?): associated type for PluginHtml
    // type Handler: HtmlSink;

    /// The name of this plugin, for debugging
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// create a new html sink
    async fn accept(&self, unfurler: &Unfurler, head: &Head<'_>) -> Option<Box<dyn HtmlSink>>;
}

#[async_trait]
pub trait HttpSink: Send {
    async fn push(&mut self, chunk: Bytes) -> ControlFlow<UnfurlResult>;
    async fn finish(self: Box<Self>) -> UnfurlResult;
}

#[async_trait]
pub trait HtmlSink: Send {
    fn push(
        &mut self,
        token: &html5ever::tokenizer::Token,
        line_number: u64,
    ) -> ControlFlow<UnfurlResult>;
    fn finish(self: Box<Self>) -> UnfurlResult;
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

    /// return an error
    pub fn error(error: UnfurlError) -> Self {
        Self {
            embeds: vec![],
            errors: vec![error],
            stop: false,
        }
    }
}
