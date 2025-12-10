use async_trait::async_trait;
use common::v1::types::{
    room_analytics::{
        Aggregation, RoomAnalyticsChannel, RoomAnalyticsChannelParams, RoomAnalyticsInvites,
        RoomAnalyticsMembersCount, RoomAnalyticsMembersJoin, RoomAnalyticsMembersLeave,
        RoomAnalyticsOverview, RoomAnalyticsParams,
    },
    RoomId,
};
use sqlx::{query, query_scalar};
use time::{Duration, OffsetDateTime, PrimitiveDateTime};

use crate::data::DataRoomAnalytics;
use crate::error::Result;

use super::Postgres;

fn aggregate_points<T: Clone + Send>(
    points: Vec<T>,
    q: &RoomAnalyticsParams,
    get_bucket: impl Fn(&T) -> OffsetDateTime,
) -> Vec<T> {
    if points.is_empty() {
        return vec![];
    }

    let limit = q.limit.unwrap_or(100).max(1).min(1024) as usize;

    let aggregation_duration = match q.aggregate {
        Aggregation::Hourly => Duration::hours(1),
        Aggregation::Daily => Duration::days(1),
        Aggregation::Weekly => Duration::weeks(1),
        Aggregation::Monthly => Duration::days(30), // approximation
    };

    let mut aggregated_points = Vec::new();
    let mut last_bucket_time: Option<OffsetDateTime> = None;

    // iterate from newest to oldest
    for point in points.into_iter().rev() {
        let current_bucket_time = get_bucket(&point);

        if let Some(last_time) = last_bucket_time {
            if last_time - current_bucket_time < aggregation_duration {
                continue;
            }
        }

        aggregated_points.push(point.clone());
        last_bucket_time = Some(current_bucket_time);

        if aggregated_points.len() >= limit {
            break;
        }
    }

    // return in ascending order of time
    aggregated_points.reverse();
    aggregated_points
}

#[async_trait]
impl DataRoomAnalytics for Postgres {
    async fn room_analytics_members_count(
        &self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersCount>> {
        let start_time: Option<PrimitiveDateTime> = q.start.map(|t| t.into());
        let end_time: Option<PrimitiveDateTime> = q.end.map(|t| t.into());

        let points = query!(
            "select ts, members from metric_room where room_id = $1 AND ($2::timestamp IS NULL OR ts >= $2) AND ($3::timestamp IS NULL OR ts <= $3) ORDER BY ts ASC",
            *room_id,
            start_time,
            end_time
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|p| RoomAnalyticsMembersCount {
            bucket: p.ts.into(),
            count: p.members as u64,
        })
        .collect();

        Ok(aggregate_points(points, &q, |p| p.bucket.into_inner()))
    }

    async fn room_analytics_members_join(
        &self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersJoin>> {
        let start_time: Option<PrimitiveDateTime> = q.start.map(|t| t.into());
        let end_time: Option<PrimitiveDateTime> = q.end.map(|t| t.into());

        let points = query!(
            "SELECT ts, members_join from metric_room where room_id = $1 AND ($2::timestamp IS NULL OR ts >= $2) AND ($3::timestamp IS NULL OR ts <= $3) ORDER BY ts ASC",
            *room_id,
            start_time,
            end_time
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|p| RoomAnalyticsMembersJoin {
            bucket: p.ts.into(),
            count: p.members_join as u64,
        })
        .collect();

        Ok(aggregate_points(points, &q, |p| p.bucket.into_inner()))
    }

    async fn room_analytics_members_leave(
        &self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsMembersLeave>> {
        let start_time: Option<PrimitiveDateTime> = q.start.map(|t| t.into());
        let end_time: Option<PrimitiveDateTime> = q.end.map(|t| t.into());

        let points = query!(
            "SELECT ts, members_leave from metric_room where room_id = $1 AND ($2::timestamp IS NULL OR ts >= $2) AND ($3::timestamp IS NULL OR ts <= $3) ORDER BY ts ASC",
            *room_id,
            start_time,
            end_time
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|p| RoomAnalyticsMembersLeave {
            bucket: p.ts.into(),
            count: p.members_leave as u64,
        })
        .collect();

        Ok(aggregate_points(points, &q, |p| p.bucket.into_inner()))
    }

    async fn room_analytics_channels(
        &self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
        q2: RoomAnalyticsChannelParams,
    ) -> Result<Vec<RoomAnalyticsChannel>> {
        let start_time: Option<PrimitiveDateTime> = q.start.map(|t| t.into());
        let end_time: Option<PrimitiveDateTime> = q.end.map(|t| t.into());

        let points = query!(
            "SELECT ts, channel_id, message_count, media_count, media_size
        FROM metric_channel
        WHERE room_id = $1 AND ($2::uuid IS NULL OR channel_id = $2) AND ($3::timestamp IS NULL OR ts >= $3) AND ($4::timestamp IS NULL OR ts <= $4)
        ORDER BY ts ASC",
            *room_id,
            q2.channel_id.map(|c| *c),
            start_time,
            end_time
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|p| RoomAnalyticsChannel {
            bucket: p.ts.into(),
            channel_id: p.channel_id.into(),
            message_count: p.message_count as u64,
            media_count: p.media_count as u64,
            media_size: p.media_size as u64,
        })
        .collect();

        Ok(aggregate_points(points, &q, |p| p.bucket.into_inner()))
    }

    async fn room_analytics_overview(
        &self,
        room_id: RoomId,
        q: RoomAnalyticsParams,
    ) -> Result<Vec<RoomAnalyticsOverview>> {
        let start_time: Option<PrimitiveDateTime> = q.start.map(|t| t.into());
        let end_time: Option<PrimitiveDateTime> = q.end.map(|t| t.into());

        let points = query!(
            "SELECT ts, sum(message_count)::int as message_count, sum(media_count)::int as media_count, sum(media_size)::int as media_size
        FROM metric_channel
        WHERE room_id = $1 AND ($2::timestamp IS NULL OR ts >= $2) AND ($3::timestamp IS NULL OR ts <= $3)
        GROUP BY ts
        ORDER BY ts ASC",
            *room_id,
            start_time,
            end_time
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|p| RoomAnalyticsOverview {
            bucket: p.ts.into(),
            message_count: p.message_count.unwrap_or(0) as u64,
            media_count: p.media_count.unwrap_or(0) as u64,
            media_size: p.media_size.unwrap_or(0) as u64,
        })
        .collect();

        Ok(aggregate_points(points, &q, |p| p.bucket.into_inner()))
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

    async fn gc_room_analytics(&self) -> Result<u64> {
        let interval = Duration::days(crate::consts::RETENTION_ROOM_ANALYTICS as i64);
        let cutoff = OffsetDateTime::now_utc() - interval;
        let cutoff_primitive = PrimitiveDateTime::new(cutoff.date(), cutoff.time());

        let r1 = query!("DELETE FROM metric_room WHERE ts < $1", cutoff_primitive)
            .execute(&self.pool)
            .await?;

        let r2 = query!("DELETE FROM metric_channel WHERE ts < $1", cutoff_primitive)
            .execute(&self.pool)
            .await?;

        Ok(r1.rows_affected() + r2.rows_affected())
    }
}
