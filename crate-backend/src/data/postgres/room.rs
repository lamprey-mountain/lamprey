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
impl DataRoom for Postgres {
    async fn room_create(&self, create: RoomCreate) -> Result<Room> {
        let mut conn = self.pool.acquire().await?;
        let room_id = Uuid::now_v7();
        let room = query_as!(
            Room,
            "
    	    INSERT INTO room (id, version_id, name, description)
    	    VALUES ($1, $2, $3, $4)
    	    RETURNING id, version_id, name, description
        ",
            room_id,
            room_id,
            create.name,
            create.description
        )
        .fetch_one(&mut *conn)
        .await?;
        info!("inserted room");
        Ok(room)
    }

    async fn room_get(&self, id: RoomId) -> Result<Room> {
        let id: Uuid = id.into();
        let mut conn = self.pool.acquire().await?;
        let room = query_as!(
            Room,
            "SELECT id, version_id, name, description FROM room WHERE id = $1",
            id
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(room)
    }

    async fn room_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<RoomId>,
    ) -> Result<PaginationResponse<Room>> {
        let p: Pagination<_> = pagination.try_into()?;
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let rooms = query_as!(
            Room,
            "
        	SELECT room.id, room.version_id, room.name, room.description FROM room_member
        	JOIN room ON room_member.room_id = room.id
        	WHERE room_member.user_id = $1 AND room.id > $2 AND room.id < $3
        	ORDER BY (CASE WHEN $4 = 'f' THEN room.id END), room.id DESC LIMIT $5
        ",
            user_id.into_inner(),
            p.after.into_inner(),
            p.before.into_inner(),
            p.dir.to_string(),
            (p.limit + 1) as i32
        )
        .fetch_all(&mut *tx)
        .await?;
        let total = query_scalar!(
            "SELECT count(*) FROM room_member WHERE room_member.user_id = $1",
            user_id.into_inner()
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.rollback().await?;
        let has_more = rooms.len() > p.limit as usize;
        let mut items: Vec<_> = rooms.into_iter().take(p.limit as usize).collect();
        if p.dir == PaginationDirection::B {
            items.reverse();
        }
        Ok(PaginationResponse {
            items,
            total: total.unwrap_or(0) as u64,
            has_more,
        })
    }

    async fn room_update(&self, id: RoomId, patch: RoomPatch) -> Result<RoomVerId> {
        todo!()
    }
}
