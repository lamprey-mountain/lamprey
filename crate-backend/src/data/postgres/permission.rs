use async_trait::async_trait;
use common::v1::types::{Permission, PermissionOverwriteType, RoomId, ThreadId, UserId};
use sqlx::{query_scalar, types::Json};
use uuid::Uuid;

use crate::{
    data::DataPermission,
    types::{DbPermission, Permissions},
    Result,
};

use super::Postgres;

#[async_trait]
impl DataPermission for Postgres {
    async fn permission_room_get(&self, user_id: UserId, room_id: RoomId) -> Result<Permissions> {
        let mut perms: Permissions = query_scalar!(
            r#"
            WITH perms AS (
                SELECT m.room_id, m.user_id, unnest(role.permissions) AS permission
                FROM room_member AS m
                JOIN role_member AS r ON r.user_id = m.user_id
                JOIN role ON r.role_id = role.id AND role.room_id = m.room_id
                UNION
                SELECT room_id, user_id, 'View' AS permission
                FROM room_member
            )
            SELECT permission as "permission!: DbPermission"
            FROM perms
            WHERE user_id = $1 AND room_id = $2
        "#,
            user_id.into_inner(),
            room_id.into_inner()
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

        // Apply role overwrites
        let role_overwrites = sqlx::query!(
            r#"
            SELECT
                po.allow as "allow!: Json<Vec<Permission>>",
                po.deny as "deny!: Json<Vec<Permission>>"
            FROM permission_overwrite po
            JOIN role_member rm ON po.actor_id = rm.role_id
            WHERE po.target_id = $1 AND rm.user_id = $2
            "#,
            room_id.into_inner(),
            user_id.into_inner(),
        )
        .fetch_all(&self.pool)
        .await?;

        for overwrite in role_overwrites {
            for p in overwrite.allow.0 {
                perms.add(p);
            }
            for p in overwrite.deny.0 {
                perms.remove(p);
            }
        }

        // Apply user overwrites
        let user_overwrites = sqlx::query!(
            r#"
            SELECT
                allow as "allow!: Json<Vec<Permission>>",
                deny as "deny!: Json<Vec<Permission>>"
            FROM permission_overwrite
            WHERE target_id = $1 AND actor_id = $2
            "#,
            room_id.into_inner(),
            user_id.into_inner(),
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(overwrite) = user_overwrites {
            for p in overwrite.allow.0 {
                perms.add(p);
            }
            for p in overwrite.deny.0 {
                perms.remove(p);
            }
        }

        Ok(perms)
    }

    async fn permission_thread_get(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
    ) -> Result<Permissions> {
        let room_id: Option<Uuid> = query_scalar!(
            "SELECT room_id FROM thread WHERE id = $1",
            thread_id.into_inner()
        )
        .fetch_one(&self.pool)
        .await?;

        let mut perms = if let Some(room_id_uuid) = room_id {
            self.permission_room_get(user_id, room_id_uuid.into())
                .await?
        } else {
            Permissions::empty()
        };

        // Apply role overwrites
        let role_overwrites = sqlx::query!(
            r#"
            SELECT
                po.allow as "allow!: Json<Vec<Permission>>",
                po.deny as "deny!: Json<Vec<Permission>>"
            FROM permission_overwrite po
            JOIN role_member rm ON po.actor_id = rm.role_id
            WHERE po.target_id = $1 AND rm.user_id = $2
            "#,
            thread_id.into_inner(),
            user_id.into_inner(),
        )
        .fetch_all(&self.pool)
        .await?;

        for overwrite in role_overwrites {
            for p in overwrite.allow.0 {
                perms.add(p);
            }
            for p in overwrite.deny.0 {
                perms.remove(p);
            }
        }

        // Apply user overwrites
        let user_overwrites = sqlx::query!(
            r#"
            SELECT
                allow as "allow!: Json<Vec<Permission>>",
                deny as "deny!: Json<Vec<Permission>>"
            FROM permission_overwrite
            WHERE target_id = $1 AND actor_id = $2
            "#,
            thread_id.into_inner(),
            user_id.into_inner(),
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(overwrite) = user_overwrites {
            for p in overwrite.allow.0 {
                perms.add(p);
            }
            for p in overwrite.deny.0 {
                perms.remove(p);
            }
        }

        Ok(perms.into_iter().collect())
    }

    async fn permission_is_mutual(&self, a: UserId, b: UserId) -> Result<bool> {
        let exists = query_scalar!(
            r#"
            select 1
            from room_member a
            join room_member b on a.room_id = b.room_id
            where a.user_id = $1 and b.user_id = $2
            "#,
            a.into_inner(),
            b.into_inner(),
        )
        .fetch_optional(&self.pool)
        .await?
        .is_some();
        Ok(exists)
    }

    async fn permission_overwrite_upsert(
        &self,
        target_id: ThreadId,
        actor_id: Uuid,
        ty: PermissionOverwriteType,
        allow: Vec<Permission>,
        deny: Vec<Permission>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO permission_overwrite (target_id, actor_id, type, allow, deny)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (target_id, actor_id) DO UPDATE SET type = $3, allow = $4, deny = $5
            "#,
            *target_id,
            actor_id,
            match ty {
                PermissionOverwriteType::Role => "Role",
                PermissionOverwriteType::User => "User",
            },
            serde_json::to_value(&allow)?,
            serde_json::to_value(&deny)?,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn permission_overwrite_delete(
        &self,
        thread_id: ThreadId,
        overwrite_id: Uuid,
    ) -> Result<()> {
        sqlx::query!(
            "DELETE FROM permission_overwrite WHERE target_id = $1 AND actor_id = $2",
            *thread_id,
            overwrite_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
