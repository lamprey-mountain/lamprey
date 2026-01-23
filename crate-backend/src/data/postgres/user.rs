use async_trait::async_trait;
use common::v1::types::util::Time;
use common::v1::types::{
    self, PaginationDirection, PaginationQuery, PaginationResponse, Suspended, User, UserId,
    UserListFilter,
};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{query, query_as, query_scalar, Acquire};
use uuid::Uuid;

use crate::data::postgres::Pagination;
use crate::data::DataUser;
use crate::error::Result;
use crate::gen_paginate;
use crate::types::{DbUserCreate, UserPatch, UserVerId};

use super::Postgres;

#[derive(Deserialize)]
pub struct DbUser {
    pub id: UserId,
    pub version_id: UserVerId,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub avatar: Option<Uuid>,
    pub banner: Option<Uuid>,
    pub bot: bool,
    pub system: bool,
    pub suspended: Option<Value>,
    pub registered_at: Option<time::PrimitiveDateTime>,
    pub deleted_at: Option<time::PrimitiveDateTime>,
    pub webhook_channel_id: Option<Uuid>,
    pub webhook_creator_id: Option<Uuid>,
    pub webhook_room_id: Option<Uuid>,
    pub puppet_application_id: Option<Uuid>,
    pub puppet_external_id: Option<String>,
    pub puppet_external_url: Option<String>,
    pub puppet_alias_id: Option<Uuid>,
}

impl From<DbUser> for User {
    fn from(row: DbUser) -> Self {
        let webhook = if let (Some(channel_id), Some(creator_id)) =
            (row.webhook_channel_id, row.webhook_creator_id)
        {
            Some(common::v1::types::UserWebhook {
                room_id: row.webhook_room_id.map(Into::into),
                channel_id: channel_id.into(),
                creator_id: creator_id.into(),
            })
        } else {
            None
        };

        let puppet = if let (Some(application_id), Some(external_id)) =
            (row.puppet_application_id, row.puppet_external_id)
        {
            Some(types::Puppet {
                owner_id: application_id.into(),
                external_id,
                external_url: row.puppet_external_url.and_then(|u| u.parse().ok()),
                alias_id: row.puppet_alias_id.map(Into::into),
            })
        } else {
            None
        };

        User {
            id: row.id,
            version_id: row.version_id,
            name: row.name,
            description: row.description,
            presence: types::presence::Presence::offline(),
            avatar: row.avatar.map(Into::into),
            banner: row.banner.map(Into::into),
            bot: row.bot,
            puppet,
            webhook,
            suspended: row
                .suspended
                .and_then(|r| serde_json::from_value(r).unwrap()),
            system: row.system,
            registered_at: row.registered_at.map(|i| i.into()),
            deleted_at: row.deleted_at.map(|i| i.into()),
            emails: None,
            user_config: None,
            has_mfa: None,
        }
    }
}

#[async_trait]
impl DataUser for Postgres {
    async fn user_create(&self, patch: DbUserCreate) -> Result<User> {
        let user_id = patch.id.unwrap_or_else(|| Uuid::now_v7().into());
        let user = User {
            id: user_id,
            version_id: user_id.into_inner().into(),
            name: patch.name,
            description: patch.description,
            avatar: None,
            banner: None,
            presence: types::presence::Presence::online(),
            bot: false,
            system: patch.system,
            puppet: patch.puppet,
            webhook: None,
            suspended: None,
            registered_at: patch.registered_at,
            deleted_at: None,
            user_config: Default::default(),
            emails: None,
            has_mfa: None,
        };

        let mut tx = self.pool.begin().await?;

        query!(
            r#"
            INSERT INTO usr (id, version_id, parent_id, name, description, avatar, suspended, can_fork, registered_at, system, bot)
    	    VALUES ($1, $2, $3, $4, $5, $6, $7, false, $8, $9, false)
        "#,
            *user.id,
            *user.version_id,
            patch.parent_id.map(|i| *i),
            user.name,
            user.description,
            user.avatar.map(|i| *i),
            serde_json::to_value(user.suspended)?,
            user.registered_at.map(|t| time::PrimitiveDateTime::from(t)),
            user.system,
        )
        .execute(&mut *tx)
        .await?;

        if let Some(puppet) = &user.puppet {
            query!(
                r#"
                INSERT INTO puppet (id, application_id, external_id, external_url, alias_id)
                VALUES ($1, $2, $3, $4, $5)
                "#,
                *user.id,
                *puppet.owner_id,
                puppet.external_id,
                puppet.external_url.as_ref().map(|u| u.as_str()),
                puppet.alias_id.map(|id| *id),
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        self.user_get(user_id).await
    }

    async fn user_update(&self, user_id: UserId, patch: UserPatch) -> Result<UserVerId> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let user = query_as!(
            DbUser,
            r#"
            SELECT u.id, u.version_id, u.parent_id, u.name, u.description, u.avatar, u.banner, u.system, u.bot, u.registered_at, u.deleted_at, u.suspended,
                   w.channel_id as "webhook_channel_id?", w.creator_id as "webhook_creator_id?", c.room_id as "webhook_room_id?",
                   p.application_id as "puppet_application_id?", p.external_id as "puppet_external_id?", p.external_url as "puppet_external_url?", p.alias_id as "puppet_alias_id?"
            FROM usr u
            LEFT JOIN webhook w ON u.id = w.id
            LEFT JOIN channel c ON w.channel_id = c.id
            LEFT JOIN puppet p ON u.id = p.id
            WHERE u.id = $1
            FOR UPDATE OF u
            "#,
            *user_id
        )
        .fetch_one(&mut *tx)
        .await?;
        let user: User = user.into();
        let version_id = UserVerId::new();
        let avatar = patch.avatar.unwrap_or(user.avatar).map(|i| *i);
        let banner = patch.banner.unwrap_or(user.banner).map(|i| *i);
        query!(
            "UPDATE usr SET version_id = $2, name = $3, description = $4, avatar = $5, banner = $6 WHERE id = $1",
            *user_id,
            *version_id,
            patch.name.unwrap_or(user.name),
            patch.description.unwrap_or(user.description),
            avatar,
            banner,
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

    async fn user_undelete(&self, user_id: UserId) -> Result<()> {
        query!("UPDATE usr SET deleted_at = NULL WHERE id = $1", *user_id,)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn user_get(&self, id: UserId) -> Result<User> {
        let row = query_as!(
            DbUser,
            r#"
            SELECT u.id, u.version_id, u.parent_id, u.name, u.description, u.avatar, u.banner, u.system, u.bot, u.registered_at, u.deleted_at, u.suspended,
                   w.channel_id as "webhook_channel_id?", w.creator_id as "webhook_creator_id?", c.room_id as "webhook_room_id?",
                   p.application_id as "puppet_application_id?", p.external_id as "puppet_external_id?", p.external_url as "puppet_external_url?", p.alias_id as "puppet_alias_id?"
            FROM usr u
            LEFT JOIN webhook w ON u.id = w.id
            LEFT JOIN channel c ON w.channel_id = c.id
            LEFT JOIN puppet p ON u.id = p.id
            WHERE u.id = $1
            "#,
            *id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn user_list(
        &self,
        pagination: PaginationQuery<UserId>,
        filter: Option<UserListFilter>,
    ) -> Result<PaginationResponse<User>> {
        let p: Pagination<_> = pagination.try_into()?;
        match filter {
            Some(UserListFilter::Guest) => {
                gen_paginate!(
                    p,
                    self.pool,
                    query_as!(
                        DbUser,
                        r#"
                        SELECT u.id, u.version_id, u.parent_id, u.name, u.description, u.avatar, u.banner, u.system, u.bot, u.registered_at, u.deleted_at, u.suspended,
                               w.channel_id as "webhook_channel_id?", w.creator_id as "webhook_creator_id?", c.room_id as "webhook_room_id?",
                               p.application_id as "puppet_application_id?", p.external_id as "puppet_external_id?", p.external_url as "puppet_external_url?", p.alias_id as "puppet_alias_id?"
                        FROM usr u
                        LEFT JOIN webhook w ON u.id = w.id
                        LEFT JOIN channel c ON w.channel_id = c.id
                        LEFT JOIN puppet p ON u.id = p.id
                        WHERE u.id > $1 AND u.id < $2
                          AND u.registered_at IS NULL AND u.bot = false AND p.id IS NULL AND u.system = false
                        ORDER BY (CASE WHEN $3 = 'f' THEN u.id END), u.id DESC LIMIT $4
                        "#,
                        *p.after,
                        *p.before,
                        p.dir.to_string(),
                        (p.limit + 1) as i32
                    ),
                    query_scalar!("SELECT count(*) FROM usr u LEFT JOIN puppet p ON u.id = p.id WHERE u.registered_at IS NULL AND u.bot = false AND p.id IS NULL AND u.system = false"),
                    |i: &User| i.id.to_string()
                )
            }
            Some(UserListFilter::Registered) => {
                gen_paginate!(
                    p,
                    self.pool,
                    query_as!(
                        DbUser,
                        r#"
                        SELECT u.id, u.version_id, u.parent_id, u.name, u.description, u.avatar, u.banner, u.system, u.bot, u.registered_at, u.deleted_at, u.suspended,
                               w.channel_id as "webhook_channel_id?", w.creator_id as "webhook_creator_id?", c.room_id as "webhook_room_id?",
                               p.application_id as "puppet_application_id?", p.external_id as "puppet_external_id?", p.external_url as "puppet_external_url?", p.alias_id as "puppet_alias_id?"
                        FROM usr u
                        LEFT JOIN webhook w ON u.id = w.id
                        LEFT JOIN channel c ON w.channel_id = c.id
                        LEFT JOIN puppet p ON u.id = p.id
                        WHERE u.id > $1 AND u.id < $2
                          AND u.registered_at IS NOT NULL AND u.bot = false AND p.id IS NULL AND u.system = false
                        ORDER BY (CASE WHEN $3 = 'f' THEN u.id END), u.id DESC LIMIT $4
                        "#,
                        *p.after,
                        *p.before,
                        p.dir.to_string(),
                        (p.limit + 1) as i32
                    ),
                    query_scalar!("SELECT count(*) FROM usr u LEFT JOIN puppet p ON u.id = p.id WHERE u.registered_at IS NOT NULL AND u.bot = false AND p.id IS NULL AND u.system = false"),
                    |i: &User| i.id.to_string()
                )
            }
            Some(UserListFilter::Bot) => {
                gen_paginate!(
                    p,
                    self.pool,
                    query_as!(
                        DbUser,
                        r#"
                        SELECT u.id, u.version_id, u.parent_id, u.name, u.description, u.avatar, u.banner, u.system, u.bot, u.registered_at, u.deleted_at, u.suspended,
                               w.channel_id as "webhook_channel_id?", w.creator_id as "webhook_creator_id?", c.room_id as "webhook_room_id?",
                               p.application_id as "puppet_application_id?", p.external_id as "puppet_external_id?", p.external_url as "puppet_external_url?", p.alias_id as "puppet_alias_id?"
                        FROM usr u
                        LEFT JOIN webhook w ON u.id = w.id
                        LEFT JOIN channel c ON w.channel_id = c.id
                        LEFT JOIN puppet p ON u.id = p.id
                        WHERE u.id > $1 AND u.id < $2
                        AND u.bot = true
                        ORDER BY (CASE WHEN $3 = 'f' THEN u.id END), u.id DESC LIMIT $4
                        "#,
                        *p.after,
                        *p.before,
                        p.dir.to_string(),
                        (p.limit + 1) as i32
                    ),
                    query_scalar!("SELECT count(*) FROM usr u WHERE u.bot = true"),
                    |i: &User| i.id.to_string()
                )
            }
            Some(UserListFilter::Puppet) => {
                gen_paginate!(
                    p,
                    self.pool,
                    query_as!(
                        DbUser,
                        r#"
                        SELECT u.id, u.version_id, u.parent_id, u.name, u.description, u.avatar, u.banner, u.system, u.bot, u.registered_at, u.deleted_at, u.suspended,
                               w.channel_id as "webhook_channel_id?", w.creator_id as "webhook_creator_id?", c.room_id as "webhook_room_id?",
                               p.application_id as "puppet_application_id?", p.external_id as "puppet_external_id?", p.external_url as "puppet_external_url?", p.alias_id as "puppet_alias_id?"
                        FROM usr u
                        LEFT JOIN webhook w ON u.id = w.id
                        LEFT JOIN channel c ON w.channel_id = c.id
                        JOIN puppet p ON u.id = p.id
                        WHERE u.id > $1 AND u.id < $2
                        ORDER BY (CASE WHEN $3 = 'f' THEN u.id END), u.id DESC LIMIT $4
                        "#,
                        *p.after,
                        *p.before,
                        p.dir.to_string(),
                        (p.limit + 1) as i32
                    ),
                    query_scalar!("SELECT count(*) FROM usr u JOIN puppet p ON u.id = p.id"),
                    |i: &User| i.id.to_string()
                )
            }
            None => {
                gen_paginate!(
                    p,
                    self.pool,
                    query_as!(
                        DbUser,
                        r#"
                        SELECT u.id, u.version_id, u.parent_id, u.name, u.description, u.avatar, u.banner, u.system, u.bot, u.registered_at, u.deleted_at, u.suspended,
                               w.channel_id as "webhook_channel_id?", w.creator_id as "webhook_creator_id?", c.room_id as "webhook_room_id?",
                               p.application_id as "puppet_application_id?", p.external_id as "puppet_external_id?", p.external_url as "puppet_external_url?", p.alias_id as "puppet_alias_id?"
                        FROM usr u
                        LEFT JOIN webhook w ON u.id = w.id
                        LEFT JOIN channel c ON w.channel_id = c.id
                        LEFT JOIN puppet p ON u.id = p.id
                        WHERE u.id > $1 AND u.id < $2
                          AND u.bot = false AND p.id IS NULL AND u.system = false
                        ORDER BY (CASE WHEN $3 = 'f' THEN u.id END), u.id DESC LIMIT $4
                        "#,
                        *p.after,
                        *p.before,
                        p.dir.to_string(),
                        (p.limit + 1) as i32
                    ),
                    query_scalar!("SELECT count(*) FROM usr u LEFT JOIN puppet p ON u.id = p.id WHERE u.bot = false AND p.id IS NULL AND u.system = false"),
                    |i: &User| i.id.to_string()
                )
            }
        }
    }

    async fn user_lookup_puppet(
        &self,
        owner_id: UserId,
        external_id: &str,
    ) -> Result<Option<UserId>> {
        let id = query_scalar!(
            r#"
            SELECT p.id FROM puppet p
            WHERE p.application_id = $1 AND p.external_id = $2
            "#,
            *owner_id,
            external_id,
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(id.map(Into::into))
    }

    async fn user_get_many(&self, user_ids: &[UserId]) -> Result<Vec<User>> {
        let ids: Vec<Uuid> = user_ids.iter().map(|id| id.into_inner()).collect();
        let rows = query_as!(
            DbUser,
            r#"
            SELECT u.id, u.version_id, u.parent_id, u.name, u.description, u.avatar, u.banner, u.system, u.bot, u.registered_at, u.deleted_at, u.suspended,
                   w.channel_id as "webhook_channel_id?", w.creator_id as "webhook_creator_id?", c.room_id as "webhook_room_id?",
                   p.application_id as "puppet_application_id?", p.external_id as "puppet_external_id?", p.external_url as "puppet_external_url?", p.alias_id as "puppet_alias_id?"
            FROM usr u
            LEFT JOIN webhook w ON u.id = w.id
            LEFT JOIN channel c ON w.channel_id = c.id
            LEFT JOIN puppet p ON u.id = p.id
            WHERE u.id = ANY($1)
            "#,
            &ids
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn user_set_registered(
        &self,
        user_id: UserId,
        registered_at: Option<Time>,
        parent_invite: Option<String>,
    ) -> Result<UserVerId> {
        let version_id = UserVerId::new();
        query!(
            "UPDATE usr SET version_id = $2, registered_at = $3, parent_invite = $4 WHERE id = $1",
            *user_id,
            *version_id,
            registered_at.map(|t| time::PrimitiveDateTime::from(t)),
            parent_invite,
        )
        .execute(&self.pool)
        .await?;
        Ok(version_id)
    }

    async fn user_suspended(
        &self,
        user_id: UserId,
        suspended: Option<Suspended>,
    ) -> Result<UserVerId> {
        let version_id = UserVerId::new();
        query!(
            "UPDATE usr SET version_id = $2, suspended = $3 WHERE id = $1",
            *user_id,
            *version_id,
            suspended.map(|t| serde_json::to_value(t).unwrap()),
        )
        .execute(&self.pool)
        .await?;
        Ok(version_id)
    }
}
