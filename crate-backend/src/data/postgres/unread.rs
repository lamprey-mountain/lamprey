use async_trait::async_trait;
use common::v1::types::MessageId;
use sqlx::{query, query_file};

use crate::error::Result;
use crate::types::{ChannelId, MessageVerId, RoomId, UserId};

use crate::data::DataUnread;

use super::Postgres;

#[async_trait]
impl DataUnread for Postgres {
    async fn unread_put(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        version_id: MessageVerId,
    ) -> Result<()> {
        query!(
            r#"
			INSERT INTO unread (channel_id, user_id, message_id, version_id)
			VALUES ($1, $2, $3, $4)
			ON CONFLICT ON CONSTRAINT unread_pkey DO UPDATE SET
    			message_id = excluded.message_id,
    			version_id = excluded.version_id;
        "#,
            *channel_id,
            *user_id,
            *message_id,
            *version_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn unread_put_all_in_room(
        &self,
        user_id: UserId,
        room_id: RoomId,
    ) -> Result<Vec<(ChannelId, MessageId, MessageVerId)>> {
        let records = query_file!(
            "sql/unread_put_all_in_room.sql",
            user_id.into_inner(),
            room_id.into_inner()
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records
            .into_iter()
            .map(|r| {
                (
                    r.channel_id.into(),
                    r.message_id.into(),
                    r.version_id.into(),
                )
            })
            .collect())
    }

    async fn unread_increment_mentions(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        version_id: MessageVerId,
        count: u32,
    ) -> Result<()> {
        query!(
            r#"
            INSERT INTO unread (channel_id, user_id, message_id, version_id, mention_count, is_unread)
            VALUES ($1, $2, $3, $4, $5, true)
            ON CONFLICT ON CONSTRAINT unread_pkey DO UPDATE SET
                mention_count = unread.mention_count + excluded.mention_count,
                is_unread = true,
                message_id = excluded.message_id,
                version_id = excluded.version_id;
            "#,
            *channel_id,
            *user_id,
            *message_id,
            *version_id,
            count as i32
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
