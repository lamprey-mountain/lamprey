use async_trait::async_trait;
use common::v1::types::{
    PaginationDirection, PaginationQuery, PaginationResponse, RoomMember, RoomMemberPatch,
    RoomMembership,
};
use sqlx::{query, query_as, query_scalar, Acquire};
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
    pub membership_updated_at: time::PrimitiveDateTime,
    pub roles: Vec<Uuid>,
}

impl From<DbRoomMember> for RoomMember {
    fn from(row: DbRoomMember) -> Self {
        RoomMember {
            user_id: row.user_id.into(),
            room_id: row.room_id.into(),
            membership: match row.membership {
                DbMembership::Join => RoomMembership::Join {
                    override_name: row.override_name,
                    override_description: row.override_description,
                    roles: row.roles.into_iter().map(Into::into).collect(),
                },
                DbMembership::Leave => RoomMembership::Leave {},
                DbMembership::Ban => RoomMembership::Ban {},
            },
            membership_updated_at: row.membership_updated_at.assume_utc().into(),
        }
    }
}

#[async_trait]
impl DataRoomMember for Postgres {
    async fn room_member_put(
        &self,
        room_id: RoomId,
        user_id: UserId,
        membership: RoomMembership,
    ) -> Result<()> {
        let membership: DbMembership = membership.into();
        query!(
            r#"
            INSERT INTO room_member (user_id, room_id, membership)
            VALUES ($1, $2, $3)
			ON CONFLICT ON CONSTRAINT room_member_pkey DO UPDATE SET
    			membership = excluded.membership
            "#,
            user_id.into_inner(),
            room_id.into_inner(),
            membership as _
        )
        .execute(&self.pool)
        .await?;
        info!("inserted room member");
        Ok(())
    }

    async fn room_member_delete(&self, room_id: RoomId, user_id: UserId) -> Result<()> {
        query!(
            "DELETE FROM room_member WHERE room_id = $1 AND user_id = $2",
            room_id.into_inner(),
            user_id.into_inner(),
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
                    membership_updated_at,
                	coalesce(r.roles, '{}') as "roles!"
                FROM room_member m
                left join r on r.user_id = m.user_id
            	WHERE room_id = $1 AND m.user_id > $2 AND m.user_id < $3 AND membership = 'Join'
            	ORDER BY (CASE WHEN $4 = 'f' THEN m.user_id END), m.user_id DESC LIMIT $5
                "#,
                room_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                "SELECT count(*) FROM room_member WHERE room_id = $1 AND membership = 'Join'",
                room_id.into_inner()
            )
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
            	membership_updated_at, 
            	coalesce(r.roles, '{}') as "roles!"
            FROM room_member m
            left join r on r.user_id = m.user_id
            WHERE room_id = $1 AND m.user_id = $2
        "#,
            room_id.into_inner(),
            user_id.into_inner(),
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
            	membership_updated_at, 
            	coalesce(r.roles, '{}') as "roles!"
            FROM room_member m
            left join r on r.user_id = m.user_id
            WHERE room_id = $1 AND m.user_id = $2
        "#,
            room_id.into_inner(),
            user_id.into_inner(),
        )
        .fetch_one(&mut *tx)
        .await?;
        query!(
            r#"
            UPDATE room_member
        	SET override_name = $3, override_description = $4
            WHERE room_id = $1 AND user_id = $2 AND membership = 'Join'
        "#,
            room_id.into_inner(),
            user_id.into_inner(),
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
            room_id.into_inner(),
            user_id.into_inner(),
            membership as _,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
