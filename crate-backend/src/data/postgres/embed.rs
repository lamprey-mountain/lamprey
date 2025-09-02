use async_trait::async_trait;
use common::v1::types::{Embed, UserId};
use sqlx::{query, query_as};

use uuid::Uuid;

use crate::data::postgres::Postgres;
use crate::data::DataEmbed;
use crate::types::UrlEmbedQueue;
use crate::Result;

#[async_trait]
impl DataEmbed for Postgres {
    async fn url_embed_queue_insert(
        &self,
        message_ref: Option<crate::types::MessageRef>,
        user_id: UserId,
        url: String,
    ) -> Result<Uuid> {
        let id = Uuid::now_v7();
        query!(
            "INSERT INTO url_embed_queue (id, message_ref, user_id, url) VALUES ($1, $2, $3, $4)",
            id,
            message_ref.map(|m| serde_json::to_value(m).unwrap()),
            *user_id,
            url
        )
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    async fn url_embed_queue_claim(&self) -> Result<Option<UrlEmbedQueue>> {
        let row = query_as!(UrlEmbedQueue, "UPDATE url_embed_queue SET claimed_at = NOW() WHERE id = (SELECT id FROM url_embed_queue WHERE claimed_at IS NULL AND finished_at IS NULL ORDER BY created_at ASC LIMIT 1 FOR UPDATE SKIP LOCKED) RETURNING *")
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn url_embed_queue_finish(&self, id: Uuid, embed: Option<&Embed>) -> Result<()> {
        query!(
            "UPDATE url_embed_queue SET finished_at = NOW() WHERE id = $1",
            id
        )
        .execute(&self.pool)
        .await?;
        if let Some(embed) = embed {
            query!(
                "UPDATE message SET embeds = embeds || $1::jsonb WHERE version_id = (SELECT (message_ref->>'version_id')::uuid FROM url_embed_queue WHERE id = $2)",
                serde_json::to_value(embed)?,
                id
            )
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }
}
