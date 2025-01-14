use async_trait::async_trait;
use sqlx::{query, Acquire};

use crate::error::Result;
use crate::types::{MessageVerId, ThreadId, UserId};

use crate::data::DataUnread;

use super::Postgres;

#[async_trait]
impl DataUnread for Postgres {
    async fn unread_put(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
        version_id: MessageVerId,
    ) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        query!(
            r#"
			INSERT INTO unread (thread_id, user_id, version_id)
			VALUES ($1, $2, $3)
			ON CONFLICT ON CONSTRAINT unread_pkey DO UPDATE SET version_id = excluded.version_id;
        "#,
            thread_id.into_inner(),
            user_id.into_inner(),
            version_id.into_inner()
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }
}
