use async_trait::async_trait;
use common::v1::types::{
    webhook::{Webhook, WebhookCreate, WebhookUpdate},
    ChannelId, PaginationDirection, PaginationQuery, PaginationResponse, RoomId, UserId, WebhookId,
};
use sqlx::Acquire;
use uuid::Uuid;

use crate::{
    data::{postgres::Pagination, DataWebhook},
    error::{Error, Result},
    gen_paginate,
};

use super::Postgres;

struct DbWebhook {
    id: Uuid,
    room_id: Option<Uuid>,
    channel_id: Uuid,
    creator_id: Option<Uuid>,
    name: String,
    avatar: Option<Uuid>,
    token: String,
}

#[async_trait]
impl DataWebhook for Postgres {
    async fn webhook_create(
        &self,
        channel_id: ChannelId,
        creator_id: UserId,
        create: WebhookCreate,
    ) -> Result<Webhook> {
        let mut tx = self.pool.begin().await?;

        let webhook_id = WebhookId::new();
        let version_id = webhook_id.into_inner();

        sqlx::query!(
            r#"
            INSERT INTO usr (id, version_id, parent_id, name, description, avatar, can_fork, system, registered_at)
            VALUES ($1, $2, $3, $4, $5, $6, false, false, now())
            "#,
            *webhook_id,
            version_id,
            *creator_id,
            create.name,
            Option::<String>::None,
            create.avatar.map(|i| *i)
        )
        .execute(&mut *tx)
        .await?;

        if let Some(avatar_id) = create.avatar {
            sqlx::query!(
                "INSERT INTO media_link (media_id, target_id, link_type) VALUES ($1, $2, 'AvatarUser')",
                *avatar_id,
                *webhook_id
            )
            .execute(&mut *tx)
            .await?;
        }

        let token: String = Uuid::new_v4().to_string();

        sqlx::query!(
            r#"
            INSERT INTO webhook (id, token, channel_id, creator_id)
            VALUES ($1, $2, $3, $4)
            "#,
            *webhook_id,
            token,
            *channel_id,
            *creator_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.webhook_get(webhook_id).await
    }

    async fn webhook_get(&self, webhook_id: WebhookId) -> Result<Webhook> {
        let row = sqlx::query!(
            r#"
            SELECT w.id, c.room_id, w.channel_id, u.name, u.avatar, w.token, w.creator_id
            FROM webhook w
            JOIN usr u ON w.id = u.id
            JOIN channel c ON w.channel_id = c.id
            WHERE w.id = $1
            "#,
            *webhook_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Webhook {
            id: row.id.into(),
            room_id: row.room_id.map(Into::into),
            channel_id: row.channel_id.into(),
            creator_id: row.creator_id.map(Into::into),
            name: row.name,
            avatar: row.avatar.map(Into::into),
            token: Some(row.token),
        })
    }

    async fn webhook_get_with_token(&self, webhook_id: WebhookId, token: &str) -> Result<Webhook> {
        let row = sqlx::query!(
            r#"
            SELECT w.id, c.room_id, w.channel_id, u.name, u.avatar, w.token, w.creator_id
            FROM webhook w
            JOIN usr u ON w.id = u.id
            JOIN channel c ON w.channel_id = c.id
            WHERE w.id = $1 AND w.token = $2
            "#,
            *webhook_id,
            token
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Webhook {
            id: row.id.into(),
            room_id: row.room_id.map(Into::into),
            channel_id: row.channel_id.into(),
            creator_id: row.creator_id.map(Into::into),
            name: row.name,
            avatar: row.avatar.map(Into::into),
            token: Some(row.token),
        })
    }

    async fn webhook_list_channel(
        &self,
        channel_id: ChannelId,
        pagination: PaginationQuery<WebhookId>,
    ) -> Result<PaginationResponse<Webhook>> {
        let p: Pagination<_> = pagination.try_into()?;

        gen_paginate!(
            p,
            self.pool,
            sqlx::query_as!(
                DbWebhook,
                r#"
                SELECT w.id, c.room_id, w.channel_id, u.name, u.avatar, w.token, w.creator_id
                FROM webhook w
                JOIN usr u ON w.id = u.id
                JOIN channel c ON w.channel_id = c.id
                WHERE w.channel_id = $1 AND w.id > $2 AND w.id < $3
                ORDER BY (CASE WHEN $4 = 'f' THEN w.id END), w.id DESC LIMIT $5
                "#,
                *channel_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            sqlx::query_scalar!(
                "SELECT count(*) FROM webhook WHERE channel_id = $1",
                *channel_id
            ),
            |row: DbWebhook| Webhook {
                id: row.id.into(),
                room_id: row.room_id.map(Into::into),
                channel_id: row.channel_id.into(),
                creator_id: row.creator_id.map(Into::into),
                name: row.name,
                avatar: row.avatar.map(Into::into),
                token: Some(row.token),
            },
            |i: &Webhook| i.id.to_string()
        )
    }

    async fn webhook_list_room(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<WebhookId>,
    ) -> Result<PaginationResponse<Webhook>> {
        let p: Pagination<_> = pagination.try_into()?;

        gen_paginate!(
            p,
            self.pool,
            sqlx::query_as!(
                DbWebhook,
                r#"
                SELECT w.id, c.room_id, w.channel_id, u.name, u.avatar, w.token, w.creator_id
                FROM webhook w
                JOIN usr u ON w.id = u.id
                JOIN channel c ON w.channel_id = c.id
                WHERE c.room_id = $1 AND w.id > $2 AND w.id < $3
                ORDER BY (CASE WHEN $4 = 'f' THEN w.id END), w.id DESC LIMIT $5
                "#,
                *room_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            sqlx::query_scalar!(
                "SELECT count(*) FROM webhook w JOIN channel c ON w.channel_id = c.id WHERE c.room_id = $1",
                *room_id
            ),
            |row: DbWebhook| Webhook {
                id: row.id.into(),
                room_id: row.room_id.map(Into::into),
                channel_id: row.channel_id.into(),
                creator_id: row.creator_id.map(Into::into),
                name: row.name,
                avatar: row.avatar.map(Into::into),
                token: Some(row.token),
            },
            |i: &Webhook| i.id.to_string()
        )
    }

    async fn webhook_update(&self, webhook_id: WebhookId, patch: WebhookUpdate) -> Result<Webhook> {
        let mut tx = self.pool.begin().await?;

        if let Some(name) = patch.name {
            sqlx::query!("UPDATE usr SET name = $1 WHERE id = $2", name, *webhook_id)
                .execute(&mut *tx)
                .await?;
        }
        if let Some(avatar) = patch.avatar {
            sqlx::query!(
                "DELETE FROM media_link WHERE target_id = $1 AND link_type = 'AvatarUser'",
                *webhook_id
            )
            .execute(&mut *tx)
            .await?;
            if let Some(avatar_id) = avatar {
                sqlx::query!("INSERT INTO media_link (media_id, target_id, link_type) VALUES ($1, $2, 'AvatarUser')", *avatar_id, *webhook_id)
                    .execute(&mut *tx).await?;
            }
            sqlx::query!(
                "UPDATE usr SET avatar = $1 WHERE id = $2",
                avatar.map(|i| *i),
                *webhook_id
            )
            .execute(&mut *tx)
            .await?;
        }
        if let Some(channel_id) = patch.channel_id {
            sqlx::query!(
                "UPDATE webhook SET channel_id = $1 WHERE id = $2",
                *channel_id,
                *webhook_id
            )
            .execute(&mut *tx)
            .await?;
        }
        if patch.rotate_token {
            let token: String = Uuid::new_v4().to_string();
            sqlx::query!(
                "UPDATE webhook SET token = $1 WHERE id = $2",
                token,
                *webhook_id
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.webhook_get(webhook_id).await
    }

    async fn webhook_update_with_token(
        &self,
        webhook_id: WebhookId,
        token: &str,
        patch: WebhookUpdate,
    ) -> Result<Webhook> {
        let mut tx = self.pool.begin().await?;

        let res = sqlx::query!(
            "SELECT w.id, c.room_id, w.channel_id, u.name, u.avatar, w.token, w.creator_id
             FROM webhook w
             JOIN usr u ON w.id = u.id
             JOIN channel c ON w.channel_id = c.id
             WHERE w.id = $1 AND w.token = $2",
            *webhook_id,
            token
        )
        .fetch_optional(&mut *tx)
        .await?;

        let original = match res {
            Some(row) => row,
            None => return Err(Error::NotFound),
        };

        if let Some(name) = &patch.name {
            sqlx::query!("UPDATE usr SET name = $1 WHERE id = $2", name, *webhook_id)
                .execute(&mut *tx)
                .await?;
        }
        if let Some(avatar) = patch.avatar {
            sqlx::query!(
                "DELETE FROM media_link WHERE target_id = $1 AND link_type = 'AvatarUser'",
                *webhook_id
            )
            .execute(&mut *tx)
            .await?;
            if let Some(avatar_id) = avatar {
                sqlx::query!("INSERT INTO media_link (media_id, target_id, link_type) VALUES ($1, $2, 'AvatarUser')", *avatar_id, *webhook_id)
                    .execute(&mut *tx).await?;
            }
            sqlx::query!(
                "UPDATE usr SET avatar = $1 WHERE id = $2",
                avatar.map(|i| *i),
                *webhook_id
            )
            .execute(&mut *tx)
            .await?;
        }
        if let Some(channel_id) = patch.channel_id {
            sqlx::query!(
                "UPDATE webhook SET channel_id = $1 WHERE id = $2",
                *channel_id,
                *webhook_id
            )
            .execute(&mut *tx)
            .await?;
        }
        let new_token = if patch.rotate_token {
            let new_token: String = Uuid::new_v4().to_string();
            sqlx::query!(
                "UPDATE webhook SET token = $1 WHERE id = $2",
                new_token,
                *webhook_id
            )
            .execute(&mut *tx)
            .await?;
            Some(new_token)
        } else {
            Some(original.token.clone())
        };

        tx.commit().await?;

        Ok(Webhook {
            id: original.id.into(),
            room_id: original.room_id.map(Into::into),
            channel_id: original.channel_id.into(),
            creator_id: original.creator_id.map(Into::into),
            name: if patch.name.is_some() {
                patch.name.unwrap()
            } else {
                original.name
            },
            avatar: if patch.avatar.is_some() {
                patch.avatar.unwrap()
            } else {
                original.avatar.map(Into::into)
            },
            token: new_token,
        })
    }

    async fn webhook_delete(&self, webhook_id: WebhookId) -> Result<()> {
        sqlx::query!("DELETE FROM usr WHERE id = $1", *webhook_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn webhook_delete_with_token(&self, webhook_id: WebhookId, token: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let res = sqlx::query!(
            "SELECT id FROM webhook WHERE id = $1 AND token = $2",
            *webhook_id,
            token
        )
        .fetch_optional(&mut *tx)
        .await?;
        if res.is_none() {
            return Err(Error::NotFound);
        }
        sqlx::query!("DELETE FROM usr WHERE id = $1", *webhook_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
