use async_trait::async_trait;
use serde::Deserialize;
use sqlx::{query, query_as, Acquire};
use types::{UserState, UserType};
use uuid::Uuid;

use crate::error::Result;
use crate::types::{DbUserCreate, User, UserId, UserPatch, UserVerId};

use crate::data::DataUser;

use super::Postgres;

#[derive(Deserialize)]
pub struct DbUser {
    pub id: UserId,
    pub version_id: UserVerId,
    pub parent_id: Option<uuid::Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub avatar: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub r#type: DbUserType,
    pub state: DbUserState,
}

#[derive(Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_type")]
pub enum DbUserType {
    Default,
    Alias,
    Bot,
    System,
}

#[derive(Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_state")]
pub enum DbUserState {
    Active,
    Suspended,
    Deleted,
}

impl From<DbUser> for User {
    fn from(row: DbUser) -> Self {
        User {
            id: row.id,
            version_id: row.version_id,
            name: row.name,
            description: row.description,
            status: row.status,
            avatar: row.avatar.map(Into::into),
            user_type: match row.r#type {
                DbUserType::Default => UserType::Default,
                DbUserType::Alias => UserType::Alias {
                    alias_id: row.parent_id.unwrap().into(),
                },
                DbUserType::Bot => UserType::Bot {
                    owner_id: row.parent_id.unwrap().into(),
                },
                DbUserType::System => UserType::System,
            },
            state: match row.state {
                DbUserState::Active => UserState::Active,
                DbUserState::Suspended => UserState::Suspended,
                DbUserState::Deleted => UserState::Deleted,
            },
        }
    }
}

#[async_trait]
impl DataUser for Postgres {
    async fn user_create(&self, patch: DbUserCreate) -> Result<User> {
        let user_id = Uuid::now_v7();
        let user_type = if patch.is_bot {
            DbUserType::Bot
        } else {
            DbUserType::Default
        };
        let row = query_as!(
            DbUser,
            r#"
            INSERT INTO usr (id, version_id, parent_id, name, description, status, can_fork, type, state)
            VALUES ($1, $2, $3, $4, $5, $6, false, $7, $8)
            RETURNING id, version_id, parent_id, name, description, status, state as "state: _", type as "type: _", avatar
        "#,
            user_id,
            user_id,
            patch.parent_id.map(|i| i.into_inner()),
            patch.name,
            patch.description,
            patch.status,
            user_type as _,
            DbUserState::Active as _,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn user_update(&self, user_id: UserId, patch: UserPatch) -> Result<UserVerId> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let user = query_as!(
            DbUser,
            r#"
            SELECT id, version_id, parent_id, name, description, status, state as "state: _", type as "type: _", avatar
            FROM usr WHERE id = $1
            FOR UPDATE
            "#,
            user_id.into_inner()
        )
        .fetch_one(&mut *tx)
        .await?;
        let user: User = user.into();
        let version_id = UserVerId(Uuid::now_v7());
        let avatar = patch
            .avatar
            .unwrap_or(user.avatar)
            .map(|i| i.into_inner());
        query!(
            "UPDATE usr SET version_id = $2, name = $3, description = $4, avatar = $5 WHERE id = $1",
            user_id.into_inner(),
            version_id.into_inner(),
            patch.name.unwrap_or(user.name),
            patch.description.unwrap_or(user.description),
            avatar,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(version_id)
    }

    async fn user_delete(&self, user_id: UserId) -> Result<()> {
        let now = time::OffsetDateTime::now_utc();
        let now = time::PrimitiveDateTime::new(now.date(), now.time());
        query!(
            "UPDATE usr SET state = 'Deleted', state_updated_at = $2 WHERE id = $1",
            user_id.into_inner(),
            now
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn user_get(&self, id: UserId) -> Result<User> {
        let row = query_as!(
            DbUser,
            r#"
            SELECT id, version_id, parent_id, name, description, status, state as "state: _", type as "type: _", avatar
            FROM usr WHERE id = $1
        "#,
            id.into_inner()
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }
}
