use std::collections::HashMap;

use async_trait::async_trait;
use common::v1::types::MessageId;
use common::v1::types::ack::{AckBulkItem, AckType};
use sqlx::{Row, query, query_file};
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::data::{DataUnread, DbReadState};
use crate::error::Result;
use crate::types::{ChannelId, RoomId, UserId};

use super::Postgres;

#[async_trait]
impl DataUnread for Postgres {
    async fn unread_ack_bulk(&mut self, user_id: UserId, acks: &[AckBulkItem]) -> Result<()> {
        let mut conn = self.acquire().await?;

        let mut channel_ids = Vec::new();
        let mut message_ids = Vec::new();
        let mut mention_counts = Vec::new();
        let mut pins_channel_ids = Vec::new();
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
                AckType::Pins { channel_id } => {
                    pins_channel_ids.push(channel_id.into_inner());
                }
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

        if !pins_channel_ids.is_empty() {
            query!(
                r#"
                INSERT INTO unread (channel_id, user_id, pins_read_at)
                SELECT u.channel_id, $2, NOW()
                FROM UNNEST($1::uuid[]) AS u(channel_id)
                ON CONFLICT ON CONSTRAINT unread_pkey DO UPDATE SET
                    pins_read_at = EXCLUDED.pins_read_at;
                "#,
                &pins_channel_ids,
                *user_id,
            )
            .execute(conn.ext())
            .await?;
        }

        Ok(())
    }

    async fn unread_read_state_get(
        &mut self,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Result<Option<DbReadState>> {
        let mut conn = self.acquire().await?;
        let row = query(
            r#"
            SELECT
                u.message_id,
                u.last_read_at,
                coalesce(u.mention_count, 0) AS mention_count,
                coalesce(u.message_id < c.last_message_id, true) AS is_unread,
                u.pins_read_at
            FROM channel c
            LEFT JOIN unread u ON u.channel_id = c.id AND u.user_id = $2
            WHERE c.id = $1
            "#,
        )
        .bind(channel_id.into_inner())
        .bind(user_id.into_inner())
        .fetch_optional(conn.ext())
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let last_read_at: Option<PrimitiveDateTime> = row.try_get("last_read_at")?;
        let pins_read_at: Option<PrimitiveDateTime> = row.try_get("pins_read_at")?;
        let last_read_id: Option<Uuid> = row.try_get("message_id")?;
        let mention_count: i32 = row.try_get("mention_count")?;
        let unread: bool = row.try_get("is_unread")?;

        Ok(Some(DbReadState {
            last_read_id: last_read_id.map(Into::into),
            last_read_at: last_read_at.map(Into::into),
            mention_count: mention_count.max(0) as u64,
            unread,
            pins_read_at: pins_read_at.map(Into::into),
        }))
    }

    async fn unread_read_state_put(
        &mut self,
        user_id: UserId,
        states: &[(ChannelId, DbReadState)],
    ) -> Result<()> {
        if states.is_empty() {
            return Ok(());
        }

        let mut conn = self.begin_tx().await?;
        for (channel_id, state) in states {
            query(
                r#"
                INSERT INTO unread (channel_id, user_id, message_id, mention_count, pins_read_at, last_read_at)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT ON CONSTRAINT unread_pkey DO UPDATE SET
                    message_id = EXCLUDED.message_id,
                    mention_count = EXCLUDED.mention_count,
                    pins_read_at = coalesce(EXCLUDED.pins_read_at, unread.pins_read_at),
                    last_read_at = coalesce(EXCLUDED.last_read_at, unread.last_read_at)
                "#,
            )
            .bind(channel_id.into_inner())
            .bind(user_id.into_inner())
            .bind(state.last_read_id.map(|id| id.into_inner()))
            .bind(state.mention_count as i32)
            .bind(state.pins_read_at.map(PrimitiveDateTime::from))
            .bind(state.last_read_at.map(PrimitiveDateTime::from))
            .execute(conn.ext())
            .await?;
        }
        conn.commit().await?;
        Ok(())
    }

    async fn unread_pins_read_at_many(
        &mut self,
        user_id: UserId,
        channel_ids: &[ChannelId],
    ) -> Result<HashMap<ChannelId, Option<common::v1::types::util::Time>>> {
        if channel_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let ids: Vec<Uuid> = channel_ids.iter().map(|id| id.into_inner()).collect();
        let mut conn = self.acquire().await?;
        let rows = query(
            r#"
            SELECT channel_id, pins_read_at
            FROM unread
            WHERE user_id = $1 AND channel_id = ANY($2)
            "#,
        )
        .bind(user_id.into_inner())
        .bind(&ids)
        .fetch_all(conn.ext())
        .await?;

        let mut result = HashMap::with_capacity(rows.len());
        for row in rows {
            let channel_id: Uuid = row.try_get("channel_id")?;
            let pins_read_at: Option<PrimitiveDateTime> = row.try_get("pins_read_at")?;
            result.insert(channel_id.into(), pins_read_at.map(Into::into));
        }

        Ok(result)
    }

    async fn unread_room_last_messages(
        &mut self,
        room_id: RoomId,
    ) -> Result<Vec<(ChannelId, MessageId)>> {
        let mut conn = self.acquire().await?;
        let records = query_file!("sql/unread_room_last_messages.sql", *room_id,)
            .fetch_all(conn.ext())
            .await?;

        Ok(records
            .into_iter()
            .map(|r| (r.channel_id.into(), r.message_id.into()))
            .collect())
    }
}
