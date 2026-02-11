use async_trait::async_trait;
use sqlx::query;

use crate::{
    data::DataSearchQueue,
    error::Result,
    types::{ChannelId, MessageId},
};

use super::Postgres;

#[async_trait]
impl DataSearchQueue for Postgres {
    async fn search_reindex_queue_upsert(
        &self,
        channel_id: ChannelId,
        last_message_id: Option<MessageId>,
    ) -> Result<()> {
        query!(
            "INSERT INTO search_reindex_queue (channel_id, last_message_id) VALUES ($1, $2) ON CONFLICT (channel_id) DO UPDATE SET last_message_id = $2, updated_at = NOW()",
            *channel_id,
            last_message_id.map(|id| *id),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn search_reindex_queue_list(
        &self,
        limit: u32,
    ) -> Result<Vec<(ChannelId, Option<MessageId>)>> {
        let rows = query!(
            r#"SELECT channel_id, last_message_id FROM search_reindex_queue ORDER BY updated_at ASC LIMIT $1"#,
            limit as i64
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.channel_id.into(), r.last_message_id.map(|id| id.into())))
            .collect())
    }

    async fn search_reindex_queue_delete(&self, channel_id: ChannelId) -> Result<()> {
        query!(
            "DELETE FROM search_reindex_queue WHERE channel_id = $1",
            *channel_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}