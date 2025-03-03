use async_trait::async_trait;
use sqlx::{query, query_file_as, query_scalar, Acquire};
use tracing::info;
use types::ThreadState;

use crate::error::Result;
use crate::gen_paginate;
use crate::types::{
    DbThread, DbThreadCreate, DbThreadState, PaginationDirection, PaginationQuery,
    PaginationResponse, RoomId, Thread, ThreadId, ThreadPatch, ThreadVerId, UserId,
};

use crate::data::DataThread;

use super::{Pagination, Postgres};

#[async_trait]
impl DataThread for Postgres {
    async fn thread_create(&self, create: DbThreadCreate) -> Result<ThreadId> {
        let thread_id = ThreadId::new();
        query!(
            "
			INSERT INTO thread (id, version_id, creator_id, room_id, name, description, state, visibility)
			VALUES ($1, $2, $3, $4, $5, $6, 'Active', 'Room')
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
        let state = match patch.state.unwrap_or(thread.state) {
            // ThreadState::Pinned { .. } => DbThreadState::Pinned,
            ThreadState::Pinned { .. } => todo!(),
            ThreadState::Active => DbThreadState::Active,
            ThreadState::Temporary => DbThreadState::Temporary,
            ThreadState::Archived => DbThreadState::Archived,
            ThreadState::Deleted => DbThreadState::Deleted,
        };
        let version_id = ThreadVerId::new();
        query!(
            r#"
            UPDATE thread SET
                version_id = $2,
                name = $3, 
                description = $4,
                state = $5
            WHERE id = $1
        "#,
            thread_id.into_inner(),
            version_id.into_inner(),
            patch.name.unwrap_or(thread.name),
            patch.description.unwrap_or(thread.description),
            state as _,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(version_id)
    }
}
