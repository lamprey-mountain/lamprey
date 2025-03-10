use async_trait::async_trait;
use common::v1::types::util::Time;
use common::v1::types::{self, BotOwner, BotVisibility, ExternalPlatform, UserState, UserType};
use serde::Deserialize;
use sqlx::{query, query_as, Acquire};
use time::PrimitiveDateTime;
use url::Url;
use uuid::Uuid;

use crate::error::Result;
use crate::types::{DbUserCreate, User, UserId, UserPatch, UserVerId};

use crate::data::DataUser;

use super::Postgres;

#[derive(Deserialize)]
pub struct DbUserBase<T> {
    pub id: UserId,
    pub version_id: UserVerId,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub avatar: Option<Uuid>,
    pub r#type: DbUserType,
    pub state: DbUserState,
    pub state_updated_at: T,
    pub puppet_external_platform: Option<String>,
    pub puppet_external_id: Option<String>,
    pub puppet_external_url: Option<String>,
    pub puppet_alias_id: Option<Uuid>,
    pub bot_is_bridge: bool,
    pub bot_visibility: DbBotVisibility,
}

type DbUser = DbUserBase<PrimitiveDateTime>;

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
#[sqlx(type_name = "bot_visibility_type")]
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

impl From<BotVisibility> for DbBotVisibility {
    fn from(value: BotVisibility) -> Self {
        match value {
            BotVisibility::Private => DbBotVisibility::Private,
            BotVisibility::Public { is_discoverable } => {
                if is_discoverable {
                    DbBotVisibility::Public
                } else {
                    DbBotVisibility::PublicDiscoverable
                }
            }
        }
    }
}

impl From<DbUserState> for UserState {
    fn from(value: DbUserState) -> Self {
        match value {
            DbUserState::Active => UserState::Active,
            DbUserState::Suspended => UserState::Suspended,
            DbUserState::Deleted => UserState::Deleted,
        }
    }
}

impl From<UserState> for DbUserState {
    fn from(value: UserState) -> Self {
        match value {
            UserState::Active => DbUserState::Active,
            UserState::Suspended => DbUserState::Suspended,
            UserState::Deleted => DbUserState::Deleted,
        }
    }
}

impl<T: Into<Time>> From<DbUserBase<T>> for User {
    fn from(row: DbUserBase<T>) -> Self {
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
                    is_bridge: row.bot_is_bridge,
                },
                DbUserType::System => UserType::System,
            },
            state: row.state.into(),
            state_updated_at: row.state_updated_at.into(),
        }
    }
}

impl From<User> for DbUser {
    fn from(u: User) -> Self {
        let base = DbUser {
            id: u.id,
            version_id: u.version_id,
            parent_id: None,
            name: u.name,
            description: u.description,
            avatar: u.avatar.map(|i| i.into_inner()),
            r#type: match &u.user_type {
                UserType::Default => DbUserType::Default,
                UserType::Bot { .. } => DbUserType::Bot,
                UserType::Puppet { .. } => DbUserType::Puppet,
                UserType::System => DbUserType::System,
            },
            state: u.state.into(),
            state_updated_at: {
                let ts = u.state_updated_at.into_inner();
                PrimitiveDateTime::new(ts.date(), ts.time())
            },
            puppet_external_platform: None,
            puppet_external_id: None,
            puppet_external_url: None,
            puppet_alias_id: None,
            bot_is_bridge: false,
            bot_visibility: DbBotVisibility::Private,
        };
        match u.user_type {
            UserType::Bot {
                owner,
                visibility,
                is_bridge,
            } => DbUser {
                parent_id: match owner {
                    BotOwner::User { user_id } => Some(user_id.into_inner()),
                    _ => todo!(),
                },
                bot_visibility: visibility.into(),
                bot_is_bridge: is_bridge,
                ..base
            },
            UserType::Puppet {
                owner_id,
                external_platform,
                external_id,
                external_url,
                alias_id,
            } => DbUser {
                parent_id: Some(owner_id.into_inner()),
                puppet_external_id: Some(external_id),
                puppet_external_url: external_url.map(|u| u.to_string()),
                puppet_external_platform: Some(match external_platform {
                    ExternalPlatform::Discord => "Discord".to_string(),
                    ExternalPlatform::Other(p) => p,
                }),
                puppet_alias_id: alias_id.map(Into::into),
                ..base
            },
            _ => base,
        }
    }
}

#[async_trait]
impl DataUser for Postgres {
    async fn user_create(&self, patch: DbUserCreate) -> Result<User> {
        let user_id = Uuid::now_v7();
        let user: DbUser = User {
            id: user_id.into(),
            version_id: user_id.into(),
            name: patch.name,
            description: patch.description,
            avatar: None,
            user_type: patch.user_type,
            state: UserState::Active,
            state_updated_at: Time::now_utc(),
            status: types::user_status::Status::online(),
        }
        .into();
        let row = query_as!(
            DbUser,
            r#"
            INSERT INTO usr (
                id, version_id, parent_id, name, description, state, state_updated_at, type, avatar,
                puppet_external_platform, puppet_external_id, puppet_external_url, puppet_alias_id, bot_is_bridge, bot_visibility
            )
    	    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING
                id, version_id, parent_id, name, description, state as "state: _", state_updated_at, type as "type: _", avatar,
                puppet_external_platform, puppet_external_id, puppet_external_url, puppet_alias_id, bot_is_bridge, bot_visibility as "bot_visibility: _"
        "#,
            user.id.into_inner(),
            user.version_id.into_inner(),
            user.parent_id,
            user.name,
            user.description,
            user.state as _,
            user.state_updated_at,
            user.r#type as _,
            user.avatar,
            user.puppet_external_platform,
            user.puppet_external_id,
            user.puppet_external_url,
            user.puppet_alias_id,
            user.bot_is_bridge,
            user.bot_visibility as _,
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
                id, version_id, parent_id, name, description, state as "state: _", state_updated_at, type as "type: _", avatar,
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
                id, version_id, parent_id, name, description, state as "state: _", state_updated_at, type as "type: _", avatar,
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
