use async_trait::async_trait;
use common::v1::types::util::Time;
use common::v1::types::{
    PaginationDirection, PaginationQuery, PaginationResponse, RoomBan, RoomMember,
    RoomMemberOrigin, RoomMemberPatch, RoomMemberPut, RoomMembership,
};
use sqlx::{query, query_as, query_scalar, Acquire};
use time::PrimitiveDateTime;
use tracing::info;
use uuid::Uuid;

use crate::error::Result;
use crate::gen_paginate;
use crate::types::{DbMembership, RoomId, UserId};

use crate::data::DataRoomMember;

use super::{Pagination, Postgres};

pub struct DbRoomMember {
    pub user_id: Uuid,
    pub room_id: Uuid,
    pub membership: DbMembership,
    pub override_name: Option<String>,
    pub override_description: Option<String>,
    // override_avatar: z.string().url().or(z.literal("")),
    pub joined_at: time::PrimitiveDateTime,
    pub roles: Vec<Uuid>,
    pub origin: Option<serde_json::Value>,
}

pub struct DbRoomBan {
    pub user_id: UserId,
    pub reason: Option<String>,
    pub created_at: time::PrimitiveDateTime,
    pub expires_at: Option<time::PrimitiveDateTime>,
}

impl From<DbRoomBan> for RoomBan {
    fn from(row: DbRoomBan) -> Self {
        RoomBan {
            user_id: row.user_id,
            reason: row.reason,
            created_at: row.created_at.assume_utc().into(),
            expires_at: row.expires_at.map(|t| t.assume_utc().into()),
        }
    }
}

impl From<DbRoomMember> for RoomMember {
    fn from(row: DbRoomMember) -> Self {
        RoomMember {
            user_id: row.user_id.into(),
            room_id: row.room_id.into(),
            membership: match row.membership {
                DbMembership::Join => RoomMembership::Join,
                DbMembership::Leave => RoomMembership::Leave,
                DbMembership::Ban => RoomMembership::Leave,
            },
            membership_updated_at: row.joined_at.assume_utc().into(),
            joined_at: row.joined_at.assume_utc().into(),
            override_name: row.override_name,
            override_description: row.override_description,
            roles: row.roles.into_iter().map(Into::into).collect(),

            // FIXME: only return for moderators
            origin: row
                .origin
                .map(|o| serde_json::from_value(o).expect("invalid data in db")),
        }
    }
}

#[async_trait]
impl DataRoomMember for Postgres {
    async fn room_member_put(
        &self,
        room_id: RoomId,
        user_id: UserId,
        origin: RoomMemberOrigin,
        put: RoomMemberPut,
    ) -> Result<()> {
        query!(
            r#"
            INSERT INTO room_member (user_id, room_id, membership, override_name, override_description, joined_at, origin)
            VALUES ($1, $2, $3, $4, $5, now(), $6)
			ON CONFLICT ON CONSTRAINT room_member_pkey DO UPDATE SET
    			membership = excluded.membership,
                joined_at = case
                    when excluded.membership = 'Leave'
                    then now()
                    else room_member.joined_at
                end
            "#,
            *user_id,
            *room_id,
            DbMembership::Join as _,
            put.override_name,
            put.override_description,
            &serde_json::to_value(origin)?,
        )
        .execute(&self.pool)
        .await?;
        info!("inserted room member");
        Ok(())
    }

    async fn room_member_delete(&self, room_id: RoomId, user_id: UserId) -> Result<()> {
        query!(
            "DELETE FROM room_member WHERE room_id = $1 AND user_id = $2",
            *room_id,
            *user_id,
        )
        .execute(&self.pool)
        .await?;
        info!("deleted room member");
        Ok(())
    }

    async fn room_member_list(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RoomMember>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbRoomMember,
                r#"
                with r as (
                    select user_id, array_agg(role_id) as roles from role_member
                    join role on role.room_id = $1 and role_member.role_id = role.id
                    group by user_id
                )
            	SELECT
                	room_id,
                	m.user_id,
                	membership as "membership: _",
                	override_name,
                    override_description,
                    joined_at,
                	origin,
                	coalesce(r.roles, '{}') as "roles!"
                FROM room_member m
                left join r on r.user_id = m.user_id
            	WHERE room_id = $1 AND m.user_id > $2 AND m.user_id < $3 AND membership = 'Join'
            	ORDER BY (CASE WHEN $4 = 'f' THEN m.user_id END), m.user_id DESC LIMIT $5
                "#,
                *room_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                "SELECT count(*) FROM room_member WHERE room_id = $1 AND membership = 'Join'",
                *room_id
            ),
            |i: &RoomMember| i.user_id.to_string()
        )
    }

    async fn room_member_get(&self, room_id: RoomId, user_id: UserId) -> Result<RoomMember> {
        let item = query_as!(
            DbRoomMember,
            r#"
            with r as (
                select user_id, array_agg(role_id) as roles from role_member
                join role on role.room_id = $1 and role_member.role_id = role.id
                group by user_id
            )
        	SELECT
            	room_id,
            	m.user_id,
            	membership as "membership: _",
            	override_name,
            	override_description,
            	joined_at,
            	origin,
            	coalesce(r.roles, '{}') as "roles!"
            FROM room_member m
            left join r on r.user_id = m.user_id
            WHERE room_id = $1 AND m.user_id = $2
        "#,
            *room_id,
            *user_id,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(item.into())
    }

    async fn room_member_patch(
        &self,
        room_id: RoomId,
        user_id: UserId,
        patch: RoomMemberPatch,
    ) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let item = query_as!(
            DbRoomMember,
            r#"
            with r as (
                select user_id, array_agg(role_id) as roles from role_member
                join role on role.room_id = $1 and role_member.role_id = role.id
                group by user_id
            )
        	SELECT
            	room_id,
            	m.user_id,
            	membership as "membership: _",
            	override_name,
            	override_description,
            	joined_at,
            	origin,
            	coalesce(r.roles, '{}') as "roles!"
            FROM room_member m
            left join r on r.user_id = m.user_id
            WHERE room_id = $1 AND m.user_id = $2
        "#,
            *room_id,
            *user_id,
        )
        .fetch_one(&mut *tx)
        .await?;
        query!(
            r#"
            UPDATE room_member
        	SET override_name = $3, override_description = $4
            WHERE room_id = $1 AND user_id = $2 AND membership = 'Join'
        "#,
            *room_id,
            *user_id,
            patch.override_name.unwrap_or(item.override_name),
            patch
                .override_description
                .unwrap_or(item.override_description),
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn room_member_set_membership(
        &self,
        room_id: RoomId,
        user_id: UserId,
        membership: RoomMembership,
    ) -> Result<()> {
        let membership: DbMembership = membership.into();
        query!(
            r#"
            UPDATE room_member
        	SET membership = $3, membership_updated_at = now()
            WHERE room_id = $1 AND user_id = $2
            "#,
            *room_id,
            *user_id,
            membership as _,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn room_ban_create(
        &self,
        room_id: RoomId,
        ban_id: UserId,
        reason: Option<String>,
        expires_at: Option<Time>,
    ) -> Result<()> {
        query!(
            r#"
            INSERT INTO room_ban (room_id, user_id, reason, created_at, expires_at)
            VALUES ($1, $2, $3, now(), $4)
            ON CONFLICT (room_id, user_id) DO UPDATE
            SET expires_at = EXCLUDED.expires_at
            "#,
            *room_id,
            *ban_id,
            reason,
            expires_at.map(|t| PrimitiveDateTime::new(t.date(), t.time())),
        )
        .execute(&self.pool)
        .await?;
        info!("inserted room ban");
        Ok(())
    }

    async fn room_ban_delete(&self, room_id: RoomId, ban_id: UserId) -> Result<()> {
        query!(
            "DELETE FROM room_ban WHERE room_id = $1 AND user_id = $2",
            *room_id,
            *ban_id
        )
        .execute(&self.pool)
        .await?;
        info!("deleted room ban");
        Ok(())
    }

    async fn room_ban_get(&self, room_id: RoomId, ban_id: UserId) -> Result<RoomBan> {
        let row = query_as!(
            DbRoomBan,
            r#"
            SELECT user_id, reason, created_at, expires_at
            FROM room_ban
            WHERE room_id = $1 AND user_id = $2
            "#,
            *room_id,
            *ban_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn room_ban_list(
        &self,
        room_id: RoomId,
        paginate: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RoomBan>> {
        let p: Pagination<_> = paginate.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbRoomBan,
                r#"
                SELECT user_id, reason, created_at, expires_at
                FROM room_ban
                WHERE room_id = $1 AND user_id > $2 AND user_id < $3
                ORDER BY (CASE WHEN $4 = 'f' THEN user_id END), user_id DESC
                LIMIT $5
                "#,
                *room_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!("SELECT count(*) FROM room_ban WHERE room_id = $1", *room_id),
            |i: &RoomBan| i.user_id.to_string()
        )
    }
}
