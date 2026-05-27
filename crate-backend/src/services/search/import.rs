use std::{sync::Arc, time::Duration};

use common::{
    v1::types::{ChannelId, MediaVerId, MessageSync, PaginationDirection, PaginationQuery},
    v2::types::media::Media,
};
use dashmap::DashSet;
use moka::future::Cache;
use tantivy::Term;
use tokio::task::JoinSet;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    services::search::{
        index::{CommitIndex, DeleteTerm, IndexActorRef, UpdateDocument, UpdateDocuments},
        schema::{
            tantivy_document_from_channel, tantivy_document_from_media,
            tantivy_document_from_message, unified::UnifiedSchema,
        },
    },
    Result, ServerStateInner,
};

#[derive(Hash, Eq, PartialEq, Clone)]
pub enum IngestKey {
    Message(Uuid),
    Channel(Uuid),
    Media(Uuid),
}

/// importer for the content index
pub struct ContentIngestionManager {
    s: Arc<ServerStateInner>,
    index_writer: IndexActorRef,
    schema: UnifiedSchema,
    active_channels: Arc<DashSet<ChannelId>>,
    update_throttle: Cache<IngestKey, ()>,
}

// /// importer for the content index
// pub struct ContentIngestionManager2 {
//     s: Arc<ServerStateInner>,
//     index_writer: IndexActorRef,
//     schema: ContentSchema,
//     active_channels: Arc<DashSet<ChannelId>>,
//     update_throttle: Cache<String, ()>,
// }

impl ContentIngestionManager {
    pub async fn start(s: Arc<ServerStateInner>, index_writer: IndexActorRef) -> Result<()> {
        let manager = Arc::new(Self {
            s: s.clone(),
            index_writer,
            schema: UnifiedSchema::default(),
            active_channels: Arc::new(DashSet::new()),
            update_throttle: Cache::builder()
                .time_to_live(Duration::from_secs(5))
                .build(),
        });

        Arc::clone(&manager).spawn_live_listener();
        Arc::clone(&manager).spawn_backfill_poller();
        Arc::clone(&manager).spawn_media_backfill_poller();

        Ok(())
    }

    fn spawn_live_listener(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                match self.s.subscribe_sushi().await {
                    Ok(mut stream) => {
                        info!("Search ingestion: connected to live stream");
                        while let Some(broadcast) = stream.next().await {
                            match broadcast.message {
                                MessageSync::MessageCreate { message } => {
                                    self.index_message(message, false).await;
                                }
                                MessageSync::MessageUpdate { message } => {
                                    self.index_message(message, true).await;
                                }
                                MessageSync::MessageDelete { message_id, .. } => {
                                    let term = Term::from_field_text(
                                        self.schema.id,
                                        &message_id.to_string(),
                                    );
                                    let _ = self.index_writer.tell(DeleteTerm(term)).await;
                                }
                                MessageSync::ChannelCreate { channel } => {
                                    self.index_channel(*channel, false).await;
                                }
                                MessageSync::ChannelUpdate { channel } => {
                                    self.index_channel(*channel, true).await;
                                }
                                MessageSync::MediaProcessed { media, .. } => {
                                    self.index_media(media, false).await;
                                }
                                MessageSync::MediaUpdate { media } => {
                                    self.index_media(media, true).await;
                                }
                                _ => continue,
                            }
                        }
                    }
                    Err(e) => {
                        error!("Search ingestion: sushi stream failed: {e}. Retrying in 5s...");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });
    }

    fn spawn_backfill_poller(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            let mut workers = JoinSet::new();

            loop {
                interval.tick().await;

                // clean up completed workers
                while let Some(res) = workers.try_join_next() {
                    if let Err(e) = res {
                        error!("Backfill worker panicked: {e}");
                    }
                }

                let concurrency_limit = 4i32; // TODO: allow configuring this
                let available_slots = concurrency_limit.saturating_sub(workers.len() as i32);
                if available_slots <= 0 {
                    continue;
                }

                let mut data = self.s.data();
                let queue = match data
                    .search_reindex_queue_list("channel", available_slots as u32)
                    .await
                {
                    Ok(q) => q,
                    Err(e) => {
                        error!("Failed to list reindex queue: {e}");
                        continue;
                    }
                };

                let srv = self.s.services();
                for (target_id, last_id) in queue {
                    let channel_id = ChannelId::from(target_id);
                    if self.active_channels.contains(&channel_id) {
                        continue;
                    }

                    self.active_channels.insert(channel_id);
                    let this = self.clone();
                    let srv = srv.clone();

                    workers.spawn(async move {
                        if let Ok(chan) = srv.channels.get(channel_id, None).await {
                            this.index_channel(chan, false).await;
                        }

                        if let Err(e) = this
                            .index_single_channel_messages(channel_id, last_id)
                            .await
                        {
                            error!(channel_id = ?channel_id, "Reindex task failed: {e}");
                        }
                        this.active_channels.remove(&channel_id);
                    });
                }
            }
        });
    }

    fn spawn_media_backfill_poller(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut data = self.s.data();
            let mut last_version_id: Option<MediaVerId> = data
                .search_reindex_queue_get("media", Uuid::nil())
                .await
                .ok()
                .flatten()
                .map(|id| id.into());

            loop {
                let mut data = self.s.data();
                // TODO: there may be a race condition between MessageSync and data listing; verify that this isn't a problem
                match data.media_list_indexed(last_version_id, 100).await {
                    Ok(media_list) => {
                        if media_list.is_empty() {
                            tokio::time::sleep(Duration::from_secs(10)).await;
                            continue;
                        }

                        for media in media_list {
                            last_version_id = Some(media.version_id);
                            self.index_media(media, false).await;
                        }

                        if let Some(vid) = last_version_id {
                            let _ = data
                                .search_reindex_queue_upsert("media", Uuid::nil(), Some(*vid))
                                .await;
                        }

                        // Commit every batch
                        let _ = self.index_writer.tell(CommitIndex).await;
                    }
                    Err(e) => {
                        error!("Media backfill failed: {e}. Retrying in 10s...");
                        tokio::time::sleep(Duration::from_secs(10)).await;
                    }
                }
                tokio::task::yield_now().await;
            }
        });
    }

    async fn index_single_channel_messages(
        &self,
        channel_id: ChannelId,
        mut last_id: Option<Uuid>,
    ) -> Result<()> {
        let mut data = self.s.data();
        let srv = self.s.services();

        info!(channel_id = ?channel_id, "Starting channel backfill");

        loop {
            let messages = srv
                .messages
                .list(
                    channel_id,
                    None,
                    PaginationQuery {
                        from: last_id.map(|id| id.into()),
                        to: None,
                        dir: Some(PaginationDirection::B),
                        limit: Some(500), // TODO: extract this into a const (configurable?)
                    },
                )
                .await?;

            if messages.items.is_empty() {
                data.search_reindex_queue_delete("channel", *channel_id)
                    .await?;
                // TODO: handle error
                let _ = self.index_writer.tell(CommitIndex).await;
                break;
            }

            let last_processed_id = messages.items.last().unwrap().id;
            let chan = match srv.channels.get(channel_id, None).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Could not get channel for backfill: {}", e);
                    let _ = data
                        .search_ingestion_dlq_insert(*channel_id, "channel", &e.to_string())
                        .await;
                    let _ = data
                        .search_reindex_queue_delete("channel", *channel_id)
                        .await;
                    break;
                }
            };

            let mut batch = Vec::with_capacity(messages.items.len());
            for msg in messages.items {
                let key = IngestKey::Message(*msg.id);
                if self.update_throttle.get(&key).await.is_some() {
                    continue;
                }

                let term = Term::from_field_text(self.schema.id, &msg.id.to_string());
                let doc =
                    tantivy_document_from_message(&self.schema, msg, chan.room_id, chan.parent_id);
                batch.push((term, doc));
            }

            if !batch.is_empty() {
                let _ = self.index_writer.tell(UpdateDocuments(batch)).await;
            }

            data.search_reindex_queue_upsert("channel", *channel_id, Some(*last_processed_id))
                .await?;
            last_id = Some(last_processed_id.into());

            if !messages.has_more {
                data.search_reindex_queue_delete("channel", *channel_id)
                    .await?;
                let _ = self.index_writer.tell(CommitIndex).await;
                break;
            }

            // avoid blocking the executor for too long
            tokio::task::yield_now().await;
        }

        info!(channel_id = ?channel_id, "Finished channel backfill");
        Ok(())
    }

    async fn index_message(&self, message: common::v1::types::Message, is_update: bool) {
        if is_update {
            let key = IngestKey::Message(*message.id);
            if self.update_throttle.get(&key).await.is_some() {
                return;
            }
            self.update_throttle.insert(key, ()).await;
        }

        let srv = self.s.services();
        // TODO: error handling instead of unwrap
        if let Ok(chan) = srv.channels.get(message.channel_id, None).await {
            let term = Term::from_field_text(self.schema.id, &message.id.to_string());
            let doc = tantivy_document_from_message(
                &self.schema,
                message.clone(),
                message.room_id,
                chan.parent_id,
            );

            let _ = self.index_writer.tell(UpdateDocument { term, doc }).await;
        }
    }

    async fn index_channel(&self, channel: common::v1::types::Channel, is_update: bool) {
        if is_update {
            let key = IngestKey::Channel(*channel.id);
            if self.update_throttle.get(&key).await.is_some() {
                return;
            }
            self.update_throttle.insert(key, ()).await;
        }

        let doc = tantivy_document_from_channel(&self.schema, channel.clone());
        let term = Term::from_field_text(self.schema.id, &channel.id.to_string());
        let _ = self.index_writer.tell(UpdateDocument { term, doc }).await;
    }

    async fn index_media(&self, media: Media, is_update: bool) {
        if is_update {
            let key = IngestKey::Media(*media.id);
            if self.update_throttle.get(&key).await.is_some() {
                return;
            }
            self.update_throttle.insert(key, ()).await;
        }

        if let Some(doc) = tantivy_document_from_media(&self.schema, media.clone()) {
            let term = Term::from_field_text(self.schema.id, &media.id.to_string());
            let _ = self.index_writer.tell(UpdateDocument { term, doc }).await;
        } else {
            warn!(media_id = ?media.id, "Skipping media without user_id");
        }
    }
}
