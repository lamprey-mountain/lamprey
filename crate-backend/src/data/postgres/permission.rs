use async_trait::async_trait;
use sqlx::query_scalar;
use uuid::Uuid;

use crate::error::Result;
use crate::types::{DbPermission, Permissions, RoomId, ThreadId, UserId};

use crate::data::DataPermission;

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

    // TODO: thread overwrites
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

        let perms = if let Some(room_id_uuid) = room_id {
            self.permission_room_get(user_id, room_id_uuid.into())
                .await?
        } else {
            Permissions::empty()
        };
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
