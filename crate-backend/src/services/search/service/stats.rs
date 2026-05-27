use crate::error::Error;
use crate::services::search::ServiceSearch;
use crate::Result;
use common::v1::types::{ChannelId, RoomId};
use lamprey_backend_core::types::admin::SearchIndexStats;

impl ServiceSearch {
    pub async fn get_room_stats(&self, room_id: RoomId) -> Result<SearchIndexStats> {
        todo!()
    }

    pub async fn get_channel_stats(&self, channel_id: ChannelId) -> Result<SearchIndexStats> {
        let mut data = self.state.data();
        let searcher = self.get_content_searcher().await?;

        let documents_indexed =
            tokio::task::spawn_blocking(move || searcher.count_documents_for_channel(channel_id))
                .await
                .map_err(|e| Error::Internal(format!("Search task failed: {}", e)))?
                .map_err(|e| Error::Internal(format!("Failed to count documents: {}", e)))?;

        let last_message_id = data
            .search_reindex_queue_get("channel", *channel_id)
            .await?
            .map(Into::into);

        Ok(SearchIndexStats {
            documents_indexed,
            last_message_id,
        })
    }

    pub async fn get_overall_stats(
        &self,
    ) -> Result<lamprey_backend_core::types::admin::SearchStats> {
        let searcher = self.get_content_searcher().await?;
        let mut data = self.state.data();

        let (document_count, index_size_bytes) =
            tokio::task::spawn_blocking(move || searcher.get_index_stats())
                .await
                .map_err(|e| crate::Error::Internal(format!("Search task failed: {}", e)))?
                .map_err(|e| crate::Error::Internal(format!("Failed to get index stats: {}", e)))?;

        let backfill_queue_size =
            data.search_reindex_queue_list("channel", 1000).await?.len() as u64;

        Ok(lamprey_backend_core::types::admin::SearchStats {
            document_count,
            index_size_bytes,
            backfill_queue_size,
        })
    }
}
