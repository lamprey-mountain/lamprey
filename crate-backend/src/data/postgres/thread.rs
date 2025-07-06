use async_trait::async_trait;
use sqlx::{query, query_file_as, query_scalar, Acquire};
use tracing::info;

use crate::error::Result;
use crate::gen_paginate;
use crate::types::{
    DbThread, DbThreadCreate, PaginationDirection, PaginationQuery, PaginationResponse, RoomId,
    Thread, ThreadId, ThreadPatch, ThreadVerId, UserId,
};

use crate::data::DataThread;

use super::{Pagination, Postgres};

#[async_trait]
impl DataThread for Postgres {
    async fn thread_create(&self, create: DbThreadCreate) -> Result<ThreadId> {
        let thread_id = ThreadId::new();
        query!(
            "
			INSERT INTO thread (id, version_id, creator_id, room_id, name, description)
			VALUES ($1, $2, $3, $4, $5, $6)
        ",
            thread_id.into_inner(),
            thread_id.into_inner(),
            create.creator_id.into_inner(),
            create.room_id.into_inner(),
            create.name,
            create.description,
        )
        .execute(&self.pool)
        .await?;
        info!("inserted thread");
        Ok(thread_id)
    }

    /// get a thread, panics if there are no messages
    async fn thread_get(&self, thread_id: ThreadId, user_id: Option<UserId>) -> Result<Thread> {
        let thread = query_file_as!(
            DbThread,
            "sql/thread_get.sql",
            thread_id.into_inner(),
            user_id.map(|id| id.into_inner())
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(thread.into())
    }

    async fn thread_list(
        &self,
        user_id: UserId,
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
                room_id.into_inner(),
                user_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r#"SELECT count(*) FROM thread WHERE room_id = $1"#,
                room_id.into_inner()
            )
        )
    }

    async fn thread_update(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        patch: ThreadPatch,
    ) -> Result<ThreadVerId> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let thread = query_file_as!(
            DbThread,
            "sql/thread_get.sql",
            thread_id.into_inner(),
            user_id.into_inner(),
        )
        .fetch_one(&self.pool)
        .await?;
        let thread: Thread = thread.into();
        let version_id = ThreadVerId::new();
        query!(
            r#"
            UPDATE thread SET
                version_id = $2,
                name = $3, 
                description = $4
            WHERE id = $1
        "#,
            thread_id.into_inner(),
            version_id.into_inner(),
            patch.name.unwrap_or(thread.name),
            patch.description.unwrap_or(thread.description),
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(version_id)
    }

    async fn thread_delete(&self, thread_id: ThreadId, _user_id: UserId) -> Result<()> {
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

    async fn thread_undelete(&self, thread_id: ThreadId, _user_id: UserId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
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
}
