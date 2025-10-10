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
    pub puppet: Option<Value>,
    pub bot: Option<Value>,
    pub system: bool,
    pub suspended: Option<Value>,
    pub registered_at: Option<time::PrimitiveDateTime>,
    pub deleted_at: Option<time::PrimitiveDateTime>,
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
            bot: row.bot.and_then(|r| serde_json::from_value(r).ok()),
            puppet: row.puppet.and_then(|r| serde_json::from_value(r).ok()),
            suspended: row
                .suspended
                .and_then(|r| serde_json::from_value(r).unwrap()),
            system: row.system,
            registered_at: row.registered_at.map(|i| i.into()),
            deleted_at: row.deleted_at.map(|i| i.into()),
            user_config: None,
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
            status: types::user_status::Status::online(),
            bot: patch.bot,
            system: patch.system,
            puppet: patch.puppet,
            suspended: None,
            registered_at: patch.registered_at,
            deleted_at: None,
            user_config: Default::default(),
        };
        query!(
            r#"
            INSERT INTO usr (id, version_id, parent_id, name, description, avatar, puppet, bot, suspended, can_fork, registered_at)
    	    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, false, $10)
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
            user.registered_at.map(|t| time::PrimitiveDateTime::from(t)),
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
            SELECT id, version_id, parent_id, name, description, avatar, puppet, bot, system, registered_at, deleted_at, suspended
            FROM usr WHERE id = $1
            FOR UPDATE
            "#,
            *user_id
        )
        .fetch_one(&mut *tx)
        .await?;
        let user: User = user.into();
        let version_id = UserVerId::new();
        let avatar = patch.avatar.unwrap_or(user.avatar).map(|i| *i);
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
            SELECT id, version_id, parent_id, name, description, avatar, puppet, bot, system, registered_at, deleted_at, suspended
            FROM usr WHERE id = $1
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
                        SELECT id, version_id, parent_id, name, description, avatar, puppet, bot, system, registered_at, deleted_at, suspended
                        FROM usr
                        WHERE id > $1 AND id < $2
                          AND registered_at IS NULL AND bot->'access' IS NULL AND puppet->'owner_id' IS NULL AND system = false
                        ORDER BY (CASE WHEN $3 = 'f' THEN id END), id DESC LIMIT $4
                        "#,
                        *p.after,
                        *p.before,
                        p.dir.to_string(),
                        (p.limit + 1) as i32
                    ),
                    query_scalar!("SELECT count(*) FROM usr WHERE registered_at IS NULL AND bot->'access' IS NULL AND puppet->'owner_id' IS NULL AND system = false"),
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
                        SELECT id, version_id, parent_id, name, description, avatar, puppet, bot, system, registered_at, deleted_at, suspended
                        FROM usr
                        WHERE id > $1 AND id < $2
                          AND registered_at IS NOT NULL AND bot->'access' IS NULL AND puppet->'owner_id' IS NULL AND system = false
                        ORDER BY (CASE WHEN $3 = 'f' THEN id END), id DESC LIMIT $4
                        "#,
                        *p.after,
                        *p.before,
                        p.dir.to_string(),
                        (p.limit + 1) as i32
                    ),
                    query_scalar!("SELECT count(*) FROM usr WHERE registered_at IS NOT NULL AND bot->'access' IS NULL AND puppet->'owner_id' IS NULL AND system = false"),
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
                        SELECT id, version_id, parent_id, name, description, avatar, puppet, bot, system, registered_at, deleted_at, suspended
                        FROM usr
                        WHERE id > $1 AND id < $2
                          AND bot->'access' IS NOT NULL
                        ORDER BY (CASE WHEN $3 = 'f' THEN id END), id DESC LIMIT $4
                        "#,
                        *p.after,
                        *p.before,
                        p.dir.to_string(),
                        (p.limit + 1) as i32
                    ),
                    query_scalar!("SELECT count(*) FROM usr WHERE bot->'access' IS NOT NULL"),
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
                        SELECT id, version_id, parent_id, name, description, avatar, puppet, bot, system, registered_at, deleted_at, suspended
                        FROM usr
                        WHERE id > $1 AND id < $2
                          AND puppet->'owner_id' IS NOT NULL
                        ORDER BY (CASE WHEN $3 = 'f' THEN id END), id DESC LIMIT $4
                        "#,
                        *p.after,
                        *p.before,
                        p.dir.to_string(),
                        (p.limit + 1) as i32
                    ),
                    query_scalar!("SELECT count(*) FROM usr WHERE puppet->'owner_id' IS NOT NULL"),
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
                        SELECT id, version_id, parent_id, name, description, avatar, puppet, bot, system, registered_at, deleted_at, suspended
                        FROM usr
                        WHERE id > $1 AND id < $2
                          AND bot->'access' IS NULL AND puppet->'owner_id' IS NULL AND system = false
                        ORDER BY (CASE WHEN $3 = 'f' THEN id END), id DESC LIMIT $4
                        "#,
                        *p.after,
                        *p.before,
                        p.dir.to_string(),
                        (p.limit + 1) as i32
                    ),
                    query_scalar!("SELECT count(*) FROM usr WHERE bot->'access' IS NULL AND puppet->'owner_id' IS NULL AND system = false"),
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
            SELECT id FROM usr
            WHERE parent_id = $1 AND puppet->>'external_id' = $2
            "#,
            *owner_id,
            external_id,
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(id.map(Into::into))
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
