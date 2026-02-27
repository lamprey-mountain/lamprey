use async_trait::async_trait;
use common::v1::types::{ChannelId, Permission, PermissionOverwriteType, UserId};
use sqlx::query_scalar;
use uuid::Uuid;

use crate::{data::DataPermission, Result};

use super::Postgres;

// TODO: remove this trait and move all permission calculations into permissions.rs
#[async_trait]
impl DataPermission for Postgres {
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
        target_id: ChannelId,
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
        thread_id: ChannelId,
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

    async fn permission_allows_dm_from_user(
        &self,
        source_user_id: UserId,
        target_user_id: UserId,
    ) -> Result<bool> {
        // Check if target user allows DMs:
        // 1. If global dms setting is true, allow
        // 2. Otherwise, check if any shared room has dms enabled
        let allows = query_scalar!(
            r#"
            select coalesce(
                (select (config->>'dms')::bool from usr where id = $2),
                false
            )
            or exists (
                select 1
                from room_member rm1
                join room_member rm2 on rm1.room_id = rm2.room_id
                join preferences_room ucr on rm1.room_id = ucr.room_id
                where rm1.user_id = $1
                  and rm2.user_id = $2
                  and rm1.membership = 'Join'
                  and rm2.membership = 'Join'
                  and (ucr.config->>'dms')::bool = true
            )
            as "allows!"
            "#,
            source_user_id.into_inner(),
            target_user_id.into_inner(),
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(allows)
    }

    async fn permission_allows_friend_request_from_user(
        &self,
        source_user_id: UserId,
        target_user_id: UserId,
    ) -> Result<bool> {
        // Check if target user allows friend requests:
        // 1. If global allow_everyone is true, allow
        // 2. If global allow_mutual_room is true, check if any shared room has friends enabled
        // Note: allow_mutual_friend is handled separately in the caller
        let allows = query_scalar!(
            r#"
            select coalesce(
                (select (config->'friends'->>'allow_everyone')::bool from usr where id = $2),
                false
            )
            or (
                coalesce(
                    (select (config->'friends'->>'allow_mutual_room')::bool from usr where id = $2),
                    false
                )
                and exists (
                    select 1
                    from room_member rm1
                    join room_member rm2 on rm1.room_id = rm2.room_id
                    join preferences_room ucr on rm1.room_id = ucr.room_id
                    where rm1.user_id = $1
                      and rm2.user_id = $2
                      and rm1.membership = 'Join'
                      and rm2.membership = 'Join'
                      and (ucr.config->>'friends')::bool = true
                )
            )
            as "allows!"
            "#,
            source_user_id.into_inner(),
            target_user_id.into_inner(),
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(allows)
    }
}
