use async_trait::async_trait;
use common::v1::types::{
    room_analytics::{
        RoomAnalyticsChannel, RoomAnalyticsChannelParams, RoomAnalyticsInvites,
        RoomAnalyticsMembersCount, RoomAnalyticsMembersJoin, RoomAnalyticsMembersLeave,
        RoomAnalyticsOverview, RoomAnalyticsParams,
    },
    RoomId,
};
use sqlx::{query, query_scalar};
use time::{OffsetDateTime, PrimitiveDateTime};

use crate::data::DataRoomAnalytics;
use crate::error::Result;

use super::Postgres;

#[async_trait]
impl DataRoomAnalytics for Postgres {
    async fn room_analytics_members_count(
        &self,
        _room_id: RoomId,
        _q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersCount>> {
        todo!()
    }

    async fn room_analytics_members_join(
        &self,
        _room_id: RoomId,
        _q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersJoin>> {
        todo!()
    }

    async fn room_analytics_members_leave(
        &self,
        _room_id: RoomId,
        _q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersLeave>> {
        todo!()
    }

    async fn room_analytics_channels(
        &self,
        _room_id: RoomId,
        _q: RoomAnalyticsParams,
        _q2: RoomAnalyticsChannelParams,
    ) -> Result<Vec<RoomAnalyticsChannel>> {
        todo!()
    }

    async fn room_analytics_overview(
        &self,
        _room_id: RoomId,
        _q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsOverview>> {
        todo!()
    }

    async fn room_analytics_invites(
        &self,
        _room_id: RoomId,
        _q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsInvites>> {
        todo!()
    }

    async fn room_analytics_snapshot_all(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // snapshot channel metrics
        query!(
            r#"
            WITH media_total_size AS (
                SELECT
                    m.id as media_id,
                    coalesce(sum((track.value->>'size')::bigint), 0) as total_size
                FROM media m,
                jsonb_array_elements(m.data->'tracks') AS track
                GROUP BY m.id
            ),
            channel_media_stats AS (
                SELECT
                    msg.channel_id,
                    count(DISTINCT ma.media_id) AS media_count,
                    sum(mts.total_size) as media_size
                FROM message msg
                JOIN message_attachment ma ON ma.version_id = msg.version_id
                JOIN media_total_size mts ON mts.media_id = ma.media_id
                WHERE msg.is_latest AND msg.deleted_at IS NULL
                GROUP BY msg.channel_id
            ),
            channel_message_stats AS (
                SELECT
                    channel_id,
                    count(*) as message_count
                FROM message
                WHERE is_latest AND deleted_at IS NULL
                GROUP BY channel_id
            )
            INSERT INTO metric_channel (ts, channel_id, room_id, message_count, media_count, media_size)
            SELECT
                now(),
                c.id as channel_id,
                c.room_id,
                coalesce(cms.message_count, 0) as message_count,
                coalesce(cmeds.media_count, 0) as media_count,
                coalesce(cmeds.media_size, 0) as media_size
            FROM channel c
            LEFT JOIN channel_message_stats cms ON cms.channel_id = c.id
            LEFT JOIN channel_media_stats cmeds ON cmeds.channel_id = c.id
            WHERE c.room_id IS NOT NULL
            ON CONFLICT (ts, channel_id) DO UPDATE SET
                message_count = EXCLUDED.message_count,
                media_count = EXCLUDED.media_count,
                media_size = EXCLUDED.media_size;
            "#,
        )
        .execute(&mut *tx)
        .await?;

        // snapshot room metrics
        let last_ts = query_scalar!("SELECT max(ts) FROM metric_room")
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or_else(|| {
                let epoch = OffsetDateTime::UNIX_EPOCH;
                PrimitiveDateTime::new(epoch.date(), epoch.time())
            });

        query!(
            r#"
            WITH room_member_counts AS (
                SELECT
                    room_id,
                    count(*) as member_count
                FROM room_member
                WHERE left_at IS NULL
                GROUP BY room_id
            ),
            room_joins AS (
                SELECT
                    room_id,
                    count(*) as join_count
                FROM room_member
                WHERE joined_at > $1
                GROUP BY room_id
            ),
            room_leaves AS (
                SELECT
                    room_id,
                    count(*) as leave_count
                FROM room_member
                WHERE left_at > $1
                GROUP BY room_id
            )
            INSERT INTO metric_room (ts, room_id, members, members_join, members_leave)
            SELECT
                now(),
                r.id as room_id,
                coalesce(rmc.member_count, 0) as members,
                coalesce(rj.join_count, 0) as members_join,
                coalesce(rl.leave_count, 0) as members_leave
            FROM room r
            LEFT JOIN room_member_counts rmc ON rmc.room_id = r.id
            LEFT JOIN room_joins rj ON rj.room_id = r.id
            LEFT JOIN room_leaves rl ON rl.room_id = r.id
            ON CONFLICT (ts, room_id) DO UPDATE SET
                members = EXCLUDED.members,
                members_join = EXCLUDED.members_join,
                members_leave = EXCLUDED.members_leave;
            "#,
            last_ts
        )
        .execute(&mut *tx)
        .await?;

        // TODO: snapshot invite metrics

        tx.commit().await?;

        Ok(())
    }

    async fn room_analytics_get_last_snapshot_ts(&self) -> Result<Option<time::PrimitiveDateTime>> {
        let ts = query_scalar!("SELECT max(ts) FROM metric_room")
            .fetch_one(&self.pool)
            .await?;
        Ok(ts)
    }
}
