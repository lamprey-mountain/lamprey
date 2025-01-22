use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire, PgExecutor};
use tracing::info;
use uuid::Uuid;

use crate::error::Result;
use crate::types::{
    DbThread, PaginationDirection, PaginationQuery, PaginationResponse, RoomId, Thread,
    ThreadCreate, ThreadId, ThreadPatch, ThreadVerId, UserId,
};

use crate::data::DataThread;

use super::{Pagination, Postgres};

async fn thread_get_with_executor(
    exec: impl PgExecutor<'_>,
    thread_id: ThreadId,
    user_id: UserId,
) -> Result<Thread> {
    let row = query_as!(
        DbThread,
        r#"
        with last_id as (
            select thread_id, max(version_id) as last_version_id from message group by thread_id
        ), message_coalesced AS (
            select *
            from (select *, row_number() over(partition by id order by version_id desc) as row_num
                from message)
            where row_num = 1
        ),
        message_count as (
            select thread_id, count(*) as count
            from message_coalesced
            group by thread_id
        )
        select
            thread.id as id,
            thread.room_id as room_id,
            thread.creator_id as creator_id,
            thread.name as name,
            thread.description,
            thread.is_closed as is_closed,
            thread.is_locked as is_locked,
            false as "is_pinned!",
            coalesce(count, 0) as "message_count!",
            last_version_id as "last_version_id!",
            unread.version_id as "last_read_id?",
            coalesce(last_version_id != unread.version_id, true) as "is_unread!"
        from thread
        join message_count on message_count.thread_id = thread.id
        join last_id on last_id.thread_id = thread.id
        full outer join usr on true
        left join unread on usr.id = unread.user_id and thread.id = unread.thread_id
        where thread.id = $1 and usr.id = $2
    "#,
        thread_id.into_inner(),
        user_id.into_inner()
    )
    .fetch_one(exec)
    .await?;
    Ok(row.into())
}

#[async_trait]
impl DataThread for Postgres {
    async fn thread_create(&self, create: ThreadCreate) -> Result<ThreadId> {
        let thread_id = Uuid::now_v7();
        query!(
            "
			INSERT INTO thread (id, creator_id, room_id, name, description, is_closed, is_locked)
			VALUES ($1, $2, $3, $4, $5, $6, $7)
        ",
            thread_id,
            create.creator_id.into_inner(),
            create.room_id.into_inner(),
            create.name,
            create.description,
            create.is_closed,
            create.is_locked,
        )
        .execute(&self.pool)
        .await?;
        info!("inserted thread");
        Ok(ThreadId(thread_id))
    }

    /// get a thread, panics if there are no messages
    async fn thread_get(&self, thread_id: ThreadId, user_id: UserId) -> Result<Thread> {
        let mut conn = self.pool.acquire().await?;
        let thread = thread_get_with_executor(&mut *conn, thread_id, user_id).await?;
        Ok(thread)
    }

    async fn thread_list(
        &self,
        user_id: UserId,
        room_id: RoomId,
        pagination: PaginationQuery<ThreadId>,
    ) -> Result<PaginationResponse<Thread>> {
        let p: Pagination<_> = pagination.try_into()?;
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let items = query_as!(
            DbThread,
            r#"
        with last_id as (
            select thread_id, max(version_id) as last_version_id from message group by thread_id
        ), message_coalesced AS (
            select *
            from (select *, row_number() over(partition by id order by version_id desc) as row_num
                from message)
            where row_num = 1
        ),
        message_count as (
            select thread_id, count(*) as count
            from message_coalesced
            group by thread_id
        )
        select
            thread.id as id,
            thread.room_id as room_id,
            thread.creator_id as creator_id,
            thread.name as name,
            thread.description,
            thread.is_closed as is_closed,
            thread.is_locked as is_locked,
            false as "is_pinned!",
            coalesce(count, 0) as "message_count!",
            last_version_id as "last_version_id!",
            unread.version_id as "last_read_id?",
            coalesce(last_version_id != unread.version_id, true) as "is_unread!"
        from thread
        join message_count on message_count.thread_id = thread.id
        join last_id on last_id.thread_id = thread.id
        full outer join usr on true
        left join unread on usr.id = unread.user_id and thread.id = unread.thread_id
		where room_id = $1 AND user_id = $2 AND thread.id > $3 AND thread.id < $4
		order by (CASE WHEN $5 = 'f' THEN thread.id END), thread.id DESC LIMIT $6
            "#,
            room_id.into_inner(),
            user_id.into_inner(),
            p.after.into_inner(),
            p.before.into_inner(),
            p.dir.to_string(),
            (p.limit + 1) as i32
        )
        .fetch_all(&mut *tx)
        .await?;
        let total = query_scalar!(
            r#"SELECT count(*) FROM thread WHERE room_id = $1"#,
            room_id.into_inner()
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.rollback().await?;
        let has_more = items.len() > p.limit as usize;
        let mut items: Vec<_> = items
            .into_iter()
            .take(p.limit as usize)
            .map(Into::into)
            .collect();
        if p.dir == PaginationDirection::B {
            items.reverse();
        }
        Ok(PaginationResponse {
            items,
            total: total.unwrap_or(0) as u64,
            has_more,
        })
    }

    async fn thread_update(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        patch: ThreadPatch,
    ) -> Result<ThreadVerId> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let thread = thread_get_with_executor(&mut *tx, thread_id, user_id).await?;
        let version_id = ThreadVerId(Uuid::now_v7());
        query!(
            r#"
            UPDATE thread SET
                version_id = $2,
                name = $3, 
                description = $4,
                is_closed = $5,
                is_locked = $6
            WHERE id = $1
        "#,
            thread_id.into_inner(),
            version_id.into_inner(),
            patch.name.unwrap_or(thread.name),
            patch.description.unwrap_or(thread.description),
            patch.is_closed.unwrap_or(thread.is_closed),
            patch.is_locked.unwrap_or(thread.is_locked),
            // patch.is_pinned.unwrap_or(room.is_pinned),
        )
        .execute(&mut *tx)
        .await?;
        Ok(version_id)
    }
}
