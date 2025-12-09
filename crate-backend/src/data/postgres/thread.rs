use async_trait::async_trait;
use common::v1::types::{ChannelId, PaginationQuery, PaginationResponse, RoomId, UserId};
use sqlx::{query, query_as, query_file_as, query_file_scalar, Acquire};

use crate::data::postgres::Pagination;
use crate::data::DataThread;
use crate::error::Result;
use crate::gen_paginate;
use crate::types::{Channel, ChannelVerId, DbChannel, PaginationDirection};

use super::Postgres;

#[async_trait]
impl DataThread for Postgres {
    async fn thread_list_active(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: ChannelId,
        include_all: bool,
    ) -> Result<PaginationResponse<Channel>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbChannel,
                "sql/thread_list_active.sql",
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32,
                *parent_id,
                *user_id,
                include_all
            ),
            query_file_scalar!(
                "sql/thread_list_active_count.sql",
                *parent_id,
                *user_id,
                include_all
            ),
            |i: &Channel| i.id.to_string()
        )
    }

    async fn thread_list_archived(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: ChannelId,
        include_all: bool,
    ) -> Result<PaginationResponse<Channel>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbChannel,
                "sql/thread_list_archived.sql",
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32,
                *parent_id,
                *user_id,
                include_all
            ),
            query_file_scalar!(
                "sql/thread_list_archived_count.sql",
                *parent_id,
                *user_id,
                include_all
            ),
            |i: &Channel| i.id.to_string()
        )
    }

    async fn thread_list_removed(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: ChannelId,
        include_all: bool,
    ) -> Result<PaginationResponse<Channel>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbChannel,
                "sql/thread_list_removed.sql",
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32,
                *parent_id,
                *user_id,
                include_all
            ),
            query_file_scalar!(
                "sql/thread_list_removed_count.sql",
                *parent_id,
                *user_id,
                include_all
            ),
            |i: &Channel| i.id.to_string()
        )
    }

    async fn thread_auto_archive(&self) -> Result<Vec<ChannelId>> {
        let archived_threads = query!(
            r#"
            UPDATE channel
            SET
                version_id = $1,
                archived_at = NOW()
            WHERE
                archived_at IS NULL
                AND deleted_at IS NULL
                AND type IN ('ThreadPublic', 'ThreadPrivate')
                AND auto_archive_duration IS NOT NULL
                AND last_activity_at IS NOT NULL
                AND last_activity_at + (auto_archive_duration * INTERVAL '1 second') < NOW()
            RETURNING id
            "#,
            ChannelVerId::new().into_inner()
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| ChannelId::from(row.id))
        .collect();

        Ok(archived_threads)
    }

    async fn thread_all_active_room(&self, room_id: RoomId) -> Result<Vec<Channel>> {
        let items = query_as!(
            DbChannel,
            r#"
            SELECT
                c.id, c.version_id, c.name, c.description, c.icon, c.nsfw, c.archived_at, c.deleted_at,
                c.last_activity_at, c.type as "ty: _", c.owner_id, c.parent_id,
                c.room_id, c.bitrate, c.user_limit, c.invitable, c.auto_archive_duration,
                c.default_auto_archive_duration, c.slowmode_thread, c.slowmode_message,
                c.default_slowmode_message, c.locked,
                c.creator_id, c.position,
                (SELECT coalesce(COUNT(*), 0) FROM thread_member WHERE channel_id = c.id AND membership = 'Join') AS "member_count!",
                (SELECT coalesce(COUNT(*), 0) FROM message WHERE channel_id = c.id AND deleted_at IS NULL) AS "message_count!",
                coalesce((SELECT json_agg(json_build_object('id', actor_id, 'type', type, 'allow', allow, 'deny', deny)) FROM permission_overwrite WHERE target_id = c.id), '[]'::json) as "permission_overwrites!",
                (SELECT version_id FROM message WHERE channel_id = c.id AND deleted_at IS NULL ORDER BY id DESC LIMIT 1) as last_version_id,
                (SELECT json_agg(tag_id) FROM channel_tag WHERE channel_id = c.id) as tags,
                (SELECT json_agg(tag.*) FROM tag WHERE channel_id = c.id) as tags_available
            FROM channel c
            WHERE
                c.room_id = $1
                AND c.archived_at IS NULL
                AND c.deleted_at IS NULL
                AND c.type IN ('ThreadPublic', 'ThreadPrivate')
            ORDER BY c.last_activity_at DESC NULLS LAST
            "#,
            room_id.into_inner()
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(items.into_iter().map(Into::into).collect())
    }
}
