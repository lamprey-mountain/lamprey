use async_trait::async_trait;
use serde::Deserialize;
use sqlx::{query, query_as, Acquire};
use types::{BotOwner, BotVisibility, ExternalPlatform, UserState, UserType};
use url::Url;
use uuid::Uuid;

use crate::error::Result;
use crate::types::{DbUserCreate, User, UserId, UserPatch, UserVerId};

use crate::data::DataUser;

use super::Postgres;

#[derive(Deserialize)]
pub struct DbUser {
    pub id: UserId,
    pub version_id: UserVerId,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub avatar: Option<Uuid>,
    pub r#type: DbUserType,
    pub state: DbUserState,
    pub puppet_external_platform: Option<String>,
    pub puppet_external_id: Option<String>,
    pub puppet_external_url: Option<String>,
    pub puppet_alias_id: Option<Uuid>,
    pub bot_is_bridge: Option<bool>,
    pub bot_visibility: DbBotVisibility,
}

#[derive(Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_type")]
pub enum DbUserType {
    Default,
    Bot,
    System,
    Puppet,
}

#[derive(Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_state")]
pub enum DbUserState {
    Active,
    Suspended,
    Deleted,
}

#[derive(Deserialize, sqlx::Type)]
#[sqlx(type_name = "bot_visibility")]
pub enum DbBotVisibility {
    Private,
    Public,
    PublicDiscoverable,
}

impl From<DbBotVisibility> for BotVisibility {
    fn from(value: DbBotVisibility) -> Self {
        match value {
            DbBotVisibility::Private => BotVisibility::Private,
            DbBotVisibility::Public => BotVisibility::Public {
                is_discoverable: false,
            },
            DbBotVisibility::PublicDiscoverable => BotVisibility::Public {
                is_discoverable: true,
            },
        }
    }
}

impl From<DbUser> for User {
    fn from(row: DbUser) -> Self {
        User {
            id: row.id,
            version_id: row.version_id,
            name: row.name,
            description: row.description,
            status: types::user_status::Status::offline(),
            avatar: row.avatar.map(Into::into),
            user_type: match row.r#type {
                DbUserType::Default => UserType::Default,
                DbUserType::Puppet => UserType::Puppet {
                    owner_id: row.parent_id.unwrap().into(),
                    external_platform: match row.puppet_external_platform {
                        Some(p) => match p.as_str() {
                            "discord" => ExternalPlatform::Discord,
                            _ => ExternalPlatform::Other(p),
                        },
                        None => panic!("no platform set for puppet"),
                    },
                    external_id: row.puppet_external_id.expect("no id set for puppet"),
                    external_url: row.puppet_external_url.map(|s| Url::parse(&s).unwrap()),
                    alias_id: row.puppet_alias_id.map(Into::into),
                },
                DbUserType::Bot => UserType::Bot {
                    owner: BotOwner::User {
                        user_id: row.parent_id.unwrap().into(),
                    },
                    visibility: row.bot_visibility.into(),
                    is_bridge: row.bot_is_bridge.unwrap_or(false),
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
        let user_type: DbUserType = match patch.user_type {
            UserType::Default => DbUserType::Default,
            UserType::Bot { .. } => DbUserType::Bot,
            UserType::Puppet { .. } => DbUserType::Puppet,
            UserType::System => DbUserType::System,
        };
        let row = query_as!(
            DbUser,
            r#"
            INSERT INTO usr (id, version_id, parent_id, name, description, can_fork, type, state)
            VALUES ($1, $2, $3, $4, $5, false, $6, $7)
            RETURNING
                id, version_id, parent_id, name, description, state as "state: _", type as "type: _", avatar,
                puppet_external_platform, puppet_external_id, puppet_external_url, puppet_alias_id, bot_is_bridge, bot_visibility as "bot_visibility: _"
        "#,
            user_id,
            user_id,
            patch.parent_id.map(|i| i.into_inner()),
            patch.name,
            patch.description,
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
            SELECT
                id, version_id, parent_id, name, description, state as "state: _", type as "type: _", avatar,
                puppet_external_platform, puppet_external_id, puppet_external_url, puppet_alias_id, bot_is_bridge, bot_visibility as "bot_visibility: _"
            FROM usr WHERE id = $1
            FOR UPDATE
            "#,
            user_id.into_inner()
        )
        .fetch_one(&mut *tx)
        .await?;
        let user: User = user.into();
        let version_id = UserVerId::new();
        let avatar = patch.avatar.unwrap_or(user.avatar).map(|i| i.into_inner());
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
            SELECT
                id, version_id, parent_id, name, description, state as "state: _", type as "type: _", avatar,
                puppet_external_platform, puppet_external_id, puppet_external_url, puppet_alias_id, bot_is_bridge, bot_visibility as "bot_visibility: _"
            FROM usr WHERE id = $1
        "#,
            id.into_inner()
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }
}
