use async_trait::async_trait;
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
    	    INSERT INTO room (id, version_id, name, description, icon, public, type, quarantined, security_require_mfa, security_require_sudo, afk_channel_id, afk_channel_timeout)
    	    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ",
            room_id,
            room_id,
            create.name,
            create.description,
            create.icon.map(|i| *i),
            create.public.unwrap_or(false),
            ty as _,
            false,
            false,
            false,
            None::<uuid::Uuid>,
            300000,
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
                room.welcome_channel_id,
                (SELECT COUNT(*) FROM room_member WHERE room_id = room.id AND membership = 'Join') AS "member_count!",
                (SELECT COUNT(*) FROM channel WHERE room_id = room.id AND deleted_at IS NULL AND archived_at IS NULL) AS "channel_count!",
                room.quarantined,
                room.security_require_mfa,
                room.security_require_sudo,
                room.afk_channel_id,
                room.afk_channel_timeout
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
                    room.welcome_channel_id,
                    (SELECT COUNT(*) FROM room_member WHERE room_id = room.id AND membership = 'Join') AS "member_count!",
                    (SELECT COUNT(*) FROM channel WHERE room_id = room.id AND deleted_at IS NULL AND archived_at IS NULL) AS "channel_count!",
                    room.quarantined,
                    room.security_require_mfa,
                    room.security_require_sudo,
                    room.afk_channel_id,
                    room.afk_channel_timeout
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
                    room.welcome_channel_id,
                    (SELECT COUNT(*) FROM room_member WHERE room_id = room.id AND membership = 'Join') AS "member_count!",
                    (SELECT COUNT(*) FROM channel WHERE room_id = room.id AND deleted_at IS NULL AND archived_at IS NULL) AS "channel_count!",
                    room.quarantined,
                    room.security_require_mfa,
                    room.security_require_sudo,
                    room.afk_channel_id,
                    room.afk_channel_timeout
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
            SELECT id, name, description, icon, archived_at, public, welcome_channel_id, quarantined, security_require_mfa, security_require_sudo, afk_channel_id, afk_channel_timeout
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
            "UPDATE room SET version_id = $2, name = $3, description = $4, icon = $5, public = $6, welcome_channel_id = $7, afk_channel_id = $8, afk_channel_timeout = $9 WHERE id = $1",
            id.into_inner(),
            version_id.into_inner(),
            patch.name.unwrap_or(room.name),
            patch.description.unwrap_or(room.description),
            patch.icon.map(|i| i.map(|i| *i)).unwrap_or(room.icon),
            patch.public.unwrap_or(room.public),
            patch.welcome_channel_id.map(|i| i.map(|i| *i)).unwrap_or(room.welcome_channel_id),
            patch.afk_channel_id.map(|i| i.map(|i| *i)).unwrap_or(room.afk_channel_id),
            patch.afk_channel_timeout.map(|i| i as i64).unwrap_or(room.afk_channel_timeout),
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
                    r.welcome_channel_id,
                    r.quarantined,
                    r.security_require_mfa,
                    r.security_require_sudo,
                    r.afk_channel_id,
                    r.afk_channel_timeout,
                    (SELECT COUNT(*) FROM room_member WHERE room_id = r.id AND membership = 'Join') AS "member_count!",
                    (SELECT COUNT(*) FROM channel WHERE room_id = r.id AND deleted_at IS NULL AND archived_at IS NULL) AS "channel_count!"
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

    async fn room_quarantine(&self, room_id: RoomId) -> Result<RoomVerId> {
        let version_id = RoomVerId::new();
        query!(
            r#"update room set quarantined = true, version_id = $2 where id = $1"#,
            *room_id,
            *version_id
        )
        .execute(&self.pool)
        .await?;
        Ok(version_id)
    }

    async fn room_unquarantine(&self, room_id: RoomId) -> Result<RoomVerId> {
        let version_id = RoomVerId::new();
        query!(
            r#"update room set quarantined = false, version_id = $2 where id = $1"#,
            *room_id,
            *version_id
        )
        .execute(&self.pool)
        .await?;
        Ok(version_id)
    }

    async fn user_room_count(&self, user_id: UserId) -> Result<u64> {
        let count = query_scalar!(
            r#"
            SELECT count(*) FROM room_member
            WHERE user_id = $1 AND membership = 'Join'
            "#,
            *user_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count.unwrap_or(0) as u64)
    }

    async fn room_security_update(
        &self,
        room_id: RoomId,
        require_mfa: Option<bool>,
        require_sudo: Option<bool>,
    ) -> Result<RoomVerId> {
        let mut tx = self.pool.begin().await?;
        let room = query!(
            r#"
            SELECT security_require_mfa, security_require_sudo
            FROM room
            WHERE id = $1
            FOR UPDATE
            "#,
            room_id.into_inner()
        )
        .fetch_one(&mut *tx)
        .await?;

        let new_require_mfa = require_mfa.unwrap_or(room.security_require_mfa);
        let new_require_sudo = require_sudo.unwrap_or(room.security_require_sudo);

        let version_id = RoomVerId::new();
        query!(
            "UPDATE room SET version_id = $2, security_require_mfa = $3, security_require_sudo = $4 WHERE id = $1",
            room_id.into_inner(),
            version_id.into_inner(),
            new_require_mfa,
            new_require_sudo
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(version_id)
    }
}
