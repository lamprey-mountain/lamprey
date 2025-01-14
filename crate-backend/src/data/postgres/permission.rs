use async_trait::async_trait;
use sqlx::query_scalar;

use crate::error::Result;
use crate::types::{Permission, Permissions, RoomId, ThreadId, UserId};

use crate::data::DataPermission;

use super::Postgres;

#[async_trait]
impl DataPermission for Postgres {
    async fn permission_room_get(&self, user_id: UserId, room_id: RoomId) -> Result<Permissions> {
        let mut conn = self.pool.acquire().await?;
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
            SELECT permission as "permission!: Permission"
            FROM perms
            WHERE user_id = $1 AND room_id = $2
        "#,
            user_id.into_inner(),
            room_id.into_inner()
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(perms.into_iter().collect())
    }

    // TODO: thread overwrites? or would that be overkill?
    async fn permission_thread_get(
        &self,
        user_id: UserId,
        thread_id: ThreadId,
    ) -> Result<Permissions> {
        let mut conn = self.pool.acquire().await?;
        let room_id = query_scalar!(
            "SELECT room_id FROM thread WHERE id = $1",
            thread_id.into_inner()
        )
        .fetch_one(&mut *conn)
        .await?;
        let perms = self.permission_room_get(user_id, room_id.into()).await?;
        Ok(perms.into_iter().collect())
    }
}
