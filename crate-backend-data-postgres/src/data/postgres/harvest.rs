use crate::Postgres;
use crate::data::DataHarvest;
use crate::error::Result;
use async_trait::async_trait;
use common::v1::types::harvest::{Harvest, HarvestType};
use common::v1::types::{HarvestId, RoomId, UserId};
use sqlx::{query, query_scalar};

#[async_trait]
impl DataHarvest for Postgres {
    async fn harvest_put(&mut self, harvest: &Harvest) -> Result<()> {
        let target_id = match &harvest.ty {
            HarvestType::User { target_user_id, .. } => **target_user_id,
            HarvestType::Room { target_room_id, .. } => **target_room_id,
        };

        let queued_at = harvest.queued_at;
        let queued_at = time::PrimitiveDateTime::new(queued_at.date(), queued_at.time());

        let mut conn = self.acquire().await?;
        query!(
            r#"
            INSERT INTO harvest (id, target_id, queued_at, data)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO UPDATE SET data = EXCLUDED.data, queued_at = EXCLUDED.queued_at
            "#,
            harvest.id.to_string(),
            target_id,
            queued_at,
            serde_json::to_value(harvest)?
        )
        .execute(conn.ext())
        .await?;
        Ok(())
    }

    async fn harvest_get(&mut self, harvest_id: HarvestId) -> Result<Option<Harvest>> {
        let mut conn = self.acquire().await?;
        let data = query_scalar!(
            r#"SELECT data as "data: serde_json::Value" FROM harvest WHERE id = $1"#,
            harvest_id.to_string()
        )
        .fetch_optional(conn.ext())
        .await?;

        Ok(data.map(|d| serde_json::from_value(d)).transpose()?)
    }

    async fn harvest_get_user(&mut self, user_id: UserId) -> Result<Option<Harvest>> {
        let mut conn = self.acquire().await?;
        let data = query_scalar!(
            r#"SELECT data as "data" FROM harvest WHERE target_id = $1 ORDER BY queued_at DESC LIMIT 1"#,
            *user_id
        )
        .fetch_optional(conn.ext())
        .await?;

        Ok(data.map(|d| serde_json::from_value(d)).transpose()?)
    }

    async fn harvest_get_room(&mut self, room_id: RoomId) -> Result<Option<Harvest>> {
        let mut conn = self.acquire().await?;
        let data = query_scalar!(
            r#"SELECT data as "data" FROM harvest WHERE target_id = $1 ORDER BY queued_at DESC LIMIT 1"#,
            *room_id
        )
        .fetch_optional(conn.ext())
        .await?;

        Ok(data.map(|d| serde_json::from_value(d)).transpose()?)
    }

    async fn harvest_claim(&mut self) -> Result<Option<Harvest>> {
        let mut conn = self.acquire().await?;
        let data = sqlx::query_scalar!(
            r#"
            UPDATE harvest
            SET claimed_at = NOW()
            WHERE id = (
                SELECT id FROM harvest
                WHERE claimed_at IS NULL AND finished_at IS NULL
                ORDER BY queued_at ASC
                LIMIT 1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING data
            "#
        )
        .fetch_optional(conn.ext())
        .await?;

        Ok(data.map(|d| serde_json::from_value(d)).transpose()?)
    }
}
