use async_trait::async_trait;
use common::v1::types::util::Time;
use common::v1::types::{
    ApplicationId, PaginationDirection, PaginationQuery, PaginationResponse, RoomBan, RoomMember,
    RoomMemberOrigin, RoomMemberPatch, RoomMemberPut, RoomMemberSearchAdvanced,
    RoomMemberSearchResponse, RoomMembership, User,
};
use sqlx::{query, query_as, query_file_as, query_scalar, Acquire};
use time::PrimitiveDateTime;
use tracing::info;
use uuid::Uuid;

use crate::data::postgres::user::DbUser;
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
    pub joined_at: time::PrimitiveDateTime,
    pub roles: Vec<Uuid>,
    pub origin: Option<serde_json::Value>,
    pub mute: bool,
    pub deaf: bool,
    pub timeout_until: Option<time::PrimitiveDateTime>,
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
        let mut roles: Vec<_> = row.roles.into_iter().map(Into::into).collect();
        roles.sort();
        RoomMember {
            user_id: row.user_id.into(),
            room_id: row.room_id.into(),
            membership: match row.membership {
                DbMembership::Join => RoomMembership::Join,
                DbMembership::Leave => RoomMembership::Leave,
                DbMembership::Ban => RoomMembership::Leave,
            },
            joined_at: row.joined_at.assume_utc().into(),
            override_name: row.override_name,
            override_description: row.override_description,
            roles,
            mute: row.mute,
            deaf: row.deaf,
            timeout_until: row.timeout_until.map(|t| t.assume_utc().into()),

            // FIXME: only return for moderators
            origin: row
                .origin
                .map(|o| serde_json::from_value(o).expect("invalid data in db")),
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct DbRoomMemberWithUser {
    pub user_id: Uuid,
    pub room_id: Uuid,
    pub membership: DbMembership,
    pub override_name: Option<String>,
    pub override_description: Option<String>,
    pub joined_at: time::PrimitiveDateTime,
    pub roles: Vec<Uuid>,
    pub origin: Option<serde_json::Value>,
    pub mute: bool,
    pub deaf: bool,
    pub timeout_until: Option<time::PrimitiveDateTime>,
    pub u_id: Uuid,
    pub u_version_id: Uuid,
    pub u_parent_id: Option<Uuid>,
    pub u_name: String,
    pub u_description: Option<String>,
    pub u_avatar: Option<Uuid>,
    pub u_banner: Option<Uuid>,
    pub u_puppet: Option<serde_json::Value>,
    pub u_system: bool,
    pub u_suspended: Option<serde_json::Value>,
    pub u_registered_at: Option<time::PrimitiveDateTime>,
    pub u_deleted_at: Option<time::PrimitiveDateTime>,
    pub u_app_owner_id: Option<Uuid>,
    pub u_app_bridge: Option<bool>,
    pub u_app_public: Option<bool>,
    pub u_webhook_channel_id: Option<Uuid>,
    pub u_webhook_creator_id: Option<Uuid>,
    pub u_webhook_room_id: Option<Uuid>,
}

impl From<DbRoomMemberWithUser> for (RoomMember, User) {
    fn from(row: DbRoomMemberWithUser) -> Self {
        let room_member = RoomMember {
            user_id: row.user_id.into(),
            room_id: row.room_id.into(),
            membership: match row.membership {
                DbMembership::Join => RoomMembership::Join,
                DbMembership::Leave => RoomMembership::Leave,
                DbMembership::Ban => RoomMembership::Leave,
            },
            joined_at: row.joined_at.assume_utc().into(),
            override_name: row.override_name,
            override_description: row.override_description,
            roles: row.roles.into_iter().map(Into::into).collect(),
            origin: row
                .origin
                .map(|o| serde_json::from_value(o).expect("invalid data in db")),
            mute: row.mute,
            deaf: row.deaf,
            timeout_until: row.timeout_until.map(|t| t.assume_utc().into()),
        };

        let user: User = DbUser {
            id: row.u_id.into(),
            version_id: row.u_version_id.into(),
            parent_id: row.u_parent_id,
            name: row.u_name,
            description: row.u_description,
            avatar: row.u_avatar,
            banner: row.u_banner,
            puppet: row.u_puppet,
            system: row.u_system,
            suspended: row.u_suspended,
            registered_at: row.u_registered_at,
            deleted_at: row.u_deleted_at,
            app_owner_id: row.u_app_owner_id,
            app_bridge: row.u_app_bridge,
            app_public: row.u_app_public,
            webhook_channel_id: row.u_webhook_channel_id,
            webhook_creator_id: row.u_webhook_creator_id,
            webhook_room_id: row.u_webhook_room_id,
        }
        .into();

        (room_member, user)
    }
}

#[async_trait]
impl DataRoomMember for Postgres {
    async fn room_member_put(
        &self,
        room_id: RoomId,
        user_id: UserId,
        origin: Option<RoomMemberOrigin>,
        put: RoomMemberPut,
    ) -> Result<()> {
        query!(
            r#"
            INSERT INTO room_member (user_id, room_id, membership, override_name, override_description, joined_at, origin, mute, deaf, timeout_until)
            VALUES ($1, $2, $3, $4, $5, now(), $6, $7, $8, $9)
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
            origin.and_then(|o| serde_json::to_value(o).ok()),
            put.mute.unwrap_or(false),
            put.deaf.unwrap_or(false),
            put.timeout_until.map(|t| PrimitiveDateTime::new(t.date(), t.time())),
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
                    mute,
                    deaf,
                    timeout_until,
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
                mute,
                deaf,
                timeout_until,
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
                mute,
                deaf,
                timeout_until,
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
        	SET override_name = $3, override_description = $4, mute = $5, deaf = $6, timeout_until = $7
            WHERE room_id = $1 AND user_id = $2 AND membership = 'Join'
        "#,
            *room_id,
            *user_id,
            patch.override_name.unwrap_or(item.override_name),
            patch
                .override_description
                .unwrap_or(item.override_description),
            patch.mute.unwrap_or(item.mute),
            patch.deaf.unwrap_or(item.deaf),
            patch
                .timeout_until
                .map(|t| t.map(|t| PrimitiveDateTime::new(t.date(), t.time())))
                .unwrap_or(item.timeout_until),
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
        if membership == DbMembership::Join {
            query!(
                r#"
            UPDATE room_member
        	SET membership = $3, left_at = null, joined_at = now()
            WHERE room_id = $1 AND user_id = $2
            "#,
                *room_id,
                *user_id,
                membership as _,
            )
            .execute(&self.pool)
            .await?;
        } else {
            query!(
                r#"
            UPDATE room_member
        	SET membership = $3, left_at = now()
            WHERE room_id = $1 AND user_id = $2
            "#,
                *room_id,
                *user_id,
                membership as _,
            )
            .execute(&self.pool)
            .await?;
        }
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

    async fn room_ban_search(
        &self,
        room_id: RoomId,
        query: String,
        paginate: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RoomBan>> {
        let p: Pagination<_> = paginate.try_into()?;
        let query = format!("%{}%", query);

        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbRoomBan,
                r#"
                SELECT b.user_id, b.reason, b.created_at, b.expires_at
                FROM room_ban b
                JOIN usr u ON b.user_id = u.id
                WHERE b.room_id = $1 AND u.name ILIKE $2 AND b.user_id > $3 AND b.user_id < $4
                ORDER BY (CASE WHEN $5 = 'f' THEN b.user_id END), b.user_id DESC
                LIMIT $6
                "#,
                *room_id,
                query,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r#"
                SELECT count(*)
                FROM room_ban b
                JOIN usr u ON b.user_id = u.id
                WHERE b.room_id = $1 AND u.name ILIKE $2
                "#,
                *room_id,
                query
            ),
            |i: &RoomBan| i.user_id.to_string()
        )
    }

    async fn room_ban_create_bulk(
        &self,
        room_id: RoomId,
        ban_ids: &[UserId],
        reason: Option<String>,
        expires_at: Option<Time>,
    ) -> Result<()> {
        let ban_ids: Vec<Uuid> = ban_ids.iter().map(|id| id.into_inner()).collect();
        query!(
            r#"
            INSERT INTO room_ban (room_id, user_id, reason, created_at, expires_at)
            SELECT $1, user_id, $3, now(), $4
            FROM UNNEST($2::uuid[]) as user_id
            ON CONFLICT (room_id, user_id) DO UPDATE
            SET expires_at = EXCLUDED.expires_at, reason = EXCLUDED.reason
            "#,
            *room_id,
            &ban_ids,
            reason,
            expires_at.map(|t| PrimitiveDateTime::new(t.date(), t.time())),
        )
        .execute(&self.pool)
        .await?;
        info!("inserted room bans");
        Ok(())
    }

    async fn room_bot_list(
        &self,
        room_id: RoomId,
        paginate: PaginationQuery<ApplicationId>,
    ) -> Result<PaginationResponse<ApplicationId>> {
        let p: Pagination<_> = paginate.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_scalar!(
                r#"
                SELECT user_id FROM room_member
                WHERE room_id = $1 AND user_id > $2 AND user_id < $3 AND EXISTS (SELECT 1 FROM application WHERE application.id = room_member.user_id)
                ORDER BY (CASE WHEN $4 = 'f' THEN user_id END), user_id DESC
                LIMIT $5
                "#,
                *room_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!("SELECT count(*) FROM room_member WHERE room_id = $1 AND EXISTS (SELECT 1 FROM application WHERE application.id = room_member.user_id)", *room_id),
            |i: &ApplicationId| i.to_string()
        )
    }

    async fn room_member_list_all(&self, room_id: RoomId) -> Result<Vec<RoomMember>> {
        let items = query_as!(
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
                mute,
                deaf,
                timeout_until,
                coalesce(r.roles, '{}') as "roles!"
            FROM room_member m
            left join r on r.user_id = m.user_id
            WHERE room_id = $1 AND membership = 'Join'
            "#,
            *room_id,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(items.into_iter().map(Into::into).collect())
    }

    async fn room_member_search(
        &self,
        room_id: RoomId,
        query: String,
        limit: u16,
    ) -> Result<Vec<RoomMember>> {
        let query = format!("%{}%", query);
        let items = query_as!(
            DbRoomMember,
            r#"
            with r as (
                select user_id, array_agg(role_id) as roles from role_member
                join role on role.room_id = $1 and role_member.role_id = role.id
                group by user_id
            )
            SELECT
                m.room_id,
                m.user_id,
                membership as "membership: _",
                override_name,
                override_description,
                joined_at,
                origin,
                mute,
                deaf,
                timeout_until,
                coalesce(r.roles, '{}') as "roles!"
            FROM room_member m
            JOIN usr u ON m.user_id = u.id
            left join r on r.user_id = m.user_id
            WHERE m.room_id = $1 AND m.membership = 'Join' AND (u.name ILIKE $2 OR m.override_name ILIKE $2)
            ORDER BY u.name
            LIMIT $3
            "#,
            *room_id,
            query,
            limit as i64
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(items.into_iter().map(Into::into).collect())
    }

    async fn room_member_search_advanced(
        &self,
        room_id: RoomId,
        search: RoomMemberSearchAdvanced,
    ) -> Result<RoomMemberSearchResponse> {
        let limit = search.limit.unwrap_or(10).min(1024);
        let query = search.query.map(|q| format!("%{}%", q));
        let role_ids: Vec<Uuid> = search.roles.iter().map(|r| r.into_inner()).collect();

        let rows = query_file_as!(
            DbRoomMemberWithUser,
            "sql/room_member_search_advanced.sql",
            *room_id,
            query,
            limit as i64,
            &role_ids,
            search.invite.map(|i| i.to_string()),
            search.timeout,
            search.mute,
            search.deaf,
            search.nickname,
            search.guest,
            search.join_before.map(PrimitiveDateTime::from),
            search.join_after.map(PrimitiveDateTime::from),
            search.create_before.map(PrimitiveDateTime::from),
            search.create_after.map(PrimitiveDateTime::from)
        )
        .fetch_all(&self.pool)
        .await?;

        let (room_members, users) = rows.into_iter().map(Into::into).unzip();

        Ok(RoomMemberSearchResponse {
            room_members,
            users,
        })
    }
}
