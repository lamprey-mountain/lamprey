use async_trait::async_trait;
use common::v1::types::RoomId;
use sqlx::query;

use crate::error::Result;
use crate::types::UserId;

use crate::data::DataDm;

use super::Postgres;

fn ensure_canonical(a: UserId, b: UserId) -> (UserId, UserId) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

#[async_trait]
impl DataDm for Postgres {
    async fn dm_put(&self, user_a_id: UserId, user_b_id: UserId, room_id: RoomId) -> Result<()> {
        let (user_a_id, user_b_id) = ensure_canonical(user_a_id, user_b_id);
        query!(
            r#"
            INSERT INTO dm (user_a_id, user_b_id, room_id)
            VALUES ($1, $2, $3)
            ON CONFLICT ON CONSTRAINT dm_pkey DO NOTHING
            "#,
            user_a_id.into_inner(),
            user_b_id.into_inner(),
            room_id.into_inner(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn dm_get(&self, user_a_id: UserId, user_b_id: UserId) -> Result<RoomId> {
        let (user_a_id, user_b_id) = ensure_canonical(user_a_id, user_b_id);
        let row = query!(
            r#"
                SELECT room_id FROM dm
                WHERE user_a_id = $1 AND user_b_id = $2
         "#,
            user_a_id.into_inner(),
            user_b_id.into_inner(),
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.room_id.into())
    }
}
