use std::{sync::Arc, time::Duration};

use common::v1::types::{AuditLogEntry, Channel, Message, MessageSync, Room, User};
use common::v2::types::media::Media;
use tantivy::Term;
use tokio_stream::StreamExt;
use tracing::{error, info};

use crate::globals::messaging::Broadcast;
use crate::prelude::*;
use crate::services::search::{index::AsyncIndexHandle, util::SCHEMA};

pub struct LiveEtl {
    s: Globals,
    index: AsyncIndexHandle,
}

impl LiveEtl {
    pub fn new(s: Globals, index: AsyncIndexHandle) -> Self {
        Self { s, index }
    }

    fn srv(&self) -> Arc<Services> {
        self.s.services()
    }

    pub async fn spawn(self) {
        loop {
            match self.s.messaging().subscribe().await {
                Ok(mut stream) => {
                    info!("Search ingestion: connected to live stream");
                    while let Some(broadcast) = stream.next().await {
                        if let Broadcast::Sync(sync) = broadcast {
                            if let Err(err) = self.handle_sync(sync.message).await {
                                error!("error while handling search index sync: {err}");
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Search ingestion: sushi stream failed: {e}. Retrying in 5s...");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn handle_sync(&self, sync: MessageSync) -> Result<()> {
        match sync {
            MessageSync::MessageCreate { message } => self.index_message(message).await?,
            MessageSync::MessageUpdate { message } => self.index_message(message).await?,
            MessageSync::MessageDelete { message_id, .. } => {
                let term = Term::from_field_text(SCHEMA.id, &message_id.to_string());
                self.index.delete_term(term).await?;
            }
            MessageSync::ChannelCreate { channel } => self.index_channel(*channel).await?,
            MessageSync::ChannelUpdate { channel } => self.index_channel(*channel).await?,
            MessageSync::RoomCreate { room } => self.index_room(room).await?,
            MessageSync::RoomUpdate { room } => self.index_room(room).await?,
            MessageSync::UserCreate { user } => self.index_user(user).await?,
            MessageSync::UserUpdate { user } => self.index_user(user).await?,
            MessageSync::RoomDelete { room_id } => {
                let term = Term::from_field_text(SCHEMA.id, &room_id.to_string());
                self.index.delete_term(term).await?;
            }
            MessageSync::UserDelete { id } => {
                let term = Term::from_field_text(SCHEMA.id, &id.to_string());
                self.index.delete_term(term).await?;
            }
            MessageSync::AuditLogEntryCreate { entry } => self.index_audit_log(entry).await?,
            MessageSync::MediaProcessed { media, .. } => self.index_media(media).await?,
            MessageSync::MediaUpdate { media } => self.index_media(media).await?,
            _ => {}
        }

        Ok(())
    }

    async fn index_message(&self, message: Message) -> Result<()> {
        let srv = self.srv();
        let chan = srv.channels.get(message.channel_id, None).await?;
        let term = Term::from_field_text(SCHEMA.id, &message.id.to_string());
        let doc = SCHEMA.transform_message(&message, chan.room_id, chan.parent_id)?;
        self.index.update_document(term, doc).await?;
        Ok(())
    }

    async fn index_channel(&self, channel: Channel) -> Result<()> {
        let first_message = self.srv().messages.get_first(channel.id, None).await.ok();
        let term = Term::from_field_text(SCHEMA.id, &channel.id.to_string());
        let doc = SCHEMA.transform_channel(&channel, first_message.as_ref())?;
        self.index.update_document(term, doc).await?;
        Ok(())
    }

    async fn index_room(&self, room: Room) -> Result<()> {
        let term = Term::from_field_text(SCHEMA.id, &room.id.to_string());
        let doc = SCHEMA.transform_room(&room)?;
        self.index.update_document(term, doc).await?;
        Ok(())
    }

    async fn index_user(&self, user: User) -> Result<()> {
        let term = Term::from_field_text(SCHEMA.id, &user.id.to_string());
        let doc = SCHEMA.transform_user(&user)?;
        self.index.update_document(term, doc).await?;
        Ok(())
    }

    async fn index_media(&self, media: Media) -> Result<()> {
        let term = Term::from_field_text(SCHEMA.id, &media.id.to_string());
        let doc = SCHEMA.transform_media(&media)?;
        self.index.update_document(term, doc).await?;
        Ok(())
    }

    async fn index_audit_log(&self, entry: AuditLogEntry) -> Result<()> {
        let term = Term::from_field_text(SCHEMA.id, &entry.id.to_string());
        let doc = SCHEMA.transform_audit_log_entry(&entry)?;
        self.index.update_document(term, doc).await?;
        Ok(())
    }
}
