use async_trait::async_trait;
use common::v1::types::ThreadReorder;
use sqlx::{query, query_file_as, query_scalar, Acquire};
use tracing::info;

use crate::error::Result;
use crate::types::{
    DbThread, DbThreadCreate, DbThreadPrivate, DbThreadType, PaginationDirection, PaginationQuery,
    PaginationResponse, RoomId, Thread, ThreadId, ThreadPatch, ThreadVerId, UserId,
};
use crate::{gen_paginate, Error};

use crate::data::DataThread;

use super::{Pagination, Postgres};

#[async_trait]
impl DataThread for Postgres {
    async fn thread_create(&self, create: DbThreadCreate) -> Result<ThreadId> {
        let thread_id = ThreadId::new();
        let mut tx = self.pool.begin().await?;

        if let Some(room_id) = create.room_id {
            let count: i64 = query_scalar!(
                "SELECT count(*) FROM thread WHERE room_id = $1 AND archived_at IS NULL AND deleted_at IS NULL",
                room_id
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(0);

            if count as u32 >= crate::consts::MAX_ACTIVE_THREAD_COUNT {
                return Err(Error::BadRequest(format!(
                    "too many active threads (max {})",
                    crate::consts::MAX_ACTIVE_THREAD_COUNT
                )));
            }
        }

        query!(
            "
			INSERT INTO thread (id, version_id, creator_id, room_id, name, description, type, nsfw, locked, bitrate, user_limit, parent_id)
			VALUES ($1, $2, $3, $4, $5, $6, $7, $8, false, $9, $10, $11)
        ",
            thread_id.into_inner(),
            thread_id.into_inner(),
            create.creator_id.into_inner(),
            create.room_id.map(|id| id),
            create.name,
            create.description,
            create.ty as _,
            create.nsfw,
            create.bitrate,
            create.user_limit,
            create.parent_id,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        info!("inserted thread");
        Ok(thread_id)
    }

    /// get a thread
    async fn thread_get(&self, thread_id: ThreadId) -> Result<Thread> {
        let thread = query_file_as!(DbThread, "sql/thread_get.sql", thread_id.into_inner())
            .fetch_one(&self.pool)
            .await?;
        Ok(thread.into())
    }

    async fn thread_list(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<ThreadId>,
    ) -> Result<PaginationResponse<Thread>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbThread,
                "sql/thread_paginate.sql",
                *room_id,
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r#"SELECT count(*) FROM thread WHERE room_id = $1 AND deleted_at IS NULL AND archived_at IS NULL"#,
                room_id.into_inner()
            ),
            |i: &Thread| i.id.to_string()
        )
    }

    async fn thread_list_archived(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<ThreadId>,
    ) -> Result<PaginationResponse<Thread>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbThread,
                "sql/thread_paginate_archived.sql",
                *room_id,
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r#"SELECT count(*) FROM thread WHERE room_id = $1 AND deleted_at IS NULL AND archived_at IS NOT NULL"#,
                room_id.into_inner()
            ),
            |i: &Thread| i.id.to_string()
        )
    }

    async fn thread_list_removed(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<ThreadId>,
    ) -> Result<PaginationResponse<Thread>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbThread,
                "sql/thread_paginate_removed.sql",
                *room_id,
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r#"SELECT count(*) FROM thread WHERE room_id = $1 AND deleted_at IS NOT NULL"#,
                room_id.into_inner()
            ),
            |i: &Thread| i.id.to_string()
        )
    }

    async fn thread_get_private(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
    ) -> Result<DbThreadPrivate> {
        let thread_private = query_file_as!(
            DbThreadPrivate,
            "sql/thread_get_private.sql",
            *thread_id,
            *user_id,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(thread_private.into())
    }

    async fn thread_update(&self, thread_id: ThreadId, patch: ThreadPatch) -> Result<ThreadVerId> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let thread = query_file_as!(DbThread, "sql/thread_get.sql", *thread_id,)
            .fetch_one(&self.pool)
            .await?;
        let thread: Thread = thread.into();
        let version_id = ThreadVerId::new();
        query!(
            r#"
            UPDATE thread SET
                version_id = $2,
                name = $3,
                description = $4,
                nsfw = $5,
                bitrate = $6,
                user_limit = $7
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
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(version_id)
    }

    async fn thread_delete(&self, thread_id: ThreadId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let version_id = ThreadVerId::new();
        query!(
            r#"
            UPDATE thread SET
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

    async fn thread_undelete(&self, thread_id: ThreadId) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        if let Some(room_id) = query_scalar!("SELECT room_id FROM thread WHERE id = $1", *thread_id)
            .fetch_one(&mut *tx)
            .await?
        {
            let count: i64 = query_scalar!(
                "SELECT count(*) FROM thread WHERE room_id = $1 AND archived_at IS NULL AND deleted_at IS NULL",
                room_id
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(0);

            if count as u32 >= crate::consts::MAX_ACTIVE_THREAD_COUNT {
                return Err(Error::BadRequest(format!(
                    "too many active threads (max {})",
                    crate::consts::MAX_ACTIVE_THREAD_COUNT
                )));
            }
        }

        let version_id = ThreadVerId::new();
        query!(
            r#"
            UPDATE thread SET
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

    async fn thread_archive(&self, thread_id: ThreadId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let version_id = ThreadVerId::new();
        query!(
            r#"
            UPDATE thread SET
                version_id = $2,
                archived_at = NOW()
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

    async fn thread_unarchive(&self, thread_id: ThreadId) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        if let Some(room_id) = query_scalar!("SELECT room_id FROM thread WHERE id = $1", *thread_id)
            .fetch_one(&mut *tx)
            .await?
        {
            let count: i64 = query_scalar!(
                "SELECT count(*) FROM thread WHERE room_id = $1 AND archived_at IS NULL AND deleted_at IS NULL",
                room_id
            )
            .fetch_one(&mut *tx)
            .await?
            .unwrap_or(0);

            if count as u32 >= crate::consts::MAX_ACTIVE_THREAD_COUNT {
                return Err(Error::BadRequest(format!(
                    "too many active threads (max {})",
                    crate::consts::MAX_ACTIVE_THREAD_COUNT
                )));
            }
        }

        let version_id = ThreadVerId::new();
        query!(
            r#"
            UPDATE thread SET
                version_id = $2,
                archived_at = NULL
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

    async fn thread_lock(&self, thread_id: ThreadId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let version_id = ThreadVerId::new();
        query!(
            r#"
            UPDATE thread SET
                version_id = $2,
                locked = true
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

    async fn thread_unlock(&self, thread_id: ThreadId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let version_id = ThreadVerId::new();
        query!(
            r#"
            UPDATE thread SET
                version_id = $2,
                locked = false
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

    async fn thread_reorder(&self, data: ThreadReorder) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;

        for thread in data.threads {
            let old = query!(
                r#"SELECT position, parent_id FROM thread WHERE id = $1"#,
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
                let version_id = ThreadVerId::new();
                query!(
                    r#"UPDATE thread SET version_id = $2, position = $3, parent_id = $4 WHERE id = $1"#,
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
}
