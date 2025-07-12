use async_trait::async_trait;
use sqlx::query_scalar;
use uuid::Uuid;

use crate::types::{DbPermission, Permissions, RoomId, ThreadId, UserId};
use crate::data::DataPermission;
use crate::error::Result;

use super::Postgres;

#[async_trait]
impl DataPermission for Postgres {
    async fn permission_room_get(&self, user_id: UserId, room_id: RoomId) -> Result<Permissions> {
        let perms = query_scalar!(
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
        .await?;
        Ok(perms.into_iter().map(Into::into).collect())
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

        // FIXME: role overwrites
        // apply in order: role allow, role deny, user allow, user deny (explicit deny, user overwrites role perms)
        let overwrites = sqlx::query!(
            r#"
            SELECT
                allow as "allow!: Vec<DbPermission>",
                deny as "deny!: Vec<DbPermission>"
            FROM permission_overwrite
            WHERE target_id = $1 AND actor_id = $2
            "#,
            thread_id.into_inner(),
            user_id.into_inner(),
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(overwrite) = overwrites {
            for p in overwrite.allow {
                perms.add(p.into());
            }
            for p in overwrite.deny {
                perms.remove(p.into());
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
}
