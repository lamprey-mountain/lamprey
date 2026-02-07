use async_trait::async_trait;
use common::v1::types::ack::AckBulkItem;
use common::v1::types::MessageId;
use sqlx::{query, query_file};
use uuid::Uuid;

use crate::error::Result;
use crate::types::{ChannelId, MessageVerId, RoomId, UserId};

use crate::data::DataUnread;

use super::Postgres;

#[async_trait]
impl DataUnread for Postgres {
    async fn unread_ack(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        message_id: MessageId,
        version_id: MessageVerId,
        mention_count: Option<u64>,
    ) -> Result<()> {
        let mention_count = mention_count.unwrap_or(0) as i32;
        query!(
            r#"
			INSERT INTO unread (channel_id, user_id, message_id, version_id, mention_count)
			VALUES ($1, $2, $3, $4, $5)
			ON CONFLICT ON CONSTRAINT unread_pkey DO UPDATE SET
    			message_id = excluded.message_id,
    			version_id = excluded.version_id,
            mention_count = excluded.mention_count;
            "#,
            *channel_id,
            *user_id,
            *message_id,
            *version_id,
            mention_count
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn unread_ack_bulk(&self, user_id: UserId, acks: Vec<AckBulkItem>) -> Result<()> {
        let channel_ids: Vec<Uuid> = acks.iter().map(|a| *a.channel_id).collect();
        let message_ids: Vec<Uuid> = acks
            .iter()
            .map(|a| a.message_id.map(|m| *m).unwrap_or_default())
            .collect();
        let version_ids: Vec<Uuid> = acks.iter().map(|a| *a.version_id).collect();
        let mention_counts: Vec<i32> = acks.iter().map(|a| a.mention_count as i32).collect();

        query!(
            r#"
            INSERT INTO unread (channel_id, user_id, message_id, version_id, mention_count)
            SELECT unnest($1::uuid[]), $2, unnest($3::uuid[]), unnest($4::uuid[]), unnest($5::int4[])
            ON CONFLICT ON CONSTRAINT unread_pkey DO UPDATE SET
                message_id = excluded.message_id,
                version_id = excluded.version_id,
                mention_count = excluded.mention_count;
            "#,
            &channel_ids,
            *user_id,
            &message_ids,
            &version_ids,
            &mention_counts
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
