use std::{sync::Arc, time::Duration};

use common::v1::types::{ChannelId, MessageSync, PaginationDirection, PaginationQuery};
use dashmap::DashSet;
use kameo::{actor::Spawn, Actor};
use tantivy::Term;
use tokio::task::JoinSet;
use tokio_stream::StreamExt;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    services::search::{
        index::{AddDocument, CommitIndex, DeleteTerm, IndexActorRef, UpdateDocument},
        schema::{content::ContentSchema, tantivy_document_from_message},
    },
    Result, ServerStateInner,
};

/// importer for the content index
pub struct ContentIngestionManager {
    s: Arc<ServerStateInner>,
    index_writer: IndexActorRef,
    schema: ContentSchema,
    active_channels: Arc<DashSet<ChannelId>>,
}

impl ContentIngestionManager {
    pub async fn start(s: Arc<ServerStateInner>, index_writer: IndexActorRef) -> Result<()> {
        let manager = Arc::new(Self {
            s: s.clone(),
            index_writer,
            schema: ContentSchema::default(),
            active_channels: Arc::new(DashSet::new()),
        });

        Arc::clone(&manager).spawn_live_listener();
        Arc::clone(&manager).spawn_backfill_poller();

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
                                // TODO: handle delete bulk
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
                if available_slots == 0 {
                    continue;
                }

                let data = self.s.data();
                let queue = match data.search_reindex_queue_list(available_slots as u32).await {
                    Ok(q) => q,
                    Err(e) => {
                        error!("Failed to list reindex queue: {e}");
                        continue;
                    }
                };

                for (channel_id, last_id) in queue {
                    if self.active_channels.contains(&channel_id) {
                        continue;
                    }

                    self.active_channels.insert(channel_id);
                    let this = self.clone();

                    workers.spawn(async move {
                        if let Err(e) = this
                            .index_single_channel(channel_id, last_id.map(|i| *i))
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

    async fn index_single_channel(
        &self,
        channel_id: ChannelId,
        mut last_id: Option<Uuid>,
    ) -> Result<()> {
        let data = self.s.data();
        let srv = self.s.services();

        info!(channel_id = ?channel_id, "Starting channel backfill");

        loop {
            let messages = srv
                .messages
                .list(
                    channel_id,
                    None,
                    PaginationQuery {
                        from: last_id
                            .map(|id| id.into())
                            .or_else(|| Some(Uuid::nil().into())),
                        to: None,
                        dir: Some(PaginationDirection::F),
                        limit: Some(500), // TODO: extract this into a const (configurable?)
                    },
                )
                .await?;

            if messages.items.is_empty() {
                data.search_reindex_queue_delete(channel_id).await?;
                // TODO: handle error
                let _ = self.index_writer.tell(CommitIndex).await;
                break;
            }

            let last_processed_id = messages.items.last().unwrap().id;
            let chan = srv.channels.get(channel_id, None).await?;

            for msg in messages.items {
                // FIXME: live listener handles MessageUpdate before we handle an old message
                let doc =
                    tantivy_document_from_message(&self.schema, msg, chan.room_id, chan.parent_id);
                let _ = self.index_writer.tell(AddDocument(doc)).await;
            }

            data.search_reindex_queue_upsert(channel_id, Some(last_processed_id.into()))
                .await?;
            last_id = Some(last_processed_id.into());

            if !messages.has_more {
                data.search_reindex_queue_delete(channel_id).await?;
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
        let srv = self.s.services();
        // TODO: error handling instead of unwrap
        let chan = srv.channels.get(message.channel_id, None).await.unwrap();
        let doc = tantivy_document_from_message(
            &self.schema,
            message.clone(),
            message.room_id,
            chan.parent_id,
        );

        if is_update {
            let term = Term::from_field_text(self.schema.id, &message.id.to_string());
            let _ = self.index_writer.tell(UpdateDocument { term, doc }).await;
        } else {
            let _ = self.index_writer.tell(AddDocument(doc)).await;
        }
    }
}
