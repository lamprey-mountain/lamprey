use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire, PgPool};
use tracing::info;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{
    Identifier, Media, MediaId, MediaLink, MediaLinkType, Message, MessageCreate, MessageId, MessageType, MessageVerId, PaginationDirection, PaginationQuery, PaginationResponse, Permission, Role, RoleCreate, RoleId, Room, RoomCreate, RoomId, RoomMemberPut, RoomPatch, RoomVerId, Thread, ThreadCreate, ThreadId, UserId
};

use crate::data::{
    DataMedia, DataMessage, DataPermission, DataRole, DataRoleMember, DataRoom, DataRoomMember, DataThread, DataUnread
};

use super::{Pagination, Postgres};

#[async_trait]
impl DataThread for Postgres {
    async fn thread_create(&self, create: ThreadCreate) -> Result<ThreadId> {
        let mut conn = self.pool.acquire().await?;
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
        .execute(&mut *conn)
        .await?;
        info!("inserted thread");
        Ok(ThreadId(thread_id))
    }

    /// get a thread, panics if there are no messages
    async fn thread_get(&self, thread_id: ThreadId, user_id: UserId) -> Result<Thread> {
        let mut conn = self.pool.acquire().await?;
        let row = query!(r#"
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
        "#, thread_id.into_inner(), user_id.into_inner())
            .fetch_one(&mut *conn)
            .await?;
        let thread = Thread {
            id: row.id.into(),
            room_id: row.room_id.into(),
            creator_id: row.creator_id.into(),
            name: row.name,
            description: row.description,
            is_closed: row.is_closed,
            is_locked: row.is_locked,
            is_pinned: row.is_pinned,
            is_unread: row.is_unread,
            last_version_id: row.last_version_id.into(),
            last_read_id: row.last_read_id.map(Into::into),
            message_count: row.message_count.try_into().expect("count is negative?"),
        };
        Ok(thread)
    }

    async fn thread_list(
            &self,
            user_id: UserId,
            room_id: RoomId,
            pagination: PaginationQuery<ThreadId>,
        ) -> Result<PaginationResponse<Thread>> {
        todo!()
    }

    async fn thread_update(&self, id: ThreadId, patch: crate::types::ThreadPatch) -> Result<crate::types::ThreadVerId> {
        todo!()
    }
}
