use crate::services::search::schema::Doctype;
use crate::services::search::util::SCHEMA;
use crate::services::search::ServiceSearch;
use crate::Result;
use common::v1::types::{ChannelId, RoomId};
use lamprey_backend_core::types::admin::{
    SearchIndexStats, SearchIndexStatsChannel, SearchIndexStatsRoom,
};
use tantivy::Term;

impl ServiceSearch {
    /// get search index stats for a room
    pub async fn get_room_stats(&self, _room_id: RoomId) -> Result<SearchIndexStatsRoom> {
        todo!()
    }

    /// get search index stats for a channel
    pub async fn get_channel_stats(
        &self,
        channel_id: ChannelId,
    ) -> Result<SearchIndexStatsChannel> {
        todo!()
    }

    /// get search index stats for the overall index
    pub async fn get_stats(&self) -> Result<SearchIndexStats> {
        let index = self.get_index().await?;
        let searcher = index.searcher().await?;

        let count_documents = searcher.num_docs().await?;
        let count_messages = searcher
            .doc_freq(&Term::from_field_text(
                SCHEMA.doctype,
                Doctype::Message.as_str(),
            ))
            .await?;

        let index_size_bytes = searcher.space_usage().await?.total().get_bytes();

        // let mut data = self.state.data();
        // let (document_count, index_size_bytes) =
        //     tokio::task::spawn_blocking(move || searcher.get_index_stats())
        //         .await
        //         .map_err(|e| Error::Internal(format!("Search task failed: {}", e)))?
        //         .map_err(|e| Error::Internal(format!("Failed to get index stats: {}", e)))?;

        // let backfill_queue_size =
        //     data.search_reindex_queue_list("channel", 1000).await?.len() as u64;

        Ok(SearchIndexStats {
            count_documents,
            count_messages,
            count_channels: todo!(),
            count_rooms: todo!(),
            count_media: todo!(),
            count_users: todo!(),
            index_size_bytes,
            reindex_queues: todo!(),
        })
    }
}
