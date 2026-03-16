use std::sync::Arc;

use lamprey_common::{
    v1::types::{Embed, EmbedId, MediaId},
    v2::types::media::Media,
};
use reqwest::{Client, ClientBuilder};
use url::Url;

use crate::{
    error::UnfurlError,
    plugin::UnfurlPlugin,
    util::{EmbedGenerationTemplate, EmbedMedia, EmbedMediaPending},
};

/// Helper function to extract finished media from EmbedMedia
/// Returns None for Pending, Downloading, or Failed states
fn media_to_finished(media: EmbedMedia) -> Option<Media> {
    match media {
        EmbedMedia::Finished(m) => Some(m),
        EmbedMedia::Downloading(m) => Some(m),
        _ => None,
    }
}

/// The progressive state of an Embed.
#[derive(Debug, Clone)]
pub struct EmbedGeneration {
    pub(crate) embed: EmbedGenerationTemplate,
}

pub struct Unfurler {
    client: Client,
    plugins: Vec<Arc<dyn UnfurlPlugin>>,
}

pub struct UnfurlerBuilder {
    client_builder: ClientBuilder,
    plugins: Vec<Arc<dyn UnfurlPlugin>>,
}

impl Unfurler {
    pub fn builder() -> UnfurlerBuilder {
        UnfurlerBuilder {
            // Safe defaults for external fetching
            client_builder: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .user_agent("Mozilla/5.0 (compatible; LampreyBot/1.0; +https://example.com)"),
            plugins: Vec::new(),
        }
    }

    /// Generate some url embeds for this url. Only runs the first sucessful plugin, but the plugin may return multiple embeds.
    pub async fn unfurl(&self, url: &Url) -> Result<Vec<EmbedGeneration>, UnfurlError> {
        // 1. Try URL-based plugins (e.g. magnet://, ipfs://)
        for plugin in &self.plugins {
            if let Some(generation) = plugin.process_url(url).await? {
                return Ok(generation);
            }
        }

        // 2. We need an HTTP response. Reject non-HTTP protocols at this point.
        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(UnfurlError::UnsupportedProtocol);
        }

        let res = self.client.get(url.clone()).send().await?;
        let final_url = res.url().clone();

        // 3. Find a plugin that handles this specific response
        for plugin in &self.plugins {
            if plugin.accepts_response(&res) {
                return plugin.process_response(&final_url, res).await;
            }
        }

        Err(UnfurlError::NoPluginMatch)
    }
}

impl UnfurlerBuilder {
    pub fn client_config<F>(mut self, f: F) -> Self
    where
        F: FnOnce(ClientBuilder) -> ClientBuilder,
    {
        self.client_builder = f(self.client_builder);
        self
    }

    pub fn add_plugin<P: UnfurlPlugin + 'static>(mut self, plugin: P) -> Self {
        self.plugins.push(Arc::new(plugin));
        self
    }

    pub fn build(self) -> Result<Unfurler, reqwest::Error> {
        Ok(Unfurler {
            client: self.client_builder.build()?,
            plugins: self.plugins,
        })
    }
}

impl EmbedGeneration {
    /// converts to a standard embed
    ///
    /// return None for pending or failed media
    pub fn to_embed(self) -> Embed {
        Embed {
            id: EmbedId::new(),
            ty: self.embed.ty,
            url: self.embed.url,
            canonical_url: self.embed.canonical_url,
            title: self.embed.title,
            description: self.embed.description,
            color: self.embed.color,
            media: self.embed.media.and_then(media_to_finished),
            thumbnail: self.embed.thumbnail.and_then(media_to_finished),
            author_name: self.embed.author_name,
            author_url: self.embed.author_url,
            author_avatar: self.embed.author_avatar.and_then(media_to_finished),
            site_name: self.embed.site_name,
            site_avatar: self.embed.site_avatar.and_then(media_to_finished),
        }
    }

    fn iter_media_mut(&mut self) -> impl Iterator<Item = &mut Option<EmbedMedia>> {
        let t = &mut self.embed;
        std::iter::once(&mut t.media)
            .chain(std::iter::once(&mut t.thumbnail))
            .chain(std::iter::once(&mut t.author_avatar))
            .chain(std::iter::once(&mut t.site_avatar))
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
