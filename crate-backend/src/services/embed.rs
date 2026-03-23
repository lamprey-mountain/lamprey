use std::{sync::Arc, time::Duration};

use common::v1::types::MessageSync;
use common::v1::types::MessageType;
use common::v1::types::UserId;
use common::v2::types::embed::Embed;
use common::v2::types::media::{MediaCreate, MediaCreateSource};
use lamprey_unfurl::{DirectMediaPlugin, HtmlStreamPlugin, Unfurler};
use moka::future::Cache;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use url::Url;

use crate::error::Error;
use crate::types::{DbMessageUpdate, MediaLinkType, MessageRef};
use crate::Result;
use crate::ServerStateInner;

const MAX_SIZE_ATTACHMENT: u64 = 1024 * 1024 * 8;
const MAX_EMBED_AGE: Duration = Duration::from_secs(60 * 5);

pub struct ServiceEmbed {
    state: Arc<ServerStateInner>,
    unfurler: Arc<Unfurler>,
    cache: Cache<Url, Embed>,
    stop: broadcast::Sender<()>,
    workers: Mutex<Vec<JoinHandle<()>>>,
}

impl ServiceEmbed {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        let (tx, _) = broadcast::channel(1);
        let unfurler = Arc::new(
            Unfurler::builder()
                .client_config(|builder| {
                    builder
                        .timeout(std::time::Duration::from_secs(15))
                        .connect_timeout(std::time::Duration::from_secs(5))
                        .user_agent(&state.config.http.user_agent)
                })
                .add_plugin(DirectMediaPlugin)
                .add_plugin(HtmlStreamPlugin {
                    max_bytes: 1024 * 1024 * 4,
                })
                .build()
                .expect("failed to build unfurler"),
        );

        Self {
            state,
            unfurler,
            cache: Cache::builder()
                .max_capacity(1000)
                .time_to_live(MAX_EMBED_AGE)
                .build(),
            stop: tx,
            workers: Mutex::new(Vec::new()),
        }
    }

    pub async fn start_workers(&self) {
        let mut workers_guard = self.workers.lock().await;
        if !workers_guard.is_empty() {
            warn!("embed workers already started");
            return;
        }
        for i in 0..self.state.config.http.max_parallel_jobs {
            let state = self.state.clone();
            let mut stop = self.stop.subscribe();
            workers_guard.push(tokio::spawn(async move {
                info!("starting embed worker {i}");
                loop {
                    tokio::select! {
                        _ = stop.recv() => {
                            info!("stopping embed worker {i}");
                            break;
                        }
                        _ = tokio::time::sleep(Duration::from_secs(1)) => {}
                    }
                    if let Err(e) = Self::worker(&state).await {
                        error!("embed worker failed: {e:?}");
                    }
                }
            }));
        }
    }

    pub async fn stop(self) {
        if self.stop.send(()).is_err() {
            warn!("no embed workers to stop");
        }
        let workers = self.workers.into_inner();
        for worker in workers {
            if let Err(e) = worker.await {
                error!("failed to stop embed worker: {e:?}");
            }
        }
    }

    pub fn purge_cache(&self) {
        self.cache.invalidate_all();
    }

    async fn worker(state: &Arc<ServerStateInner>) -> Result<()> {
        let data = state.data();
        let Some(job) = data.url_embed_queue_claim().await? else {
            return Ok(());
        };

        let url: Url = job.url.parse()?;

        let embed = match state
            .services()
            .embed
            .cache
            .try_get_with(url.clone(), async {
                debug!("generating embed for {}", url);
                state
                    .services()
                    .embed
                    .generate_inner(job.user_id.into(), url)
                    .await
                    .map_err(Arc::new)
            })
            .await
        {
            Ok(embed) => embed,
            Err(e_arc) => {
                if let Err(e) = data.url_embed_queue_finish(job.id, None).await {
                    error!("failed to finish url embed queue job with error: {e:?}");
                }
                return Err(e_arc.fake_clone());
            }
        };

        if let Err(e) = data.url_embed_queue_finish(job.id, Some(&embed)).await {
            error!("failed to finish url embed queue job: {e:?}");
        }
        if let Err(e) = Self::attach_embed(
            state,
            job.message_ref.map(|v| serde_json::from_value(v).unwrap()),
            job.user_id.into(),
            embed,
        )
        .await
        {
            error!("failed to attach embed: {e:?}");
        }
        Ok(())
    }

    pub async fn queue(
        &self,
        message_ref: Option<MessageRef>,
        user_id: UserId,
        url: Url,
    ) -> Result<()> {
        if let Some(embed) = self.cache.get(&url).await {
            if let Some(message_ref) = message_ref {
                info!(
                    "reuse embed message: version_id = {} url = {:?}",
                    message_ref.version_id,
                    url.as_str()
                );
                if let Err(e) =
                    Self::attach_embed(&self.state, Some(message_ref), user_id, embed).await
                {
                    error!("failed to attach embed from cache: {e:?}");
                }
            }
            return Ok(());
        }

        self.state
            .data()
            .url_embed_queue_insert(message_ref, user_id, url.to_string())
            .await?;
        Ok(())
    }

    /// Unfurl a single URL without logging
    pub async fn unfurl(
        &self,
        url: &Url,
    ) -> crate::Result<Vec<lamprey_unfurl::unfurler::EmbedGeneration>> {
        self.unfurler
            .unfurl(url)
            .await
            .map_err(|e| Error::UrlEmbedOther(e.to_string()))
    }

    /// Unfurl a single URL with logging support
    pub async fn unfurl_with_logger(
        &self,
        url: &Url,
        log_sink: &mut dyn lamprey_unfurl::logging::LogSink,
    ) -> crate::Result<Vec<lamprey_unfurl::unfurler::EmbedGeneration>> {
        self.unfurler
            .unfurl_with_logger(url, log_sink)
            .await
            .map_err(|e| Error::UrlEmbedOther(e.to_string()))
    }

    #[tracing::instrument(level = "info", skip(self))]
    pub(crate) async fn generate_inner(&self, user_id: UserId, url: Url) -> Result<Embed> {
        // Use unfurler to generate embed
        let mut generations = self.unfurl(&url).await?;

        // Take first generation (unfurler may return multiple)
        let mut generation = generations
            .pop()
            .ok_or(Error::UrlEmbedOther("No embed generated".into()))?;

        // Resolve pending media
        let pending = generation.pending_media();
        for p in pending {
            let media = self
                .state
                .services()
                .media
                .import_from_url_with_max_size(
                    user_id,
                    MediaCreate {
                        alt: p.alt,
                        strip_exif: false,
                        source: MediaCreateSource::Download {
                            filename: None,
                            size: None,
                            source_url: p.url,
                        },
                    },
                    MAX_SIZE_ATTACHMENT,
                )
                .await?;

            generation.update_media(
                p.placeholder_media_id,
                lamprey_unfurl::util::EmbedMedia::Finished(media.clone()),
            );
        }

        // Convert to final embed
        let embed = generation.to_embed();

        debug!("done! {:?}", embed);
        Ok(embed)
    }

    async fn attach_embed(
        state: &Arc<ServerStateInner>,
        message_ref: Option<MessageRef>,
        user_id: UserId,
        embed: Embed,
    ) -> Result<()> {
        let Some(mref) = message_ref else {
            return Ok(());
        };
        let data = state.data();
        let mut message = data
            .message_get(mref.thread_id, mref.message_id, user_id)
            .await?;
        let ver = data
            .message_version_get(mref.thread_id, mref.version_id, user_id)
            .await?;
        message.latest_version = ver;

        let mut message_type = message.latest_version.message_type;
        let (embeds, attachments) = match &mut message_type {
            MessageType::DefaultMarkdown(m) => {
                if m.embeds
                    .iter()
                    .any(|e| e.url.as_ref() == embed.url.as_ref())
                {
                    info!(
                        "skip embed message: version_id = {} url = {:?}",
                        mref.version_id,
                        embed.url.as_ref().map(|u| u.as_str())
                    );
                    return Ok(());
                }

                if let Some(media) = &embed.media {
                    data.media_link_insert(media.id, *mref.version_id, MediaLinkType::Embed)
                        .await?;
                }
                if let Some(media) = &embed.thumbnail {
                    data.media_link_insert(media.id, *mref.version_id, MediaLinkType::Embed)
                        .await?;
                }
                if let Some(media) = &embed.author_avatar {
                    data.media_link_insert(media.id, *mref.version_id, MediaLinkType::Embed)
                        .await?;
                }
                if let Some(media) = &embed.site_avatar {
                    data.media_link_insert(media.id, *mref.version_id, MediaLinkType::Embed)
                        .await?;
                }

                info!(
                    "add embed message: version_id = {} url = {:?}",
                    mref.version_id,
                    embed.url.as_ref().map(|u| u.as_str())
                );

                m.embeds.push(embed);
                (
                    m.embeds.clone(),
                    m.attachments
                        .iter()
                        .filter_map(|a| match &a.ty {
                            common::v2::types::message::MessageAttachmentType::Media { media } => {
                                Some(media.id)
                            }
                        })
                        .collect(),
                )
            }
            _ => return Ok(()),
        };

        data.message_update_in_place(
            mref.thread_id,
            mref.version_id,
            DbMessageUpdate {
                attachment_ids: attachments,
                author_id: message.author_id,
                embeds: embeds.into_iter().map(|e| e.into()).collect(),
                message_type,
                created_at: Some(message.latest_version.created_at.into()),
                mentions: message.latest_version.mentions,
            },
        )
        .await?;

        let mut message = data
            .message_get(mref.thread_id, mref.message_id, user_id)
            .await?;
        if message.latest_version.version_id == mref.version_id {
            state.presign_message(&mut message).await?;
            state
                .broadcast_channel(
                    mref.thread_id,
                    user_id,
                    MessageSync::MessageUpdate { message },
                )
                .await?;
        } else {
            info!("not sending update because message is not latest");
        }
        Ok(())
    }
}

/// In-memory log sink that collects log entries for debug responses
#[derive(Debug, Default, Clone)]
pub struct DebugLogSink {
    entries: Vec<lamprey_unfurl::logging::LogEntry>,
}

impl DebugLogSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn into_entries(self) -> Vec<lamprey_unfurl::logging::LogEntry> {
        self.entries
    }
}

impl lamprey_unfurl::logging::LogSink for DebugLogSink {
    fn handle(&mut self, entry: lamprey_unfurl::logging::LogEntry) {
        self.entries.push(entry);
    }
}

/// Tracing log sink that logs to the tracing subsystem
pub struct TracingLogSink;

impl lamprey_unfurl::logging::LogSink for TracingLogSink {
    fn handle(&mut self, entry: lamprey_unfurl::logging::LogEntry) {
        match entry {
            lamprey_unfurl::logging::LogEntry::SelectPlugin(entry) => {
                tracing::debug!(
                    "Selected plugin: {} via {:?}",
                    entry.plugin_name,
                    entry.reason
                );
            }
            lamprey_unfurl::logging::LogEntry::Fetch(fetch) => {
                tracing::debug!("HTTP fetch: {:?} {}", fetch.reason, fetch.http_status);
            }
            lamprey_unfurl::logging::LogEntry::Error(err) => {
                tracing::warn!("Unfurl error: {:?} - {}", err.code, err.message);
            }
            lamprey_unfurl::logging::LogEntry::Failed(failed) => {
                tracing::error!("Unfurl failed: {:?} - {}", failed.code, failed.message);
            }
        }
    }
}
