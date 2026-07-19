use crate::services::search::ServiceSearch;
use crate::services::search::index::glue::TantivyMessage;
use crate::services::search::schema::Doctype;
use crate::services::search::util::SCHEMA;
use crate::{Result, services::search::util::BqBuilder};
use common::v1::types::{ChannelId, RoomId};
use lamprey_backend_core::types::admin::{
    SearchIndexStats, SearchIndexStatsChannel, SearchIndexStatsRoom,
};
use tantivy::Term;
use tantivy::collector::{Count, TopDocs};

impl ServiceSearch {
    /// get search index stats for a room
    pub async fn get_room_stats(&self, room_id: RoomId) -> Result<SearchIndexStatsRoom> {
        let index = self.get_index().await?;
        let searcher = index.searcher().await?;

        let mut bq_docs = BqBuilder::new();
        bq_docs.must(SCHEMA.query_room_id(room_id));
        let count_documents = searcher.search(&bq_docs.build(), &Count).await? as u64;

        let mut bq_channels = BqBuilder::new();
        bq_channels.must(SCHEMA.query_room_id(room_id));
        bq_channels.must(SCHEMA.query_doctype(Doctype::Channel));
        let count_channels = searcher.search(&bq_channels.build(), &Count).await? as u64;

        let mut bq_msg = BqBuilder::new();
        bq_msg.must(SCHEMA.query_room_id(room_id));
        bq_msg.must(SCHEMA.query_doctype(Doctype::Message));
        let count_messages = searcher.search(&bq_msg.build(), &Count).await? as u64;

        let mut bq_media = BqBuilder::new();
        bq_media.must(SCHEMA.query_room_id(room_id));
        bq_media.must(SCHEMA.query_doctype(Doctype::Media));
        let count_media = searcher.search(&bq_media.build(), &Count).await? as u64;

        Ok(SearchIndexStatsRoom {
            room_id,
            count_documents,
            count_channels,
            count_messages,
            count_media,
        })
    }

    /// get search index stats for a channel
    pub async fn get_channel_stats(
        &self,
        channel_id: ChannelId,
    ) -> Result<SearchIndexStatsChannel> {
        let index = self.get_index().await?;
        let searcher = index.searcher().await?;

        let mut bq_docs = BqBuilder::new();
        bq_docs.must(SCHEMA.query_channel_id(channel_id));
        let count_documents = searcher.search(&bq_docs.build(), &Count).await? as u64;

        let mut bq_msg = BqBuilder::new();
        bq_msg.must(SCHEMA.query_channel_id(channel_id));
        bq_msg.must(SCHEMA.query_doctype(Doctype::Message));
        let query_msg = bq_msg.build();
        let count_messages = searcher.search(&query_msg, &Count).await? as u64;

        let mut bq_media = BqBuilder::new();
        bq_media.must(SCHEMA.query_channel_id(channel_id));
        bq_media.must(SCHEMA.query_doctype(Doctype::Media));
        let count_media = searcher.search(&bq_media.build(), &Count).await? as u64;

        let top_docs = TopDocs::with_limit(1)
            .order_by_fast_field::<tantivy::DateTime>("created_at", tantivy::Order::Desc);
        let docs: Vec<(Option<tantivy::DateTime>, tantivy::DocAddress)> =
            searcher.search(&query_msg, &top_docs).await?;

        let last_indexed_message_id = if let Some((_, doc_addr)) = docs.first() {
            let doc: TantivyMessage = searcher.doc(*doc_addr).await?;
            Some(doc.id)
        } else {
            None
        };

        Ok(SearchIndexStatsChannel {
            channel_id,
            count_documents,
            count_messages,
            count_media,
            last_indexed_message_id,
        })
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
        let count_channels = searcher
            .doc_freq(&Term::from_field_text(
                SCHEMA.doctype,
                Doctype::Channel.as_str(),
            ))
            .await?;
        let count_rooms = searcher
            .doc_freq(&Term::from_field_text(
                SCHEMA.doctype,
                Doctype::Room.as_str(),
            ))
            .await?;
        let count_media = searcher
            .doc_freq(&Term::from_field_text(
                SCHEMA.doctype,
                Doctype::Media.as_str(),
            ))
            .await?;
        let count_users = searcher
            .doc_freq(&Term::from_field_text(
                SCHEMA.doctype,
                Doctype::User.as_str(),
            ))
            .await?;

        let index_size_bytes = searcher.space_usage().await?.total().get_bytes();

        Ok(SearchIndexStats {
            count_documents,
            count_messages,
            count_channels,
            count_rooms,
            count_media,
            count_users,
            index_size_bytes,
        })
    }
}
