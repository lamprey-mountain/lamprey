use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire};
use tracing::info;
use types::{PaginationDirection, UserId};
use uuid::Uuid;

use crate::data::postgres::Pagination;
use crate::error::Result;
use crate::types::{
    DbPermission, DbRole, PaginationQuery, PaginationResponse, Role, RoleCreate, RoleId, RolePatch,
    RoleVerId, RoomId,
};

use crate::data::DataRole;

use super::Postgres;

#[async_trait]
impl DataRole for Postgres {
    async fn role_create(&self, create: RoleCreate) -> Result<Role> {
        let role_id = Uuid::now_v7();
        let perms: Vec<DbPermission> = create.permissions.into_iter().map(Into::into).collect();
        let role = query_as!(DbRole, r#"
            INSERT INTO role (id, version_id, room_id, name, description, permissions, is_mentionable, is_self_applicable, is_default)
            VALUES ($1, $1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, version_id, room_id, name, description, permissions as "permissions: _", is_mentionable, is_self_applicable, is_default
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
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let items = query_as!(
            DbRole,
            r#"
        	SELECT
            	id,
            	description,
            	is_default,
            	is_mentionable,
            	permissions as "permissions: _",
            	version_id,
            	room_id,
            	is_self_applicable,
            	name
            FROM role
        	WHERE room_id = $1 AND id > $2 AND id < $3
        	ORDER BY (CASE WHEN $4 = 'f' THEN id END), id DESC LIMIT $5
        "#,
            room_id.into_inner(),
            p.after.into_inner(),
            p.before.into_inner(),
            p.dir.to_string(),
            (p.limit + 1) as i32
        )
        .fetch_all(&mut *tx)
        .await?;
        let total = query_scalar!(
            "SELECT count(*) FROM role WHERE room_id = $1",
            room_id.into_inner(),
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.rollback().await?;
        let has_more = items.len() > p.limit as usize;
        let mut items: Vec<_> = items
            .into_iter()
            .take(p.limit as usize)
            .map(Into::into)
            .collect();
        if p.dir == PaginationDirection::B {
            items.reverse();
        }
        Ok(PaginationResponse {
            items,
            total: total.unwrap_or(0) as u64,
            has_more,
        })
    }

    async fn role_delete(&self, _room_id: RoomId, _role_id: RoleId) -> Result<()> {
        todo!()
    }

    async fn role_select(&self, room_id: RoomId, role_id: RoleId) -> Result<Role> {
        let role = query_as!(DbRole, r#"
            SELECT id, version_id, room_id, name, description, permissions as "permissions: _", is_mentionable, is_self_applicable, is_default
            FROM role
            WHERE room_id = $1 AND id = $2
        "#, room_id.into_inner(), role_id.into_inner())
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
            .map(|p| p.into_iter().map(Into::into).collect());
        let role = query_as!(DbRole, r#"
            SELECT id, version_id, room_id, name, description, permissions as "permissions: _", is_mentionable, is_self_applicable, is_default
            FROM role
            WHERE room_id = $1 AND id = $2
            FOR UPDATE
        "#, room_id.into_inner(), role_id.into_inner())
    	    .fetch_one(&mut *tx)
        	.await?;
        let version_id = RoleVerId(Uuid::now_v7());
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
            perms.unwrap_or(role.permissions) as _,
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
        ",
            room_id.into_inner(),
            user_id.into_inner()
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
