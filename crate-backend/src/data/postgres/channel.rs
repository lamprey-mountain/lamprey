use async_trait::async_trait;
use common::v1::types::util::{Diff, Time};
use common::v1::types::ChannelReorder;
use sqlx::{query, query_file_as, query_scalar, Acquire};
use tracing::info;

use crate::error::Result;
use crate::types::{
    Channel, ChannelId, ChannelPatch, ChannelVerId, DbChannel, DbChannelCreate, DbChannelPrivate,
    DbChannelType, PaginationDirection, PaginationQuery, PaginationResponse, RoomId, UserId,
};
use crate::{gen_paginate, Error};

use crate::data::DataChannel;

use super::{Pagination, Postgres};

#[async_trait]
impl DataChannel for Postgres {
    async fn channel_create(&self, create: DbChannelCreate) -> Result<ChannelId> {
        let channel_id = ChannelId::new();
        self.channel_create_with_id(channel_id, create).await?;
        Ok(channel_id)
    }

    async fn channel_create_with_id(
        &self,
        channel_id: ChannelId,
        create: DbChannelCreate,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        if let Some(room_id) = create.room_id {
            let count: i64 = query_scalar!(
                "SELECT count(*) FROM channel WHERE room_id = $1 AND archived_at IS NULL AND deleted_at IS NULL",
                room_id
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(0);

            if count as u32 >= crate::consts::MAX_CHANNEL_COUNT {
                return Err(Error::BadRequest(format!(
                    "too many active channels (max {})",
                    crate::consts::MAX_CHANNEL_COUNT
                )));
            }
        }

        query!(
            "
			INSERT INTO channel (id, version_id, creator_id, room_id, name, description, type, nsfw, locked, bitrate, user_limit, parent_id, owner_id, icon, invitable, auto_archive_duration, default_auto_archive_duration, slowmode_thread, slowmode_message, default_slowmode_message)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8, false, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
        ",
            channel_id.into_inner(),
            channel_id.into_inner(),
            create.creator_id.into_inner(),
            create.room_id,
            create.name,
            create.description,
            create.ty as _,
            create.nsfw,
            create.bitrate,
            create.user_limit,
            create.parent_id,
            create.owner_id,
            create.icon,
            create.invitable,
            create.auto_archive_duration,
            create.default_auto_archive_duration,
            create.slowmode_thread.map(|s| s as i32),
            create.slowmode_message.map(|s| s as i32),
            create.default_slowmode_message.map(|s| s as i32),
        )
        .execute(&mut *tx)
        .await?;

        if let Some(tags) = &create.tags {
            if !tags.is_empty() {
                let tag_ids: Vec<_> = tags.iter().map(|t| t.into_inner()).collect();
                query!(
                    "INSERT INTO channel_tag (channel_id, tag_id) SELECT $1, * FROM UNNEST($2::uuid[])",
                    channel_id.into_inner(),
                    &tag_ids
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        info!("inserted channel");
        Ok(())
    }

    async fn channel_get(&self, channel_id: ChannelId) -> Result<Channel> {
        let thread = query_file_as!(DbChannel, "sql/channel_get.sql", channel_id.into_inner())
            .fetch_one(&self.pool)
            .await?;
        Ok(thread.into())
    }

    async fn channel_get_many(&self, channel_ids: &[ChannelId]) -> Result<Vec<Channel>> {
        let ids: Vec<uuid::Uuid> = channel_ids.iter().map(|id| id.into_inner()).collect();
        let threads = query_file_as!(DbChannel, "sql/channel_get_many.sql", &ids)
            .fetch_all(&self.pool)
            .await?;
        Ok(threads.into_iter().map(Into::into).collect())
    }

    async fn channel_list(
        &self,
        room_id: RoomId,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: Option<ChannelId>,
    ) -> Result<PaginationResponse<Channel>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbChannel,
                "sql/channel_paginate.sql",
                *room_id,
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32,
                parent_id.map(|id| *id),
                *user_id
            ),
            query_scalar!(
                r#"SELECT count(*) FROM channel WHERE room_id = $1 AND deleted_at IS NULL AND archived_at IS NULL AND ($2::uuid IS NULL OR parent_id = $2)"#,
                room_id.into_inner(),
                parent_id.map(|id| *id)
            ),
            |i: &Channel| i.id.to_string()
        )
    }

    async fn channel_list_archived(
        &self,
        room_id: RoomId,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: Option<ChannelId>,
    ) -> Result<PaginationResponse<Channel>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbChannel,
                "sql/channel_paginate_archived.sql",
                *room_id,
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32,
                parent_id.map(|id| *id),
                *user_id
            ),
            query_scalar!(
                r#"SELECT count(*) FROM channel WHERE room_id = $1 AND deleted_at IS NULL AND archived_at IS NOT NULL AND ($2::uuid IS NULL OR parent_id = $2)"#,
                room_id.into_inner(),
                parent_id.map(|id| *id)
            ),
            |i: &Channel| i.id.to_string()
        )
    }

    async fn channel_list_removed(
        &self,
        room_id: RoomId,
        user_id: UserId,
        pagination: PaginationQuery<ChannelId>,
        parent_id: Option<ChannelId>,
    ) -> Result<PaginationResponse<Channel>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbChannel,
                "sql/channel_paginate_removed.sql",
                *room_id,
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32,
                parent_id.map(|id| *id),
                *user_id,
            ),
            query_scalar!(
                r#"SELECT count(*) FROM channel WHERE room_id = $1 AND deleted_at IS NOT NULL AND ($2::uuid IS NULL OR parent_id = $2)"#,
                room_id.into_inner(),
                parent_id.map(|id| *id)
            ),
            |i: &Channel| i.id.to_string()
        )
    }

    async fn channel_get_private(
        &self,
        thread_id: ChannelId,
        user_id: UserId,
    ) -> Result<DbChannelPrivate> {
        let thread_private = query_file_as!(
            DbChannelPrivate,
            "sql/channel_get_private.sql",
            *thread_id,
            *user_id,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(thread_private)
    }

    async fn channel_update(
        &self,
        thread_id: ChannelId,
        patch: ChannelPatch,
    ) -> Result<ChannelVerId> {
        let mut tx = self.pool.begin().await?;
        let db_chan = query_file_as!(DbChannel, "sql/channel_get.sql", *thread_id)
            .fetch_one(&mut *tx)
            .await?;
        let mut last_activity_at = db_chan.last_activity_at;
        let thread: Channel = db_chan.into();

        if patch.archived == Some(false) && thread.archived_at.is_some() {
            if let Some(room_id) = thread.room_id {
                let count: i64 = query_scalar!(
                    "SELECT count(*) FROM channel WHERE room_id = $1 AND archived_at IS NULL AND deleted_at IS NULL",
                    *room_id
                )
                .fetch_one(&mut *tx)
                .await?
                .unwrap_or(0);

                if count as u32 >= crate::consts::MAX_CHANNEL_COUNT {
                    return Err(Error::BadRequest(format!(
                        "too many active channel (max {})",
                        crate::consts::MAX_CHANNEL_COUNT
                    )));
                }
            }
        }

        if let Some(tags) = &patch.tags {
            query!(
                "DELETE FROM channel_tag WHERE channel_id = $1",
                thread_id.into_inner()
            )
            .execute(&mut *tx)
            .await?;

            if !tags.is_empty() {
                let tag_ids: Vec<_> = tags.iter().map(|t| t.into_inner()).collect();
                query!(
                    "INSERT INTO channel_tag (channel_id, tag_id) SELECT $1, * FROM UNNEST($2::uuid[])",
                    thread_id.into_inner(),
                    &tag_ids
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        let version_id = ChannelVerId::new();

        let archived_at = match patch.archived {
            Some(true) => Some(time::OffsetDateTime::now_utc()),
            Some(false) => None,
            None => thread.archived_at.map(|t| *t),
        };

        let new_parent_id = match patch.parent_id {
            Some(id) => id.map(|i| i.into_inner()),
            None => thread.parent_id.map(|i| i.into_inner()),
        };

        let new_ty: DbChannelType = patch.ty.map(Into::into).unwrap_or_else(|| thread.ty.into());

        if patch.archived == Some(false)
            || patch
                .auto_archive_duration
                .changes(&thread.auto_archive_duration)
        {
            let now = time::OffsetDateTime::now_utc();
            let now = time::PrimitiveDateTime::new(now.date(), now.time());
            last_activity_at = Some(now);
        }

        query!(
            r#"
            UPDATE channel SET
                version_id = $2,
                name = $3,
                description = $4,
                nsfw = $5,
                bitrate = $6,
                user_limit = $7,
                owner_id = $8,
                icon = $9,
                locked = $10,
                archived_at = $11,
                invitable = $12,
                type = $13,
                parent_id = $14,
                auto_archive_duration = $15,
                default_auto_archive_duration = $16,
                slowmode_thread = $17,
                slowmode_message = $18,
                default_slowmode_message = $19,
                last_activity_at = $20
            WHERE id = $1
        "#,
            thread_id.into_inner(),
            version_id.into_inner(),
            patch.name.unwrap_or(thread.name),
            patch.description.unwrap_or(thread.description),
            patch.nsfw.unwrap_or(thread.nsfw),
            patch.bitrate.unwrap_or(thread.bitrate).map(|i| i as i32),
            patch
                .user_limit
                .unwrap_or(thread.user_limit)
                .map(|i| i as i32),
            patch
                .owner_id
                .unwrap_or(thread.owner_id)
                .map(|i| i.into_inner()),
            patch.icon.unwrap_or(thread.icon).map(|id| *id),
            patch.locked.unwrap_or(thread.locked),
            archived_at as _,
            patch.invitable.unwrap_or(thread.invitable),
            new_ty as _,
            new_parent_id,
            patch
                .auto_archive_duration
                .unwrap_or(thread.auto_archive_duration)
                .map(|i| i as i64),
            patch
                .default_auto_archive_duration
                .unwrap_or(thread.default_auto_archive_duration)
                .map(|i| i as i64),
            patch
                .slowmode_thread
                .unwrap_or(thread.slowmode_thread)
                .map(|i| i as i32),
            patch
                .slowmode_message
                .unwrap_or(thread.slowmode_message)
                .map(|i| i as i32),
            patch
                .default_slowmode_message
                .unwrap_or(thread.default_slowmode_message)
                .map(|i| i as i32),
            last_activity_at as _,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(version_id)
    }

    async fn channel_delete(&self, thread_id: ChannelId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let version_id = ChannelVerId::new();
        query!(
            r#"
            UPDATE channel SET
                version_id = $2,
                deleted_at = NOW()
            WHERE id = $1
            "#,
            thread_id.into_inner(),
            version_id.into_inner(),
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn channel_undelete(&self, thread_id: ChannelId) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        if let Some(room_id) =
            query_scalar!("SELECT room_id FROM channel WHERE id = $1", *thread_id)
                .fetch_one(&mut *tx)
                .await?
        {
            let count: i64 = query_scalar!(
                "SELECT count(*) FROM channel WHERE room_id = $1 AND archived_at IS NULL AND deleted_at IS NULL",
                room_id
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(0);

            if count as u32 >= crate::consts::MAX_CHANNEL_COUNT {
                return Err(Error::BadRequest(format!(
                    "too many active channel (max {})",
                    crate::consts::MAX_CHANNEL_COUNT
                )));
            }
        }

        let version_id = ChannelVerId::new();
        query!(
            r#"
            UPDATE channel SET
                version_id = $2,
                deleted_at = NULL
            WHERE id = $1
            "#,
            thread_id.into_inner(),
            version_id.into_inner(),
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn channel_reorder(&self, data: ChannelReorder) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        for thread in data.channels {
            let old = query!(
                r#"SELECT position, parent_id FROM channel WHERE id = $1"#,
                *thread.id,
            )
            .fetch_one(&mut *tx)
            .await?;
            let new_position = thread
                .position
                .map(|i| i.map(|i| i as i32))
                .unwrap_or(old.position);

            let new_parent_id = thread
                .parent_id
                .map(|i| i.map(|i| *i))
                .unwrap_or(old.parent_id);

            if new_position != old.position || new_parent_id != old.parent_id {
                let version_id = ChannelVerId::new();
                query!(
                    r#"UPDATE channel SET version_id = $2, position = $3, parent_id = $4 WHERE id = $1"#,
                    *thread.id,
                    *version_id,
                    thread.position.map(|i| i.map(|i| i as i32)).unwrap_or(old.position),
                    thread.parent_id.map(|i| i.map(|i| *i)).unwrap_or(old.parent_id),
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    async fn channel_upgrade_gdm(&self, thread_id: ChannelId, room_id: RoomId) -> Result<()> {
        let version_id = ChannelVerId::new();
        let ty = DbChannelType::Text;
        query!(
            r#"
            UPDATE channel SET
                version_id = $2,
                room_id = $3,
                type = $4
            WHERE id = $1
            "#,
            *thread_id,
            *version_id,
            *room_id,
            ty as _,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn channel_get_message_slowmode_expire_at(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<Option<Time>> {
        let row = query_scalar!(
            "SELECT expires_at FROM channel_slowmode_message WHERE channel_id = $1 AND user_id = $2",
            *channel_id,
            *user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Time::from))
    }

    async fn channel_set_message_slowmode_expire_at(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        expires_at: Time,
    ) -> Result<()> {
        query!(
            "INSERT INTO channel_slowmode_message (channel_id, user_id, expires_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (channel_id, user_id)
             DO UPDATE SET expires_at = $3",
            *channel_id,
            *user_id,
            time::PrimitiveDateTime::new(
                expires_at.into_inner().date(),
                expires_at.into_inner().time()
            )
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn channel_get_thread_slowmode_expire_at(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<Option<Time>> {
        let row = query_scalar!(
            "SELECT expires_at FROM channel_slowmode_thread WHERE channel_id = $1 AND user_id = $2",
            *channel_id,
            *user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Time::from))
    }

    async fn channel_set_thread_slowmode_expire_at(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
        expires_at: Time,
    ) -> Result<()> {
        query!(
            "INSERT INTO channel_slowmode_thread (channel_id, user_id, expires_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (channel_id, user_id)
             DO UPDATE SET expires_at = $3",
            *channel_id,
            *user_id,
            time::PrimitiveDateTime::new(
                expires_at.into_inner().date(),
                expires_at.into_inner().time()
            )
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
