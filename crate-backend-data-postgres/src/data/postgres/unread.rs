use async_trait::async_trait;
use common::v1::types::MessageId;
use common::v1::types::ack::{AckBulkItem, AckType};
use sqlx::{query, query_file};
use uuid::Uuid;

use crate::error::Result;
use crate::types::{ChannelId, RoomId, UserId};

use crate::data::DataUnread;

use super::Postgres;

#[async_trait]
impl DataUnread for Postgres {
    async fn unread_ack_bulk(&mut self, user_id: UserId, acks: &[AckBulkItem]) -> Result<()> {
        let mut conn = self.acquire().await?;

        let mut channel_ids = Vec::new();
        let mut message_ids = Vec::new();
        let mut mention_counts = Vec::new();
        // let mut unread_counts = Vec::new();

        for ack in acks {
            match &ack.ty {
                AckType::Message {
                    channel_id,
                    message_id,
                    mention_count,
                } => {
                    channel_ids.push(channel_id.into_inner());
                    message_ids.push(message_id.into_inner());
                    mention_counts.push(*mention_count as i32);
                    // TODO: unread_counts
                }
                _ => continue,
            }
        }

        if !channel_ids.is_empty() {
            query!(
                r#"
                INSERT INTO unread (channel_id, user_id, message_id, mention_count)
                SELECT u.channel_id, $2, u.message_id, u.mention_count
                FROM UNNEST($1::uuid[], $3::uuid[], $4::int4[])
                    AS u(channel_id, message_id, mention_count)
                ON CONFLICT ON CONSTRAINT unread_pkey DO UPDATE SET
                    message_id = excluded.message_id,
                    mention_count = excluded.mention_count;
                "#,
                &channel_ids,
                *user_id,
                &message_ids,
                &mention_counts,
            )
            .execute(conn.ext())
            .await?;
        }

        Ok(())
    }

    async fn unread_ack_room(
        &mut self,
        user_id: UserId,
        room_id: RoomId,
    ) -> Result<Vec<(ChannelId, MessageId)>> {
        let mut conn = self.acquire().await?;
        let records = query_file!("sql/unread_ack_room.sql", *user_id, *room_id,)
            .fetch_all(conn.ext())
            .await?;

        Ok(records
            .into_iter()
            .filter_map(|r| r.message_id.map(|m| (r.channel_id.into(), m.into())))
            .collect())
    }

    async fn unread_increment_counts(
        &mut self,
        channel_id: ChannelId,
        mentioned_user_ids: &[UserId],
        unread_user_ids: &[UserId],
    ) -> Result<()> {
        use std::collections::HashMap;

        let mut combined: HashMap<Uuid, bool> = HashMap::new();
        for id in unread_user_ids {
            combined.entry(id.into_inner()).or_insert(false);
        }
        for id in mentioned_user_ids {
            combined.insert(id.into_inner(), true);
        }

        if combined.is_empty() {
            return Ok(());
        }

        let mut conn = self.begin_tx().await?;
        let (user_ids, is_mentioned): (Vec<Uuid>, Vec<bool>) = combined.into_iter().unzip();

        query!(
            r#"
            INSERT INTO unread (channel_id, user_id, mention_count, unread_count)
            SELECT $1, u.user_id, CASE WHEN u.is_mentioned THEN 1 ELSE 0 END, 1
            FROM UNNEST($2::uuid[], $3::bool[]) AS u(user_id, is_mentioned)
            ON CONFLICT ON CONSTRAINT unread_pkey DO UPDATE SET
                mention_count = unread.mention_count + excluded.mention_count,
                unread_count = unread.unread_count + excluded.unread_count;
            "#,
            *channel_id,
            &user_ids,
            &is_mentioned,
        )
        .execute(conn.ext())
        .await?;
        conn.commit().await?;
        Ok(())
    }
}
