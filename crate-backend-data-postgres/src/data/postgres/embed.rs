use async_trait::async_trait;
use common::v1::types::UserId;
use common::v2::types::embed::Embed;
use sqlx::{query, query_as};

use uuid::Uuid;

use crate::data::postgres::Postgres;
use crate::data::DataEmbed;
use crate::types::UrlEmbedQueue;
use crate::Result;

#[async_trait]
impl DataEmbed for Postgres {
    async fn url_embed_queue_insert(
        &mut self,
        message_ref: Option<crate::types::MessageRef>,
        user_id: Option<UserId>,
        url: String,
    ) -> Result<Uuid> {
        let mut conn = self.acquire().await?;
        let id = Uuid::now_v7();
        query!(
            "INSERT INTO url_embed_queue (id, message_ref, user_id, url) VALUES ($1, $2, $3, $4)",
            id,
            message_ref.map(|m| serde_json::to_value(m).unwrap()),
            user_id.map(|u| *u),
            url
        )
        .execute(conn.ext())
        .await?;
        Ok(id)
    }

    async fn url_embed_queue_claim(&mut self) -> Result<Option<UrlEmbedQueue>> {
        let mut conn = self.acquire().await?;
        let row = query_as!(UrlEmbedQueue, "UPDATE url_embed_queue SET claimed_at = NOW() WHERE id = (SELECT id FROM url_embed_queue WHERE claimed_at IS NULL AND finished_at IS NULL ORDER BY created_at ASC LIMIT 1 FOR UPDATE SKIP LOCKED) RETURNING *")
        .fetch_optional(conn.ext())
        .await?;
        Ok(row)
    }

    async fn url_embed_queue_finish(&mut self, id: Uuid, _embed: Option<&Embed>) -> Result<()> {
        let mut conn = self.acquire().await?;
        query!(
            "UPDATE url_embed_queue SET finished_at = NOW() WHERE id = $1",
            id
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }
}
