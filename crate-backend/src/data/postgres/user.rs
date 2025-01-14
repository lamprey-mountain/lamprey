use async_trait::async_trait;
use sqlx::{query, query_as, Acquire};
use uuid::Uuid;

use crate::error::Result;
use crate::types::{User, UserCreate, UserId, UserPatch, DbUser, UserVerId};

use crate::data::DataUser;

use super::Postgres;

#[async_trait]
impl DataUser for Postgres {
    async fn user_create(&self, user_id: UserId, patch: UserCreate) -> Result<User> {
        let mut conn = self.pool.acquire().await?;
        let row = query_as!(
            DbUser,
            r#"
            INSERT INTO usr (id, version_id, parent_id, name, description, status, is_bot, is_alias, is_system)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, version_id, parent_id, name, description, status, is_bot, is_alias, is_system
        "#,
            user_id.into_inner(),
            user_id.into_inner(),
            patch.parent_id.map(|i| i.into_inner()),
            patch.name,
            patch.description,
            patch.status,
            patch.is_bot,
            patch.is_alias,
            patch.is_system,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row.into())
    }
    
    async fn user_update(&self, user_id: UserId, patch: UserPatch) -> Result<UserVerId> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let user = query_as!(
            DbUser,
            "
            SELECT id, version_id, parent_id, name, description, status, is_bot, is_alias, is_system
            FROM usr WHERE id = $1
            ",
            user_id.into_inner()
        )
        .fetch_one(&mut *tx)
        .await?;
        let user: User = user.into();
        let version_id = UserVerId(Uuid::now_v7());
        query!(
            "UPDATE usr SET version_id = $2, name = $3, description = $4 WHERE id = $1",
            user_id.into_inner(),
            version_id.into_inner(),
            patch.name.unwrap_or(user.name),
            patch.description.unwrap_or(user.description),
        )
        .execute(&mut *tx)
        .await?;
        Ok(version_id)
    }
    
    async fn user_delete(&self, user_id: UserId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        query!("UPDATE usr SET deleted_at = $2 WHERE id = $1", user_id.into_inner(), now)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
    
    async fn user_get(&self, id: UserId) -> Result<User> {
        let mut conn = self.pool.acquire().await?;
        let row = query_as!(
            DbUser,
            r#"
            SELECT id, version_id, parent_id, name, description, status, is_bot, is_alias, is_system
            FROM usr WHERE id = $1
        "#,
            id.into_inner()
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row.into())
    }
}
