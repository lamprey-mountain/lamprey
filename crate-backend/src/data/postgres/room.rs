use async_trait::async_trait;
use common::v1::types::RoomMetrics;
use sqlx::{query, query_as, query_scalar, Acquire};
use tracing::info;
use uuid::Uuid;

use crate::error::Result;
use crate::types::{
    DbRoom, DbRoomCreate, DbRoomType, PaginationDirection, PaginationQuery, PaginationResponse,
    Room, RoomCreate, RoomId, RoomPatch, RoomVerId, UserId,
};
use crate::{gen_paginate, Error};

use crate::data::DataRoom;

use super::{Pagination, Postgres};

#[async_trait]
impl DataRoom for Postgres {
    async fn room_create(&self, create: RoomCreate, extra: DbRoomCreate) -> Result<Room> {
        let mut conn = self.pool.acquire().await?;
        let room_id = extra.id.map(|i| *i).unwrap_or_else(Uuid::now_v7);
        let ty: DbRoomType = extra.ty.into();
        query!(
            "
    	    INSERT INTO room (id, version_id, name, description, icon, public, type)
    	    VALUES ($1, $2, $3, $4, $5, $6, $7)
        ",
            room_id,
            room_id,
            create.name,
            create.description,
            create.icon.map(|i| *i),
            create.public.unwrap_or(false),
            ty as _,
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
                room.type as "ty: _",
                room.name,
                room.description,
                room.icon,
                room.archived_at,
                room.public,
                room.owner_id,
                room.welcome_thread_id
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
        include_server_room: bool,
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
                    room.type as "ty: _",
                    room.name,
                    room.description,
                    room.icon,
                    room.archived_at,
                    room.public,
                    room.owner_id,
                    room.welcome_thread_id
                FROM room_member
            	JOIN room ON room_member.room_id = room.id
            	WHERE room_member.user_id = $1 AND room.id > $2 AND room.id < $3
            	  AND room_member.membership = 'Join'
            	  AND (room.type != 'Server' OR $6)
            	ORDER BY (CASE WHEN $4 = 'f' THEN room.id END), room.id DESC LIMIT $5
                "#,
                *user_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32,
                include_server_room,
            ),
            query_scalar!(
                r#"
                SELECT count(*) FROM room_member rm
                JOIN room ON room.id = rm.room_id
                WHERE rm.user_id = $1 AND rm.membership = 'Join'
                  AND (room.type != 'Server' OR $2)
                "#,
                *user_id,
                include_server_room,
            ),
            |i: &Room| i.id.to_string()
        )
    }

    async fn room_list_all(
        &self,
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
                    room.type as "ty: _",
                    room.name,
                    room.description,
                    room.icon,
                    room.archived_at,
                    room.public,
                    room.owner_id,
                    room.welcome_thread_id
                FROM room
                WHERE room.id > $1 AND room.id < $2
                ORDER BY (CASE WHEN $3 = 'f' THEN room.id END), room.id DESC LIMIT $4
                "#,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32,
            ),
            query_scalar!(
                r#"
                SELECT count(*) FROM room
                "#,
            ),
            |i: &Room| i.id.to_string()
        )
    }

    async fn room_update(&self, id: RoomId, patch: RoomPatch) -> Result<RoomVerId> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let room = query!(
            r#"
            SELECT id, name, description, icon, archived_at, public, welcome_thread_id
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
            "UPDATE room SET version_id = $2, name = $3, description = $4, icon = $5, public = $6, welcome_thread_id = $7 WHERE id = $1",
            id.into_inner(),
            version_id.into_inner(),
            patch.name.unwrap_or(room.name),
            patch.description.unwrap_or(room.description),
            patch.icon.map(|i| i.map(|i| *i)).unwrap_or(room.icon),
            patch.public.unwrap_or(room.public),
            patch.welcome_thread_id.map(|i| i.map(|i| *i)).unwrap_or(room.welcome_thread_id),
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(version_id)
    }

    async fn room_list_mutual(
        &self,
        user_a_id: UserId,
        user_b_id: UserId,
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
                    r.id,
                    r.version_id,
                    r.type as "ty: _",
                    r.name,
                    r.description,
                    r.icon,
                    r.archived_at,
                    r.public,
                    r.owner_id,
                    r.welcome_thread_id
                FROM room_member rm1
                JOIN room_member rm2 ON rm1.room_id = rm2.room_id
                JOIN room r ON rm1.room_id = r.id
                WHERE rm1.user_id = $1 AND rm2.user_id = $2
                  AND rm1.membership = 'Join' AND rm2.membership = 'Join'
                  AND r.id > $3 AND r.id < $4
                  AND r.type != 'Server'
                ORDER BY (CASE WHEN $5 = 'f' THEN r.id END), r.id DESC
                LIMIT $6
                "#,
                *user_a_id,
                *user_b_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r#"
                SELECT count(*)
                FROM room_member rm1
                JOIN room_member rm2 ON rm1.room_id = rm2.room_id
                JOIN room r ON rm1.room_id = r.id
                WHERE rm1.user_id = $1 AND rm2.user_id = $2
                  AND rm1.membership = 'Join' AND rm2.membership = 'Join'
                  AND r.type != 'Server'
                "#,
                *user_a_id,
                *user_b_id,
            ),
            |i: &Room| i.id.to_string()
        )
    }

    async fn room_metrics(&self, room_id: RoomId) -> Result<RoomMetrics> {
        let thread_count =
            query_scalar!("select count(*) from thread where room_id = $1", *room_id)
                .fetch_one(&self.pool)
                .await?
                .unwrap_or_default();
        let active_thread_count = query_scalar!(
            "select count(*) from thread where room_id = $1 and archived_at is null",
            *room_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or_default();
        let member_count = query_scalar!(
            "select count(*) from room_member where room_id = $1",
            *room_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or_default();
        let message_media_counts = query!(
            r#"
            select
                count(distinct s.id) as total_messages,
                count(distinct l.media_id) as total_media
            from thread t
            join message s on s.thread_id = t.id
            left join media_link l
                   on l.target_id = s.id
                  and l.link_type = 'Message'
            where t.room_id = $1
            "#,
            *room_id
        )
        .fetch_one(&self.pool)
        .await?;
        let media_size = query_scalar!(
            r#"
            select sum((m.data->'source'->'size')::int) as total_size
            from room r
            join thread t on t.room_id = r.id
            join message s on s.thread_id = t.id
            join media_link l on l.target_id = s.id and l.link_type = 'Message'
            join media m on m.id = l.media_id
            where r.id = $1
            "#,
            *room_id
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or_default();

        Ok(RoomMetrics {
            thread_count: thread_count as u64,
            active_thread_count: active_thread_count as u64,
            message_count: message_media_counts.total_messages.unwrap_or_default() as u64,
            member_count: member_count as u64,
            media_count: message_media_counts.total_media.unwrap_or_default() as u64,
            media_size: media_size as u64,
        })
    }

    async fn room_set_owner(&self, id: RoomId, owner_id: UserId) -> Result<RoomVerId> {
        let version_id = RoomVerId::new();
        query!(
            r#"update room set owner_id = $2, version_id = $3 where id = $1"#,
            *id,
            *owner_id,
            *version_id
        )
        .execute(&self.pool)
        .await?;
        Ok(version_id)
    }

    async fn room_delete(&self, room_id: RoomId) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let version_id = RoomVerId::new();
        query!(
            r#"update room set deleted_at = now(), version_id = $2 where id = $1"#,
            *room_id,
            *version_id
        )
        .execute(&mut *tx)
        .await?;
        query!(
            r#"update room_member set membership = 'Leave', left_at = now() where room_id = $1 and membership = 'Join'"#,
            *room_id,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn room_undelete(&self, room_id: RoomId) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let room_deleted_at =
            query_scalar!(r#"select deleted_at from room where id = $1"#, *room_id)
                .fetch_one(&mut *tx)
                .await?
                .ok_or(Error::BadStatic("room is not deleted"))?;

        let version_id = RoomVerId::new();
        query!(
            r#"update room set deleted_at = null, version_id = $2 where id = $1"#,
            *room_id,
            *version_id
        )
        .execute(&mut *tx)
        .await?;

        query!(
            r#"
            update room_member
            set membership = 'Join', left_at = null
            where room_id = $1
            and membership = 'Leave'
            and left_at = $2
            "#,
            *room_id,
            room_deleted_at
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}
