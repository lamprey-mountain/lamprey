use async_trait::async_trait;
use sqlx::{query, Acquire};

use crate::error::Result;
use crate::types::{User, UserCreate, UserId, UserPatch};

use crate::data::DataUser;

use super::Postgres;

#[async_trait]
impl DataUser for Postgres {
    async fn user_insert(&self, id: UserId, patch: UserCreate) -> Result<UserId> {
        todo!()
    }
    async fn user_update(&self, id: UserId, patch: UserPatch) -> Result<User> {
        todo!()
    }
    async fn user_delete(&self, id: UserId) -> Result<()> {
        todo!()
    }

    async fn user_get(&self, id: UserId) -> Result<User> {
        let mut conn = self.pool.acquire().await?;
        let row = query!(
            r#"
            SELECT id, parent_id, name, description, status, is_bot, is_alias, is_system
            FROM usr WHERE id = $1
        "#,
            id.into_inner()
        )
        .fetch_one(&mut *conn)
        .await?;
        let user = User {
            id,
            parent_id: row.parent_id.map(UserId),
            name: row.name,
            description: row.description,
            status: row.status,
            is_bot: row.is_bot,
            is_alias: row.is_alias,
            is_system: row.is_system,
        };
        Ok(user)
    }
}
