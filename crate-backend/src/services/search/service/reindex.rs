use crate::services::search::{index, ServiceSearch};
use crate::Result;
use common::v1::types::{ChannelId, RoomId};

impl ServiceSearch {
    pub async fn reindex_channel(&self, channel_id: ChannelId) -> Result<()> {
        let mut data = self.state.data();

        if let Some(index_actor) = self.index_manager.get_index_actor("content") {
            let delete_term = index::delete_term_for_channel(channel_id);
            if let Err(e) = index_actor.tell(delete_term).await {
                tracing::warn!(
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

    pub async fn reindex_room(&self, room_id: RoomId) -> Result<()> {
        let mut data = self.state.data();

        if let Some(index_actor) = self.index_manager.get_index_actor("content") {
            let delete_term = index::delete_term_for_room(room_id);
            if let Err(e) = index_actor.tell(delete_term).await {
                tracing::warn!("failed to delete documents for room {}: {}", room_id, e);
            }
        }

        data.search_reindex_queue_upsert_room(room_id).await?;
        Ok(())
    }

    pub async fn reindex_everything(&self) -> Result<()> {
        if let Some(index_actor) = self.index_manager.get_index_actor("content") {
            if let Err(e) = index_actor.tell(index::DeleteAllDocuments).await {
                tracing::warn!("failed to delete all documents: {}", e);
            }
        }

        let mut data = self.state.data();
        data.search_reindex_queue_upsert_all().await?;
        Ok(())
    }
}
