use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire};
use tracing::info;
use uuid::Uuid;

use crate::error::Result;
use crate::gen_paginate;
use crate::types::{
    DbRoom, PaginationDirection, PaginationQuery, PaginationResponse, Room, RoomCreate, RoomId,
    RoomPatch, RoomVerId, UserId,
};

use crate::data::DataRoom;

use super::{Pagination, Postgres};

#[async_trait]
impl DataRoom for Postgres {
    async fn room_create(&self, create: RoomCreate) -> Result<Room> {
        let mut conn = self.pool.acquire().await?;
        let room_id = Uuid::now_v7();
        query!(
            "
    	    INSERT INTO room (id, version_id, name, description, icon, public)
    	    VALUES ($1, $2, $3, $4, $5, $6)
        ",
            room_id,
            room_id,
            create.name,
            create.description,
            create.icon.map(|i| *i),
            create.public.unwrap_or(false),
        )
        .execute(&mut *conn)
        .await?;
        info!("inserted room");
        self.room_get(room_id.into()).await
    }

    async fn room_get(&self, id: RoomId) -> Result<Room> {
        let id: Uuid = id.into();
        let mut conn = self.pool.acquire().await?;
        let room = query_as!(
            DbRoom,
            r#"
            SELECT
                room.id,
                room.version_id,
                room.name,
                room.description,
                room.icon,
                room.archived_at,
                room.public,
                NULL::uuid as dm_uid_a,
                NULL::uuid as dm_uid_b
            FROM room
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(room.into())
    }

    async fn room_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<RoomId>,
    ) -> Result<PaginationResponse<Room>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbRoom,
                r#"
                SELECT
                    room.id,
                    room.version_id,
                    room.name,
                    room.description,
                    room.icon,
                    room.archived_at,
                    room.public,
                    NULL::uuid as dm_uid_a,
                    NULL::uuid as dm_uid_b
                FROM room_member
            	JOIN room ON room_member.room_id = room.id
            	WHERE room_member.user_id = $1 AND room.id > $2 AND room.id < $3 AND room_member.membership = 'Join'
            	ORDER BY (CASE WHEN $4 = 'f' THEN room.id END), room.id DESC LIMIT $5
                "#,
                user_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                "SELECT count(*) FROM room_member WHERE room_member.user_id = $1",
                user_id.into_inner()
            ),
            |i: &Room| i.id.to_string()
        )
    }

    async fn room_update(&self, id: RoomId, patch: RoomPatch) -> Result<RoomVerId> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let room = query!(
            r#"
            SELECT id, name, description, icon, archived_at, public
            FROM room
            WHERE id = $1
            FOR UPDATE
            "#,
            id.into_inner()
        )
        .fetch_one(&mut *tx)
        .await?;
        let version_id = RoomVerId::new();
        query!(
            "UPDATE room SET version_id = $2, name = $3, description = $4, icon = $5, public = $6 WHERE id = $1",
            id.into_inner(),
            version_id.into_inner(),
            patch.name.unwrap_or(room.name),
            patch.description.unwrap_or(room.description),
            patch.icon.map(|i| i.map(|i| *i)).unwrap_or(room.icon),
            patch.public.unwrap_or(room.public),
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(version_id)
    }
}
