use async_trait::async_trait;
use common::v1::types::{self};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{query, query_as, query_scalar, Acquire};
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
    pub puppet: Option<Value>,
    pub bot: Option<Value>,
    pub system: bool,
    pub guest: Option<Value>,
    pub suspended: Option<Value>,
    pub registered_at: Option<time::PrimitiveDateTime>,
    pub deleted_at: Option<time::PrimitiveDateTime>,
}

// #[derive(Deserialize, sqlx::Type)]
// #[sqlx(type_name = "bot_access_type")]
// pub enum DbBotAccess {
//     Private,
//     Public,
//     PublicDiscoverable,
// }

// impl From<DbBotAccess> for BotAccess {
//     fn from(value: DbBotAccess) -> Self {
//         match value {
//             DbBotAccess::Private => BotAccess::Private,
//             DbBotAccess::Public => BotAccess::Public {
//                 is_discoverable: false,
//             },
//             DbBotAccess::PublicDiscoverable => BotAccess::Public {
//                 is_discoverable: true,
//             },
//         }
//     }
// }

// impl From<BotAccess> for DbBotAccess {
//     fn from(value: BotAccess) -> Self {
//         match value {
//             BotAccess::Private => DbBotAccess::Private,
//             BotAccess::Public { is_discoverable } => {
//                 if is_discoverable {
//                     DbBotAccess::Public
//                 } else {
//                     DbBotAccess::PublicDiscoverable
//                 }
//             }
//         }
//     }
// }

// impl From<DbUserState> for UserState {
//     fn from(value: DbUserState) -> Self {
//         match value {
//             DbUserState::Active => UserState::Active,
//             DbUserState::Suspended => UserState::Suspended,
//             DbUserState::Deleted => UserState::Deleted,
//         }
//     }
// }

// impl From<UserState> for DbUserState {
//     fn from(value: UserState) -> Self {
//         match value {
//             UserState::Active => DbUserState::Active,
//             UserState::Suspended => DbUserState::Suspended,
//             UserState::Deleted => DbUserState::Deleted,
//         }
//     }
// }

impl From<DbUser> for User {
    fn from(row: DbUser) -> Self {
        User {
            id: row.id,
            version_id: row.version_id,
            name: row.name,
            description: row.description,
            status: types::user_status::Status::offline(),
            avatar: row.avatar.map(Into::into),
            bot: row.bot.and_then(|r| serde_json::from_value(dbg!(r)).ok()),
            puppet: row.puppet.and_then(|r| serde_json::from_value(r).ok()),
            guest: row.guest.and_then(|r| serde_json::from_value(r).ok()),
            suspended: row
                .suspended
                .and_then(|r| serde_json::from_value(r).unwrap()),
            system: row.system,
            registered_at: row.registered_at.map(|i| i.into()),
            deleted_at: row.deleted_at.map(|i| i.into()),
        }
    }
}

#[async_trait]
impl DataUser for Postgres {
    async fn user_create(&self, patch: DbUserCreate) -> Result<User> {
        let user_id = Uuid::now_v7();
        let user = User {
            id: user_id.into(),
            version_id: user_id.into(),
            name: patch.name,
            description: patch.description,
            avatar: None,
            status: types::user_status::Status::online(),
            bot: patch.bot,
            system: false, // TODO: system users/messages
            puppet: patch.puppet,
            guest: None,         // TODO: guest users
            suspended: None,     // TODO: account suspension
            registered_at: None, // FIXME
            deleted_at: None,
        };
        query!(
            r#"
            INSERT INTO usr (id, version_id, parent_id, name, description, avatar, puppet, bot, suspended, can_fork)
    	    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, false)
        "#,
            *user.id,
            *user.version_id,
            patch.parent_id.map(|i| *i),
            user.name,
            user.description,
            user.avatar.map(|i| *i),
            serde_json::to_value(user.puppet)?,
            serde_json::to_value(user.bot)?,
            serde_json::to_value(user.suspended)?,
        )
        .execute(&self.pool)
        .await?;
        self.user_get(user_id.into()).await
    }

    async fn user_update(&self, user_id: UserId, patch: UserPatch) -> Result<UserVerId> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let user = query_as!(
            DbUser,
            r#"
            SELECT id, version_id, parent_id, name, description, avatar, puppet, bot, system, guest, registered_at, deleted_at, suspended
            FROM usr WHERE id = $1
            FOR UPDATE
            "#,
            *user_id
        )
        .fetch_one(&mut *tx)
        .await?;
        let user: User = user.into();
        let version_id = UserVerId::new();
        let avatar = patch.avatar.unwrap_or(user.avatar).map(|i| i.into_inner());
        query!(
            "UPDATE usr SET version_id = $2, name = $3, description = $4, avatar = $5 WHERE id = $1",
            *user_id,
            *version_id,
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
            "UPDATE usr SET deleted_at = $2 WHERE id = $1",
            *user_id,
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
            SELECT id, version_id, parent_id, name, description, avatar, puppet, bot, system, guest, registered_at, deleted_at, suspended
            FROM usr WHERE id = $1
        "#,
            id.into_inner()
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn user_lookup_puppet(
        &self,
        owner_id: UserId,
        external_id: &str,
    ) -> Result<Option<UserId>> {
        let id = query_scalar!(
            r#"
            SELECT id FROM usr
            WHERE parent_id = $1 AND puppet->>'external_id' = $2
            "#,
            owner_id.into_inner(),
            external_id,
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(id.map(Into::into))
    }
}
