use crate::services::search::{index, ServiceSearch};
use crate::Result;
use common::v1::types::{ChannelId, MediaId, RoomId, UserId};
use tracing::warn;

impl ServiceSearch {
    /// reindex everything in a channel
    ///
    /// includes the channel itself and messages
    // TODO: reindex channel's threads
    pub async fn reindex_channel(&self, channel_id: ChannelId) -> Result<()> {
        let mut data = self.state.data();

        if let Some(index_actor) = self.index_manager.get_index_actor("content") {
            let delete_term = index::delete_term_for_channel(channel_id);
            if let Err(e) = index_actor.tell(delete_term).await {
                warn!(
                    "failed to delete documents for channel {}: {}",
                    channel_id,
                    e
                );
            }
        }

        data.search_reindex_queue_upsert("channel", *channel_id, None)
            .await?;
        Ok(())
    }

    /// reindex everything in a channel
    ///
    /// includes the room itself. reindexes all of the room's channels.
    // TODO: reindex room's audit log entries
    pub async fn reindex_room(&self, room_id: RoomId) -> Result<()> {
        let mut data = self.state.data();

        if let Some(index_actor) = self.index_manager.get_index_actor("content") {
            let delete_term = index::delete_term_for_room(room_id);
            if let Err(e) = index_actor.tell(delete_term).await {
                warn!("failed to delete documents for room {}: {}", room_id, e);
            }
        }

        data.search_reindex_queue_upsert_room(room_id).await?;
        Ok(())
    }

    /// reindex a user
    pub async fn reindex_user(&self, user_id: UserId) -> Result<()> {
        let mut data = self.state.data();

        if let Some(index_actor) = self.index_manager.get_index_actor("content") {
            let delete_term = index::delete_term_for_user(user_id);
            if let Err(e) = index_actor.tell(delete_term).await {
                warn!("failed to delete documents for user {}: {}", user_id, e);
            }
        }

        data.search_reindex_queue_upsert("user", *user_id, None)
            .await?;
        Ok(())
    }

    /// reindex a piece of media
    pub async fn reindex_media(&self, media_id: MediaId) -> Result<()> {
        let mut data = self.state.data();

        if let Some(index_actor) = self.index_manager.get_index_actor("content") {
            let delete_term = index::delete_term_for_media(media_id);
            if let Err(e) = index_actor.tell(delete_term).await {
                warn!("failed to delete documents for media {}: {}", media_id, e);
            }
        }

        data.search_reindex_queue_upsert("media", *media_id, None)
            .await?;
        Ok(())
    }

    /// reindex **everything** on the server
    ///
    /// this is a **very** heavy operation and should not be done often (or at all, ideally)
    pub async fn reindex_everything(&self) -> Result<()> {
        if let Some(index_actor) = self.index_manager.get_index_actor("content") {
            if let Err(e) = index_actor.tell(index::DeleteAllDocuments).await {
                warn!("failed to delete all documents: {}", e);
            }
        }

        let mut data = self.state.data();
        data.search_reindex_queue_upsert_all().await?;
        Ok(())
    }
}
