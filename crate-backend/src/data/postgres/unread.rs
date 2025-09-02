use async_trait::async_trait;
use common::v1::types::MessageId;
use sqlx::query;

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
        message_id: MessageId,
        version_id: MessageVerId,
    ) -> Result<()> {
        query!(
            r#"
			INSERT INTO unread (thread_id, user_id, message_id, version_id)
			VALUES ($1, $2, $3, $4)
			ON CONFLICT ON CONSTRAINT unread_pkey DO UPDATE SET
    			message_id = excluded.message_id,
    			version_id = excluded.version_id;
        "#,
            *thread_id,
            *user_id,
            *message_id,
            *version_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
