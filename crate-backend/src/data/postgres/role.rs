use async_trait::async_trait;
use common::v1::types::{PaginationDirection, UserId};
use sqlx::{query, query_as, query_scalar, Acquire};
use tracing::info;
use uuid::Uuid;

use crate::data::postgres::Pagination;
use crate::error::Result;
use crate::gen_paginate;
use crate::types::{
    DbPermission, DbRoleCreate, PaginationQuery, PaginationResponse, Role, RoleId, RolePatch,
    RoleVerId, RoomId,
};

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
    pub is_default: bool,
    pub member_count: i64,
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
            is_default: row.is_default,
            member_count: row.member_count as u64,
        }
    }
}

#[async_trait]
impl DataRole for Postgres {
    async fn role_create(&self, create: DbRoleCreate) -> Result<Role> {
        let role_id = Uuid::now_v7();
        let perms: Vec<DbPermission> = create.permissions.into_iter().map(Into::into).collect();
        let role = query_as!(DbRole, r#"
            INSERT INTO role (id, version_id, room_id, name, description, permissions, is_mentionable, is_self_applicable, is_default)
            VALUES ($1, $1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, version_id, room_id, name, description, permissions as "permissions: _", is_mentionable, is_self_applicable, is_default, 0 as "member_count!"
        "#, role_id, create.room_id.into_inner(), create.name, create.description, perms as _, create.is_mentionable, create.is_self_applicable, create.is_default)
    	    .fetch_one(&self.pool)
        	.await?;
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
                	r.is_default,
                	r.is_mentionable,
                	r.permissions as "permissions: _",
                	r.version_id,
                	r.room_id,
                	r.is_self_applicable,
                	r.name,
                    coalesce(rm.count, 0) as "member_count!"
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
                r.is_mentionable, r.is_self_applicable, r.is_default,
                coalesce(rm.count, 0) as "member_count!"
            FROM role r
            LEFT JOIN (
                SELECT role_id, count(*) as count
                FROM role_member
                GROUP BY role_id
            ) rm ON rm.role_id = r.id
            WHERE r.room_id = $1 AND r.id = $2
        "#,
            room_id.into_inner(),
            role_id.into_inner()
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
                is_mentionable, is_self_applicable, is_default, 0 as "member_count!"
            FROM role
            WHERE room_id = $1 AND id = $2
            FOR UPDATE
        "#,
            room_id.into_inner(),
            role_id.into_inner()
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
                is_default = $8
            WHERE id = $1
        "#,
            role_id.into_inner(),
            version_id.into_inner(),
            patch.name.unwrap_or(role.name),
            patch.description.unwrap_or(role.description),
            perms.unwrap_or(role.permissions.into_iter().map(|p| p.into()).collect()) as _,
            patch.is_mentionable.unwrap_or(role.is_mentionable),
            patch.is_self_applicable.unwrap_or(role.is_self_applicable),
            patch.is_default.unwrap_or(role.is_default),
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(version_id)
    }

    async fn role_apply_default(&self, room_id: RoomId, user_id: UserId) -> Result<()> {
        query!(
            "
    	  	INSERT INTO role_member (user_id, role_id)
    	  	SELECT $2 as u, id FROM role
    	  	WHERE room_id = $1 AND is_default = true
    	  	ON CONFLICT DO NOTHING
        ",
            room_id.into_inner(),
            user_id.into_inner()
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
