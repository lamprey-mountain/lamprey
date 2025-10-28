use async_trait::async_trait;
use common::v1::types::{PaginationDirection, RoleReorder, UserId};
use sqlx::{query, query_as, query_scalar, Acquire};
use tracing::info;

use crate::data::postgres::Pagination;
use crate::error::Result;
use crate::types::{
    DbPermission, DbRoleCreate, PaginationQuery, PaginationResponse, Role, RoleId, RolePatch,
    RoleVerId, RoomId,
};
use crate::{gen_paginate, Error};

use crate::data::DataRole;

use super::Postgres;

pub struct DbRole {
    pub id: RoleId,
    pub version_id: RoleVerId,
    pub room_id: RoomId,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<DbPermission>,
    pub is_self_applicable: bool,
    pub is_mentionable: bool,
    pub member_count: i64,
    pub position: i64,
    pub hoist: bool,
}

impl From<DbRole> for Role {
    fn from(row: DbRole) -> Self {
        Role {
            id: row.id,
            version_id: row.version_id,
            room_id: row.room_id,
            name: row.name,
            description: row.description,
            permissions: row.permissions.into_iter().map(Into::into).collect(),
            is_self_applicable: row.is_self_applicable,
            is_mentionable: row.is_mentionable,
            member_count: row.member_count as u64,
            position: row.position as u64,
            hoist: row.hoist,
        }
    }
}

#[async_trait]
impl DataRole for Postgres {
    async fn role_create(&self, create: DbRoleCreate, position: u64) -> Result<Role> {
        let role_id = *create.id;
        let perms: Vec<DbPermission> = create.permissions.into_iter().map(Into::into).collect();
        let mut tx = self.pool.begin().await?;

        // lock all roles to prevent race conditions
        query!(
            "select from role where room_id = $1 for update",
            *create.room_id
        )
        .execute(&mut *tx)
        .await?;

        let count = query_scalar!(
            r#"select count(*) from role where room_id = $1"#,
            *create.room_id
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or_default() as u32;
        if count >= crate::consts::MAX_ROLE_COUNT {
            return Err(Error::BadRequest(format!(
                "too many roles (max {})",
                crate::consts::MAX_ROLE_COUNT
            )));
        }
        query!(
            r#"update role set position = position + 1 where id != room_id and room_id = $1"#,
            *create.room_id
        )
        .execute(&mut *tx)
        .await?;
        let role = query_as!(DbRole, r#"
            INSERT INTO role (id, version_id, room_id, name, description, permissions, is_mentionable, is_self_applicable, position, hoist)
            VALUES ($1, $1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, version_id, room_id, name, description, permissions as "permissions: _", is_mentionable, is_self_applicable, 0 as "member_count!", position, hoist
        "#,
            role_id,
            *create.room_id,
            create.name,
            create.description,
            perms as _,
            create.is_mentionable,
            create.is_self_applicable,
            position as i64,
            create.hoist,
        )
            .fetch_one(&mut *tx)
        	.await?;
        tx.commit().await?;
        info!("inserted role");
        Ok(role.into())
    }

    async fn role_list(
        &self,
        room_id: RoomId,
        paginate: PaginationQuery<RoleId>,
    ) -> Result<PaginationResponse<Role>> {
        let p: Pagination<_> = paginate.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbRole,
                r#"
            	SELECT
                	r.id,
                	r.description,
                	r.is_mentionable,
                	r.permissions as "permissions: _",
                	r.version_id,
                	r.room_id,
                	r.is_self_applicable,
                	r.name,
                    coalesce(rm.count, 0) as "member_count!",
                    r.position,
                    r.hoist
                FROM role r
                LEFT JOIN (
                    SELECT role_id, count(*) as count
                    FROM role_member
                    GROUP BY role_id
                ) rm ON rm.role_id = r.id
            	WHERE r.room_id = $1 AND r.id > $2 AND r.id < $3
            	ORDER BY (CASE WHEN $4 = 'f' THEN r.id END), r.id DESC LIMIT $5
                "#,
                room_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                "SELECT count(*) FROM role WHERE room_id = $1",
                room_id.into_inner(),
            ),
            |i: &Role| i.id.to_string()
        )
    }

    async fn role_delete(&self, _room_id: RoomId, role_id: RoleId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        query!(
            "DELETE FROM role_member WHERE role_id = $1",
            role_id.into_inner()
        )
        .execute(&mut *tx)
        .await?;
        query!("DELETE FROM role WHERE id = $1", role_id.into_inner())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn role_select(&self, room_id: RoomId, role_id: RoleId) -> Result<Role> {
        let role = query_as!(
            DbRole,
            r#"
            SELECT
                r.id, r.version_id, r.room_id, r.name, r.description, r.permissions as "permissions: _",
                r.is_mentionable, r.is_self_applicable,
                coalesce(rm.count, 0) as "member_count!",
                r.position,
                r.hoist
            FROM role r
            LEFT JOIN (
                SELECT role_id, count(*) as count
                FROM role_member
                GROUP BY role_id
            ) rm ON rm.role_id = r.id
            WHERE r.room_id = $1 AND r.id = $2
        "#,
            *room_id,
            *role_id,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(role.into())
    }

    async fn role_update(
        &self,
        room_id: RoomId,
        role_id: RoleId,
        patch: RolePatch,
    ) -> Result<RoleVerId> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let perms = patch
            .permissions
            .map(|p| p.into_iter().map(Into::into).collect::<Vec<DbPermission>>());
        let role = query_as!(
            DbRole,
            r#"
            SELECT
                id, version_id, room_id, name, description, permissions as "permissions: _",
                is_mentionable, is_self_applicable, 0 as "member_count!", position, hoist
            FROM role
            WHERE room_id = $1 AND id = $2
            FOR UPDATE
        "#,
            *room_id,
            *role_id,
        )
        .fetch_one(&mut *tx)
        .await?;
        let version_id = RoleVerId::new();
        query!(
            r#"
            UPDATE role SET
                version_id = $2,
                name = $3,
                description = $4,
                permissions = $5,
                is_mentionable = $6,
                is_self_applicable = $7,
                hoist = $8
            WHERE id = $1
        "#,
            *role_id,
            *version_id,
            patch.name.unwrap_or(role.name),
            patch.description.unwrap_or(role.description),
            perms.unwrap_or(role.permissions.into_iter().map(|p| p.into()).collect()) as _,
            patch.is_mentionable.unwrap_or(role.is_mentionable),
            patch.is_self_applicable.unwrap_or(role.is_self_applicable),
            patch.hoist.unwrap_or(role.hoist),
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(version_id)
    }

    async fn role_reorder(&self, room_id: RoomId, reorder: RoleReorder) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        query!("select from role where room_id = $1 for update", *room_id)
            .execute(&mut *tx)
            .await?;
        for r in reorder.roles {
            if *r.role_id == *room_id {
                tx.rollback().await?;
                return Err(Error::BadStatic(
                    "can't change base/@everyone role's position",
                ));
            }
            let pos: i32 = r
                .position
                .try_into()
                .map_err(|_| Error::BadStatic("invalid position"))?;
            query!(
                "update role set position = $3 where id = $1 and room_id = $2",
                *r.role_id,
                *room_id,
                pos,
            )
            .execute(&mut *tx)
            .await?;
        }
        query!(
            r#"
            with ranked_roles as (
                select
                    id,
                    row_number() over (partition by room_id order by position, id) - 1 as rn
                from role
                where room_id = $1
            )
            update role
            set position = ranked_roles.rn
            from ranked_roles
            where role.id = ranked_roles.id;
        "#,
            *room_id
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn role_user_rank(&self, room_id: RoomId, user_id: UserId) -> Result<u64> {
        let rank = query_scalar!(
            r#"
            select max(role.position) from room_member
            join role_member on role_member.user_id = room_member.user_id
            join role on role_member.role_id = role.id and role.room_id = room_member.room_id
            where room_member.room_id = $1 and room_member.user_id = $2
            "#,
            *room_id,
            *user_id,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(rank.unwrap_or_default() as u64)
    }
}
