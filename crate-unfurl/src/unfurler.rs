use std::sync::Arc;

use lamprey_common::{
    v1::types::{Embed, EmbedId, MediaId},
    v2::types::media::Media,
};
use reqwest::{Client, ClientBuilder};
use url::Url;

use crate::{
    Plugin,
    error::UnfurlError,
    plugin::{PluginHtml, PluginHttp, PluginUrl, UnfurlPlugin},
    util::{EmbedGenerationTemplate, EmbedMedia, EmbedMediaPending},
};

/// The progressive state of an Embed.
#[derive(Debug, Clone)]
pub struct EmbedGeneration {
    pub(crate) embed: EmbedGenerationTemplate,
}

pub struct Unfurler {
    client: Client,
    url_plugins: Vec<Box<dyn PluginUrl>>,
    http_plugins: Vec<Box<dyn PluginHttp>>,
    html_plugins: Vec<Box<dyn PluginHtml>>,
}

pub struct UnfurlerBuilder {
    client_builder: ClientBuilder,
    url_plugins: Vec<Box<dyn PluginUrl>>,
    http_plugins: Vec<Box<dyn PluginHttp>>,
    html_plugins: Vec<Box<dyn PluginHtml>>,
}

// TODO: use this
// /// an error that occured while building an unfurler
// #[derive(Debug, thiserror::Error)]
// pub enum UnfurlerBuilderError {
//     #[error("{0}")]
//     Reqwest(#[from] reqwest::Error),
// }

impl Unfurler {
    pub fn builder() -> UnfurlerBuilder {
        UnfurlerBuilder {
            client_builder: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .user_agent("Mozilla/5.0 (compatible; LampreyBot/1.0; +https://example.com)"),
            url_plugins: Vec::new(),
            http_plugins: Vec::new(),
            html_plugins: Vec::new(),
        }
    }

    /// generate embeds for this url
    #[deprecated = "use unfurl() directly"]
    pub async fn unfurl_with_tracing(
        &self,
        url: &Url,
    ) -> Result<Vec<EmbedGeneration>, UnfurlError> {
        self.unfurl(url).await
    }

    /// generate embeds for this url
    // TODO: return embeds *and* errors, not a result
    pub async fn unfurl(&self, url: &Url) -> Result<Vec<EmbedGeneration>, UnfurlError> {
        // PERF: try to run as many plugins in parallel as possible?

        let mut embeds = vec![];

        // 1. url based plugins (like magnet:// or ipfs://)
        for plugin in &self.url_plugins {
            let res = plugin.handle(self, url).await;
            embeds.extend(res.embeds);
            // TODO: handle res.errors

            // tracing::info!(
            //     plugin = %plugin.name(),
            //     reason = "url",
            //     "Selected plugin"
            // );

            if res.stop {
                return Ok(embeds);
            }
        }

        // TODO: use or remove SelectHttp and SelectHtml

        // 2. http based plugins
        if url.scheme() != "http" && url.scheme() != "https" {
            return Ok(embeds);
        }

        let res = self.client.get(url.clone()).send().await?;

        for plugin in &self.http_plugins {
            let res = plugin.handle(self, res.url(), &res).await;
            embeds.extend(res.embeds);
            // TODO: handle res.errors

            if res.stop {
                return Ok(embeds);
            }
        }

        // 3. html based plugins
        for plugin in &self.html_plugins {
            // TODO: implement this
            // let sink = plugin.create_sink(self, res.url(), res.status(), res.headers());
            // sink.process_token(token, line_number);
            // let res = sink.finish();
        }

        Ok(embeds)
    }

    /// get the reqwest http client
    pub fn http_client(&self) -> &reqwest::Client {
        &self.client
    }
}

impl UnfurlerBuilder {
    /// configure the http client
    pub fn client_config<F>(mut self, f: F) -> Self
    where
        F: FnOnce(ClientBuilder) -> ClientBuilder,
    {
        self.client_builder = f(self.client_builder);
        self
    }

    /// add a plugin to this unfurler
    pub fn add_plugin<P: Plugin + 'static>(mut self, plugin: P) -> Self {
        plugin.register(self)
    }

    /// add a url plugin to this unfurler
    pub fn add_plugin_url<P: PluginUrl + 'static>(mut self, plugin: P) -> Self {
        self.url_plugins.push(Box::new(plugin));
        self
    }

    /// add a http plugin to this unfurler
    pub fn add_plugin_http<P: PluginHttp + 'static>(mut self, plugin: P) -> Self {
        todo!()
    }

    /// add a html plugin to this unfurler
    pub fn add_plugin_html<P: PluginHtml + 'static>(mut self, plugin: P) -> Self {
        todo!()
    }

    pub fn build(self) -> Result<Unfurler, reqwest::Error> {
        Ok(Unfurler {
            client: self.client_builder.build()?,
            url_plugins: self.url_plugins,
            http_plugins: self.http_plugins,
            html_plugins: self.html_plugins,
        })
    }
}

impl EmbedGeneration {
    /// converts to a standard embed
    ///
    /// return None for pending or failed media
    pub fn into_embed(self) -> Embed {
        Embed {
            id: EmbedId::new(),
            ty: self.embed.ty,
            url: self.embed.url,
            canonical_url: self.embed.canonical_url,
            title: self.embed.title,
            description: self.embed.description,
            color: self.embed.color,
            media: self.embed.media.and_then(|m| m.to_finished()),
            thumbnail: self.embed.thumbnail.and_then(|m| m.to_finished()),
            author_name: self.embed.author_name,
            author_url: self.embed.author_url,
            author_avatar: self.embed.author_avatar.and_then(|m| m.to_finished()),
            site_name: self.embed.site_name,
            site_avatar: self.embed.site_avatar.and_then(|m| m.to_finished()),
        }
    }

    /// iterates over all mutable media fields
    pub fn iter_media_mut(&mut self) -> impl Iterator<Item = &mut Option<EmbedMedia>> {
        let t = &mut self.embed;
        std::iter::once(&mut t.media)
            .chain(std::iter::once(&mut t.thumbnail))
            .chain(std::iter::once(&mut t.author_avatar))
            .chain(std::iter::once(&mut t.site_avatar))
    }

    /// iterates over all immutable media fields
    pub fn iter_media(&self) -> impl Iterator<Item = &Option<EmbedMedia>> {
        let t = &self.embed;
        std::iter::once(&t.media)
            .chain(std::iter::once(&t.thumbnail))
            .chain(std::iter::once(&t.author_avatar))
            .chain(std::iter::once(&t.site_avatar))
    }

    pub fn pending_media(&self) -> Vec<EmbedMediaPending> {
        let mut pending = Vec::new();
        let t = &self.embed;

        // Helper array for easy extraction
        let fields = [&t.media, &t.thumbnail, &t.author_avatar, &t.site_avatar];
        for field in fields.into_iter().flatten() {
            if let EmbedMedia::Pending(p) = field {
                pending.push(p.clone());
            }
        }
        pending
    }

    /// Replaces a pending media item with its finished/downloading state based on the ID
    pub fn update_media(&mut self, pending_id: MediaId, new_state: EmbedMedia) -> bool {
        let mut updated = false;
        for field in self.iter_media_mut() {
            if let Some(EmbedMedia::Pending(p)) = field {
                if p.placeholder_media_id == pending_id {
                    *field = Some(new_state.clone());
                    updated = true;
                }
            }
        }
        updated
    }
}

// TODO: find a better way to debug print without `Url`s taking up a dozen lines of space
pub struct PrettyEmbedGeneration<'a>(pub &'a EmbedGeneration);

impl std::fmt::Debug for PrettyEmbedGeneration<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let e = &self.0.embed;
        f.debug_struct("PrettyEmbedGeneration")
            .field("type", &e.ty)
            .field("url", &e.url.as_ref().map(|u| u.as_str()))
            .field(
                "canonical_url",
                &e.canonical_url.as_ref().map(|u| u.as_str()),
            )
            .field("title", &e.title)
            .field("description", &e.description)
            .field("color", &e.color)
            .field("media ", &e.media)
            .field("thumbnail", &e.thumbnail)
            .field("author_name", &e.author_name)
            .field("author_url", &e.author_url.as_ref().map(|u| u.as_str()))
            .field("author_avatar", &e.author_avatar)
            .field("site_name", &e.site_name)
            .field("site_avatar", &e.site_avatar)
            .finish()
    }
}
